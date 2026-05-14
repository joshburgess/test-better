//! Acceptance tests for the async `expect!` method `resolves_to`
//! (PROJECT_BUILD_PLAN.md Iteration 5.1), exercised through the `test-better`
//! facade.
//!
//! `resolves_to` is runtime-agnostic: it only awaits the future. These tests
//! prove that by driving the same assertion under two unrelated executors,
//! `pollster::block_on` and `#[tokio::test]`.

use std::time::Duration;

use test_better::prelude::*;

/// A future whose output is only known after it has actually been polled to
/// completion, so a passing assertion proves the `await` really happened.
async fn doubled(n: i32) -> i32 {
    n + n
}

#[test]
fn resolves_to_matches_the_output_under_pollster() -> TestResult {
    pollster::block_on(async {
        expect!(doubled(21)).resolves_to(eq(42)).await?;
        Ok(())
    })
}

#[test]
fn resolves_to_reports_the_output_on_a_mismatch() -> TestResult {
    pollster::block_on(async {
        let error = expect!(doubled(21))
            .resolves_to(eq(0))
            .await
            .expect_err("doubled(21) resolves to 42, not 0");
        let rendered = error.to_string();
        expect!(rendered.contains("doubled(21)")).to(is_true())?;
        expect!(rendered.contains("actual: 42")).to(is_true())?;
        Ok(())
    })
}

#[tokio::test]
async fn resolves_to_works_under_tokio_test() -> TestResult {
    expect!(doubled(50)).resolves_to(eq(100)).await?;
    Ok(())
}

#[tokio::test]
async fn resolves_to_composes_with_other_matchers() -> TestResult {
    // Nothing about `resolves_to` is special to `eq`: any matcher over the
    // output type works, including ones that arrive from an `async` block.
    expect!(async { vec![1, 2, 3] })
        .resolves_to(have_len(3usize))
        .await?;
    expect!(tokio::time::sleep(Duration::from_millis(1)))
        .resolves_to(eq(()))
        .await?;
    Ok(())
}
