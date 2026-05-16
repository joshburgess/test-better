//! The [`predicate`] escape hatch: a matcher built from an arbitrary
//! Boolean-returning closure.
//!
//! When no standard matcher fits, `predicate` wraps a `Fn(&T) -> bool`. It
//! takes a `name` so a failure reads `expected: even` rather than the useless
//! `expected: <closure>`.

use std::fmt;
use std::marker::PhantomData;

use crate::description::Description;
use crate::matcher::{MatchResult, Matcher, Mismatch};

/// The matcher behind [`predicate`].
struct PredicateMatcher<T, F> {
    name: &'static str,
    pred: F,
    // `T` appears only behind `&T` in `F`'s signature, not in a field, so it
    // would otherwise be an unconstrained type parameter on the struct.
    _marker: PhantomData<fn(&T)>,
}

impl<T, F> Matcher<T> for PredicateMatcher<T, F>
where
    T: fmt::Debug,
    F: Fn(&T) -> bool,
{
    fn check(&self, actual: &T) -> MatchResult {
        if (self.pred)(actual) {
            MatchResult::pass()
        } else {
            MatchResult::fail(Mismatch::new(self.description(), format!("{actual:?}")))
        }
    }

    fn description(&self) -> Description {
        Description::text(self.name)
    }
}

/// Matches a value for which `pred` returns `true`.
///
/// The escape hatch for when no standard matcher fits. `name` is what the
/// failure reports as the expectation, so give it a readable one: a closure has
/// no name of its own, and a failure that says `expected: <closure>` helps no
/// one.
///
/// The matcher pairs naturally with [`Subject::satisfies`](crate::Subject::satisfies),
/// reading as "x satisfies the predicate `<name>`".
///
/// ```
/// use test_better_core::{OrFail, TestResult};
/// use test_better_matchers::{Matcher, check, eq, predicate};
///
/// fn main() -> TestResult {
///     check!(4).satisfies(predicate("an even number", |n: &i32| n % 2 == 0))?;
///
///     // The `name` is what a failure reports, not `<closure>`.
///     let failure = predicate("an even number", |n: &i32| n % 2 == 0)
///         .check(&3)
///         .failure
///         .or_fail_with("3 is not even")?;
///     check!(failure.expected.to_string()).satisfies(eq("an even number".to_string()))?;
///     Ok(())
/// }
/// ```
#[must_use]
pub fn predicate<T, F>(name: &'static str, pred: F) -> impl Matcher<T>
where
    T: fmt::Debug,
    F: Fn(&T) -> bool,
{
    PredicateMatcher {
        name,
        pred,
        _marker: PhantomData,
    }
}

#[cfg(test)]
mod tests {
    use test_better_core::{OrFail, TestResult};

    use super::*;
    use crate::{check, eq, is_false, is_true};

    #[test]
    fn predicate_runs_the_closure() -> TestResult {
        check!(predicate("even", |n: &i32| n % 2 == 0).check(&4).matched).satisfies(is_true())?;
        check!(predicate("even", |n: &i32| n % 2 == 0).check(&3).matched).satisfies(is_false())?;
        Ok(())
    }

    #[test]
    fn predicate_failure_reports_the_name_not_the_closure() -> TestResult {
        let failure = predicate("a positive number", |n: &i32| *n > 0)
            .check(&-1)
            .failure
            .or_fail_with("-1 is not positive")?;
        check!(failure.expected.to_string()).satisfies(eq("a positive number".to_string()))?;
        check!(failure.actual).satisfies(eq("-1".to_string()))?;
        Ok(())
    }
}
