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
//! `TestError`), and the channel that carries it is a **marker-wrapped JSON
//! line in the test's own captured output** (the emitting side lives in
//! `test-better-core`'s `runner` module):
//!
//! - The runner exports [`RUNNER_ENV`]`=1` into the `cargo test` it spawns.
//! - When that variable is set, a failing `test-better` test prints one line
//!   of the form `<STRUCTURED_MARKER><json><STRUCTURED_MARKER>` to stdout, in
//!   addition to its normal human-readable failure.
//! - `cargo test` already captures test output and replays it for *failing*
//!   tests, which is exactly when the runner needs it, so no side-channel file
//!   and no `--nocapture` is required.
//! - A failure with no marker line (a plain `panic!`, or non-`test-better`
//!   code) is shown ungrouped and labelled "unstructured"; the runner never
//!   falls back to parsing prose.
//!
//! # Grouping (Phase 9.2)
//!
//! [`run`] pipes the wrapped `cargo test`'s stdout, tees every non-marker line
//! straight through, and feeds the stream to [`scan_output`], which builds a
//! [`GroupedReport`]: structured failures bucketed by their top
//! [`ContextFrame`](test_better::ContextFrame) message, plus a flat list of
//! unstructured ones. [`run`] prints that report after the wrapped build exits.

use std::ffi::{OsStr, OsString};
use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};

use test_better::StructuredError;
pub use test_better::{RUNNER_ENV, STRUCTURED_MARKER};

/// The subcommand name cargo passes as the first argument when this binary is
/// invoked as `cargo test-better`.
const SUBCOMMAND: &str = "test-better";

/// The bucket label for a structured failure whose error carries no context
/// chain at all.
const NO_CONTEXT: &str = "(no context)";

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

/// Runs the wrapped `cargo test` to completion and returns the exit code to
/// propagate, after printing the grouped failure report.
///
/// Stdout is piped so the runner can pick out the structured-error marker
/// lines; every other line is teed straight through, so the wrapped build
/// still looks like an ordinary `cargo test` run. Stderr is inherited
/// untouched. The report is printed once the child has exited.
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
    let mut command = cargo_test_command(args);
    command.stdout(Stdio::piped());
    let mut child = command.spawn()?;

    // Drain the child's stdout while it runs: scan for marker lines and tee
    // everything else to our own stdout. `take` leaves `None` behind, so the
    // borrow ends before `wait`.
    let report = match child.stdout.take() {
        Some(stdout) => {
            let lines = BufReader::new(stdout).lines().map_while(Result::ok);
            scan_output(lines, |line| println!("{line}"))
        }
        None => GroupedReport::default(),
    };

    let status = child.wait()?;
    print_report(&report);
    Ok(status.code().unwrap_or(101))
}

/// One structured failure: the test that produced it and its structured error.
#[derive(Debug, Clone)]
pub struct StructuredFailure {
    /// The libtest name of the failing test (`module::path::test_name`).
    pub test: String,
    /// The structured error the test emitted on the channel.
    pub error: StructuredError,
}

/// Structured failures that share a top context-frame message.
#[derive(Debug, Clone)]
pub struct ContextGroup {
    /// The shared top context-frame message, or `(no context)` when the errors
    /// carry no context chain.
    pub context: String,
    /// The failures in this group, in the order they were scanned.
    pub failures: Vec<StructuredFailure>,
}

/// The result of scanning a wrapped `cargo test` run: structured failures
/// bucketed by feature area, plus the unstructured ones left ungrouped.
#[derive(Debug, Clone, Default)]
pub struct GroupedReport {
    /// Structured failures, grouped by their top context frame.
    pub groups: Vec<ContextGroup>,
    /// Names of failing tests that emitted no structured payload (a plain
    /// `panic!`, or non-`test-better` code). Shown ungrouped, never parsed.
    pub unstructured: Vec<String>,
}

/// Scans a wrapped `cargo test`'s stdout, line by line, into a [`GroupedReport`].
///
/// `echo` is called with every line that is *not* a structured-error marker,
/// so the caller can tee the ordinary `cargo test` output through unchanged.
/// Marker lines are consumed silently: they are tooling traffic, and the
/// human-readable failure they accompany has already been echoed.
///
/// The scan is a small state machine over libtest's failure-replay format. A
/// `---- <name> stdout ----` header starts a test section; a marker line seen
/// inside one attaches a structured error to that test. Any test that opened a
/// section but emitted no parseable marker is recorded as unstructured.
pub fn scan_output<L, E>(lines: L, mut echo: E) -> GroupedReport
where
    L: IntoIterator<Item = String>,
    E: FnMut(&str),
{
    let mut current_test: Option<String> = None;
    let mut structured: Vec<StructuredFailure> = Vec::new();
    let mut sectioned: Vec<String> = Vec::new();
    let mut with_marker: Vec<String> = Vec::new();

    for line in lines {
        if let Some(payload) = marker_payload(&line) {
            if let (Some(test), Ok(error)) = (
                current_test.as_ref(),
                serde_json::from_str::<StructuredError>(payload),
            ) {
                structured.push(StructuredFailure {
                    test: test.clone(),
                    error,
                });
                with_marker.push(test.clone());
            }
            // A marker line is tooling traffic, not part of the human output.
            continue;
        }
        if let Some(name) = test_section_header(&line) {
            current_test = Some(name.to_string());
            if !sectioned.iter().any(|seen| seen == name) {
                sectioned.push(name.to_string());
            }
        }
        echo(&line);
    }

    let unstructured = sectioned
        .into_iter()
        .filter(|test| !with_marker.contains(test))
        .collect();
    GroupedReport {
        groups: group(structured),
        unstructured,
    }
}

/// Buckets structured failures by their top context-frame message, preserving
/// first-seen order both of the groups and of the failures within each.
fn group(failures: Vec<StructuredFailure>) -> Vec<ContextGroup> {
    let mut groups: Vec<ContextGroup> = Vec::new();
    for failure in failures {
        let context = failure
            .error
            .context
            .first()
            .map_or_else(|| NO_CONTEXT.to_string(), |frame| frame.message.clone());
        match groups
            .iter_mut()
            .find(|existing| existing.context == context)
        {
            Some(existing) => existing.failures.push(failure),
            None => groups.push(ContextGroup {
                context,
                failures: vec![failure],
            }),
        }
    }
    groups
}

/// If `line` is a structured-error marker line, returns the JSON payload
/// between the two markers; otherwise returns `None`.
fn marker_payload(line: &str) -> Option<&str> {
    line.trim()
        .strip_prefix(STRUCTURED_MARKER)?
        .strip_suffix(STRUCTURED_MARKER)
}

/// If `line` is a libtest `---- <name> stdout ----` section header, returns the
/// test name; otherwise returns `None`.
fn test_section_header(line: &str) -> Option<&str> {
    line.strip_prefix("---- ")?.strip_suffix(" stdout ----")
}

/// Prints the grouped failure report to stdout, after the wrapped build's own
/// output. Nothing is printed when there were no failures at all.
fn print_report(report: &GroupedReport) {
    if report.groups.is_empty() && report.unstructured.is_empty() {
        return;
    }
    println!();
    println!("test-better: grouped failures");
    for group in &report.groups {
        println!();
        println!("  {}", group.context);
        for failure in &group.failures {
            let summary = failure
                .error
                .message
                .as_deref()
                .unwrap_or_else(|| failure.error.kind.headline());
            println!("    {}: {summary}", failure.test);
            println!(
                "      at {}:{}:{}",
                failure.error.location.file,
                failure.error.location.line,
                failure.error.location.column,
            );
        }
    }
    if !report.unstructured.is_empty() {
        println!();
        println!("  unstructured (no test-better failure data)");
        for test in &report.unstructured {
            println!("    {test}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use test_better::prelude::*;
    use test_better::{ErrorKind, SourceLocation, StructuredContextFrame};

    /// Builds a minimal `StructuredError` with the given kind, message, and
    /// context chain (outermost-first, like the real one).
    fn structured_error(kind: ErrorKind, message: &str, context: &[&str]) -> StructuredError {
        StructuredError {
            kind,
            message: Some(message.to_string()),
            location: SourceLocation {
                file: "src/lib.rs".to_string(),
                line: 7,
                column: 5,
            },
            context: context
                .iter()
                .map(|frame| StructuredContextFrame {
                    message: (*frame).to_string(),
                    location: None,
                })
                .collect(),
            trace: Vec::new(),
            payload: None,
        }
    }

    /// A libtest section header line for `test`.
    fn header(test: &str) -> String {
        format!("---- {test} stdout ----")
    }

    /// A structured-error marker line carrying `error`.
    fn marker_line(error: &StructuredError) -> TestResult<String> {
        let json = serde_json::to_string(error).or_fail_with("serialize structured error")?;
        Ok(format!("{STRUCTURED_MARKER}{json}{STRUCTURED_MARKER}"))
    }

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

    #[test]
    fn groups_structured_failures_by_top_context_frame() -> TestResult {
        let db_one = structured_error(
            ErrorKind::Assertion,
            "row count differs",
            &["the user store"],
        );
        let db_two = structured_error(ErrorKind::Setup, "no connection", &["the user store"]);
        let http = structured_error(ErrorKind::Assertion, "status was 500", &["the http layer"]);
        let lines = vec![
            header("store::counts_match"),
            marker_line(&db_one)?,
            header("store::connects"),
            marker_line(&db_two)?,
            header("http::returns_ok"),
            marker_line(&http)?,
        ];

        let report = scan_output(lines, |_| {});

        expect!(report.groups.len()).to(eq(2))?;
        expect!(report.groups[0].context.as_str()).to(eq("the user store"))?;
        expect!(report.groups[0].failures.len()).to(eq(2))?;
        expect!(report.groups[0].failures[0].test.as_str()).to(eq("store::counts_match"))?;
        expect!(report.groups[1].context.as_str()).to(eq("the http layer"))?;
        expect!(report.groups[1].failures.len()).to(eq(1))?;
        expect!(report.unstructured.is_empty()).to(is_true())
    }

    #[test]
    fn keeps_failures_without_a_marker_as_unstructured() -> TestResult {
        let lines = vec![
            header("math::adds"),
            "thread 'math::adds' panicked at src/lib.rs:3:5:".to_string(),
            "assertion `left == right` failed".to_string(),
        ];

        let report = scan_output(lines, |_| {});

        expect!(report.groups.is_empty()).to(is_true())?;
        expect!(report.unstructured).to(eq(vec!["math::adds".to_string()]))
    }

    #[test]
    fn echoes_every_non_marker_line() -> TestResult {
        let error = structured_error(ErrorKind::Assertion, "boom", &["an area"]);
        let lines = vec![
            "running 1 test".to_string(),
            header("suite::case"),
            marker_line(&error)?,
            "test result: FAILED. 0 passed; 1 failed".to_string(),
        ];

        let mut echoed: Vec<String> = Vec::new();
        scan_output(lines, |line| echoed.push(line.to_string()));

        // The marker line is swallowed; everything else passes through.
        expect!(echoed).to(eq(vec![
            "running 1 test".to_string(),
            header("suite::case"),
            "test result: FAILED. 0 passed; 1 failed".to_string(),
        ]))
    }

    #[test]
    fn an_unparseable_marker_leaves_the_test_unstructured() -> TestResult {
        let lines = vec![
            header("suite::case"),
            format!("{STRUCTURED_MARKER}not json{STRUCTURED_MARKER}"),
        ];

        let report = scan_output(lines, |_| {});

        expect!(report.groups.is_empty()).to(is_true())?;
        expect!(report.unstructured).to(eq(vec!["suite::case".to_string()]))
    }

    #[test]
    fn errors_without_context_land_in_the_no_context_bucket() -> TestResult {
        let bare = structured_error(ErrorKind::Custom, "something off", &[]);
        let lines = vec![header("suite::case"), marker_line(&bare)?];

        let report = scan_output(lines, |_| {});

        expect!(report.groups.len()).to(eq(1))?;
        expect!(report.groups[0].context.as_str()).to(eq(NO_CONTEXT))
    }
}
