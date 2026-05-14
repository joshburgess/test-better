//! Iteration 9.3 acceptance: `cargo test-better` prints a run-summary table
//! with the right pass/fail/ignored counts, and propagates the wrapped build's
//! exit code.
//!
//! This drives the freshly built `cargo-test-better` binary against the
//! `mixed-results` fixture workspace (its own `[workspace]` root, dependency
//! free, so the nested `cargo test` is real but quick), which has a fixed mix
//! of three passing, two failing, and one ignored test.

use std::path::{Path, PathBuf};
use std::process::Command;

use test_better::prelude::*;

/// The absolute path of a fixture workspace under `tests/fixtures/`.
fn fixture_dir(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(name)
}

#[test]
fn reports_the_run_summary_counts_and_propagates_the_exit_code() -> TestResult {
    let dir = fixture_dir("mixed-results");
    let output = Command::new(env!("CARGO_BIN_EXE_cargo-test-better"))
        .current_dir(&dir)
        .output()
        .or_fail_with("spawn the cargo-test-better binary")?;
    let stdout = String::from_utf8(output.stdout).or_fail_with("runner stdout is utf-8")?;

    // Two tests in the fixture fail, so the wrapped `cargo test` exits non-zero
    // and the runner propagates that.
    expect!(output.status.success()).to(is_false())?;

    // The summary table reports the fixture's known mix: three passing, two
    // failing, one ignored. (The lib's six tests; the fixture has no doctests,
    // so the doctest binary contributes a clean `0 passed; 0 failed`.)
    let summary = stdout
        .split_once("test-better: summary")
        .or_fail_with("the runner printed its summary table")?
        .1;
    expect!(summary.contains("3 passed, 2 failed, 1 ignored")).to(is_true())?;
    expect!(summary.contains("finished in")).to(is_true())
}
