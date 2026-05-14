//! Acceptance tests for the property-testing seam and runner
//! (PROJECT_BUILD_PLAN.md Iteration 6.1b), exercised through the `test-better`
//! facade.
//!
//! These use numeric-range strategies (`0u32..1_000`), which are
//! `proptest::strategy::Strategy` and therefore seam `Strategy` values through
//! the blanket impl, with no `proptest` import at the call site. Richer
//! strategies and the `property!` macro arrive in Iteration 6.2.

use test_better::prelude::*;
use test_better::{PropertyConfig, Runner, check, check_with};

#[test]
fn check_passes_a_property_that_holds_for_every_input() -> TestResult {
    // Doubling any value below 1000 stays below 2000.
    let outcome = check(0u32..1_000, |n| expect!(n * 2).to(lt(2_000u32)));
    expect!(outcome.is_ok()).to(is_true())
}

#[test]
fn check_reports_a_shrunk_counterexample_carrying_the_matcher_failure() -> TestResult {
    // "every value in 0..1000 is below 500" is false; `proptest` shrinks the
    // counterexample down to exactly 500, the smallest input that breaks it.
    let failure = check(0u32..1_000, |n| expect!(n).to(lt(500u32)))
        .err()
        .or_fail_with("values at or above 500 exist in 0..1000")?;
    expect!(failure.shrunk).to(eq(500u32))?;
    expect!(failure.original).to(ge(500u32))?;
    // The carried `TestError` is a full matcher failure: it names the bound.
    expect!(failure.failure.to_string().contains("less than 500")).to(is_true())
}

#[test]
fn check_with_lets_the_caller_set_the_case_count_and_runner() -> TestResult {
    let mut runner = Runner::randomized();
    let outcome = check_with(PropertyConfig { cases: 32 }, &mut runner, 0u64..10, |n| {
        expect!(n).to(lt(10u64))
    });
    expect!(outcome.is_ok()).to(is_true())
}
