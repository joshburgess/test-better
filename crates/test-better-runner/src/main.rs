//! `cargo-test-better`: optional pretty runner binary (PROJECT_BUILD_PLAN.md §14).
//!
//! A thin shell over [`test_better_runner::run`]: it wraps `cargo test`,
//! forwards every argument, and propagates the exit code so that, as
//! Iteration 9.1 requires, `cargo test-better` and `cargo test` agree on
//! success and failure.

use std::process::ExitCode;

fn main() -> ExitCode {
    match test_better_runner::run(std::env::args_os().skip(1)) {
        Ok(code) => ExitCode::from(u8::try_from(code).unwrap_or(101)),
        Err(error) => {
            eprintln!("cargo-test-better: could not run `cargo test`: {error}");
            ExitCode::from(101)
        }
    }
}
