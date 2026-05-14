//! The [`satisfies`] escape hatch: a matcher built from an arbitrary
//! predicate.
//!
//! When no standard matcher fits, `satisfies` wraps a `Fn(&T) -> bool`. It
//! takes a `name` so a failure reads `expected: even` rather than the useless
//! `expected: <closure>`.

use std::fmt;
use std::marker::PhantomData;

use crate::description::Description;
use crate::matcher::{MatchResult, Matcher, Mismatch};

/// The matcher behind [`satisfies`].
struct SatisfiesMatcher<T, F> {
    name: &'static str,
    pred: F,
    // `T` appears only behind `&T` in `F`'s signature, not in a field, so it
    // would otherwise be an unconstrained type parameter on the struct.
    _marker: PhantomData<fn(&T)>,
}

impl<T, F> Matcher<T> for SatisfiesMatcher<T, F>
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
/// ```
/// use test_better_core::{OrFail, TestResult};
/// use test_better_matchers::{Matcher, eq, expect, satisfies};
///
/// fn main() -> TestResult {
///     expect!(4).to(satisfies("an even number", |n: &i32| n % 2 == 0))?;
///
///     // The `name` is what a failure reports, not `<closure>`.
///     let failure = satisfies("an even number", |n: &i32| n % 2 == 0)
///         .check(&3)
///         .failure
///         .or_fail_with("3 is not even")?;
///     expect!(failure.expected.to_string()).to(eq("an even number".to_string()))?;
///     Ok(())
/// }
/// ```
#[must_use]
pub fn satisfies<T, F>(name: &'static str, pred: F) -> impl Matcher<T>
where
    T: fmt::Debug,
    F: Fn(&T) -> bool,
{
    SatisfiesMatcher {
        name,
        pred,
        _marker: PhantomData,
    }
}

#[cfg(test)]
mod tests {
    use test_better_core::{OrFail, TestResult};

    use super::*;
    use crate::{eq, expect, is_false, is_true};

    #[test]
    fn satisfies_runs_the_predicate() -> TestResult {
        expect!(satisfies("even", |n: &i32| n % 2 == 0).check(&4).matched).to(is_true())?;
        expect!(satisfies("even", |n: &i32| n % 2 == 0).check(&3).matched).to(is_false())?;
        Ok(())
    }

    #[test]
    fn satisfies_failure_reports_the_name_not_the_closure() -> TestResult {
        let failure = satisfies("a positive number", |n: &i32| *n > 0)
            .check(&-1)
            .failure
            .or_fail_with("-1 is not positive")?;
        expect!(failure.expected.to_string()).to(eq("a positive number".to_string()))?;
        expect!(failure.actual).to(eq("-1".to_string()))?;
        Ok(())
    }
}
