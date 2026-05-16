//! Acceptance tests for soft assertions (`soft`), exercised through the
//! `test-better` facade.

use test_better::prelude::*;

#[test]
fn soft_scope_with_no_failures_passes() -> TestResult {
    soft(|s| {
        s.check(&2, eq(2));
        s.check(&"alice", eq("alice"));
        s.record(Ok(()));
    })?;
    Ok(())
}

#[test]
fn soft_scope_reports_every_failure_with_its_own_location() -> TestResult {
    let result = soft(|s| {
        s.check(&1, eq(2));
        s.check(&3, eq(4));
        s.check(&5, eq(6));
    });
    let error = result.expect_err("three soft assertions failed");
    let rendered = error.to_string();

    // All three failures are present, each rendering its own actual value.
    check!(rendered.contains("3 failures")).satisfies(is_true())?;
    check!(rendered.contains("actual: 1")).satisfies(is_true())?;
    check!(rendered.contains("actual: 3")).satisfies(is_true())?;
    check!(rendered.contains("actual: 5")).satisfies(is_true())?;

    // ...and a distinct source location for each.
    let locations = rendered
        .lines()
        .filter(|line| line.trim_start().starts_with("at "))
        .count();
    // One `at` line per recorded failure, plus the wrapping error's own.
    check!(locations).satisfies(ge(3usize))?;
    Ok(())
}

#[test]
fn soft_scope_collects_propagated_errors_via_check() -> TestResult {
    let result = soft(|s| {
        s.record(check!(2 + 2).satisfies(eq(4)));
        s.record(check!(2 + 2).satisfies(eq(5)));
    });
    let error = result.expect_err("one of the checked results failed");
    check!(error.to_string().contains("1 failure")).satisfies(is_true())?;
    Ok(())
}

#[test]
fn a_panic_inside_soft_is_re_raised_after_recording_failures() -> TestResult {
    // The closure records a soft failure, then panics for an unrelated reason.
    // `soft` runs it under `catch_unwind`, so the panic is re-raised here
    // rather than swallowed — this nested `catch_unwind` catches it.
    let outcome = std::panic::catch_unwind(|| {
        soft(|s| {
            s.check(&1, eq(2));
            panic!("unrelated explosion");
        })
    });

    let panic = outcome.expect_err("the panic must propagate out of soft");
    let message = panic
        .downcast_ref::<&str>()
        .copied()
        .or_else(|| panic.downcast_ref::<String>().map(String::as_str));
    check!(message).satisfies(eq(Some("unrelated explosion")))?;
    Ok(())
}

#[test]
fn soft_scope_nested_context_renders_for_each_failure() -> TestResult {
    let result = soft(|s| {
        let mut user = s.context("while validating the user");
        user.check(&1, eq(2));
        let mut email = user.context("while checking the email");
        email.check(&"bad", contains_str("@"));
    });
    let error = result.expect_err("two soft assertions failed");
    let rendered = error.to_string();

    // The outer frame appears for both failures; the inner only for the second.
    check!(rendered.matches("while validating the user").count()).satisfies(eq(2usize))?;
    check!(rendered.matches("while checking the email").count()).satisfies(eq(1usize))?;
    Ok(())
}
