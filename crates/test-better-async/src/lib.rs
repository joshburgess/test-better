//! `test-better-async`: async and timing helpers.
//!
//! Home of the runtime-agnostic timeout abstraction that backs
//! `expect!(fut).to_complete_within(..)` (Phase 5).
//!
//! # The runtime gate
//!
//! Timing out a future needs a runtime-provided sleep. The runtime is chosen
//! at compile time by the mutually-exclusive `tokio`, `async-std`, and `smol`
//! cargo features (in that priority order if more than one is on, which only
//! happens under `--all-features`). One private function, `selected_sleep`,
//! is the single place that `cfg`-branches on them.
//!
//! If *no* runtime feature is enabled, the crate still compiles: the
//! [`RuntimeAvailable`] marker trait simply has no implementation. Because
//! [`run_within`] is bounded `where F: RuntimeAvailable` on its *generic*
//! future type, that bound is deferred to the call site (a bound on a concrete
//! type would be rejected at the definition instead). The user who calls
//! `to_complete_within` without a runtime feature is the one who sees the
//! error, and the `#[diagnostic::on_unimplemented]` message on
//! `RuntimeAvailable` points them at the feature flags.
//!
//! # Polling: `eventually`
//!
//! [`eventually`] retries a `bool`-returning probe until it passes or a
//! deadline is reached, sleeping with exponential [`Backoff`] in between. The
//! async form needs the same runtime gate as the timeout (its inter-probe
//! sleep is runtime-provided), so its probe closure carries the deferred
//! [`RuntimeAvailable`] bound. [`eventually_blocking`] is the runtime-free
//! sibling: it sleeps with `std::thread::sleep`, so a non-async codebase can
//! use it with no runtime feature at all.

use std::fmt;
use std::future::{Future, poll_fn};
use std::panic::Location;
use std::pin::{Pin, pin};
use std::task::Poll;
use std::time::{Duration, Instant};

use test_better_core::{ErrorKind, TestError, TestResult};

/// The error returned by [`run_within`] when the future outlives its limit.
///
/// It carries the limit it tripped, so the caller can render a message like
/// "did not complete within 50ms".
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Elapsed {
    /// The time limit the future failed to finish inside.
    pub limit: Duration,
}

impl fmt::Display for Elapsed {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "future did not complete within {:?}", self.limit)
    }
}

impl std::error::Error for Elapsed {}

/// A marker trait, implemented for every type when (and only when) a runtime
/// feature is enabled.
///
/// It carries no methods. Its only job is to be a *deferred* bound on
/// [`run_within`] and `Subject::to_complete_within`, so that "no runtime
/// feature" becomes an error at the user's call site rather than at the
/// library's definition.
#[diagnostic::on_unimplemented(
    message = "this async timing assertion needs a runtime, but no runtime feature is enabled",
    note = "enable exactly one runtime feature on `test-better`: `tokio`, `async-std`, or `smol`",
    note = "or, for `eventually`, use the runtime-free `eventually_blocking`"
)]
pub trait RuntimeAvailable {}

#[cfg(any(feature = "tokio", feature = "async-std", feature = "smol"))]
impl<T: ?Sized> RuntimeAvailable for T {}

/// Produces a future that completes after `duration`, using whichever runtime
/// feature is enabled.
///
/// The no-runtime variant returns a never-completing future. It is dead code
/// in practice: [`run_within`]'s `RuntimeAvailable` bound is unsatisfiable
/// without a runtime feature, so it can never be reached.
#[cfg(feature = "tokio")]
fn selected_sleep(duration: Duration) -> Pin<Box<dyn Future<Output = ()> + Send>> {
    Box::pin(async move { tokio::time::sleep(duration).await })
}

#[cfg(all(feature = "async-std", not(feature = "tokio")))]
fn selected_sleep(duration: Duration) -> Pin<Box<dyn Future<Output = ()> + Send>> {
    Box::pin(async move { async_std::task::sleep(duration).await })
}

#[cfg(all(feature = "smol", not(any(feature = "tokio", feature = "async-std"))))]
fn selected_sleep(duration: Duration) -> Pin<Box<dyn Future<Output = ()> + Send>> {
    Box::pin(async move {
        smol::Timer::after(duration).await;
    })
}

#[cfg(not(any(feature = "tokio", feature = "async-std", feature = "smol")))]
fn selected_sleep(_duration: Duration) -> Pin<Box<dyn Future<Output = ()> + Send>> {
    Box::pin(std::future::pending())
}

/// Polls `fut` and `timer` together; whichever resolves first wins. `fut` is
/// polled first on every wake-up, so a future that is ready *now* beats a
/// timer that is also ready now.
///
/// This is the runtime-agnostic core: it asks nothing of the runtime beyond
/// the `timer` future it is handed, which is why it can be unit-tested with
/// hand-built futures and no runtime at all.
async fn race<F, S>(fut: F, timer: S) -> Result<F::Output, ()>
where
    F: Future,
    S: Future<Output = ()>,
{
    let mut fut = pin!(fut);
    let mut timer = pin!(timer);
    poll_fn(move |cx| {
        if let Poll::Ready(output) = fut.as_mut().poll(cx) {
            return Poll::Ready(Ok(output));
        }
        if timer.as_mut().poll(cx).is_ready() {
            return Poll::Ready(Err(()));
        }
        Poll::Pending
    })
    .await
}

/// Awaits `fut`, but gives up after `limit`.
///
/// Returns the future's output if it finishes in time, or [`Elapsed`] if the
/// limit is reached first. The runtime is selected at compile time by the
/// enabled feature; with none enabled, the `F: RuntimeAvailable` bound is what
/// turns a call into the [`RuntimeAvailable`] diagnostic.
pub async fn run_within<F>(limit: Duration, fut: F) -> Result<F::Output, Elapsed>
where
    F: Future + RuntimeAvailable,
{
    match race(fut, selected_sleep(limit)).await {
        Ok(output) => Ok(output),
        Err(()) => Err(Elapsed { limit }),
    }
}

/// The inter-probe sleep schedule for [`eventually`] and
/// [`eventually_blocking`].
///
/// After a failed probe the helper naps, then doubles (or multiplies by
/// `factor`) the nap each time, never exceeding `ceiling`. The final nap before
/// the deadline is additionally clamped to the remaining time, so the helper
/// never oversleeps past its own timeout and skips a last probe.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Backoff {
    /// The first inter-probe nap.
    pub initial: Duration,
    /// The longest a single inter-probe nap may grow to.
    pub ceiling: Duration,
    /// The multiplier applied to the nap after each failed probe. A `factor`
    /// of 1 makes the nap constant; values below 1 are not possible.
    pub factor: u32,
}

impl Default for Backoff {
    /// Starts at 1ms, doubles, and caps at 100ms: tight enough that a condition
    /// which becomes true is noticed almost immediately, loose enough that a
    /// multi-second wait is a few dozen probes, not thousands.
    fn default() -> Self {
        Self {
            initial: Duration::from_millis(1),
            ceiling: Duration::from_millis(100),
            factor: 2,
        }
    }
}

impl Backoff {
    /// The nap that follows `previous`, grown by `factor` and clamped to
    /// `ceiling`.
    fn next_nap(&self, previous: Duration) -> Duration {
        previous.saturating_mul(self.factor).min(self.ceiling)
    }
}

/// Builds the failure for an `eventually` probe that never passed: a
/// [`ErrorKind::Timeout`] carrying how long it waited and how many times it
/// probed.
fn eventually_error(
    timeout: Duration,
    elapsed: Duration,
    probes: u32,
    location: &'static Location<'static>,
) -> TestError {
    let plural = if probes == 1 { "" } else { "s" };
    TestError::new(ErrorKind::Timeout)
        .with_message(format!(
            "condition was not met within {timeout:?}: gave up after {probes} probe{plural} \
             over {elapsed:?}"
        ))
        .with_location(location)
}

/// Retries `probe` until it returns `true` or `timeout` elapses, awaiting an
/// inter-probe sleep that grows per `backoff`.
async fn eventually_impl<F, Fut>(
    timeout: Duration,
    backoff: Backoff,
    mut probe: F,
    location: &'static Location<'static>,
) -> TestResult
where
    F: FnMut() -> Fut,
    Fut: Future<Output = bool>,
{
    let start = Instant::now();
    let mut nap = backoff.initial;
    let mut probes: u32 = 0;
    loop {
        probes = probes.saturating_add(1);
        if probe().await {
            return Ok(());
        }
        let elapsed = start.elapsed();
        if elapsed >= timeout {
            return Err(eventually_error(timeout, elapsed, probes, location));
        }
        selected_sleep(nap.min(timeout - elapsed)).await;
        nap = backoff.next_nap(nap);
    }
}

/// The `std::thread::sleep` twin of [`eventually_impl`], for the blocking API.
fn eventually_blocking_impl<F>(
    timeout: Duration,
    backoff: Backoff,
    mut probe: F,
    location: &'static Location<'static>,
) -> TestResult
where
    F: FnMut() -> bool,
{
    let start = Instant::now();
    let mut nap = backoff.initial;
    let mut probes: u32 = 0;
    loop {
        probes = probes.saturating_add(1);
        if probe() {
            return Ok(());
        }
        let elapsed = start.elapsed();
        if elapsed >= timeout {
            return Err(eventually_error(timeout, elapsed, probes, location));
        }
        std::thread::sleep(nap.min(timeout - elapsed));
        nap = backoff.next_nap(nap);
    }
}

/// Retries `probe` until it resolves to `true`, or fails once `timeout`
/// elapses.
///
/// This is the cure for the `sleep + assert` flake: instead of guessing how
/// long an asynchronous effect takes, state the *outcome* and a generous upper
/// bound. The probe is re-run on an exponential [`Backoff::default`] schedule
/// and the call returns the moment it passes, so the common case (the
/// condition is already true, or becomes true quickly) costs almost nothing.
///
/// Like [`run_within`], the inter-probe sleep is runtime-provided, so the probe
/// closure carries the deferred [`RuntimeAvailable`] bound: calling
/// `eventually` with no runtime feature enabled is a compile error at the call
/// site. Use [`eventually_blocking`] from non-async code.
///
/// The method is `#[track_caller]` and returns a future rather than being
/// `async` itself, so the failure points at the `eventually` call, not at the
/// `.await`.
///
/// ```ignore
/// use std::time::Duration;
/// use test_better::prelude::*;
///
/// # async fn run() -> TestResult {
/// eventually(Duration::from_secs(2), || async { queue_is_drained().await }).await?;
/// # Ok(())
/// # }
/// ```
#[track_caller]
pub fn eventually<F, Fut>(timeout: Duration, probe: F) -> impl Future<Output = TestResult>
where
    F: FnMut() -> Fut + RuntimeAvailable,
    Fut: Future<Output = bool>,
{
    eventually_impl(timeout, Backoff::default(), probe, Location::caller())
}

/// [`eventually`] with an explicit [`Backoff`] schedule instead of the default.
#[track_caller]
pub fn eventually_with<F, Fut>(
    timeout: Duration,
    backoff: Backoff,
    probe: F,
) -> impl Future<Output = TestResult>
where
    F: FnMut() -> Fut + RuntimeAvailable,
    Fut: Future<Output = bool>,
{
    eventually_impl(timeout, backoff, probe, Location::caller())
}

/// The runtime-free [`eventually`]: retries `probe` until it returns `true` or
/// `timeout` elapses, sleeping between attempts with `std::thread::sleep`.
///
/// Because it never touches an async runtime, it needs no runtime feature and
/// can be called from an ordinary `#[test]`.
///
/// ```
/// use std::time::Duration;
/// use test_better_async::eventually_blocking;
/// use test_better_core::TestResult;
///
/// # fn main() -> TestResult {
/// let mut polls = 0;
/// // The probe passes on its third call, so polling stops there.
/// eventually_blocking(Duration::from_secs(1), || {
///     polls += 1;
///     polls >= 3
/// })?;
/// # Ok(())
/// # }
/// ```
#[track_caller]
pub fn eventually_blocking<F>(timeout: Duration, probe: F) -> TestResult
where
    F: FnMut() -> bool,
{
    eventually_blocking_impl(timeout, Backoff::default(), probe, Location::caller())
}

/// [`eventually_blocking`] with an explicit [`Backoff`] schedule instead of the
/// default.
#[track_caller]
pub fn eventually_blocking_with<F>(timeout: Duration, backoff: Backoff, probe: F) -> TestResult
where
    F: FnMut() -> bool,
{
    eventually_blocking_impl(timeout, backoff, probe, Location::caller())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;
    use std::future::{pending, ready};

    use test_better_matchers::{eq, expect, ge, is_true};

    #[test]
    fn race_returns_the_future_output_when_it_is_ready_first() -> TestResult {
        // `pending` never resolves, so the only way `race` returns `Ok` is by
        // polling `fut` to completion.
        let outcome = pollster::block_on(race(ready(7), pending::<()>()));
        expect!(outcome).to(eq(Ok(7)))
    }

    #[test]
    fn race_reports_the_timer_when_the_future_is_not_ready() -> TestResult {
        let outcome = pollster::block_on(race(pending::<i32>(), ready(())));
        expect!(outcome).to(eq(Err(())))
    }

    #[test]
    fn race_prefers_the_future_when_both_are_ready() -> TestResult {
        // Both arms are ready immediately; `fut` is polled first, so it wins.
        let outcome = pollster::block_on(race(ready("done"), ready(())));
        expect!(outcome).to(eq(Ok("done")))
    }

    #[test]
    fn backoff_grows_by_factor_and_stops_at_the_ceiling() -> TestResult {
        let backoff = Backoff {
            initial: Duration::from_millis(10),
            ceiling: Duration::from_millis(25),
            factor: 2,
        };
        expect!(backoff.next_nap(Duration::from_millis(10))).to(eq(Duration::from_millis(20)))?;
        // 20 * 2 = 40, clamped down to the 25ms ceiling.
        expect!(backoff.next_nap(Duration::from_millis(20))).to(eq(Duration::from_millis(25)))
    }

    #[test]
    fn eventually_blocking_stops_as_soon_as_the_probe_passes() -> TestResult {
        let calls = Cell::new(0u32);
        eventually_blocking(Duration::from_secs(5), || {
            calls.set(calls.get() + 1);
            calls.get() >= 3
        })?;
        // The probe passed on its third call; polling must not continue past it.
        expect!(calls.get()).to(eq(3))
    }

    #[test]
    fn eventually_blocking_passes_immediately_when_the_probe_is_already_true() -> TestResult {
        let calls = Cell::new(0u32);
        eventually_blocking(Duration::from_secs(5), || {
            calls.set(calls.get() + 1);
            true
        })?;
        expect!(calls.get()).to(eq(1))
    }

    #[test]
    fn eventually_blocking_reports_elapsed_and_probe_count_on_timeout() -> TestResult {
        let calls = Cell::new(0u32);
        let error = eventually_blocking_with(
            Duration::from_millis(40),
            Backoff {
                initial: Duration::from_millis(5),
                ceiling: Duration::from_millis(5),
                factor: 2,
            },
            || {
                calls.set(calls.get() + 1);
                false
            },
        )
        .expect_err("a probe that is never true must time out");
        let rendered = error.to_string();
        expect!(rendered.contains("was not met within")).to(is_true())?;
        // The message names how many times it probed; with a 5ms nap inside a
        // 40ms budget that is at least a couple of attempts.
        expect!(calls.get()).to(ge(2))?;
        expect!(rendered.contains(&format!("{} probe", calls.get()))).to(is_true())
    }

    #[test]
    fn eventually_blocking_failure_kind_is_timeout() -> TestResult {
        let error = eventually_blocking(Duration::from_millis(1), || false)
            .expect_err("an always-false probe times out");
        expect!(error.kind).to(eq(ErrorKind::Timeout))
    }
}
