//! `#[test_case]` end-to-end through the `test-better` facade.
//!
//! Each `#[test_case(..)]` line becomes its own generated `#[test]`, gathered
//! into a module named for the annotated function. The malformed-attribute
//! diagnostics are covered separately by the `trybuild` fixtures in `tests/ui`.
//!
//! The failure path is exercised without failing the suite: a deliberately
//! failing case is marked `#[ignore]` (the attribute is forwarded onto the
//! generated test, so the harness skips it) and then called directly by path
//! from an ordinary `#[test]`, which asserts on the rendered error.

use test_better::prelude::*;
use test_better::test_case;

#[test_case("alice", 30 ; "common case")]
#[test_case("bob", 25 ; "another user")]
fn validates_user(name: &str, age: u32) -> TestResult {
    expect!(name.len()).to(gt(0usize))?;
    expect!(age).to(gt(0u32))
}

// An unlabeled case becomes `addition_works::case_0`; a labeled one keeps its
// sanitized label.
#[test_case(2, 2, 4)]
#[test_case(10, 5, 15 ; "bigger numbers")]
fn addition_works(a: i32, b: i32, sum: i32) -> TestResult {
    expect!(a + b).to(eq(sum))
}

// A zero-argument case: the attribute carries only a label.
#[test_case(; "no parameters at all")]
fn the_truth_holds() -> TestResult {
    expect!(true).to(is_true())
}

// This case fails on purpose. `#[ignore]` is forwarded onto the generated
// `expects_three::not_three` test, so `cargo test` skips it; the test below
// calls it directly to inspect the failure.
#[test_case(3 ; "not three")]
#[ignore]
fn expects_three(n: i32) -> TestResult {
    expect!(n).to(eq(99))
}

#[test]
fn a_failing_case_names_itself_in_the_context() -> TestResult {
    let result = expects_three::not_three();
    let error = result.err().or_fail()?;
    let rendered = format!("{error}");
    // The forwarded failure context carries both the case label and the
    // rendered call, so the failure points back at the case that produced it.
    expect!(rendered.as_str()).to(contains_str("not three"))?;
    expect!(rendered.as_str()).to(contains_str("expects_three(3)"))
}
