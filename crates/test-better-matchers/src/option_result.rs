//! Matchers for [`Option`] and [`Result`]: [`some`], [`none`], [`ok`], and
//! [`err`].
//!
//! `some`, `ok`, and `err` each take an *inner* matcher and apply it to the
//! wrapped value, so they nest: `some(ok(eq(42)))` matches `Some(Ok(42))`. When
//! an inner matcher fails, its expectation is wrapped in a
//! [`Description::labeled`] layer, so a nested failure renders as aligned,
//! indented `some:` / `ok:` blocks.

use std::fmt;

use crate::description::Description;
use crate::matcher::{MatchResult, Matcher, Mismatch};

/// Wraps an inner matcher's failure in a `label:`-headed [`Description`] layer,
/// keeping the inner actual and diff. This is what gives nested matchers their
/// aligned, indented expected blocks.
fn wrap_failure(label: &'static str, inner: Mismatch) -> MatchResult {
    MatchResult::fail(Mismatch {
        expected: Description::labeled(label, inner.expected),
        actual: inner.actual,
        diff: inner.diff,
    })
}

/// The matcher behind [`some`].
struct SomeMatcher<M> {
    inner: M,
}

impl<T, M> Matcher<Option<T>> for SomeMatcher<M>
where
    M: Matcher<T>,
{
    fn check(&self, actual: &Option<T>) -> MatchResult {
        match actual {
            Some(value) => match self.inner.check(value).failure {
                None => MatchResult::pass(),
                Some(inner) => wrap_failure("some", inner),
            },
            // `Matcher::description` is spelled out: `SomeMatcher<M>`
            // implements `Matcher<Option<T>>` for a family of `T`, so a bare
            // `self.description()` is ambiguous from inside `check`.
            None => MatchResult::fail(Mismatch::new(
                Matcher::<Option<T>>::description(self),
                "None",
            )),
        }
    }

    fn description(&self) -> Description {
        Description::labeled("some", self.inner.description())
    }
}

/// Matches a `Some` whose contained value satisfies `inner`.
///
/// ```
/// use test_better_core::TestResult;
/// use test_better_matchers::{eq, check, some};
///
/// fn main() -> TestResult {
///     check!(Some(42)).satisfies(some(eq(42)))?;
///     check!(None::<i32>).violates(some(eq(42)))?;
///     Ok(())
/// }
/// ```
#[must_use]
pub fn some<T, M>(inner: M) -> impl Matcher<Option<T>>
where
    M: Matcher<T>,
{
    SomeMatcher { inner }
}

/// The matcher behind [`none`].
struct NoneMatcher;

impl<T> Matcher<Option<T>> for NoneMatcher
where
    T: fmt::Debug,
{
    fn check(&self, actual: &Option<T>) -> MatchResult {
        match actual {
            None => MatchResult::pass(),
            Some(_) => MatchResult::fail(Mismatch::new(
                Matcher::<Option<T>>::description(self),
                format!("{actual:?}"),
            )),
        }
    }

    fn description(&self) -> Description {
        Description::text("none")
    }
}

/// Matches `None`.
///
/// ```
/// use test_better_core::TestResult;
/// use test_better_matchers::{check, none};
///
/// fn main() -> TestResult {
///     check!(None::<i32>).satisfies(none())?;
///     check!(Some(0)).violates(none())?;
///     Ok(())
/// }
/// ```
#[must_use]
pub fn none<T>() -> impl Matcher<Option<T>>
where
    T: fmt::Debug,
{
    NoneMatcher
}

/// The matcher behind [`ok`].
struct OkMatcher<M> {
    inner: M,
}

impl<T, E, M> Matcher<Result<T, E>> for OkMatcher<M>
where
    M: Matcher<T>,
    E: fmt::Debug,
{
    fn check(&self, actual: &Result<T, E>) -> MatchResult {
        match actual {
            Ok(value) => match self.inner.check(value).failure {
                None => MatchResult::pass(),
                Some(inner) => wrap_failure("ok", inner),
            },
            Err(error) => MatchResult::fail(Mismatch::new(
                Matcher::<Result<T, E>>::description(self),
                format!("Err({error:?})"),
            )),
        }
    }

    fn description(&self) -> Description {
        Description::labeled("ok", self.inner.description())
    }
}

/// Matches an `Ok` whose contained value satisfies `inner`.
///
/// ```
/// use test_better_core::TestResult;
/// use test_better_matchers::{eq, check, ok};
///
/// fn main() -> TestResult {
///     check!(Ok::<i32, &str>(42)).satisfies(ok(eq(42)))?;
///     check!(Err::<i32, &str>("boom")).violates(ok(eq(42)))?;
///     Ok(())
/// }
/// ```
#[must_use]
pub fn ok<T, E, M>(inner: M) -> impl Matcher<Result<T, E>>
where
    M: Matcher<T>,
    E: fmt::Debug,
{
    OkMatcher { inner }
}

/// The matcher behind [`err`].
struct ErrMatcher<M> {
    inner: M,
}

impl<T, E, M> Matcher<Result<T, E>> for ErrMatcher<M>
where
    M: Matcher<E>,
    T: fmt::Debug,
{
    fn check(&self, actual: &Result<T, E>) -> MatchResult {
        match actual {
            Err(value) => match self.inner.check(value).failure {
                None => MatchResult::pass(),
                Some(inner) => wrap_failure("err", inner),
            },
            Ok(value) => MatchResult::fail(Mismatch::new(
                Matcher::<Result<T, E>>::description(self),
                format!("Ok({value:?})"),
            )),
        }
    }

    fn description(&self) -> Description {
        Description::labeled("err", self.inner.description())
    }
}

/// Matches an `Err` whose contained value satisfies `inner`.
///
/// ```
/// use test_better_core::TestResult;
/// use test_better_matchers::{eq, err, check};
///
/// fn main() -> TestResult {
///     check!(Err::<i32, &str>("boom")).satisfies(err(eq("boom")))?;
///     check!(Ok::<i32, &str>(0)).violates(err(eq("boom")))?;
///     Ok(())
/// }
/// ```
#[must_use]
pub fn err<T, E, M>(inner: M) -> impl Matcher<Result<T, E>>
where
    M: Matcher<E>,
    T: fmt::Debug,
{
    ErrMatcher { inner }
}

#[cfg(test)]
mod tests {
    use test_better_core::{OrFail, TestResult};

    use super::*;
    use crate::{check, eq, is_false, is_true};

    #[test]
    fn some_matches_a_some_whose_value_satisfies_the_inner_matcher() -> TestResult {
        check!(some(eq(42)).check(&Some(42)).matched).satisfies(is_true())?;
        check!(some(eq(42)).check(&Some(7)).matched).satisfies(is_false())?;
        check!(some(eq(42)).check(&None).matched).satisfies(is_false())?;
        Ok(())
    }

    #[test]
    fn some_of_none_reports_none_as_the_actual() -> TestResult {
        let failure = some(eq(42))
            .check(&None)
            .failure
            .or_fail_with("None is not Some")?;
        check!(failure.expected.to_string()).satisfies(eq("some:\n  equal to 42".to_string()))?;
        check!(failure.actual).satisfies(eq("None".to_string()))?;
        Ok(())
    }

    #[test]
    fn none_matches_only_none() -> TestResult {
        check!(none::<i32>().check(&None).matched).satisfies(is_true())?;
        let failure = none()
            .check(&Some(7))
            .failure
            .or_fail_with("Some(7) is not None")?;
        check!(failure.expected.to_string()).satisfies(eq("none".to_string()))?;
        check!(failure.actual).satisfies(eq("Some(7)".to_string()))?;
        Ok(())
    }

    #[test]
    fn ok_matches_an_ok_whose_value_satisfies_the_inner_matcher() -> TestResult {
        check!(ok::<i32, &str, _>(eq(42)).check(&Ok(42)).matched).satisfies(is_true())?;
        let failure = ok::<i32, &str, _>(eq(42))
            .check(&Err("boom"))
            .failure
            .or_fail_with("Err is not Ok")?;
        check!(failure.expected.to_string()).satisfies(eq("ok:\n  equal to 42".to_string()))?;
        check!(failure.actual).satisfies(eq("Err(\"boom\")".to_string()))?;
        Ok(())
    }

    #[test]
    fn err_matches_an_err_whose_value_satisfies_the_inner_matcher() -> TestResult {
        check!(err::<i32, &str, _>(eq("boom")).check(&Err("boom")).matched).satisfies(is_true())?;
        let failure = err::<i32, &str, _>(eq("boom"))
            .check(&Ok(0))
            .failure
            .or_fail_with("Ok is not Err")?;
        check!(failure.expected.to_string())
            .satisfies(eq("err:\n  equal to \"boom\"".to_string()))?;
        check!(failure.actual).satisfies(eq("Ok(0)".to_string()))?;
        Ok(())
    }

    #[test]
    fn nested_matchers_render_aligned_indented_expected_blocks() -> TestResult {
        let matcher = some(ok::<i32, &str, _>(eq(42)));
        check!(matcher.check(&Some(Ok(42))).matched).satisfies(is_true())?;

        let failure = matcher
            .check(&Some(Ok(43)))
            .failure
            .or_fail_with("Some(Ok(43)) does not satisfy some(ok(eq(42)))")?;
        check!(failure.expected.to_string())
            .satisfies(eq("some:\n  ok:\n    equal to 42".to_string()))?;
        check!(failure.actual).satisfies(eq("43".to_string()))?;
        Ok(())
    }
}
