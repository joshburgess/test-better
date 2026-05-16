//! `test-better-accept`: applies pending inline-snapshot patches to source.
//!
//! After a test run with `UPDATE_SNAPSHOTS=1`, every inline-snapshot mismatch
//! has been recorded as a pending patch under `target/test-better-pending/`
//! (the test run itself cannot rewrite the file it is expanding from). Running
//! this binary reads those patches and rewrites the `matches_inline_snapshot`
//! literals in place, then deletes the spent patch files.
//!
//! It is a thin shell around [`test_better_snapshot::apply_pending_patches`];
//! all the logic lives in the library's `accept` module so it can be exercised
//! by tests against fixture files. Exit code is `0` on success (including
//! "nothing to do") and `1` on any failure.

use std::process::ExitCode;

fn main() -> ExitCode {
    match test_better_snapshot::apply_pending_patches() {
        Ok(applied) if applied.is_empty() => {
            println!("test-better-accept: no pending inline-snapshot patches");
            ExitCode::SUCCESS
        }
        Ok(applied) => {
            let total: usize = applied.iter().map(|file| file.patches).sum();
            println!(
                "test-better-accept: applied {total} patch(es) across {} file(s):",
                applied.len()
            );
            for file in &applied {
                println!("  {} ({} patch(es))", file.file.display(), file.patches);
            }
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("test-better-accept: {error}");
            ExitCode::FAILURE
        }
    }
}
