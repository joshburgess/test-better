//! Acceptance tests for soft assertions (`soft`, PROJECT_BUILD_PLAN.md
//! Iteration 4.1), exercised through the `test-better` facade.

use test_better::prelude::*;

#[test]
fn soft_scope_with_no_failures_passes() -> TestResult {
    soft(|s| {
        s.expect(&2, eq(2));
        s.expect(&"alice", eq("alice"));
        s.check(Ok(()));
    })?;
    Ok(())
}

#[test]
fn soft_scope_reports_every_failure_with_its_own_location() -> TestResult {
    let result = soft(|s| {
        s.expect(&1, eq(2));
        s.expect(&3, eq(4));
        s.expect(&5, eq(6));
    });
    let error = result.expect_err("three soft assertions failed");
    let rendered = error.to_string();

    // All three failures are present, each rendering its own actual value.
    expect!(rendered.contains("3 failures")).to(is_true())?;
    expect!(rendered.contains("actual: 1")).to(is_true())?;
    expect!(rendered.contains("actual: 3")).to(is_true())?;
    expect!(rendered.contains("actual: 5")).to(is_true())?;

    // ...and a distinct source location for each.
    let locations = rendered
        .lines()
        .filter(|line| line.trim_start().starts_with("at "))
        .count();
    // One `at` line per recorded failure, plus the wrapping error's own.
    expect!(locations).to(ge(3usize))?;
    Ok(())
}

#[test]
fn soft_scope_collects_propagated_errors_via_check() -> TestResult {
    let result = soft(|s| {
        s.check(expect!(2 + 2).to(eq(4)));
        s.check(expect!(2 + 2).to(eq(5)));
    });
    let error = result.expect_err("one of the checked results failed");
    expect!(error.to_string().contains("1 failure")).to(is_true())?;
    Ok(())
}
