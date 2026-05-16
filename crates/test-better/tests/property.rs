//! Acceptance tests for the property-testing seam and runner, exercised
//! through the `test-better` facade.
//!
//! These use numeric-range strategies (`0u32..1_000`), which are
//! `proptest::strategy::Strategy` and therefore seam `Strategy` values through
//! the blanket impl, with no `proptest` import at the call site. Richer
//! strategies and the `property!` macro are available in later iterations.

use test_better::prelude::*;
use test_better::{PropertyConfig, Runner, for_all, for_all_with};

#[test]
fn for_all_passes_a_property_that_holds_for_every_input() -> TestResult {
    // Doubling any value below 1000 stays below 2000.
    let outcome = for_all(0u32..1_000, |n| check!(n * 2).satisfies(lt(2_000u32)));
    check!(outcome.is_ok()).satisfies(is_true())
}

#[test]
fn for_all_reports_a_shrunk_counterexample_carrying_the_matcher_failure() -> TestResult {
    // "every value in 0..1000 is below 500" is false; `proptest` shrinks the
    // counterexample down to exactly 500, the smallest input that breaks it.
    let failure = for_all(0u32..1_000, |n| check!(n).satisfies(lt(500u32)))
        .err()
        .or_fail_with("values at or above 500 exist in 0..1000")?;
    check!(failure.shrunk).satisfies(eq(500u32))?;
    check!(failure.original).satisfies(ge(500u32))?;
    // The carried `TestError` is a full matcher failure: it names the bound.
    check!(failure.failure.to_string().contains("less than 500")).satisfies(is_true())
}

#[test]
fn for_all_with_lets_the_caller_set_the_case_count_and_runner() -> TestResult {
    let mut runner = Runner::randomized();
    let outcome = for_all_with(PropertyConfig { cases: 32 }, &mut runner, 0u64..10, |n| {
        check!(n).satisfies(lt(10u64))
    });
    check!(outcome.is_ok()).satisfies(is_true())
}

#[test]
fn property_macro_runs_an_inferred_strategy_property() -> TestResult {
    // `u32` is `Arbitrary`, so `property!` infers the strategy from the binding
    // and the whole macro call is the `TestResult`-returning test body.
    property!(|n: u32| { check!(n.wrapping_add(1)).satisfies(ne(n)) })
}

#[test]
fn property_macro_accepts_a_using_clause_for_an_explicit_strategy() -> TestResult {
    // The binding is bare; the type and the values come from the `using`
    // strategy, an ordinary numeric range.
    property!(|n| {
        check!(n).satisfies(lt(10u64))
    } using 0u64..10)
}

#[test]
fn property_macro_failure_names_the_shrunk_input_and_keeps_the_matcher_description() -> TestResult {
    // "every value in 0..1000 is below 500" is false; `property!` must surface
    // a failure that names the shrunk counterexample (proptest shrinks to 500)
    // and still carries the matcher's own description.
    let error = property!(|n: u32| {
        check!(n).satisfies(lt(500u32))
    } using 0u32..1_000)
    .err()
    .or_fail_with("values at or above 500 exist in 0..1000")?;
    let rendered = error.to_string();
    check!(rendered.contains("the shrunk (minimal) input is 500")).satisfies(is_true())?;
    check!(rendered.contains("the original failing input was")).satisfies(is_true())?;
    check!(rendered.contains("less than 500")).satisfies(is_true())
}
