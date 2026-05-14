//! The structured-output channel between a `test-better` test and the optional
//! `cargo test-better` runner (Phase 9).
//!
//! The runner never parses rendered failure text. Instead, when it is the one
//! running `cargo test`, it sets [`RUNNER_ENV`] in the child's environment; a
//! failing `test-better` test sees that variable and, in addition to its normal
//! human-readable failure, prints one line of the form
//! `<STRUCTURED_MARKER><json><STRUCTURED_MARKER>` to stdout. `cargo test`
//! captures and replays the output of *failing* tests, so the runner recovers
//! the structured failure from that captured stream with no side-channel file.
//!
//! The JSON payload is the serde serialization of [`StructuredError`]; the
//! marker brackets it so the runner can pick it out of arbitrary test output
//! and the test's own renderer output stays untouched.

use std::fmt;

use crate::error::TestError;

/// The environment variable the runner sets on the `cargo test` it spawns.
///
/// A `test-better` test emits its structured failure only when this is present
/// in its environment, so an ordinary `cargo test` run stays unaffected.
pub const RUNNER_ENV: &str = "TEST_BETTER_RUNNER";

/// The sentinel that brackets the JSON structured-error payload on its own line
/// in captured test output. Chosen to be unmistakable in prose and to sit alone
/// on a line.
pub const STRUCTURED_MARKER: &str = "@@test-better-structured-error-v1@@";

/// Appends the structured-error marker line for `error`, but only when the
/// runner asked for it via [`RUNNER_ENV`] and the `serde` feature is built in.
///
/// This is called from `TestError`'s `Debug` impl, after the human-readable
/// render, so the marker line trails the normal failure output. A serialization
/// error is swallowed: the structured channel is best-effort tooling support,
/// and the human-readable failure has already been written regardless.
#[cfg(feature = "serde")]
pub(crate) fn write_structured_marker(
    error: &TestError,
    f: &mut fmt::Formatter<'_>,
) -> fmt::Result {
    if std::env::var_os(RUNNER_ENV).is_none() {
        return Ok(());
    }
    if let Ok(json) = serde_json::to_string(&error.to_structured()) {
        write!(f, "\n{STRUCTURED_MARKER}{json}{STRUCTURED_MARKER}")?;
    }
    Ok(())
}

/// Without the `serde` feature there is no wire format, so the structured
/// channel is a no-op: the runner simply sees every failure as unstructured.
#[cfg(not(feature = "serde"))]
pub(crate) fn write_structured_marker(
    _error: &TestError,
    _f: &mut fmt::Formatter<'_>,
) -> fmt::Result {
    Ok(())
}
