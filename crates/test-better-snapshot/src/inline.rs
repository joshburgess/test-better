//! Inline snapshots: the snapshot literal lives in the test source itself
//! (PROJECT_BUILD_PLAN.md §12, Iteration 7.2).
//!
//! A proc macro cannot rewrite the file it expands, so the mechanism is split
//! in two, mirroring `insta`:
//!
//! - at **runtime**, `expect!(value).to_match_inline_snapshot(r#"..."#)` compares
//!   the value against the literal. On a match it passes. On a mismatch with
//!   `UPDATE_SNAPSHOTS` unset it fails like any assertion; with
//!   `UPDATE_SNAPSHOTS=1` it records a *pending patch* (the source file, the
//!   call-site line and column, and the new value) under
//!   `target/test-better-pending/` and passes;
//! - the **`test-better-accept` companion binary** (built with the `accept`
//!   feature) reads those pending patches and rewrites the literals in the
//!   source files.
//!
//! This module is the runtime half: literal normalization, the comparison, and
//! writing pending patches. It is `std`-only. The accept binary is in
//! `src/bin/test-better-accept.rs`.

use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use crate::SnapshotMode;

/// Where an inline-snapshot call sits in the source: enough for the
/// `test-better-accept` binary to find the literal and rewrite it.
///
/// Built from `std::panic::Location` at the call site, so `file` is whatever
/// path `rustc` was invoked with (workspace-root-relative in a normal `cargo`
/// build).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InlineLocation {
    /// The source file, as `Location::caller().file()` reports it.
    pub file: String,
    /// The 1-based line of the `to_match_inline_snapshot` call.
    pub line: u32,
    /// The 1-based column of the `to_match_inline_snapshot` call.
    pub column: u32,
}

/// An inline snapshot did not match and `UPDATE_SNAPSHOTS` was unset.
///
/// Carries both sides so `test-better-matchers` can render it as an
/// expected/actual `TestError` with a diff, exactly like a file-backed
/// mismatch.
#[derive(Debug)]
pub struct InlineSnapshotFailure {
    /// The normalized inline literal (what the source currently claims).
    pub expected: String,
    /// The value under test.
    pub actual: String,
}

impl fmt::Display for InlineSnapshotFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "inline snapshot does not match")
    }
}

impl std::error::Error for InlineSnapshotFailure {}

/// Normalizes an inline-snapshot literal to the text it actually stands for.
///
/// Test source indents the literal for readability, so the raw token is not
/// the snapshot. Normalization undoes exactly the cosmetic part: it drops a
/// single leading newline (the `r#"`-then-newline idiom), removes the common
/// leading indentation shared by every non-blank line, and trims trailing
/// whitespace. A single-line literal with no leading newline is returned
/// trimmed of trailing whitespace and otherwise untouched.
///
/// ```
/// use test_better_core::TestResult;
/// use test_better_matchers::{eq, expect};
/// use test_better_snapshot::normalize_inline_literal;
///
/// # fn main() -> TestResult {
/// let raw = "\n    User { name: \"alice\" }\n";
/// expect!(normalize_inline_literal(raw)).to(eq("User { name: \"alice\" }".to_string()))?;
/// # Ok(())
/// # }
/// ```
#[must_use]
pub fn normalize_inline_literal(raw: &str) -> String {
    // Drop one leading newline (handling a `\r\n` line ending too).
    let body = raw
        .strip_prefix("\r\n")
        .or_else(|| raw.strip_prefix('\n'))
        .unwrap_or(raw);

    // The common indentation is the minimum leading-whitespace width across
    // non-blank lines. Leading whitespace is ASCII, so byte width is fine.
    let indent = body
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| line.len() - line.trim_start().len())
        .min()
        .unwrap_or(0);

    let dedented = body
        .lines()
        .map(|line| {
            if line.len() >= indent {
                &line[indent..]
            } else {
                // A blank line shorter than the common indent: nothing to keep.
                ""
            }
        })
        .collect::<Vec<_>>()
        .join("\n");

    dedented.trim_end().to_string()
}

/// Compares `actual` against the inline-snapshot literal `raw`.
///
/// In [`Compare`](SnapshotMode::Compare) mode a mismatch is returned as an
/// [`InlineSnapshotFailure`]. In [`Update`](SnapshotMode::Update) mode a
/// mismatch is recorded as a pending patch under `target/test-better-pending/`
/// (for the `test-better-accept` binary to apply) and `Ok(())` is returned, so
/// `UPDATE_SNAPSHOTS=1` runs stay green; the literal in the source is corrected
/// by the accept step, not by the test run.
///
/// # Errors
///
/// Returns [`InlineSnapshotFailure`] when the value does not match the literal
/// and the mode is `Compare`. A failure to write the pending patch in `Update`
/// mode is intentionally swallowed: a missing patch is recoverable (rerun), and
/// failing the test would be a worse outcome than a dropped patch.
pub fn assert_inline_snapshot(
    actual: &str,
    raw: &str,
    location: &InlineLocation,
    mode: SnapshotMode,
) -> Result<(), InlineSnapshotFailure> {
    let expected = normalize_inline_literal(raw);
    // Literal normalization trims trailing whitespace, so a literal can never
    // carry a trailing newline. Drop a single one from `actual` to match: a
    // value rendered with a trailing newline should still be snapshot-able,
    // and the accept step's `format`/`normalize` round-trip drops it anyway.
    let actual = actual
        .strip_suffix("\r\n")
        .or_else(|| actual.strip_suffix('\n'))
        .unwrap_or(actual);
    if actual == expected {
        return Ok(());
    }
    match mode {
        SnapshotMode::Compare => Err(InlineSnapshotFailure {
            expected,
            actual: actual.to_string(),
        }),
        SnapshotMode::Update => {
            // Best effort: see the `# Errors` note above.
            let _ = record_pending_patch(location, actual);
            Ok(())
        }
    }
}

/// The directory pending inline-snapshot patches are written to and read from:
/// `target/test-better-pending/` under the workspace root.
///
/// The workspace root is found by walking up from the current directory to the
/// nearest ancestor containing a `Cargo.lock`; `CARGO_TARGET_DIR`, if set,
/// overrides the `target` location. Both the test process (writing) and the
/// `test-better-accept` binary (reading) resolve it the same way.
///
/// # Errors
///
/// Returns an [`std::io::Error`] if the current directory cannot be read or no
/// `Cargo.lock` is found in any ancestor.
pub fn pending_patch_dir() -> std::io::Result<PathBuf> {
    if let Some(target) = std::env::var_os("CARGO_TARGET_DIR") {
        return Ok(PathBuf::from(target).join("test-better-pending"));
    }
    let start = std::env::current_dir()?;
    let mut dir: &Path = &start;
    loop {
        if dir.join("Cargo.lock").is_file() {
            return Ok(dir.join("target").join("test-better-pending"));
        }
        match dir.parent() {
            Some(parent) => dir = parent,
            None => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "no Cargo.lock found in any ancestor of the current directory",
                ));
            }
        }
    }
}

/// Counter making each pending-patch file name unique within a process, on top
/// of the process id, so parallel tests never collide.
static PATCH_SEQ: AtomicU64 = AtomicU64::new(0);

/// Writes one pending patch as its own file under [`pending_patch_dir`].
///
/// A patch is self-contained, so each gets a distinct file and no test ever
/// has to append to a shared one. The format is line-oriented and needs no
/// escaping: line 1 is the source file, line 2 is `<line>:<column>`, and
/// everything from line 3 on is the new snapshot value verbatim (it may span
/// many lines).
fn record_pending_patch(location: &InlineLocation, actual: &str) -> std::io::Result<()> {
    let dir = pending_patch_dir()?;
    fs::create_dir_all(&dir)?;

    let seq = PATCH_SEQ.fetch_add(1, Ordering::Relaxed);
    let file_name = format!("{}-{}.patch", std::process::id(), seq);
    let body = format!(
        "{}\n{}:{}\n{}",
        location.file, location.line, location.column, actual
    );
    fs::write(dir.join(file_name), body)
}

/// Parses a pending-patch file body back into its parts: the source file, the
/// call-site line and column, and the new snapshot value.
///
/// This is the inverse of `record_pending_patch`'s format, exposed for the
/// `test-better-accept` binary.
///
/// # Errors
///
/// Returns an [`std::io::Error`] with kind `InvalidData` if the body is not at
/// least two lines or the second line is not `<line>:<column>`.
pub fn parse_pending_patch(body: &str) -> std::io::Result<(InlineLocation, String)> {
    let invalid = |msg: &str| std::io::Error::new(std::io::ErrorKind::InvalidData, msg.to_string());

    let mut lines = body.splitn(3, '\n');
    let file = lines.next().ok_or_else(|| invalid("empty patch file"))?;
    let position = lines
        .next()
        .ok_or_else(|| invalid("patch file is missing its position line"))?;
    let value = lines.next().unwrap_or("");

    let (line, column) = position
        .split_once(':')
        .ok_or_else(|| invalid("position line is not `<line>:<column>`"))?;
    let line = line
        .parse()
        .map_err(|_| invalid("patch line number is not an integer"))?;
    let column = column
        .parse()
        .map_err(|_| invalid("patch column number is not an integer"))?;

    Ok((
        InlineLocation {
            file: file.to_string(),
            line,
            column,
        },
        value.to_string(),
    ))
}

#[cfg(test)]
mod tests {
    use test_better_core::{OrFail, TestResult};
    use test_better_matchers::{eq, expect, is_true};

    use super::*;

    #[test]
    fn normalize_drops_leading_newline_and_common_indentation() -> TestResult {
        let raw = "\n        first\n        second\n    ";
        expect!(normalize_inline_literal(raw)).to(eq("first\nsecond".to_string()))
    }

    #[test]
    fn normalize_keeps_relative_indentation() -> TestResult {
        let raw = "\n    outer\n        inner\n";
        expect!(normalize_inline_literal(raw)).to(eq("outer\n    inner".to_string()))
    }

    #[test]
    fn normalize_leaves_a_bare_single_line_literal_alone() -> TestResult {
        expect!(normalize_inline_literal("just this")).to(eq("just this".to_string()))
    }

    #[test]
    fn a_matching_literal_passes_in_compare_mode() -> TestResult {
        let location = InlineLocation {
            file: "src/x.rs".to_string(),
            line: 10,
            column: 5,
        };
        assert_inline_snapshot("hello", "\n    hello\n", &location, SnapshotMode::Compare)
            .or_fail_with("a matching literal must compare equal")
    }

    #[test]
    fn a_differing_literal_fails_in_compare_mode_carrying_both_sides() -> TestResult {
        let location = InlineLocation {
            file: "src/x.rs".to_string(),
            line: 10,
            column: 5,
        };
        let failure = assert_inline_snapshot(
            "actual",
            "\n    expected\n",
            &location,
            SnapshotMode::Compare,
        )
        .err()
        .or_fail_with("a differing literal must fail in compare mode")?;
        expect!(failure.expected).to(eq("expected".to_string()))?;
        expect!(failure.actual).to(eq("actual".to_string()))
    }

    #[test]
    fn parse_pending_patch_round_trips_a_recorded_body() -> TestResult {
        let body = "tests/foo.rs\n42:9\nline one\nline two";
        let (location, value) = parse_pending_patch(body).or_fail()?;
        expect!(location.file.as_str()).to(eq("tests/foo.rs"))?;
        expect!(location.line).to(eq(42u32))?;
        expect!(location.column).to(eq(9u32))?;
        expect!(value).to(eq("line one\nline two".to_string()))?;
        // A malformed body is rejected, not silently accepted.
        expect!(parse_pending_patch("only-one-line").is_err()).to(is_true())
    }
}
