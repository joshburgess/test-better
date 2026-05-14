//! Primitive matchers: equality, ordering, and boolean checks.
//!
//! These are the leaves of the matcher library. They compare the actual value
//! against a stored expected value (`eq`, `lt`, ...) or against a fixed truth
//! (`is_true`, `is_false`). Combinators (Phase 3) build on top of them.

use std::fmt;

use crate::description::Description;
use crate::matcher::{MatchResult, Matcher, Mismatch};

/// Generates a comparison matcher: a `struct` holding the expected value and a
/// [`Matcher`] impl that compares with `$op` and describes itself with
/// `$describe`.
macro_rules! comparison_matcher {
    ($matcher:ident, $bound:ident, $op:tt, $describe:literal) => {
        struct $matcher<T> {
            expected: T,
        }

        impl<T> Matcher<T> for $matcher<T>
        where
            T: $bound + fmt::Debug,
        {
            fn check(&self, actual: &T) -> MatchResult {
                if *actual $op self.expected {
                    MatchResult::pass()
                } else {
                    MatchResult::fail(Mismatch::new(self.description(), format!("{actual:?}")))
                }
            }

            fn description(&self) -> Description {
                Description::text(format!(concat!($describe, " {:?}"), self.expected))
            }
        }
    };
}

comparison_matcher!(NeMatcher, PartialEq, !=, "not equal to");
comparison_matcher!(LtMatcher, PartialOrd, <, "less than");
comparison_matcher!(LeMatcher, PartialOrd, <=, "less than or equal to");
comparison_matcher!(GtMatcher, PartialOrd, >, "greater than");
comparison_matcher!(GeMatcher, PartialOrd, >=, "greater than or equal to");

/// The matcher behind [`eq`]. Unlike the other comparison matchers it can
/// attach a structural diff: when the expected and actual values' pretty
/// (`{:#?}`) representations span multiple lines, a line-oriented diff is the
/// readable way to show what changed.
struct EqMatcher<T> {
    expected: T,
}

impl<T> Matcher<T> for EqMatcher<T>
where
    T: PartialEq + fmt::Debug,
{
    fn check(&self, actual: &T) -> MatchResult {
        if *actual == self.expected {
            return MatchResult::pass();
        }
        let mut mismatch = Mismatch::new(self.description(), format!("{actual:?}"));
        if let Some(diff) =
            multi_line_diff(&format!("{:#?}", self.expected), &format!("{actual:#?}"))
        {
            mismatch = mismatch.with_diff(diff);
        }
        MatchResult::fail(mismatch)
    }

    fn description(&self) -> Description {
        Description::text(format!("equal to {:?}", self.expected))
    }
}

/// A line-oriented diff of two pretty-printed values, but only when at least
/// one of them actually spans multiple lines: a diff of two single-line values
/// is just noise next to the `expected:`/`actual:` lines.
///
/// With the `diff` feature off this is always `None`, so `eq` still works, it
/// just never carries a diff.
#[cfg(feature = "diff")]
fn multi_line_diff(expected: &str, actual: &str) -> Option<String> {
    if expected.contains('\n') || actual.contains('\n') {
        Some(crate::diff::diff_lines(expected, actual))
    } else {
        None
    }
}

#[cfg(not(feature = "diff"))]
fn multi_line_diff(_expected: &str, _actual: &str) -> Option<String> {
    None
}

/// Matches a value equal to `expected`.
///
/// On a mismatch where the values' pretty representations are multi-line (a
/// struct, a collection), the failure carries a line-oriented diff.
///
/// ```
/// use test_better_matchers::{eq, Matcher};
///
/// assert!(eq(4).check(&(2 + 2)).matched);
/// assert!(!eq(4).check(&5).matched);
/// ```
pub fn eq<T>(expected: T) -> impl Matcher<T>
where
    T: PartialEq + fmt::Debug,
{
    EqMatcher { expected }
}

/// Matches a value not equal to `expected`.
///
/// ```
/// use test_better_matchers::{ne, Matcher};
///
/// assert!(ne(4).check(&5).matched);
/// assert!(!ne(4).check(&4).matched);
/// ```
pub fn ne<T>(expected: T) -> impl Matcher<T>
where
    T: PartialEq + fmt::Debug,
{
    NeMatcher { expected }
}

/// Matches a value strictly less than `expected`.
///
/// ```
/// use test_better_matchers::{lt, Matcher};
///
/// assert!(lt(10).check(&9).matched);
/// assert!(!lt(10).check(&10).matched);
/// ```
pub fn lt<T>(expected: T) -> impl Matcher<T>
where
    T: PartialOrd + fmt::Debug,
{
    LtMatcher { expected }
}

/// Matches a value less than or equal to `expected`.
///
/// ```
/// use test_better_matchers::{le, Matcher};
///
/// assert!(le(10).check(&10).matched);
/// assert!(!le(10).check(&11).matched);
/// ```
pub fn le<T>(expected: T) -> impl Matcher<T>
where
    T: PartialOrd + fmt::Debug,
{
    LeMatcher { expected }
}

/// Matches a value strictly greater than `expected`.
///
/// ```
/// use test_better_matchers::{gt, Matcher};
///
/// assert!(gt(0).check(&1).matched);
/// assert!(!gt(0).check(&0).matched);
/// ```
pub fn gt<T>(expected: T) -> impl Matcher<T>
where
    T: PartialOrd + fmt::Debug,
{
    GtMatcher { expected }
}

/// Matches a value greater than or equal to `expected`.
///
/// ```
/// use test_better_matchers::{ge, Matcher};
///
/// assert!(ge(0).check(&0).matched);
/// assert!(!ge(0).check(&-1).matched);
/// ```
pub fn ge<T>(expected: T) -> impl Matcher<T>
where
    T: PartialOrd + fmt::Debug,
{
    GeMatcher { expected }
}

/// A matcher for a fixed boolean truth, behind [`is_true`] and [`is_false`].
struct BoolMatcher {
    expected: bool,
}

impl Matcher<bool> for BoolMatcher {
    fn check(&self, actual: &bool) -> MatchResult {
        if *actual == self.expected {
            MatchResult::pass()
        } else {
            MatchResult::fail(Mismatch::new(self.description(), format!("{actual:?}")))
        }
    }

    fn description(&self) -> Description {
        Description::text(if self.expected { "true" } else { "false" })
    }
}

/// Matches `true`.
///
/// ```
/// use test_better_matchers::{is_true, Matcher};
///
/// assert!(is_true().check(&(1 == 1)).matched);
/// assert!(!is_true().check(&false).matched);
/// ```
pub fn is_true() -> impl Matcher<bool> {
    BoolMatcher { expected: true }
}

/// Matches `false`.
///
/// ```
/// use test_better_matchers::{is_false, Matcher};
///
/// assert!(is_false().check(&(1 == 2)).matched);
/// assert!(!is_false().check(&true).matched);
/// ```
pub fn is_false() -> impl Matcher<bool> {
    BoolMatcher { expected: false }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn eq_passes_and_fails_with_rendered_mismatch() {
        assert!(eq(4).check(&4).matched);
        let failure = eq(4).check(&5).failure.expect("5 is not 4");
        assert_eq!(failure.expected.to_string(), "equal to 4");
        assert_eq!(failure.actual, "5");
    }

    #[test]
    fn eq_omits_a_diff_for_single_line_values() {
        let failure = eq(4).check(&5).failure.expect("5 is not 4");
        assert!(failure.diff.is_none(), "{:?}", failure.diff);
    }

    #[cfg(feature = "diff")]
    #[test]
    fn eq_attaches_a_diff_when_the_pretty_repr_is_multi_line() {
        let failure = eq(vec![1, 2, 3])
            .check(&vec![1, 2, 4])
            .failure
            .expect("the vectors differ");
        let diff = failure.diff.expect("multi-line pretty reprs get a diff");
        assert!(diff.contains("-    3,"), "{diff}");
        assert!(diff.contains("+    4,"), "{diff}");
    }

    #[test]
    fn ne_passes_and_fails_with_rendered_mismatch() {
        assert!(ne(4).check(&5).matched);
        let failure = ne(4).check(&4).failure.expect("4 is equal to 4");
        assert_eq!(failure.expected.to_string(), "not equal to 4");
        assert_eq!(failure.actual, "4");
    }

    #[test]
    fn lt_passes_and_fails_with_rendered_mismatch() {
        assert!(lt(10).check(&9).matched);
        let failure = lt(10).check(&10).failure.expect("10 is not < 10");
        assert_eq!(failure.expected.to_string(), "less than 10");
        assert_eq!(failure.actual, "10");
    }

    #[test]
    fn le_passes_and_fails_with_rendered_mismatch() {
        assert!(le(10).check(&10).matched);
        let failure = le(10).check(&11).failure.expect("11 is not <= 10");
        assert_eq!(failure.expected.to_string(), "less than or equal to 10");
        assert_eq!(failure.actual, "11");
    }

    #[test]
    fn gt_passes_and_fails_with_rendered_mismatch() {
        assert!(gt(0).check(&1).matched);
        let failure = gt(0).check(&0).failure.expect("0 is not > 0");
        assert_eq!(failure.expected.to_string(), "greater than 0");
        assert_eq!(failure.actual, "0");
    }

    #[test]
    fn ge_passes_and_fails_with_rendered_mismatch() {
        assert!(ge(0).check(&0).matched);
        let failure = ge(0).check(&-1).failure.expect("-1 is not >= 0");
        assert_eq!(failure.expected.to_string(), "greater than or equal to 0");
        assert_eq!(failure.actual, "-1");
    }

    #[test]
    fn is_true_passes_and_fails_with_rendered_mismatch() {
        assert!(is_true().check(&true).matched);
        let failure = is_true().check(&false).failure.expect("false is not true");
        assert_eq!(failure.expected.to_string(), "true");
        assert_eq!(failure.actual, "false");
    }

    #[test]
    fn is_false_passes_and_fails_with_rendered_mismatch() {
        assert!(is_false().check(&false).matched);
        let failure = is_false().check(&true).failure.expect("true is not false");
        assert_eq!(failure.expected.to_string(), "false");
        assert_eq!(failure.actual, "true");
    }
}
