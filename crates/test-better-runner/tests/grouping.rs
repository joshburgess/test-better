//! Acceptance test: `cargo test-better` groups failures by their context chain,
//! and a failure with no `test-better` structure still shows up in the summary,
//! ungrouped, without crashing the runner.
//!
//! This drives the freshly built `cargo-test-better` binary against the
//! `structured-failures` fixture workspace (its own `[workspace]` root, so the
//! nested `cargo test` is real but small) and inspects the report it prints
//! after the wrapped build exits.

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
fn groups_failures_by_feature_area_and_keeps_unstructured_ones() -> TestResult {
    let dir = fixture_dir("structured-failures");
    let output = Command::new(env!("CARGO_BIN_EXE_cargo-test-better"))
        .current_dir(&dir)
        .output()
        .or_fail_with("spawn the cargo-test-better binary")?;
    let stdout = String::from_utf8(output.stdout).or_fail_with("runner stdout is utf-8")?;

    // The fixture has failing tests, so the wrapped `cargo test` exits non-zero
    // and the runner propagates that.
    expect!(output.status.success()).to(is_false())?;

    // Inspect the grouped report the runner appends after the wrapped build,
    // not the wrapped build's own replayed failure text (which also mentions
    // the context strings).
    let report = stdout
        .split_once("test-better: grouped failures")
        .or_fail_with("the runner printed its grouped report")?
        .1;

    // One bucket per feature area, each header appearing exactly once even
    // though `the user store` has two failures under it.
    expect!(report.contains("the user store")).to(is_true())?;
    expect!(report.contains("the http layer")).to(is_true())?;
    expect!(report.matches("the user store").count()).to(eq(1))?;
    expect!(report.contains("user_count_matches")).to(is_true())?;
    expect!(report.contains("user_store_connects")).to(is_true())?;

    // The plain `panic!` carried no structure, so it is listed ungrouped.
    expect!(report.contains("unstructured (no test-better failure data)")).to(is_true())?;
    expect!(report.contains("arithmetic_is_hard")).to(is_true())
}
