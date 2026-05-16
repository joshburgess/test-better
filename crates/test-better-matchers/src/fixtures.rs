//! Trivial matchers that ignore their input: [`always_matches`] and
//! [`never_matches`].
//!
//! These exist to test the matcher machinery itself: combinators, the `check!`
//! macro, and failure rendering all need a matcher with a known, fixed outcome.
//! They are not meant for real assertions, where a matcher that ignores its
//! input says nothing useful.

use crate::description::Description;
use crate::matcher::{MatchResult, Matcher, Mismatch};

/// A matcher that matches every value.
struct AlwaysMatches;

impl<T: ?Sized> Matcher<T> for AlwaysMatches {
    fn check(&self, _actual: &T) -> MatchResult {
        MatchResult::pass()
    }

    fn description(&self) -> Description {
        Description::text("anything")
    }
}

/// A matcher that matches no value.
struct NeverMatches;

impl<T: ?Sized> Matcher<T> for NeverMatches {
    fn check(&self, _actual: &T) -> MatchResult {
        // `T` is unconstrained, so the actual value cannot be rendered; the
        // mismatch reports a fixed placeholder. This is a fixture, not a real
        // matcher, so the lost detail does not matter.
        MatchResult::fail(Mismatch::new(Matcher::<T>::description(self), "<value>"))
    }

    fn description(&self) -> Description {
        Description::text("nothing")
    }
}

/// A matcher that matches every value, for testing matcher machinery.
///
/// ```
/// use test_better_core::TestResult;
/// use test_better_matchers::{always_matches, check};
///
/// fn main() -> TestResult {
///     check!(42).satisfies(always_matches())?;
///     check!("any string").satisfies(always_matches())?;
///     Ok(())
/// }
/// ```
#[must_use]
pub fn always_matches<T: ?Sized>() -> impl Matcher<T> {
    AlwaysMatches
}

/// A matcher that matches no value, for testing matcher machinery.
///
/// ```
/// use test_better_core::TestResult;
/// use test_better_matchers::{never_matches, check};
///
/// fn main() -> TestResult {
///     check!(42).violates(never_matches())?;
///     Ok(())
/// }
/// ```
#[must_use]
pub fn never_matches<T: ?Sized>() -> impl Matcher<T> {
    NeverMatches
}

#[cfg(test)]
mod tests {
    use test_better_core::{OrFail, TestResult};

    use super::*;
    use crate::{check, eq, is_false, is_true};

    #[test]
    fn always_matches_passes_for_any_type() -> TestResult {
        check!(always_matches().check(&42).matched).satisfies(is_true())?;
        check!(always_matches().check("a str").matched).satisfies(is_true())?;
        check!(always_matches().check(&[1, 2, 3][..]).matched).satisfies(is_true())?;
        Ok(())
    }

    #[test]
    fn never_matches_fails_with_a_described_mismatch() -> TestResult {
        let result = never_matches().check(&42);
        check!(result.matched).satisfies(is_false())?;
        let failure = result.failure.or_fail_with("never_matches always fails")?;
        check!(failure.expected.to_string()).satisfies(eq("nothing".to_string()))?;
        check!(failure.actual).satisfies(eq("<value>".to_string()))?;
        Ok(())
    }
}
