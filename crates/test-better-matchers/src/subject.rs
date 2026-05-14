//! The [`expect!`](crate::expect) macro and its [`Subject`] type: the entry point for writing
//! an assertion.
//!
//! `expect!(value)` captures the value *and the source text of the expression
//! it came from*, so a failure can name `2 + 2`, not just `4`. The resulting
//! [`Subject`] is consumed by [`Subject::to`] / [`Subject::to_not`], each of
//! which returns a [`TestResult`] so the assertion chains with `?`.
//!
//! # Async
//!
//! When the expression handed to `expect!` is a [`Future`], the resulting
//! `Subject` grows an `await`-based method, [`Subject::resolves_to`]. The
//! decision (recorded in `BACKLOG.md`) is to keep a single `Subject<T>` and add
//! that method to *this* impl block with a method-level `where T: Future`
//! bound and a distinct name: a blanket `impl<T> Subject<T>` and an overlapping
//! `impl<F: Future> Subject<F>` cannot coexist as inherent impls.
//!
//! `resolves_to` is runtime-agnostic: it just awaits the future, so it works
//! under `#[tokio::test]`, `#[async_std::test]`, `pollster::block_on`, or any
//! other executor. (Runtime-specific timing methods arrive in Phase 5.2.)

use std::future::Future;
use std::panic::Location;
use std::time::Duration;

use test_better_async::{Elapsed, RuntimeAvailable, run_within};
use test_better_core::{ErrorKind, Payload, TestError, TestResult};

use crate::description::Description;
use crate::matcher::{Matcher, Mismatch};

/// A value under test, paired with the source text of the expression that
/// produced it.
///
/// `Subject` owns its value (the [`expect!`](crate::expect) macro hands it over by value) and
/// borrows nothing, so it carries no lifetime parameter.
pub struct Subject<T> {
    actual: T,
    expr: &'static str,
}

impl<T> Subject<T> {
    /// Pairs `actual` with the source text it came from. Called by [`expect!`](crate::expect);
    /// rarely constructed directly.
    #[must_use]
    pub fn new(actual: T, expr: &'static str) -> Self {
        Self { actual, expr }
    }

    /// Asserts that the value satisfies `matcher`.
    ///
    /// Returns `Ok(())` on a match and a [`TestError`] otherwise. The result is
    /// `#[must_use]` (it is a `Result`), so a forgotten `?` is a compiler
    /// warning rather than a silently-passing assertion.
    #[track_caller]
    pub fn to<M>(self, matcher: M) -> TestResult
    where
        M: Matcher<T>,
    {
        match matcher.check(&self.actual).failure {
            None => Ok(()),
            Some(mismatch) => Err(mismatch_error(self.expr, mismatch)),
        }
    }

    /// Asserts that the value does *not* satisfy `matcher`.
    ///
    /// Returns `Ok(())` when the matcher does not match, and a [`TestError`]
    /// when it unexpectedly does.
    #[track_caller]
    pub fn to_not<M>(self, matcher: M) -> TestResult
    where
        M: Matcher<T>,
    {
        if matcher.check(&self.actual).matched {
            Err(unexpected_match_error(self.expr, matcher.description()))
        } else {
            Ok(())
        }
    }

    /// Awaits the future and asserts that its output satisfies `matcher`.
    ///
    /// This is the async counterpart of [`to`](Self::to): reach for it when
    /// the expression handed to [`expect!`](crate::expect) is a [`Future`].
    /// The matcher runs against the future's *output*, so
    /// `expect!(fut).resolves_to(eq(4))` is exactly `expect!(fut.await).to(eq(4))`
    /// without the intermediate binding.
    ///
    /// The method itself is *not* `async`: it is `#[track_caller]` and returns
    /// a future. The call-site location is captured synchronously when
    /// `resolves_to` is called (an `async fn` could not be `#[track_caller]`),
    /// then carried into the failure once the returned future is awaited.
    ///
    /// ```
    /// use test_better_core::TestResult;
    /// use test_better_matchers::{eq, expect};
    ///
    /// # fn main() -> TestResult {
    /// pollster::block_on(async {
    ///     expect!(async { 2 + 2 }).resolves_to(eq(4)).await?;
    ///     TestResult::Ok(())
    /// })
    /// # }
    /// ```
    #[track_caller]
    pub fn resolves_to<M>(self, matcher: M) -> impl Future<Output = TestResult>
    where
        T: Future,
        M: Matcher<T::Output>,
    {
        // Captured here, synchronously, before the returned future is ever
        // polled: this is the user's `expect!(..).resolves_to(..)` call site.
        let location = Location::caller();
        async move {
            let output = self.actual.await;
            match matcher.check(&output).failure {
                None => Ok(()),
                Some(mismatch) => Err(mismatch_error(self.expr, mismatch).with_location(location)),
            }
        }
    }

    /// Awaits the future, but fails if it does not finish within `limit`.
    ///
    /// Like [`resolves_to`](Self::resolves_to), this is for a future-typed
    /// subject and returns a future rather than being `async` itself, so the
    /// `#[track_caller]` location is the call site. Unlike `resolves_to`, it
    /// does not look at the output: the assertion is purely about *time*.
    ///
    /// The timeout needs a runtime to provide its sleep, selected by a cargo
    /// feature on `test-better`: `tokio`, `async-std`, or `smol`. With none
    /// enabled, the `T: RuntimeAvailable` bound is unsatisfied and the call is
    /// a compile error naming those flags.
    ///
    /// ```ignore
    /// use std::time::Duration;
    /// use test_better::prelude::*;
    ///
    /// # async fn run() -> TestResult {
    /// expect!(some_future())
    ///     .to_complete_within(Duration::from_millis(50))
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    #[track_caller]
    pub fn to_complete_within(self, limit: Duration) -> impl Future<Output = TestResult>
    where
        T: Future + RuntimeAvailable,
    {
        let location = Location::caller();
        async move {
            match run_within(limit, self.actual).await {
                Ok(_) => Ok(()),
                Err(elapsed) => Err(timeout_error(self.expr, elapsed).with_location(location)),
            }
        }
    }
}

/// Builds the error for a matcher that did not match: the expected/actual pair
/// goes into the payload, the source expression into the message.
#[track_caller]
fn mismatch_error(expr: &str, mismatch: Mismatch) -> TestError {
    TestError::new(ErrorKind::Assertion)
        .with_message(format!("expect!({expr})"))
        .with_payload(Payload::ExpectedActual {
            expected: mismatch.expected.to_string(),
            actual: mismatch.actual,
            diff: mismatch.diff,
        })
}

/// Builds the error for `to_not` when the matcher matched but should not have.
/// There is no `Mismatch` in this case, so the message carries the whole story.
#[track_caller]
fn unexpected_match_error(expr: &str, description: Description) -> TestError {
    TestError::new(ErrorKind::Assertion).with_message(format!(
        "expect!({expr}): expected it not to be {description}, but it was"
    ))
}

/// Builds the error for `to_complete_within` when the future ran past its
/// limit. This is a timing failure, not a value mismatch, so it carries only
/// a message, no payload.
#[track_caller]
fn timeout_error(expr: &str, elapsed: Elapsed) -> TestError {
    TestError::new(ErrorKind::Assertion).with_message(format!(
        "expect!({expr}): did not complete within {:?}",
        elapsed.limit
    ))
}

/// Captures an expression and its source text for assertion with a matcher.
///
/// ```
/// use test_better_core::TestResult;
/// use test_better_matchers::{eq, expect};
///
/// fn main() -> TestResult {
///     expect!(2 + 2).to(eq(4))?;
///     Ok(())
/// }
/// ```
#[macro_export]
macro_rules! expect {
    ($actual:expr) => {
        $crate::Subject::new($actual, ::core::stringify!($actual))
    };
}

#[cfg(test)]
mod tests {
    use test_better_core::TestResult;

    use crate::{eq, is_true};

    #[test]
    fn to_returns_ok_on_a_match() -> TestResult {
        let result = expect!(2 + 2).to(eq(4));
        expect!(result.is_ok()).to(is_true())?;
        Ok(())
    }

    #[test]
    fn to_failure_mentions_the_expression_and_the_expected_value() -> TestResult {
        let error = expect!(2 + 2).to(eq(5)).expect_err("2 + 2 is not 5");
        let rendered = error.to_string();
        expect!(rendered.contains("2 + 2")).to(is_true())?;
        expect!(rendered.contains("equal to 5")).to(is_true())?;
        expect!(rendered.contains("actual: 4")).to(is_true())?;
        Ok(())
    }

    #[test]
    fn to_failure_captures_the_caller_location() -> TestResult {
        let line = line!() + 1;
        let error = expect!(2 + 2).to(eq(5)).expect_err("2 + 2 is not 5");
        expect!(error.location.line()).to(eq(line))?;
        expect!(error.location.file().ends_with("subject.rs")).to(is_true())?;
        Ok(())
    }

    #[test]
    fn to_not_returns_ok_when_the_matcher_does_not_match() -> TestResult {
        let result = expect!(2 + 2).to_not(eq(5));
        expect!(result.is_ok()).to(is_true())?;
        Ok(())
    }

    #[test]
    fn to_not_failure_mentions_the_expression_and_the_matcher() -> TestResult {
        let error = expect!(true).to_not(is_true()).expect_err("true is true");
        let rendered = error.to_string();
        expect!(rendered.contains("expect!(true)")).to(is_true())?;
        expect!(rendered.contains("not to be true")).to(is_true())?;
        Ok(())
    }

    #[test]
    fn to_not_captures_the_caller_location() -> TestResult {
        let line = line!() + 1;
        let error = expect!(true).to_not(is_true()).expect_err("true is true");
        expect!(error.location.line()).to(eq(line))?;
        Ok(())
    }

    #[test]
    fn resolves_to_returns_ok_when_the_output_matches() -> TestResult {
        pollster::block_on(async {
            let result = expect!(async { 2 + 2 }).resolves_to(eq(4)).await;
            expect!(result.is_ok()).to(is_true())
        })
    }

    #[test]
    fn resolves_to_failure_mentions_the_expression_and_the_output() -> TestResult {
        pollster::block_on(async {
            let error = expect!(async { 2 + 2 })
                .resolves_to(eq(5))
                .await
                .expect_err("2 + 2 does not resolve to 5");
            let rendered = error.to_string();
            expect!(rendered.contains("async { 2 + 2 }")).to(is_true())?;
            expect!(rendered.contains("equal to 5")).to(is_true())?;
            expect!(rendered.contains("actual: 4")).to(is_true())
        })
    }

    #[test]
    fn resolves_to_failure_captures_the_call_site_not_the_await() -> TestResult {
        // The location is captured where `resolves_to` is *called*, even
        // though the future is awaited on a later line.
        pollster::block_on(async {
            let line = line!() + 1;
            let pending = expect!(async { 2 + 2 }).resolves_to(eq(5));
            let error = pending.await.expect_err("2 + 2 does not resolve to 5");
            expect!(error.location.line()).to(eq(line))?;
            expect!(error.location.file().ends_with("subject.rs")).to(is_true())
        })
    }
}
