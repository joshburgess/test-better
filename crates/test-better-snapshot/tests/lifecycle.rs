//! End-to-end snapshot lifecycle: a snapshot is *created*, then *verified*,
//! then an intentional change is
//! *updated*, and the new value verifies.
//!
//! This drives [`assert_snapshot_in`] against a temporary directory so the
//! test owns the whole snapshot file lifecycle without depending on the
//! process's working directory or a committed fixture. The `check!`-facing
//! `matches_snapshot` wrapper is exercised through the `test-better` facade in
//! its own `tests/snapshot.rs`.

use std::fs;
use std::path::PathBuf;

use test_better_core::{OrFail, TestError, TestResult};
use test_better_matchers::{check, eq, is_true};
use test_better_snapshot::{SnapshotFailure, SnapshotMode, assert_snapshot_in, snapshot_path};

#[test]
fn a_snapshot_is_created_then_verified_then_updated() -> TestResult {
    let dir = scratch_dir("lifecycle");
    // A clean slate, even if a previous run left the directory behind.
    let _ = fs::remove_dir_all(&dir);

    let module = "lifecycle";
    let name = "greeting";
    let path = snapshot_path(&dir, module, name);

    // 1. Compare with no file on disk: a missing snapshot, not a silent pass.
    let missing = assert_snapshot_in(&dir, module, name, "hello", SnapshotMode::Compare)
        .err()
        .or_fail_with("comparing against an absent snapshot must fail")?;
    check!(matches!(missing, SnapshotFailure::Missing { .. })).satisfies(is_true())?;
    check!(path.exists()).satisfies(eq(false))?;

    // 2. Update mode records the snapshot.
    assert_snapshot_in(&dir, module, name, "hello", SnapshotMode::Update).or_fail()?;
    check!(path.exists()).satisfies(is_true())?;
    check!(fs::read_to_string(&path).or_fail()?).satisfies(eq("hello".to_string()))?;

    // 3. Compare against the recorded snapshot now passes.
    assert_snapshot_in(&dir, module, name, "hello", SnapshotMode::Compare).or_fail()?;

    // 4. A changed value is a mismatch carrying both sides.
    let mismatch = assert_snapshot_in(&dir, module, name, "goodbye", SnapshotMode::Compare)
        .err()
        .or_fail_with("a changed value must not match the stored snapshot")?;
    match mismatch {
        SnapshotFailure::Mismatch {
            expected, actual, ..
        } => {
            check!(expected).satisfies(eq("hello".to_string()))?;
            check!(actual).satisfies(eq("goodbye".to_string()))?;
        }
        other => {
            return Err(TestError::custom(format!(
                "expected a mismatch, got {other:?}"
            )));
        }
    }

    // 5. Update accepts the change, and comparison passes again.
    assert_snapshot_in(&dir, module, name, "goodbye", SnapshotMode::Update).or_fail()?;
    assert_snapshot_in(&dir, module, name, "goodbye", SnapshotMode::Compare).or_fail()?;
    check!(fs::read_to_string(&path).or_fail()?).satisfies(eq("goodbye".to_string()))?;

    let _ = fs::remove_dir_all(&dir);
    Ok(())
}

/// A unique scratch directory under the system temp dir, named for the calling
/// test so parallel tests never share one.
fn scratch_dir(tag: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "test-better-snapshot-{}-{}",
        std::process::id(),
        tag
    ))
}
