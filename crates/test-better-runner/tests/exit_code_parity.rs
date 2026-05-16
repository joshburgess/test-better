//! Acceptance test: `cargo test-better` exits with the same code as `cargo
//! test` itself, on both an all-pass and a has-failures fixture workspace.
//!
//! Each fixture under `tests/fixtures/` is its own `[workspace]` root with no
//! dependencies, so this drives a real (but fast) nested `cargo` build. The
//! two fixtures use separate target directories, so the two `#[test]`s here
//! can run in parallel without contending on a build lock.

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

/// Runs `command` in `dir` to completion and returns its exit code. A process
/// killed by a signal (no exit code) maps to `101`, exactly as the runner
/// itself does, so the comparison stays apples-to-apples.
fn exit_code(mut command: Command, dir: &Path) -> TestResult<i32> {
    let status = command
        .current_dir(dir)
        .status()
        .or_fail_with("spawn the nested cargo process")?;
    Ok(status.code().unwrap_or(101))
}

/// The baseline: plain `cargo test` in `dir`.
fn cargo_test(dir: &Path) -> TestResult<i32> {
    let cargo = std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
    let mut command = Command::new(cargo);
    command.arg("test");
    exit_code(command, dir)
}

/// The runner under test: the freshly built `cargo-test-better` binary in `dir`.
fn cargo_test_better(dir: &Path) -> TestResult<i32> {
    let command = Command::new(env!("CARGO_BIN_EXE_cargo-test-better"));
    exit_code(command, dir)
}

#[test]
fn matches_cargo_test_on_an_all_pass_workspace() -> TestResult {
    let dir = fixture_dir("all-pass");
    let baseline = cargo_test(&dir)?;
    let runner = cargo_test_better(&dir)?;
    check!(baseline).satisfies(eq(0))?;
    check!(runner).satisfies(eq(baseline))
}

#[test]
fn matches_cargo_test_on_a_has_failures_workspace() -> TestResult {
    let dir = fixture_dir("has-failures");
    let baseline = cargo_test(&dir)?;
    let runner = cargo_test_better(&dir)?;
    check!(baseline).satisfies(ne(0))?;
    check!(runner).satisfies(eq(baseline))
}
