//! Logical combinators: [`not`], [`all_of`], and [`any_of`].
//!
//! These take other matchers and build a compound matcher out of them. `not`
//! inverts a single matcher; `all_of` and `any_of` take a *tuple* of matchers
//! (arities 2 through 8) and require, respectively, that every one or at least
//! one of them holds. Each combinator's [`Description`] is built from its
//! children's, through the `!`/`and`/`or` combinators on [`Description`]
//! (PROJECT_BUILD_PLAN.md §8, Iteration 3.1).

use std::fmt;

use crate::description::Description;
use crate::matcher::{MatchResult, Matcher, Mismatch};

/// The matcher behind [`not`]: inverts the wrapped matcher.
struct NotMatcher<M> {
    inner: M,
}

impl<T, M> Matcher<T> for NotMatcher<M>
where
    T: ?Sized + fmt::Debug,
    M: Matcher<T>,
{
    fn check(&self, actual: &T) -> MatchResult {
        if self.inner.check(actual).matched {
            // The inner matcher matched, so `not` fails. The inner pass
            // carried no `Mismatch`, hence no rendered actual; render it here,
            // which is why `not` needs `T: Debug`.
            MatchResult::fail(Mismatch::new(self.description(), format!("{actual:?}")))
        } else {
            MatchResult::pass()
        }
    }

    fn description(&self) -> Description {
        !self.inner.description()
    }
}

/// Matches when `matcher` does *not* match.
///
/// Negating a matcher is the composable alternative to
/// [`to_not`](crate::Subject::to_not): `not` is itself a matcher, so it nests
/// inside other combinators (`all_of((not(eq(0)), lt(100)))`).
///
/// ```
/// use test_better_core::TestResult;
/// use test_better_matchers::{eq, expect, not};
///
/// fn main() -> TestResult {
///     expect!(5).to(not(eq(4)))?;
///     expect!(4).to_not(not(eq(4)))?;
///     Ok(())
/// }
/// ```
pub fn not<T, M>(matcher: M) -> impl Matcher<T>
where
    T: ?Sized + fmt::Debug,
    M: Matcher<T>,
{
    NotMatcher { inner: matcher }
}

/// A tuple of matchers, all targeting the same type `T`.
///
/// Implemented for tuples of arity 2 through 8 by a macro in this module; you
/// do not implement it yourself. It is the input to [`all_of`] and [`any_of`],
/// which interpret the tuple under conjunction and disjunction respectively.
pub trait MatcherTuple<T: ?Sized> {
    /// Every matcher in the tuple must match. Returns the first sub-matcher's
    /// failure, so the error pinpoints which expectation broke.
    fn check_all(&self, actual: &T) -> MatchResult;

    /// At least one matcher in the tuple must match. When none do, the failure
    /// describes the whole disjunction.
    fn check_any(&self, actual: &T) -> MatchResult;

    /// The conjunction (`a and b and ...`) of the tuple's descriptions.
    fn describe_all(&self) -> Description;

    /// The disjunction (`a or b or ...`) of the tuple's descriptions.
    fn describe_any(&self) -> Description;
}

/// Implements [`MatcherTuple`] for one tuple arity. The first type parameter is
/// split out from the rest so the description fold and the `check_any` actual
/// capture have a guaranteed first element without an `unwrap`.
macro_rules! impl_matcher_tuple {
    ($first:ident, $($rest:ident),+) => {
        #[allow(non_snake_case)]
        impl<T, $first, $($rest,)+> MatcherTuple<T> for ($first, $($rest,)+)
        where
            T: ?Sized,
            $first: Matcher<T>,
            $($rest: Matcher<T>,)+
        {
            fn check_all(&self, actual: &T) -> MatchResult {
                let ($first, $($rest,)+) = self;
                if let Some(mismatch) = $first.check(actual).failure {
                    return MatchResult::fail(mismatch);
                }
                $(
                    if let Some(mismatch) = $rest.check(actual).failure {
                        return MatchResult::fail(mismatch);
                    }
                )+
                MatchResult::pass()
            }

            fn check_any(&self, actual: &T) -> MatchResult {
                let ($first, $($rest,)+) = self;
                let first_actual = match $first.check(actual).failure {
                    None => return MatchResult::pass(),
                    Some(mismatch) => mismatch.actual,
                };
                $(
                    if $rest.check(actual).matched {
                        return MatchResult::pass();
                    }
                )+
                MatchResult::fail(Mismatch::new(self.describe_any(), first_actual))
            }

            fn describe_all(&self) -> Description {
                let ($first, $($rest,)+) = self;
                let desc = $first.description();
                $( let desc = desc.and($rest.description()); )+
                desc
            }

            fn describe_any(&self) -> Description {
                let ($first, $($rest,)+) = self;
                let desc = $first.description();
                $( let desc = desc.or($rest.description()); )+
                desc
            }
        }
    };
}

impl_matcher_tuple!(M1, M2);
impl_matcher_tuple!(M1, M2, M3);
impl_matcher_tuple!(M1, M2, M3, M4);
impl_matcher_tuple!(M1, M2, M3, M4, M5);
impl_matcher_tuple!(M1, M2, M3, M4, M5, M6);
impl_matcher_tuple!(M1, M2, M3, M4, M5, M6, M7);
impl_matcher_tuple!(M1, M2, M3, M4, M5, M6, M7, M8);

/// The matcher behind [`all_of`]: conjunction over a tuple of matchers.
struct AllOfMatcher<Tup> {
    matchers: Tup,
}

impl<T, Tup> Matcher<T> for AllOfMatcher<Tup>
where
    T: ?Sized,
    Tup: MatcherTuple<T>,
{
    fn check(&self, actual: &T) -> MatchResult {
        self.matchers.check_all(actual)
    }

    fn description(&self) -> Description {
        self.matchers.describe_all()
    }
}

/// Matches when *every* matcher in the tuple matches.
///
/// On failure the error is the first sub-matcher's, so it names the specific
/// expectation that broke rather than the whole conjunction.
///
/// ```
/// use test_better_core::TestResult;
/// use test_better_matchers::{all_of, expect, gt, lt};
///
/// fn main() -> TestResult {
///     expect!(50).to(all_of((gt(0), lt(100))))?;
///     Ok(())
/// }
/// ```
pub fn all_of<T, Tup>(matchers: Tup) -> impl Matcher<T>
where
    T: ?Sized,
    Tup: MatcherTuple<T>,
{
    AllOfMatcher { matchers }
}

/// The matcher behind [`any_of`]: disjunction over a tuple of matchers.
struct AnyOfMatcher<Tup> {
    matchers: Tup,
}

impl<T, Tup> Matcher<T> for AnyOfMatcher<Tup>
where
    T: ?Sized,
    Tup: MatcherTuple<T>,
{
    fn check(&self, actual: &T) -> MatchResult {
        self.matchers.check_any(actual)
    }

    fn description(&self) -> Description {
        self.matchers.describe_any()
    }
}

/// Matches when *at least one* matcher in the tuple matches.
///
/// When none match, the failure describes the whole disjunction (`a or b or
/// ...`).
///
/// ```
/// use test_better_core::TestResult;
/// use test_better_matchers::{any_of, eq, expect};
///
/// fn main() -> TestResult {
///     expect!(7).to(any_of((eq(7), eq(8), eq(9))))?;
///     Ok(())
/// }
/// ```
pub fn any_of<T, Tup>(matchers: Tup) -> impl Matcher<T>
where
    T: ?Sized,
    Tup: MatcherTuple<T>,
{
    AnyOfMatcher { matchers }
}

#[cfg(test)]
mod tests {
    use test_better_core::{OrFail, TestResult};

    use super::*;
    use crate::{eq, expect, gt, is_false, is_true, lt};

    #[test]
    fn not_inverts_the_inner_matcher() -> TestResult {
        expect!(not(eq(4)).check(&5).matched).to(is_true())?;
        expect!(not(eq(4)).check(&4).matched).to(is_false())?;
        Ok(())
    }

    #[test]
    fn not_failure_negates_the_description_and_renders_the_actual() -> TestResult {
        let failure = not(eq(4))
            .check(&4)
            .failure
            .or_fail_with("4 does match eq(4), so not(eq(4)) fails")?;
        expect!(failure.expected.to_string()).to(eq("not equal to 4".to_string()))?;
        expect!(failure.actual).to(eq("4".to_string()))?;
        Ok(())
    }

    #[test]
    fn all_of_passes_when_every_matcher_matches() -> TestResult {
        expect!(all_of((gt(0), lt(100))).check(&50).matched).to(is_true())?;
        Ok(())
    }

    #[test]
    fn all_of_fails_with_the_first_failing_sub_matcher() -> TestResult {
        let failure = all_of((gt(0), lt(100)))
            .check(&150)
            .failure
            .or_fail_with("150 is not less than 100")?;
        expect!(failure.expected.to_string()).to(eq("less than 100".to_string()))?;
        expect!(failure.actual).to(eq("150".to_string()))?;
        Ok(())
    }

    #[test]
    fn all_of_describes_itself_as_a_conjunction() -> TestResult {
        let description = all_of((gt(0), lt(100))).description();
        expect!(description.to_string()).to(eq("greater than 0 and less than 100".to_string()))?;
        Ok(())
    }

    #[test]
    fn any_of_passes_when_at_least_one_matcher_matches() -> TestResult {
        expect!(any_of((eq(7), eq(8), eq(9))).check(&8).matched).to(is_true())?;
        Ok(())
    }

    #[test]
    fn any_of_fails_when_no_matcher_matches() -> TestResult {
        let failure = any_of((eq(7), eq(8), eq(9)))
            .check(&1)
            .failure
            .or_fail_with("1 is none of 7, 8, 9")?;
        expect!(failure.expected.to_string())
            .to(eq("equal to 7 or equal to 8 or equal to 9".to_string()))?;
        expect!(failure.actual).to(eq("1".to_string()))?;
        Ok(())
    }

    #[test]
    fn combinators_nest() -> TestResult {
        // `not` is itself a matcher, so it composes inside `all_of`.
        expect!(all_of((not(eq(0)), lt(100))).check(&50).matched).to(is_true())?;
        expect!(all_of((not(eq(0)), lt(100))).check(&0).matched).to(is_false())?;
        Ok(())
    }

    #[test]
    fn all_of_supports_arity_eight() -> TestResult {
        let matcher = all_of((gt(0), lt(100), gt(1), lt(99), gt(2), lt(98), gt(3), lt(97)));
        expect!(matcher.check(&50).matched).to(is_true())?;
        Ok(())
    }
}
