//! Trivial matchers that ignore their input: [`always_matches`] and
//! [`never_matches`].
//!
//! These exist to test the matcher machinery itself: combinators (Phase 3),
//! the `expect!` macro (Iteration 2.3), and failure rendering all need a
//! matcher with a known, fixed outcome. They are not meant for real
//! assertions, where a matcher that ignores its input says nothing useful.

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
/// use test_better_matchers::{always_matches, Matcher};
///
/// assert!(always_matches().check(&42).matched);
/// assert!(always_matches().check("any string").matched);
/// ```
pub fn always_matches<T: ?Sized>() -> impl Matcher<T> {
    AlwaysMatches
}

/// A matcher that matches no value, for testing matcher machinery.
///
/// ```
/// use test_better_matchers::{never_matches, Matcher};
///
/// assert!(!never_matches().check(&42).matched);
/// ```
pub fn never_matches<T: ?Sized>() -> impl Matcher<T> {
    NeverMatches
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn always_matches_passes_for_any_type() {
        assert!(always_matches().check(&42).matched);
        assert!(always_matches().check("a str").matched);
        assert!(always_matches().check(&[1, 2, 3][..]).matched);
    }

    #[test]
    fn never_matches_fails_with_a_described_mismatch() {
        let result = never_matches().check(&42);
        assert!(!result.matched);
        let failure = result.failure.expect("never_matches always fails");
        assert_eq!(failure.expected.to_string(), "nothing");
        assert_eq!(failure.actual, "<value>");
    }
}
