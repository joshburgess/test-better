//! `test-better-runner`: optional pretty runner.
//!
//! Library half of the `cargo-test-better` subcommand (PROJECT_BUILD_PLAN.md
//! §14, Phase 9). It wraps `cargo test`, forwarding every argument and
//! propagating the exit code; from Phase 9.2 on it also groups failures by
//! their context chain.
//!
//! # The structured-output channel
//!
//! The runner never parses rendered failure text. It consumes the structured
//! `StructuredError` form (`test-better`'s owned, serializable mirror of
//! `TestError`), and the channel that carries it (the decision
//! PROJECT_BUILD_PLAN.md §9.1 defers to this cycle) is a **marker-wrapped JSON
//! line in the test's own captured output**:
//!
//! - The runner exports [`RUNNER_ENV`]`=1` into the `cargo test` it spawns.
//! - When that variable is set, a failing `test-better` test prints one line
//!   of the form `<STRUCTURED_MARKER><json><STRUCTURED_MARKER>` to stdout, in
//!   addition to its normal human-readable failure. Phase 9.2 wires this
//!   emitting side in `test-better-core`.
//! - `cargo test` already captures test output and replays it for *failing*
//!   tests, which is exactly when the runner needs it, so no side-channel file
//!   and no `--nocapture` is required.
//! - A failure with no marker line (a plain `panic!`, or non-`test-better`
//!   code) is shown ungrouped and labelled "unstructured"; the runner never
//!   falls back to parsing prose.
//!
//! A side-channel file under `target/` was the alternative. It was rejected
//! because parallel test binaries and threads would contend on it, and it
//! would need its own lifecycle (creation, cleanup, staleness detection) that
//! the capture-stream approach gets from `cargo test` for free.

use std::ffi::{OsStr, OsString};
use std::process::Command;

/// The environment variable the runner sets on the `cargo test` it spawns.
///
/// A `test-better` test emits its structured failure (see the module docs)
/// only when this is present, so an ordinary `cargo test` stays unaffected.
pub const RUNNER_ENV: &str = "TEST_BETTER_RUNNER";

/// The sentinel that brackets the JSON structured-error payload on its own
/// line in captured test output. Chosen to be unmistakable in prose and to sit
/// alone on a line.
pub const STRUCTURED_MARKER: &str = "@@test-better-structured-error-v1@@";

/// The subcommand name cargo passes as the first argument when this binary is
/// invoked as `cargo test-better`.
const SUBCOMMAND: &str = "test-better";

/// Builds the `cargo test` invocation the runner wraps.
///
/// `args` is the runner's own arguments with the program name already removed.
/// Cargo runs an external subcommand as `cargo-test-better test-better ...`, so
/// a leading `test-better` argument is dropped here and everything after it is
/// forwarded to `cargo test` verbatim. The structured-output channel is opened
/// by exporting [`RUNNER_ENV`].
#[must_use]
pub fn cargo_test_command<I, S>(args: I) -> Command
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let mut forwarded: Vec<OsString> = args
        .into_iter()
        .map(|arg| arg.as_ref().to_os_string())
        .collect();
    if forwarded.first().is_some_and(|arg| arg == SUBCOMMAND) {
        forwarded.remove(0);
    }
    // Respect the `CARGO` cargo sets for its subprocesses, so the wrapped build
    // uses the same toolchain that launched the runner.
    let cargo = std::env::var_os("CARGO").unwrap_or_else(|| OsString::from("cargo"));
    let mut command = Command::new(cargo);
    command.arg("test").args(forwarded).env(RUNNER_ENV, "1");
    command
}

/// Runs the wrapped `cargo test` to completion, inheriting stdio, and returns
/// the exit code to propagate.
///
/// A process ended by a signal reports no exit code; that maps to `101`, the
/// code cargo itself uses for an abnormally terminated test binary, so the
/// runner's exit code still means "something went wrong".
///
/// # Errors
///
/// Returns the [`std::io::Error`] from spawning `cargo` if the process could
/// not be started at all (for example, `cargo` is not on `PATH`).
pub fn run<I, S>(args: I) -> std::io::Result<i32>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let status = cargo_test_command(args).status()?;
    Ok(status.code().unwrap_or(101))
}

#[cfg(test)]
mod tests {
    use super::*;
    use test_better::prelude::*;

    #[test]
    fn forwards_args_after_dropping_the_subcommand() -> TestResult {
        let command = cargo_test_command(["test-better", "--release", "-p", "mycrate"]);
        let args: Vec<OsString> = command.get_args().map(OsStr::to_os_string).collect();
        expect!(args).to(eq(vec![
            OsString::from("test"),
            OsString::from("--release"),
            OsString::from("-p"),
            OsString::from("mycrate"),
        ]))
    }

    #[test]
    fn keeps_args_when_there_is_no_subcommand_prefix() -> TestResult {
        let command = cargo_test_command(["--lib"]);
        let args: Vec<OsString> = command.get_args().map(OsStr::to_os_string).collect();
        expect!(args).to(eq(vec![OsString::from("test"), OsString::from("--lib")]))
    }

    #[test]
    fn opens_the_structured_output_channel() -> TestResult {
        let command = cargo_test_command(["test-better"]);
        let opened = command
            .get_envs()
            .any(|(key, value)| key == OsStr::new(RUNNER_ENV) && value == Some(OsStr::new("1")));
        expect!(opened).to(is_true())
    }
}
