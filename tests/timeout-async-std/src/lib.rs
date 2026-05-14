//! Integration coverage for `expect!(fut).to_complete_within(..)` on the
//! async-std runtime (PROJECT_BUILD_PLAN.md Iteration 5.2).
//!
//! This crate is excluded from the workspace so its `async-std` runtime
//! feature is never unified with the `tokio`/`smol` crates. It is run on its
//! own: `cargo test --manifest-path tests/timeout-async-std/Cargo.toml`.

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use test_better::prelude::*;

    #[async_std::test]
    async fn a_fast_future_completes_within_the_limit() -> TestResult {
        expect!(async { 1 + 1 })
            .to_complete_within(Duration::from_secs(5))
            .await?;
        Ok(())
    }

    #[async_std::test]
    async fn a_slow_future_trips_the_limit() -> TestResult {
        let error = expect!(async_std::task::sleep(Duration::from_secs(30)))
            .to_complete_within(Duration::from_millis(10))
            .await
            .expect_err("a 30s sleep cannot finish in 10ms");
        let rendered = error.to_string();
        expect!(rendered.contains("did not complete within")).to(is_true())?;
        Ok(())
    }
}
