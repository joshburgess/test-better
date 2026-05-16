//! End-to-end coverage of the accept step: a pending patch on disk, applied by
//! `apply_patches_from`, rewrites
//! the inline-snapshot literal in a real source file and removes the spent
//! patch so a rerun is a no-op.
//!
//! The fixture is a small `.rs` file written into a scratch directory rather
//! than a committed one: the test *mutates* it, so it has to own a fresh copy.
#![cfg(feature = "accept")]

use std::fs;
use std::path::PathBuf;

use test_better_core::{OrFail, TestResult};
use test_better_matchers::{check, eq, is_false, is_true};
use test_better_snapshot::apply_patches_from;

/// A unique scratch directory under the system temp dir, named for the calling
/// test so parallel tests never share one.
fn scratch_dir(tag: &str) -> PathBuf {
    std::env::temp_dir().join(format!("test-better-accept-{}-{}", std::process::id(), tag))
}

#[test]
fn a_pending_patch_rewrites_the_literal_and_is_consumed() -> TestResult {
    let root = scratch_dir("rewrite");
    let pending = root.join("pending");
    fs::create_dir_all(&pending).or_fail()?;

    // A fixture source file with a stale inline snapshot: the call sits on
    // line 2, indented four columns.
    let fixture = root.join("fixture.rs");
    let original =
        "fn check() {\n    check!(render()).matches_inline_snapshot(r#\"old value\"#)?;\n}\n";
    fs::write(&fixture, original).or_fail()?;

    // A pending patch naming that call site (relative to the workspace root)
    // and the corrected value.
    fs::write(pending.join("1-0.patch"), "fixture.rs\n2:4\nnew value").or_fail()?;

    let applied = apply_patches_from(&pending, &root).or_fail()?;
    check!(applied.len()).satisfies(eq(1usize))?;
    check!(applied[0].patches).satisfies(eq(1usize))?;

    let rewritten = fs::read_to_string(&fixture).or_fail()?;
    check!(rewritten.contains("r#\"new value\"#")).satisfies(is_true())?;
    check!(rewritten.contains("old value")).satisfies(is_false())?;
    // Everything outside the literal is byte-for-byte the same.
    check!(rewritten.starts_with("fn check() {\n")).satisfies(is_true())?;
    check!(rewritten.ends_with("?;\n}\n")).satisfies(is_true())?;

    // The spent patch file is gone, so a second run finds nothing to do.
    check!(pending.join("1-0.patch").exists()).satisfies(is_false())?;
    let second = apply_patches_from(&pending, &root).or_fail()?;
    check!(second.is_empty()).satisfies(is_true())?;

    let _ = fs::remove_dir_all(&root);
    Ok(())
}

#[test]
fn a_missing_pending_directory_is_a_no_op() -> TestResult {
    let root = scratch_dir("absent");
    let applied = apply_patches_from(&root.join("pending"), &root).or_fail()?;
    check!(applied.is_empty()).satisfies(is_true())
}
