//! `test-better-runner`: optional pretty runner.
//!
//! Library half of the `cargo-test-better` subcommand. It wraps `cargo test`,
//! forwarding every argument and propagating the exit code; it also groups
//! failures by their context chain.
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
//! # Grouping
//!
//! [`run`] pipes the wrapped `cargo test`'s stdout, tees every non-marker line
//! straight through, and feeds the stream to [`scan_output`], which builds a
//! [`GroupedReport`]: structured failures bucketed by their top
//! [`ContextFrame`](test_better::ContextFrame) message, plus a flat list of
//! unstructured ones. [`run`] prints that report after the wrapped build exits.
//!
//! # Progress and summary
//!
//! [`scan_output`] also reads libtest's own `test result:` lines into a
//! [`RunSummary`] (passed/failed/ignored counts, summed across every test
//! binary), which [`run`] prints as a one-line summary table once the build
//! exits, alongside the wall-clock duration it measured itself.
//!
//! While the build runs, [`run`] keeps a live progress counter. It is gated
//! on stderr being a TTY: on a terminal the per-test `... ok` lines are
//! replaced by an updating `running: done/total` line on stderr; piped or
//! redirected, the output is the plain `cargo test` stream, unchanged.

use std::ffi::{OsStr, OsString};
use std::io::{BufRead, BufReader, IsTerminal, Write};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

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
/// propagate, after printing the grouped failure report and the run summary.
///
/// Stdout is piped so the runner can pick out the structured-error marker
/// lines; every other line is teed straight through, so the wrapped build
/// still looks like an ordinary `cargo test` run. Stderr is inherited
/// untouched. While the build runs, a live progress counter is shown on stderr
/// when stderr is a TTY (and the per-test `... ok` lines are then folded into
/// it instead of being teed). The grouped report and summary table are printed
/// once the child has exited.
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

    let started = Instant::now();
    let mut child = command.spawn()?;

    // Drain the child's stdout while it runs: scan for marker lines, drive the
    // live progress counter, and tee everything else to our own stdout. `take`
    // leaves `None` behind, so the borrow ends before `wait`.
    let report = match child.stdout.take() {
        Some(stdout) => {
            let lines = BufReader::new(stdout).lines().map_while(Result::ok);
            let mut progress = Progress::new(std::io::stderr().is_terminal());
            let report = scan_output(lines, |line| {
                let event = progress_event(line);
                // On a TTY the per-test `... ok` lines are the progress
                // counter's job; everywhere else they are teed as usual.
                if !(progress.enabled && matches!(event, Some(ProgressEvent::Completed))) {
                    println!("{line}");
                }
                if let Some(event) = event {
                    progress.observe(event);
                }
            });
            progress.clear();
            report
        }
        None => GroupedReport::default(),
    };

    let status = child.wait()?;
    print_report(&report);
    print_summary(&report.summary, started.elapsed());
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

/// The pass/fail/ignored tallies of a wrapped `cargo test` run, summed across
/// every test binary (libtest prints one `test result:` line per binary, and
/// [`scan_output`] adds them up).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RunSummary {
    /// Tests that passed.
    pub passed: usize,
    /// Tests that failed.
    pub failed: usize,
    /// Tests skipped with `#[ignore]` or filtered out by name.
    pub ignored: usize,
    /// Benchmarks measured (libtest's `measured` count; zero for `cargo test`).
    pub measured: usize,
    /// Tests excluded by a name filter (`cargo test <filter>`).
    pub filtered_out: usize,
}

/// The result of scanning a wrapped `cargo test` run: structured failures
/// bucketed by feature area, the unstructured ones left ungrouped, and the
/// run's pass/fail/ignored summary.
#[derive(Debug, Clone, Default)]
pub struct GroupedReport {
    /// Structured failures, grouped by their top context frame.
    pub groups: Vec<ContextGroup>,
    /// Names of failing tests that emitted no structured payload (a plain
    /// `panic!`, or non-`test-better` code). Shown ungrouped, never parsed.
    pub unstructured: Vec<String>,
    /// The pass/fail/ignored tallies, summed across every test binary.
    pub summary: RunSummary,
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
/// section but emitted no parseable marker is recorded as unstructured. Each
/// `test result:` line is parsed and its counts added into the summary.
#[must_use]
pub fn scan_output<L, E>(lines: L, mut echo: E) -> GroupedReport
where
    L: IntoIterator<Item = String>,
    E: FnMut(&str),
{
    let mut current_test: Option<String> = None;
    let mut structured: Vec<StructuredFailure> = Vec::new();
    let mut sectioned: Vec<String> = Vec::new();
    let mut with_marker: Vec<String> = Vec::new();
    let mut summary = RunSummary::default();

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
        if let Some(line_summary) = parse_result_line(&line) {
            summary.passed += line_summary.passed;
            summary.failed += line_summary.failed;
            summary.ignored += line_summary.ignored;
            summary.measured += line_summary.measured;
            summary.filtered_out += line_summary.filtered_out;
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
        summary,
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

/// The count `segment` reports for `label`, if it ends with that label.
///
/// libtest's `test result:` line is a `;`-separated list of segments like
/// `5 passed` or `0 filtered out`; the count is the last whitespace-delimited
/// token before the label.
fn segment_count(segment: &str, label: &str) -> Option<usize> {
    segment
        .trim()
        .strip_suffix(label)?
        .trim_end()
        .rsplit(' ')
        .next()
        .and_then(|count| count.parse().ok())
}

/// If `line` is a libtest `test result:` summary line, parses its
/// passed/failed/ignored/measured/filtered tallies into a [`RunSummary`].
///
/// libtest prints one such line per test binary; [`scan_output`] sums them.
fn parse_result_line(line: &str) -> Option<RunSummary> {
    let line = line.trim();
    if !line.starts_with("test result:") {
        return None;
    }
    let mut summary = RunSummary::default();
    let mut matched = false;
    for segment in line.split(';') {
        if let Some(count) = segment_count(segment, "passed") {
            summary.passed = count;
            matched = true;
        } else if let Some(count) = segment_count(segment, "failed") {
            summary.failed = count;
            matched = true;
        } else if let Some(count) = segment_count(segment, "ignored") {
            summary.ignored = count;
            matched = true;
        } else if let Some(count) = segment_count(segment, "measured") {
            summary.measured = count;
            matched = true;
        } else if let Some(count) = segment_count(segment, "filtered out") {
            summary.filtered_out = count;
            matched = true;
        }
    }
    matched.then_some(summary)
}

/// A step in the wrapped run's progress, recovered from one line of libtest
/// output by [`progress_event`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProgressEvent {
    /// A `running N tests` line: `N` more tests are about to run.
    Discovered(usize),
    /// A `test <name> ... <outcome>` line: one more test has finished.
    Completed,
}

/// Classifies one line of libtest output as a [`ProgressEvent`], or `None` if
/// it is neither a test-count announcement nor a per-test outcome line.
#[must_use]
pub fn progress_event(line: &str) -> Option<ProgressEvent> {
    let line = line.trim();
    if let Some(rest) = line.strip_prefix("running ") {
        // `running 5 tests` / `running 1 test`.
        let count = rest.split(' ').next()?.parse().ok()?;
        return Some(ProgressEvent::Discovered(count));
    }
    // `test <name> ... ok` / `... FAILED` / `... ignored`. The `test result:`
    // summary line also starts with `test `, so it is excluded explicitly.
    if line.starts_with("test ") && !line.starts_with("test result:") && line.contains(" ... ") {
        return Some(ProgressEvent::Completed);
    }
    None
}

/// A live `running: done/total` counter, shown on stderr while the wrapped
/// build runs. Disabled (every method a no-op) when stderr is not a TTY.
struct Progress {
    /// Whether the counter renders; false when stderr is not a terminal.
    enabled: bool,
    /// Tests announced by `running N tests` lines so far.
    total: usize,
    /// Tests finished so far.
    done: usize,
}

impl Progress {
    /// Creates a counter, rendering only when `enabled`.
    fn new(enabled: bool) -> Self {
        Self {
            enabled,
            total: 0,
            done: 0,
        }
    }

    /// Folds one [`ProgressEvent`] into the counter and repaints it.
    fn observe(&mut self, event: ProgressEvent) {
        match event {
            ProgressEvent::Discovered(count) => self.total += count,
            ProgressEvent::Completed => self.done += 1,
        }
        if self.enabled {
            // `\r` returns to the line start; the trailing spaces overwrite a
            // previously longer count. Errors writing the bar are ignored: it
            // is cosmetic, and the real output goes to stdout regardless.
            let mut stderr = std::io::stderr();
            let _ = write!(stderr, "\r  running: {}/{} tests   ", self.done, self.total);
            let _ = stderr.flush();
        }
    }

    /// Erases the counter line, so the final report starts on a clean line.
    fn clear(&self) {
        if self.enabled {
            let mut stderr = std::io::stderr();
            let _ = write!(stderr, "\r\u{1b}[K");
            let _ = stderr.flush();
        }
    }
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

/// Prints the one-line run summary table to stdout, after the grouped report:
/// the pass/fail/ignored tallies and the wall-clock `duration` the runner
/// measured around the wrapped build.
fn print_summary(summary: &RunSummary, duration: Duration) {
    println!();
    println!("test-better: summary");
    println!(
        "  {} passed, {} failed, {} ignored",
        summary.passed, summary.failed, summary.ignored,
    );
    println!("  finished in {:.2}s", duration.as_secs_f64());
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
        let _ = scan_output(lines, |line| echoed.push(line.to_string()));

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

    #[test]
    fn parses_a_test_result_line_into_a_summary() -> TestResult {
        let summary = parse_result_line(
            "test result: FAILED. 5 passed; 2 failed; 1 ignored; 0 measured; 3 filtered out; \
             finished in 0.42s",
        )
        .or_fail_with("the line is a test result line")?;
        expect!(summary).to(eq(RunSummary {
            passed: 5,
            failed: 2,
            ignored: 1,
            measured: 0,
            filtered_out: 3,
        }))
    }

    #[test]
    fn a_non_result_line_is_not_a_summary() -> TestResult {
        expect!(parse_result_line("running 3 tests").is_none()).to(is_true())?;
        expect!(parse_result_line("test math::adds ... ok").is_none()).to(is_true())
    }

    #[test]
    fn scan_output_sums_the_summary_across_test_binaries() -> TestResult {
        let lines = vec![
            "test result: ok. 3 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; \
             finished in 0.01s"
                .to_string(),
            "test result: FAILED. 1 passed; 2 failed; 0 ignored; 0 measured; 0 filtered out; \
             finished in 0.02s"
                .to_string(),
        ];

        let report = scan_output(lines, |_| {});

        expect!(report.summary).to(eq(RunSummary {
            passed: 4,
            failed: 2,
            ignored: 1,
            measured: 0,
            filtered_out: 0,
        }))
    }

    #[test]
    fn classifies_progress_events() -> TestResult {
        expect!(progress_event("running 5 tests")).to(eq(Some(ProgressEvent::Discovered(5))))?;
        expect!(progress_event("running 1 test")).to(eq(Some(ProgressEvent::Discovered(1))))?;
        expect!(progress_event("test math::adds ... ok")).to(eq(Some(ProgressEvent::Completed)))?;
        expect!(progress_event("test math::divides ... FAILED"))
            .to(eq(Some(ProgressEvent::Completed)))?;
        expect!(progress_event("test math::slow ... ignored"))
            .to(eq(Some(ProgressEvent::Completed)))?;
        // The `test result:` summary line is not a per-test outcome.
        expect!(progress_event("test result: ok. 1 passed; 0 failed")).to(eq(None))?;
        expect!(progress_event("some other line")).to(eq(None))
    }
}
