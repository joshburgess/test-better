//! Integration coverage for the runtime-gated async assertions on the
//! async-std runtime: `expect!(fut).to_complete_within(..)`
//! (PROJECT_BUILD_PLAN.md Iteration 5.2) and `eventually` (Iteration 5.3).
//!
//! This crate is excluded from the workspace so its `async-std` runtime
//! feature is never unified with the `tokio`/`smol` crates. It is run on its
//! own: `cargo test --manifest-path tests/timeout-async-std/Cargo.toml`.

#[cfg(test)]
mod tests {
    use std::cell::Cell;
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

    #[async_std::test]
    async fn eventually_stops_polling_once_the_probe_passes() -> TestResult {
        let polls = Cell::new(0u32);
        eventually(Duration::from_secs(5), || {
            polls.set(polls.get() + 1);
            let done = polls.get() >= 3;
            async move { done }
        })
        .await?;
        expect!(polls.get()).to(eq(3u32))?;
        Ok(())
    }

    #[async_std::test]
    async fn eventually_reports_elapsed_and_probe_count_on_timeout() -> TestResult {
        let error = eventually(Duration::from_millis(30), || async { false })
            .await
            .expect_err("a probe that is never true must time out");
        let rendered = error.to_string();
        expect!(rendered.contains("was not met within")).to(is_true())?;
        expect!(rendered.contains("probe")).to(is_true())?;
        Ok(())
    }
}
