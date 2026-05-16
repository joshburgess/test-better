//! The [`Matcher`] trait and its result types, [`MatchResult`] and
//! [`Mismatch`].
//!
//! A matcher is a reusable expectation: it inspects a borrowed value and
//! reports, in structured form, whether the value met the expectation and — if
//! not — what was expected, what was found, and an optional diff. The `check!`
//! macro turns that structured result into a [`TestError`].
//!
//! [`TestError`]: test_better_core::TestError

use crate::description::Description;

/// A reusable expectation about a value of type `T`.
///
/// `T` is `?Sized` so matchers can target unsized values directly (`str`,
/// `[u8]`) without forcing the caller to borrow through a reference type.
pub trait Matcher<T: ?Sized> {
    /// Checks `actual` against this matcher's expectation.
    fn check(&self, actual: &T) -> MatchResult;

    /// Describes what this matcher expects, for use in failure output and in
    /// combinator descriptions.
    fn description(&self) -> Description;
}

/// The structured outcome of [`Matcher::check`].
///
/// # Invariant
///
/// `matched` and `failure` always disagree: `matched == failure.is_none()`.
/// Construct values through [`MatchResult::pass`] and [`MatchResult::fail`]
/// rather than building the struct literal, so the invariant cannot be broken.
#[derive(Debug, Clone)]
pub struct MatchResult {
    /// Whether the value met the expectation.
    pub matched: bool,
    /// The mismatch detail, present exactly when `matched` is `false`.
    pub failure: Option<Mismatch>,
}

impl MatchResult {
    /// The value met the expectation.
    #[must_use]
    pub fn pass() -> Self {
        Self {
            matched: true,
            failure: None,
        }
    }

    /// The value did not meet the expectation; `mismatch` explains why.
    #[must_use]
    pub fn fail(mismatch: Mismatch) -> Self {
        Self {
            matched: false,
            failure: Some(mismatch),
        }
    }
}

/// Why a value failed a matcher: what was expected, what was found, and an
/// optional diff between the two.
#[derive(Debug, Clone)]
pub struct Mismatch {
    /// The matcher's expectation, as a composable [`Description`].
    pub expected: Description,
    /// The `Debug` rendering of the actual value.
    pub actual: String,
    /// An optional pre-rendered diff between expected and actual. `None` when
    /// no diff is available.
    pub diff: Option<String>,
}

impl Mismatch {
    /// A mismatch with no diff.
    #[must_use]
    pub fn new(expected: Description, actual: impl Into<String>) -> Self {
        Self {
            expected,
            actual: actual.into(),
            diff: None,
        }
    }

    /// Attaches a pre-rendered diff, consuming and returning `self`.
    #[must_use]
    pub fn with_diff(mut self, diff: impl Into<String>) -> Self {
        self.diff = Some(diff.into());
        self
    }
}

#[cfg(test)]
mod tests {
    use test_better_core::{OrFail, TestResult};

    use super::*;
    use crate::{eq, check, is_false, is_true};

    #[test]
    fn pass_has_no_failure() -> TestResult {
        let result = MatchResult::pass();
        check!(result.matched).satisfies(is_true())?;
        check!(result.failure.is_none()).satisfies(is_true())?;
        Ok(())
    }

    #[test]
    fn fail_carries_the_mismatch() -> TestResult {
        let mismatch = Mismatch::new(Description::text("equal to 4"), "5");
        let result = MatchResult::fail(mismatch);
        check!(result.matched).satisfies(is_false())?;
        let failure = result.failure.or_fail_with("fail() stores the mismatch")?;
        check!(failure.expected.to_string()).satisfies(eq("equal to 4".to_string()))?;
        check!(failure.actual).satisfies(eq("5".to_string()))?;
        check!(failure.diff.is_none()).satisfies(is_true())?;
        Ok(())
    }

    #[test]
    fn mismatch_with_diff_stores_the_diff() -> TestResult {
        let mismatch = Mismatch::new(Description::text("the file"), "other").with_diff("- a\n+ b");
        check!(mismatch.diff.as_deref()).satisfies(eq(Some("- a\n+ b")))?;
        Ok(())
    }
}
