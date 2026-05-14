//! Failing tests in two feature areas plus one structureless `panic!`, so
//! `cargo test-better` against this fixture exercises context-chain grouping:
//!
//! - the two `the user store` tests group together,
//! - the one `the http layer` test forms its own group,
//! - `arithmetic_is_hard` panics with no `test-better` structure, so it must
//!   still appear in the summary, ungrouped and labelled "unstructured".
//!
//! Each test fails on purpose; the crate is never built for real use.

#![allow(clippy::tests_outside_test_module)]

use test_better_core::{ContextExt, ErrorKind, TestError, TestResult};

#[test]
fn user_count_matches() -> TestResult {
    Err::<(), _>(TestError::assertion("row count differs: expected 3, got 2"))
        .context("the user store")?;
    Ok(())
}

#[test]
fn user_store_connects() -> TestResult {
    Err::<(), _>(TestError::new(ErrorKind::Setup).with_message("connection refused"))
        .context("the user store")?;
    Ok(())
}

#[test]
fn endpoint_returns_ok() -> TestResult {
    Err::<(), _>(TestError::assertion("status was 500, expected 200"))
        .context("the http layer")?;
    Ok(())
}

#[test]
fn arithmetic_is_hard() {
    assert_eq!(2 + 2, 5, "a plain panic, with no test-better structure");
}
