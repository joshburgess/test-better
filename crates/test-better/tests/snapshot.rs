//! End-to-end snapshot wiring through the `test-better` facade
//! (PROJECT_BUILD_PLAN.md Iteration 7.1).
//!
//! `expect!(value).to_match_snapshot("name")` has to capture the call site's
//! `module_path!()`, resolve `tests/snapshots/` relative to the package root
//! (where `cargo test` puts the working directory), read the committed `.snap`
//! file, and compare. These tests verify that whole path against snapshot
//! files committed next to them.
//!
//! They deliberately assert only the *matching* case. A test that asserted a
//! *mismatch* through `to_match_snapshot` would behave differently under
//! `UPDATE_SNAPSHOTS=1` (it would rewrite the committed file, or record a
//! pending inline patch), so the mismatch path and the create/update lifecycle
//! are covered where they can be driven explicitly instead:
//! `test-better-snapshot`'s own `tests/lifecycle.rs` and `tests/accept.rs`, and
//! the `snapshot_error`/`inline_snapshot_error` unit tests in
//! `test-better-matchers`.

use test_better::prelude::*;

#[test]
fn a_value_matches_its_committed_snapshot() -> TestResult {
    let rendered = "<!doctype html>\n<title>Home</title>\n<h1>Welcome</h1>";
    expect!(rendered).to_match_snapshot("rendered_page")
}

#[test]
fn a_multi_line_value_matches_its_committed_snapshot() -> TestResult {
    let report = ["name: alice", "score: 42", "status: active"].join("\n");
    expect!(report).to_match_snapshot("report")
}

#[test]
fn a_value_matches_its_inline_snapshot() -> TestResult {
    expect!(2 + 2).to_match_inline_snapshot("4")
}

#[test]
fn a_multi_line_value_matches_its_inline_snapshot() -> TestResult {
    let report = ["name: alice", "score: 42", "status: active"].join("\n");
    expect!(report).to_match_inline_snapshot(
        r#"
        name: alice
        score: 42
        status: active
        "#,
    )
}
