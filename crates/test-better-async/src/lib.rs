//! `test-better-async`: async and timing helpers.
//!
//! Home of the runtime-agnostic timeout abstraction that backs
//! `expect!(fut).to_complete_within(..)` (PROJECT_BUILD_PLAN.md §10, Phase 5).
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
//! `eventually` arrives in Iteration 5.3.

use std::fmt;
use std::future::{Future, poll_fn};
use std::pin::{Pin, pin};
use std::task::Poll;
use std::time::Duration;

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
    message = "`to_complete_within` needs an async runtime, but no runtime feature is enabled",
    note = "enable exactly one runtime feature on `test-better`: `tokio`, `async-std`, or `smol`"
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::future::{pending, ready};

    use test_better_core::TestResult;
    use test_better_matchers::{eq, expect};

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
}
