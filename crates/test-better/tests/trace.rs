//! `Trace` end-to-end through the `test-better` facade.
//!
//! A `Trace` records breadcrumbs while a test runs; any `TestError` built while
//! the trace is in scope snapshots them, and the rendered failure shows them in
//! the order they happened. These tests drive a failure on purpose and inspect
//! the captured error, so the suite still passes.

use test_better::Trace;
use test_better::prelude::*;

#[test]
fn a_failure_carries_the_trace_in_chronological_order() -> TestResult {
    let mut trace = Trace::new();
    trace.step("connecting to db");
    trace.kv("db_url", "postgres://localhost/test");
    trace.step("running the query");

    // Force a failure while the trace is in scope, then capture it instead of
    // propagating, so this test still passes.
    let failure = check!(2 + 2).satisfies(eq(5)).err().or_fail()?;
    drop(trace);

    let rendered = format!("{failure}");
    let connect = rendered
        .find("connecting to db")
        .or_fail_with("first step present")?;
    let url = rendered
        .find("db_url = postgres://localhost/test")
        .or_fail_with("kv breadcrumb present")?;
    let query = rendered
        .find("running the query")
        .or_fail_with("second step present")?;

    check!(connect < url).satisfies(is_true())?;
    check!(url < query).satisfies(is_true())
}

#[test]
fn a_failure_with_no_trace_in_scope_renders_no_trace_section() -> TestResult {
    let failure = check!(1).satisfies(eq(2)).err().or_fail()?;
    let rendered = format!("{failure}");
    check!(rendered.contains("trace:")).satisfies(is_false())
}

#[test]
fn the_structured_form_carries_the_trace() -> TestResult {
    let mut trace = Trace::new();
    trace.step("setting up the fixture");
    let failure = check!("a").satisfies(eq("b")).err().or_fail()?;
    drop(trace);

    let structured = failure.to_structured();
    check!(structured.trace.len()).satisfies(eq(1))
}
