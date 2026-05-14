//! Worked examples of writing custom matchers for `test-better`.
//!
//! This crate is the runnable companion to the `test_better::cookbook` module.
//! It shows the three ways to give a test suite its own matcher vocabulary:
//!
//! 1. [`define_matcher!`] for the common predicate-plus-description case
//!    ([`is_freezing`], [`warmer_than`]);
//! 2. a hand-written `impl Matcher<T>` for full control over the failure
//!    message ([`is_freezing_reading`]);
//! 3. a matcher that takes an inner matcher and applies it to a projection
//!    ([`as_celsius`]).
//!
//! The domain type throughout is [`Temperature`], a temperature reading in
//! degrees Celsius. Run the suite with `cargo test -p custom-matcher-example`.

use test_better::define_matcher;
use test_better::{Description, MatchResult, Matcher, Mismatch};

/// A temperature reading, in degrees Celsius.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Temperature(pub f64);

// 1. The declarative shortcut. `define_matcher!` writes the matcher type, its
//    `Matcher` impl, and the constructor function from a predicate and a
//    description. This is the right tool whenever the matcher is "a yes/no
//    question with a fixed name".

define_matcher! {
    /// Matches a temperature, in degrees Celsius, at or below freezing.
    pub fn is_freezing for f64 {
        expects: "a temperature at or below 0\u{b0}C",
        matches: |celsius| *celsius <= 0.0,
    }
}

define_matcher! {
    /// Matches a temperature strictly warmer than `floor` degrees Celsius.
    pub fn warmer_than(floor: f64) for f64 {
        expects: format!("a temperature warmer than {floor}\u{b0}C"),
        matches: |celsius| *celsius > floor,
    }
}

// 2. The hand-written form. Implementing `Matcher` directly is more code, but
//    `check` decides exactly what the failure message says: here it explains
//    *why* the reading missed, in domain terms.

struct IsFreezingReading;

impl Matcher<Temperature> for IsFreezingReading {
    fn check(&self, actual: &Temperature) -> MatchResult {
        if actual.0 <= 0.0 {
            MatchResult::pass()
        } else {
            MatchResult::fail(Mismatch::new(
                self.description(),
                format!("{:.1}\u{b0}C, which is above freezing", actual.0),
            ))
        }
    }

    fn description(&self) -> Description {
        Description::text("a temperature at or below 0\u{b0}C")
    }
}

/// Matches a [`Temperature`] reading at or below freezing.
///
/// The hand-written counterpart to [`is_freezing`]: same expectation, but the
/// failure message is phrased for the [`Temperature`] domain type.
#[must_use]
pub fn is_freezing_reading() -> impl Matcher<Temperature> {
    IsFreezingReading
}

// 3. A matcher that takes an inner matcher. `as_celsius` adapts any
//    `Matcher<f64>` to a `Matcher<Temperature>` by projecting onto the inner
//    value, wrapping a nested failure in a `labeled` description so the output
//    keeps the layer that failed.

struct AsCelsius<M>(M);

impl<M: Matcher<f64>> Matcher<Temperature> for AsCelsius<M> {
    fn check(&self, actual: &Temperature) -> MatchResult {
        let inner = self.0.check(&actual.0);
        match inner.failure {
            None => MatchResult::pass(),
            Some(mismatch) => MatchResult::fail(Mismatch {
                expected: Description::labeled("degrees Celsius", mismatch.expected),
                ..mismatch
            }),
        }
    }

    fn description(&self) -> Description {
        Description::labeled("degrees Celsius", self.0.description())
    }
}

/// Applies `inner` to the underlying degrees-Celsius value of a [`Temperature`].
///
/// This lets every numeric matcher (`gt`, `between`, `close_to`, ...) be used
/// on a [`Temperature`] without a dedicated matcher for each.
pub fn as_celsius<M: Matcher<f64>>(inner: M) -> impl Matcher<Temperature> {
    AsCelsius(inner)
}

#[cfg(test)]
mod tests {
    use super::*;
    use test_better::prelude::*;

    #[test]
    fn define_matcher_predicate_matches() -> TestResult {
        expect!(-4.0_f64).to(is_freezing())?;
        expect!(20.0_f64).to_not(is_freezing())?;
        Ok(())
    }

    #[test]
    fn define_matcher_with_a_parameter_matches() -> TestResult {
        expect!(25.0_f64).to(warmer_than(18.0))?;
        expect!(10.0_f64).to_not(warmer_than(18.0))?;
        Ok(())
    }

    #[test]
    fn define_matcher_failure_reports_the_description() -> TestResult {
        let error = expect!(30.0_f64)
            .to(is_freezing())
            .expect_err("30\u{b0}C is not freezing");
        expect!(error.to_string().contains("at or below 0\u{b0}C")).to(is_true())?;
        Ok(())
    }

    #[test]
    fn hand_written_matcher_matches() -> TestResult {
        expect!(Temperature(-1.0)).to(is_freezing_reading())?;
        expect!(Temperature(5.0)).to_not(is_freezing_reading())?;
        Ok(())
    }

    #[test]
    fn hand_written_matcher_failure_explains_why() -> TestResult {
        let error = expect!(Temperature(5.0))
            .to(is_freezing_reading())
            .expect_err("5\u{b0}C is above freezing");
        expect!(error.to_string().contains("above freezing")).to(is_true())?;
        Ok(())
    }

    #[test]
    fn inner_matcher_adapter_matches() -> TestResult {
        expect!(Temperature(21.5)).to(as_celsius(gt(0.0)))?;
        expect!(Temperature(21.5)).to(as_celsius(between(20.0, 25.0)))?;
        Ok(())
    }

    #[test]
    fn inner_matcher_adapter_failure_keeps_the_layer() -> TestResult {
        let error = expect!(Temperature(-3.0))
            .to(as_celsius(gt(0.0)))
            .expect_err("-3\u{b0}C is not greater than 0");
        expect!(error.to_string().contains("degrees Celsius")).to(is_true())?;
        Ok(())
    }

    #[test]
    fn custom_matchers_compose_with_the_built_in_combinators() -> TestResult {
        // A custom matcher is an ordinary `Matcher`, so `not` and the rest work.
        expect!(15.0_f64).to(not(is_freezing()))?;
        Ok(())
    }
}
