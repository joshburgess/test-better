//! The `property!` macro: a property test written as a closure.
//!
//! [`property!`] is a thin syntactic wrapper over [`check`](crate::check). It
//! takes a closure with a typed binding, infers a [`Strategy`](crate::Strategy)
//! from that type (or takes one explicitly via a `using` clause), runs the
//! property, and turns a [`PropertyFailure`](crate::PropertyFailure) into a
//! [`TestError`] so the call site is an ordinary `?`-returning expression.
//!
//! The shrunk-failure *rendering* lives in [`render_failure`]: the matcher's
//! own failure is kept whole, and context frames naming the case count, the
//! original failing input, and the shrunk minimal input are wrapped around it
//! (Iteration 6.3). A golden-file test (`tests/shrink_output.rs`) pins the
//! exact output.

use std::fmt::Debug;

use test_better_core::{ContextFrame, ErrorKind, TestError, TestResult};

use crate::{PropertyFailure, Strategy, check};

/// Runs a property and renders any counterexample as a [`TestError`].
///
/// This is the function [`property!`] expands to; it is the seam between the
/// macro's syntax and the [`check`] runner. It is `#[doc(hidden)]` plumbing,
/// not part of the curated surface: write `property!(...)`, or call [`check`]
/// directly for the structured [`PropertyFailure`].
#[doc(hidden)]
pub fn run_property<T, S, F>(strategy: S, property: F) -> TestResult
where
    S: Strategy<T>,
    T: Clone + Debug,
    F: FnMut(T) -> TestResult,
{
    match check(strategy, property) {
        Ok(()) => Ok(()),
        Err(failure) => Err(render_failure(failure)),
    }
}

/// Turns the structured [`PropertyFailure`] into a rendered [`TestError`].
///
/// The matcher's own failure is kept whole: its message and payload (the
/// "structured `Description`" Iteration 6.3 calls for) are left untouched.
/// Three context frames are wrapped around it, outermost-first: the property
/// summary and case count, the original failing input, and the shrunk minimal
/// input. The kind is promoted to [`ErrorKind::Property`] so the failure reads
/// as a property failure, not a bare assertion.
///
/// `#[doc(hidden)]` plumbing: [`run_property`] (and so [`property!`]) call it,
/// and the golden-file test pins its output. Callers wanting the structured
/// failure use [`check`] and read [`PropertyFailure`] directly.
#[doc(hidden)]
pub fn render_failure<T: Debug>(failure: PropertyFailure<T>) -> TestError {
    let PropertyFailure {
        original,
        shrunk,
        failure,
        cases,
    } = failure;
    let plural = if cases == 1 { "" } else { "s" };
    let mut error = failure;
    error.kind = ErrorKind::Property;
    error.push_context(ContextFrame::new(format!(
        "checking a property; it failed after {cases} generated case{plural}"
    )));
    error.push_context(ContextFrame::new(format!(
        "the original failing input was {original:?}"
    )));
    error.push_context(ContextFrame::new(format!(
        "the shrunk (minimal) input is {shrunk:?}"
    )));
    error
}

/// Checks that a property holds for every generated input.
///
/// `property!` takes a closure with a typed binding and a block body that
/// returns [`TestResult`](test_better_core::TestResult), runs it against
/// generated values, and on failure produces a `TestError` naming the shrunk
/// counterexample. It expands to an expression, so it is the body (or the tail)
/// of an ordinary `#[test]` function:
///
/// ```
/// use test_better_core::TestResult;
/// use test_better_matchers::{expect, lt};
/// use test_better_property::property;
///
/// // In a real test this is `#[test] fn doubling_stays_in_range()`.
/// # fn main() -> TestResult {
/// property!(|n: u8| {
///     expect!(u16::from(n) * 2).to(lt(512u16))
/// })
/// # }
/// ```
///
/// # Inferring vs. naming the strategy
///
/// With only a typed binding, the strategy is inferred from the type via
/// [`any`](crate::any) (the type must be `proptest::arbitrary::Arbitrary`). To
/// generate from a specific strategy instead, add a trailing `using` clause:
///
/// ```
/// use test_better_core::TestResult;
/// use test_better_matchers::{expect, lt};
/// use test_better_property::property;
///
/// # fn main() -> TestResult {
/// // `using` names the strategy explicitly; the binding need not be annotated.
/// property!(|n| {
///     expect!(n).to(lt(100u32))
/// } using 0u32..100)
/// # }
/// ```
#[macro_export]
macro_rules! property {
    // Typed binding, strategy inferred from the type.
    (| $name:ident : $ty:ty | $body:block) => {
        $crate::run_property($crate::any::<$ty>(), |$name: $ty| $body)
    };
    // Typed binding, explicit strategy via a trailing `using` clause.
    (| $name:ident : $ty:ty | $body:block using $strategy:expr) => {
        $crate::run_property($strategy, |$name: $ty| $body)
    };
    // Bare binding, explicit strategy: the type comes from the strategy.
    (| $name:ident | $body:block using $strategy:expr) => {
        $crate::run_property($strategy, |$name| $body)
    };
}

#[cfg(test)]
mod tests {
    use test_better_core::{OrFail, TestResult};
    use test_better_matchers::{eq, expect, ge, is_true, lt};

    #[test]
    fn an_inferred_strategy_property_that_holds_passes() -> TestResult {
        // `u8` is `Arbitrary`, so the strategy is inferred from the binding.
        property!(|n: u8| { expect!(u16::from(n) + 1).to(ge(1u16)) })
    }

    #[test]
    fn a_using_clause_names_the_strategy_explicitly() -> TestResult {
        // The binding is bare; the type comes from the `using` strategy.
        property!(|n| {
            expect!(n).to(lt(50u64))
        } using 0u64..50)
    }

    #[test]
    fn a_failing_property_renders_a_property_kind_error_naming_the_shrunk_input() -> TestResult {
        // "every u32 is below 100" is false; the macro must surface a
        // `Property`-kind failure that names the original and shrunk
        // counterexamples and still carries the matcher's own description.
        let error = property!(|n: u32| {
            expect!(n).to(lt(100u32))
        } using proptest::num::u32::ANY)
        .err()
        .or_fail_with("a property false for most u32 must fail")?;
        let rendered = error.to_string();
        // The shrunk counterexample (proptest shrinks to exactly 100) is named.
        expect!(rendered.contains("the shrunk (minimal) input is 100")).to(is_true())?;
        // The original failing input is named too.
        expect!(rendered.contains("the original failing input was")).to(is_true())?;
        // The matcher's full description survives.
        expect!(rendered.contains("less than 100")).to(is_true())?;
        // And the failure reads as a property failure.
        expect!(error.kind).to(eq(test_better_core::ErrorKind::Property))
    }
}
