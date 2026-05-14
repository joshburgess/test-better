//! `test-better-snapshot`: file-backed and inline snapshot testing.
//!
//! A snapshot test pins a value's rendered form to a file on disk: the first
//! run records it, later runs compare against it, and an intentional change is
//! accepted by rerunning with `UPDATE_SNAPSHOTS=1`.
//!
//! This crate is the storage-and-comparison core. It is deliberately
//! `std`-only and `TestError`-free: it knows
//! how to find a snapshot file, read it, write it, and report *what* differed,
//! as the structured [`SnapshotFailure`]. Turning that into a `TestError` with
//! a rendered diff is `test-better-matchers`' job, in
//! `expect!(value).to_match_snapshot("name")`.
//!
//! Snapshot files live at `tests/snapshots/<module-path>__<name>.snap`, with
//! `<module-path>` taken from the calling test's `module_path!()` (so two tests
//! in different modules can both name a snapshot `"output"` without colliding).
//! [`assert_snapshot`] resolves that directory from the current working
//! directory, which `cargo test` sets to the package root; [`assert_snapshot_in`]
//! takes the directory explicitly and is what tests of this crate drive.
//!
//! Inline snapshots (the `inline` module: [`assert_inline_snapshot`] and
//! friends) keep the snapshot literal in the test source instead. A mismatch
//! under `UPDATE_SNAPSHOTS=1` records a pending patch under `target/`, which
//! the `test-better-accept` companion binary (built with the `accept` feature)
//! applies back to the source.
//!
//! [`Redactions`] (the `redact` module) stabilize non-deterministic content
//! (UUIDs, timestamps) before either kind of snapshot is compared or stored, so
//! the noise never reaches the snapshot.

use std::error::Error;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

mod inline;
mod redact;

pub use inline::{
    InlineLocation, InlineSnapshotFailure, assert_inline_snapshot, normalize_inline_literal,
    parse_pending_patch, pending_patch_dir,
};
pub use redact::Redactions;

// The accept step is the only part of the crate that needs `syn`, so it is
// gated behind the `accept` feature along with the `test-better-accept` binary
// (`src/bin/test-better-accept.rs`) that drives it.
#[cfg(feature = "accept")]
mod accept;

#[cfg(feature = "accept")]
pub use accept::{
    AcceptError, Applied, apply_inline_patch, apply_patches_from, apply_pending_patches,
};

/// Whether a snapshot assertion compares against the stored file or rewrites it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SnapshotMode {
    /// Compare `actual` against the stored snapshot. A missing file is a
    /// failure: the snapshot has to be created deliberately, in [`Update`]
    /// mode, not conjured by the first passing run.
    ///
    /// [`Update`]: SnapshotMode::Update
    Compare,
    /// Write `actual` to the snapshot file, creating or overwriting it. This is
    /// how a new snapshot is recorded and how an intentional change is
    /// accepted.
    Update,
}

impl SnapshotMode {
    /// [`Update`](SnapshotMode::Update) when the `UPDATE_SNAPSHOTS` environment
    /// variable is set to a non-empty value, [`Compare`](SnapshotMode::Compare)
    /// otherwise.
    #[must_use]
    pub fn from_env() -> Self {
        match std::env::var_os("UPDATE_SNAPSHOTS") {
            Some(value) if !value.is_empty() => SnapshotMode::Update,
            _ => SnapshotMode::Compare,
        }
    }
}

/// Why a snapshot assertion did not pass.
///
/// This is the structured form: `test-better-matchers` turns it into a
/// `TestError` (a [`Mismatch`](SnapshotFailure::Mismatch) becomes an
/// expected/actual payload with a diff), but the failure is described here so
/// the crate is usable on its own.
#[derive(Debug)]
pub enum SnapshotFailure {
    /// No snapshot file exists and the mode was [`Compare`](SnapshotMode::Compare).
    Missing {
        /// Where the snapshot file was expected.
        path: PathBuf,
    },
    /// The snapshot file exists but its contents differ from `actual`.
    Mismatch {
        /// The snapshot file that was compared against.
        path: PathBuf,
        /// The stored snapshot (the file's contents).
        expected: String,
        /// The value under test.
        actual: String,
    },
    /// Resolving the snapshot directory, reading the file, or writing it failed.
    Io {
        /// The snapshot file (or directory) the operation concerned.
        path: PathBuf,
        /// A short description of what was being attempted, e.g.
        /// `"reading the snapshot file"`.
        action: &'static str,
        /// The underlying I/O error.
        source: std::io::Error,
    },
}

impl fmt::Display for SnapshotFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SnapshotFailure::Missing { path } => write!(
                f,
                "no snapshot at {}; rerun with UPDATE_SNAPSHOTS=1 to create it",
                path.display()
            ),
            SnapshotFailure::Mismatch { path, .. } => {
                write!(f, "snapshot at {} does not match", path.display())
            }
            SnapshotFailure::Io {
                path,
                action,
                source,
            } => write!(f, "I/O error {action} ({}): {source}", path.display()),
        }
    }
}

impl Error for SnapshotFailure {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            SnapshotFailure::Io { source, .. } => Some(source),
            _ => None,
        }
    }
}

/// The path of the snapshot file for `module_path` and `name` under `dir`.
///
/// The file name is `<module-path>__<name>.snap`, with both components
/// sanitized: `::` segment separators in a module path collapse to `__`, and
/// any other character that is not alphanumeric, `_`, or `-` becomes `_`.
///
/// ```
/// use std::path::{Path, PathBuf};
///
/// use test_better_core::TestResult;
/// use test_better_matchers::{eq, expect};
/// use test_better_snapshot::snapshot_path;
///
/// # fn main() -> TestResult {
/// let path = snapshot_path(Path::new("tests/snapshots"), "my_crate::ui", "homepage");
/// expect!(path).to(eq(PathBuf::from("tests/snapshots/my_crate__ui__homepage.snap")))?;
/// # Ok(())
/// # }
/// ```
#[must_use]
pub fn snapshot_path(dir: &Path, module_path: &str, name: &str) -> PathBuf {
    dir.join(format!(
        "{}__{}.snap",
        sanitize(module_path),
        sanitize(name)
    ))
}

/// Collapses `::` to `__` so module-path segments survive, then replaces any
/// remaining character that is not safe in a file name.
fn sanitize(raw: &str) -> String {
    raw.replace("::", "__")
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' || c == '-' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

/// Compares `actual` against (or, in [`Update`](SnapshotMode::Update) mode,
/// writes it to) the snapshot file for `module_path`/`name` under `dir`.
///
/// This is the directory-explicit core: [`assert_snapshot`] is the wrapper that
/// derives `dir` from the current working directory. Tests of this crate use
/// `assert_snapshot_in` against a temporary directory so they need not depend
/// on the process's working directory or a committed fixture file.
///
/// In [`Compare`](SnapshotMode::Compare) mode it returns [`SnapshotFailure`] on
/// a mismatch or a missing file. In [`Update`](SnapshotMode::Update) mode it
/// creates the directory if needed, writes `actual`, and returns `Ok(())`.
///
/// # Errors
///
/// Returns [`SnapshotFailure`] when the snapshot does not match, does not exist
/// (in `Compare` mode), or an I/O operation fails.
pub fn assert_snapshot_in(
    dir: &Path,
    module_path: &str,
    name: &str,
    actual: &str,
    mode: SnapshotMode,
) -> Result<(), SnapshotFailure> {
    let path = snapshot_path(dir, module_path, name);
    match mode {
        SnapshotMode::Update => {
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).map_err(|source| SnapshotFailure::Io {
                    path: path.clone(),
                    action: "creating the snapshot directory",
                    source,
                })?;
            }
            fs::write(&path, actual).map_err(|source| SnapshotFailure::Io {
                path,
                action: "writing the snapshot file",
                source,
            })
        }
        SnapshotMode::Compare => match fs::read_to_string(&path) {
            Ok(expected) if expected == actual => Ok(()),
            Ok(expected) => Err(SnapshotFailure::Mismatch {
                path,
                expected,
                actual: actual.to_string(),
            }),
            Err(source) if source.kind() == std::io::ErrorKind::NotFound => {
                Err(SnapshotFailure::Missing { path })
            }
            Err(source) => Err(SnapshotFailure::Io {
                path,
                action: "reading the snapshot file",
                source,
            }),
        },
    }
}

/// Compares `actual` against the snapshot for `module_path`/`name`, with the
/// snapshot directory resolved as `tests/snapshots` under the current working
/// directory and the mode read from `UPDATE_SNAPSHOTS`.
///
/// `cargo test` runs a test binary with its working directory set to the
/// package root, so `tests/snapshots` lands in the package being tested. This
/// is the entry point `expect!(value).to_match_snapshot("name")` calls; reach
/// for [`assert_snapshot_in`] when the directory or mode must be explicit.
///
/// # Errors
///
/// Returns [`SnapshotFailure`] when the snapshot does not match, does not exist
/// (and `UPDATE_SNAPSHOTS` is unset), or an I/O operation fails (including
/// failing to resolve the current directory).
pub fn assert_snapshot(module_path: &str, name: &str, actual: &str) -> Result<(), SnapshotFailure> {
    let base = std::env::current_dir().map_err(|source| SnapshotFailure::Io {
        path: PathBuf::from("tests/snapshots"),
        action: "resolving the current directory",
        source,
    })?;
    assert_snapshot_in(
        &base.join("tests").join("snapshots"),
        module_path,
        name,
        actual,
        SnapshotMode::from_env(),
    )
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use test_better_core::{OrFail, TestResult};
    use test_better_matchers::{contains_str, eq, expect, is_true};

    use super::*;

    #[test]
    fn snapshot_path_joins_module_and_name_with_a_snap_extension() -> TestResult {
        let path = snapshot_path(Path::new("tests/snapshots"), "snapshot", "homepage");
        expect!(path).to(eq(PathBuf::from("tests/snapshots/snapshot__homepage.snap")))
    }

    #[test]
    fn snapshot_path_collapses_module_separators_and_sanitizes() -> TestResult {
        let path = snapshot_path(Path::new("snaps"), "my_crate::ui::pages", "home page/v2");
        expect!(path).to(eq(PathBuf::from(
            "snaps/my_crate__ui__pages__home_page_v2.snap",
        )))
    }

    #[test]
    fn missing_file_in_compare_mode_is_a_missing_failure() -> TestResult {
        let dir = scratch_dir("missing");
        let outcome = assert_snapshot_in(&dir, "t", "absent", "value", SnapshotMode::Compare);
        let failure = outcome
            .err()
            .or_fail_with("a missing snapshot should fail")?;
        expect!(matches!(failure, SnapshotFailure::Missing { .. })).to(is_true())?;
        // The message points the reader at how to create it.
        expect!(failure.to_string().as_str()).to(contains_str("UPDATE_SNAPSHOTS=1"))?;
        let _ = fs::remove_dir_all(&dir);
        Ok(())
    }

    /// A unique scratch directory under the system temp dir, named for the
    /// calling test so parallel tests never share one.
    fn scratch_dir(tag: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "test-better-snapshot-{}-{}",
            std::process::id(),
            tag
        ))
    }
}
