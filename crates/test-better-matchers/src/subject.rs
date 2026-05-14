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
//! Phase 5 needs `expect!(some_future())` to grow `await`-based methods. The
//! decision (recorded in `BACKLOG.md`) is to keep a single `Subject<T>` and add
//! those methods to *this* impl block with method-level `where T: Future`
//! bounds and distinct names: a blanket `impl<T> Subject<T>` and an overlapping
//! `impl<F: Future> Subject<F>` cannot coexist as inherent impls.

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
}
