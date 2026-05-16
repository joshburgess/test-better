//! Primitive matchers: equality, ordering, and boolean checks.
//!
//! These are the leaves of the matcher library. They compare the actual value
//! against a stored expected value (`eq`, `lt`, ...) or against a fixed truth
//! (`is_true`, `is_false`). Combinators build on top of them.

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
/// use test_better_core::TestResult;
/// use test_better_matchers::{eq, check};
///
/// fn main() -> TestResult {
///     check!(2 + 2).satisfies(eq(4))?;
///     check!(5).violates(eq(4))?;
///     Ok(())
/// }
/// ```
#[must_use]
pub fn eq<T>(expected: T) -> impl Matcher<T>
where
    T: PartialEq + fmt::Debug,
{
    EqMatcher { expected }
}

/// Matches a value not equal to `expected`.
///
/// ```
/// use test_better_core::TestResult;
/// use test_better_matchers::{ne, check};
///
/// fn main() -> TestResult {
///     check!(5).satisfies(ne(4))?;
///     check!(4).violates(ne(4))?;
///     Ok(())
/// }
/// ```
#[must_use]
pub fn ne<T>(expected: T) -> impl Matcher<T>
where
    T: PartialEq + fmt::Debug,
{
    NeMatcher { expected }
}

/// Matches a value strictly less than `expected`.
///
/// ```
/// use test_better_core::TestResult;
/// use test_better_matchers::{lt, check};
///
/// fn main() -> TestResult {
///     check!(9).satisfies(lt(10))?;
///     check!(10).violates(lt(10))?;
///     Ok(())
/// }
/// ```
#[must_use]
pub fn lt<T>(expected: T) -> impl Matcher<T>
where
    T: PartialOrd + fmt::Debug,
{
    LtMatcher { expected }
}

/// Matches a value less than or equal to `expected`.
///
/// ```
/// use test_better_core::TestResult;
/// use test_better_matchers::{le, check};
///
/// fn main() -> TestResult {
///     check!(10).satisfies(le(10))?;
///     check!(11).violates(le(10))?;
///     Ok(())
/// }
/// ```
#[must_use]
pub fn le<T>(expected: T) -> impl Matcher<T>
where
    T: PartialOrd + fmt::Debug,
{
    LeMatcher { expected }
}

/// Matches a value strictly greater than `expected`.
///
/// ```
/// use test_better_core::TestResult;
/// use test_better_matchers::{gt, check};
///
/// fn main() -> TestResult {
///     check!(1).satisfies(gt(0))?;
///     check!(0).violates(gt(0))?;
///     Ok(())
/// }
/// ```
#[must_use]
pub fn gt<T>(expected: T) -> impl Matcher<T>
where
    T: PartialOrd + fmt::Debug,
{
    GtMatcher { expected }
}

/// Matches a value greater than or equal to `expected`.
///
/// ```
/// use test_better_core::TestResult;
/// use test_better_matchers::{ge, check};
///
/// fn main() -> TestResult {
///     check!(0).satisfies(ge(0))?;
///     check!(-1).violates(ge(0))?;
///     Ok(())
/// }
/// ```
#[must_use]
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
/// use test_better_core::TestResult;
/// use test_better_matchers::{is_true, check};
///
/// fn main() -> TestResult {
///     check!(1 == 1).satisfies(is_true())?;
///     check!(false).violates(is_true())?;
///     Ok(())
/// }
/// ```
#[must_use]
pub fn is_true() -> impl Matcher<bool> {
    BoolMatcher { expected: true }
}

/// Matches `false`.
///
/// ```
/// use test_better_core::TestResult;
/// use test_better_matchers::{is_false, check};
///
/// fn main() -> TestResult {
///     check!(1 == 2).satisfies(is_false())?;
///     check!(true).violates(is_false())?;
///     Ok(())
/// }
/// ```
#[must_use]
pub fn is_false() -> impl Matcher<bool> {
    BoolMatcher { expected: false }
}

#[cfg(test)]
mod tests {
    use test_better_core::{OrFail, TestResult};

    use super::*;
    use crate::{eq, check, is_true};

    #[test]
    fn eq_passes_and_fails_with_rendered_mismatch() -> TestResult {
        check!(eq(4).check(&4).matched).satisfies(is_true())?;
        let failure = eq(4).check(&5).failure.or_fail_with("5 is not 4")?;
        check!(failure.expected.to_string()).satisfies(eq("equal to 4".to_string()))?;
        check!(failure.actual).satisfies(eq("5".to_string()))?;
        Ok(())
    }

    #[test]
    fn eq_omits_a_diff_for_single_line_values() -> TestResult {
        let failure = eq(4).check(&5).failure.or_fail_with("5 is not 4")?;
        check!(failure.diff.is_none()).satisfies(is_true())?;
        Ok(())
    }

    #[cfg(feature = "diff")]
    #[test]
    fn eq_attaches_a_diff_when_the_pretty_repr_is_multi_line() -> TestResult {
        let failure = eq(vec![1, 2, 3])
            .check(&vec![1, 2, 4])
            .failure
            .or_fail_with("the vectors differ")?;
        let diff = failure
            .diff
            .or_fail_with("multi-line pretty reprs get a diff")?;
        check!(diff.contains("-    3,")).satisfies(is_true())?;
        check!(diff.contains("+    4,")).satisfies(is_true())?;
        Ok(())
    }

    #[test]
    fn ne_passes_and_fails_with_rendered_mismatch() -> TestResult {
        check!(ne(4).check(&5).matched).satisfies(is_true())?;
        let failure = ne(4).check(&4).failure.or_fail_with("4 is equal to 4")?;
        check!(failure.expected.to_string()).satisfies(eq("not equal to 4".to_string()))?;
        check!(failure.actual).satisfies(eq("4".to_string()))?;
        Ok(())
    }

    #[test]
    fn lt_passes_and_fails_with_rendered_mismatch() -> TestResult {
        check!(lt(10).check(&9).matched).satisfies(is_true())?;
        let failure = lt(10).check(&10).failure.or_fail_with("10 is not < 10")?;
        check!(failure.expected.to_string()).satisfies(eq("less than 10".to_string()))?;
        check!(failure.actual).satisfies(eq("10".to_string()))?;
        Ok(())
    }

    #[test]
    fn le_passes_and_fails_with_rendered_mismatch() -> TestResult {
        check!(le(10).check(&10).matched).satisfies(is_true())?;
        let failure = le(10).check(&11).failure.or_fail_with("11 is not <= 10")?;
        check!(failure.expected.to_string()).satisfies(eq("less than or equal to 10".to_string()))?;
        check!(failure.actual).satisfies(eq("11".to_string()))?;
        Ok(())
    }

    #[test]
    fn gt_passes_and_fails_with_rendered_mismatch() -> TestResult {
        check!(gt(0).check(&1).matched).satisfies(is_true())?;
        let failure = gt(0).check(&0).failure.or_fail_with("0 is not > 0")?;
        check!(failure.expected.to_string()).satisfies(eq("greater than 0".to_string()))?;
        check!(failure.actual).satisfies(eq("0".to_string()))?;
        Ok(())
    }

    #[test]
    fn ge_passes_and_fails_with_rendered_mismatch() -> TestResult {
        check!(ge(0).check(&0).matched).satisfies(is_true())?;
        let failure = ge(0).check(&-1).failure.or_fail_with("-1 is not >= 0")?;
        check!(failure.expected.to_string()).satisfies(eq("greater than or equal to 0".to_string()))?;
        check!(failure.actual).satisfies(eq("-1".to_string()))?;
        Ok(())
    }

    #[test]
    fn is_true_passes_and_fails_with_rendered_mismatch() -> TestResult {
        check!(is_true().check(&true).matched).satisfies(is_true())?;
        let failure = is_true()
            .check(&false)
            .failure
            .or_fail_with("false is not true")?;
        check!(failure.expected.to_string()).satisfies(eq("true".to_string()))?;
        check!(failure.actual).satisfies(eq("false".to_string()))?;
        Ok(())
    }

    #[test]
    fn is_false_passes_and_fails_with_rendered_mismatch() -> TestResult {
        check!(is_false().check(&false).matched).satisfies(is_true())?;
        let failure = is_false()
            .check(&true)
            .failure
            .or_fail_with("true is not false")?;
        check!(failure.expected.to_string()).satisfies(eq("false".to_string()))?;
        check!(failure.actual).satisfies(eq("true".to_string()))?;
        Ok(())
    }
}
