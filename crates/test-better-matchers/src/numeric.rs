//! Numeric matchers for floating-point values: [`close_to`], [`between`],
//! [`is_nan`], and [`is_finite`].
//!
//! These are generic over the [`Float`] trait, which is *sealed*: it is
//! implemented for `f32` and `f64` and cannot be implemented downstream.

use std::fmt;

use crate::description::Description;
use crate::matcher::{MatchResult, Matcher, Mismatch};

mod sealed {
    pub trait Sealed {}
    impl Sealed for f32 {}
    impl Sealed for f64 {}
}

/// A floating-point type the numeric matchers operate on.
///
/// Sealed: implemented for `f32` and `f64` only, so adding a method here is
/// never a breaking change for downstream code.
pub trait Float: sealed::Sealed + Copy + PartialOrd + fmt::Debug {
    /// The absolute difference between `self` and `other`.
    fn abs_diff(self, other: Self) -> Self;

    /// Whether `self` is `NaN`.
    fn float_is_nan(self) -> bool;

    /// Whether `self` is neither infinite nor `NaN`.
    fn float_is_finite(self) -> bool;
}

impl Float for f32 {
    fn abs_diff(self, other: Self) -> Self {
        (self - other).abs()
    }

    fn float_is_nan(self) -> bool {
        self.is_nan()
    }

    fn float_is_finite(self) -> bool {
        self.is_finite()
    }
}

impl Float for f64 {
    fn abs_diff(self, other: Self) -> Self {
        (self - other).abs()
    }

    fn float_is_nan(self) -> bool {
        self.is_nan()
    }

    fn float_is_finite(self) -> bool {
        self.is_finite()
    }
}

/// The matcher behind [`close_to`].
struct CloseToMatcher<F> {
    value: F,
    tolerance: F,
}

impl<F: Float> Matcher<F> for CloseToMatcher<F> {
    fn check(&self, actual: &F) -> MatchResult {
        let diff = actual.abs_diff(self.value);
        // A `NaN` actual makes `diff` `NaN`, and `NaN <= tolerance` is false,
        // so `NaN` correctly fails to be close to anything.
        if diff <= self.tolerance {
            MatchResult::pass()
        } else {
            MatchResult::fail(Mismatch::new(
                self.description(),
                format!("{actual:?} (off by {diff:?})"),
            ))
        }
    }

    fn description(&self) -> Description {
        Description::text(format!("within {:?} of {:?}", self.tolerance, self.value))
    }
}

/// Matches a float within `tolerance` of `value` (the comparison is
/// `|actual - value| <= tolerance`).
///
/// ```
/// use test_better_core::TestResult;
/// use test_better_matchers::{close_to, expect};
///
/// fn main() -> TestResult {
///     expect!(0.1_f64 + 0.2).to(close_to(0.3, 1e-9))?;
///     Ok(())
/// }
/// ```
#[must_use]
pub fn close_to<F: Float>(value: F, tolerance: F) -> impl Matcher<F> {
    CloseToMatcher { value, tolerance }
}

/// The matcher behind [`between`].
struct BetweenMatcher<F> {
    low: F,
    high: F,
}

impl<F: Float> Matcher<F> for BetweenMatcher<F> {
    fn check(&self, actual: &F) -> MatchResult {
        if self.low <= *actual && *actual <= self.high {
            MatchResult::pass()
        } else {
            MatchResult::fail(Mismatch::new(self.description(), format!("{actual:?}")))
        }
    }

    fn description(&self) -> Description {
        Description::text(format!(
            "between {:?} and {:?} (inclusive)",
            self.low, self.high
        ))
    }
}

/// Matches a float in the inclusive range `low..=high`.
///
/// ```
/// use test_better_core::TestResult;
/// use test_better_matchers::{between, expect};
///
/// fn main() -> TestResult {
///     expect!(2.5_f64).to(between(0.0, 5.0))?;
///     Ok(())
/// }
/// ```
#[must_use]
pub fn between<F: Float>(low: F, high: F) -> impl Matcher<F> {
    BetweenMatcher { low, high }
}

/// The matcher behind [`is_nan`].
struct IsNanMatcher;

impl<F: Float> Matcher<F> for IsNanMatcher {
    fn check(&self, actual: &F) -> MatchResult {
        if actual.float_is_nan() {
            MatchResult::pass()
        } else {
            MatchResult::fail(Mismatch::new(
                Description::text("NaN"),
                format!("{actual:?}"),
            ))
        }
    }

    fn description(&self) -> Description {
        Description::text("NaN")
    }
}

/// Matches a float that is `NaN`.
///
/// ```
/// use test_better_core::TestResult;
/// use test_better_matchers::{expect, is_nan};
///
/// fn main() -> TestResult {
///     expect!(f64::NAN).to(is_nan())?;
///     expect!(1.0_f64).to_not(is_nan())?;
///     Ok(())
/// }
/// ```
#[must_use]
pub fn is_nan<F: Float>() -> impl Matcher<F> {
    IsNanMatcher
}

/// The matcher behind [`is_finite`].
struct IsFiniteMatcher;

impl<F: Float> Matcher<F> for IsFiniteMatcher {
    fn check(&self, actual: &F) -> MatchResult {
        if actual.float_is_finite() {
            MatchResult::pass()
        } else {
            MatchResult::fail(Mismatch::new(
                Description::text("a finite number"),
                format!("{actual:?}"),
            ))
        }
    }

    fn description(&self) -> Description {
        Description::text("a finite number")
    }
}

/// Matches a float that is finite (neither infinite nor `NaN`).
///
/// ```
/// use test_better_core::TestResult;
/// use test_better_matchers::{expect, is_finite};
///
/// fn main() -> TestResult {
///     expect!(1.5_f64).to(is_finite())?;
///     expect!(f64::INFINITY).to_not(is_finite())?;
///     Ok(())
/// }
/// ```
#[must_use]
pub fn is_finite<F: Float>() -> impl Matcher<F> {
    IsFiniteMatcher
}

#[cfg(test)]
mod tests {
    use test_better_core::{OrFail, TestResult};

    use super::*;
    use crate::{eq, expect, is_false, is_true};

    #[test]
    fn close_to_respects_the_tolerance() -> TestResult {
        expect!(close_to(0.3, 1e-9).check(&(0.1_f64 + 0.2)).matched).to(is_true())?;
        expect!(close_to(0.3_f64, 1e-9).check(&0.4).matched).to(is_false())?;
        // The tolerance is the boundary, inclusive.
        expect!(close_to(1.0_f64, 0.5).check(&1.5).matched).to(is_true())?;
        expect!(close_to(1.0_f64, 0.5).check(&1.6).matched).to(is_false())?;
        Ok(())
    }

    #[test]
    fn close_to_failure_shows_the_tolerance_and_the_difference() -> TestResult {
        let failure = close_to(1.0_f64, 0.1)
            .check(&2.0)
            .failure
            .or_fail_with("2.0 is not within 0.1 of 1.0")?;
        expect!(failure.expected.to_string()).to(eq("within 0.1 of 1.0".to_string()))?;
        expect!(failure.actual.contains("off by")).to(is_true())?;
        Ok(())
    }

    #[test]
    fn between_is_an_inclusive_range() -> TestResult {
        expect!(between(0.0_f64, 5.0).check(&0.0).matched).to(is_true())?;
        expect!(between(0.0_f64, 5.0).check(&5.0).matched).to(is_true())?;
        expect!(between(0.0_f64, 5.0).check(&5.1).matched).to(is_false())?;
        expect!(between(0.0_f64, 5.0).check(&-0.1).matched).to(is_false())?;
        Ok(())
    }

    #[test]
    fn is_nan_matches_only_nan() -> TestResult {
        expect!(is_nan().check(&f64::NAN).matched).to(is_true())?;
        expect!(is_nan().check(&1.0_f64).matched).to(is_false())?;
        // A `NaN` is never close to anything, including itself.
        expect!(close_to(f64::NAN, 1.0).check(&f64::NAN).matched).to(is_false())?;
        Ok(())
    }

    #[test]
    fn is_finite_rejects_infinities_and_nan() -> TestResult {
        expect!(is_finite().check(&1.5_f64).matched).to(is_true())?;
        expect!(is_finite().check(&f64::INFINITY).matched).to(is_false())?;
        expect!(is_finite().check(&f64::NEG_INFINITY).matched).to(is_false())?;
        expect!(is_finite().check(&f64::NAN).matched).to(is_false())?;
        Ok(())
    }

    #[test]
    fn numeric_matchers_work_for_f32_too() -> TestResult {
        expect!(close_to(1.0_f32, 0.01).check(&1.005).matched).to(is_true())?;
        expect!(between(0.0_f32, 1.0).check(&0.5).matched).to(is_true())?;
        expect!(is_nan().check(&f32::NAN).matched).to(is_true())?;
        Ok(())
    }
}
