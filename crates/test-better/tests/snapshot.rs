//! End-to-end snapshot wiring through the `test-better` facade.
//!
//! `check!(value).matches_snapshot("name")` has to capture the call site's
//! `module_path!()`, resolve `tests/snapshots/` relative to the package root
//! (where `cargo test` puts the working directory), read the committed `.snap`
//! file, and compare. These tests verify that whole path against snapshot
//! files committed next to them.
//!
//! They deliberately assert only the *matching* case. A test that asserted a
//! *mismatch* through `matches_snapshot` would behave differently under
//! `UPDATE_SNAPSHOTS=1` (it would rewrite the committed file, or record a
//! pending inline patch), so the mismatch path and the create/update lifecycle
//! are covered where they can be driven explicitly instead:
//! `test-better-snapshot`'s own `tests/lifecycle.rs` and `tests/accept.rs`, and
//! the `snapshot_error`/`inline_snapshot_error` unit tests in
//! `test-better-matchers`.

use test_better::Redactions;
use test_better::prelude::*;

#[test]
fn a_value_matches_its_committed_snapshot() -> TestResult {
    let rendered = "<!doctype html>\n<title>Home</title>\n<h1>Welcome</h1>";
    check!(rendered).matches_snapshot("rendered_page")
}

#[test]
fn a_multi_line_value_matches_its_committed_snapshot() -> TestResult {
    let report = ["name: alice", "score: 42", "status: active"].join("\n");
    check!(report).matches_snapshot("report")
}

#[test]
fn a_value_matches_its_inline_snapshot() -> TestResult {
    check!(2 + 2).matches_inline_snapshot("4")
}

#[test]
fn a_multi_line_value_matches_its_inline_snapshot() -> TestResult {
    let report = ["name: alice", "score: 42", "status: active"].join("\n");
    check!(report).matches_inline_snapshot(
        r#"
        name: alice
        score: 42
        status: active
        "#,
    )
}

#[test]
fn a_redacted_value_matches_its_committed_snapshot() -> TestResult {
    let redactions = Redactions::new().redact_uuids();
    // Two "runs", two different UUIDs: redaction stabilizes both onto the same
    // committed snapshot, which is the whole point of the feature.
    let first = format!("session {} opened", "550e8400-e29b-41d4-a716-446655440000");
    check!(first).matches_snapshot_with("redacted_session", &redactions)?;
    let second = format!("session {} opened", "11111111-2222-3333-4444-555555555555");
    check!(second).matches_snapshot_with("redacted_session", &redactions)?;
    Ok(())
}

#[test]
fn a_redacted_value_matches_its_inline_snapshot() -> TestResult {
    let redactions = Redactions::new().redact_rfc3339_timestamps();
    let rendered = format!("event at {}", "2026-05-14T12:34:56Z");
    check!(rendered).matches_inline_snapshot_with("event at [timestamp]", &redactions)
}
