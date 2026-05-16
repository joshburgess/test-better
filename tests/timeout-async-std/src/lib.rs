//! Integration coverage for the runtime-gated async assertions on the
//! async-std runtime: `check!(fut).completes_within(..)` and `eventually`.
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
        check!(async { 1 + 1 })
            .completes_within(Duration::from_secs(5))
            .await?;
        Ok(())
    }

    #[async_std::test]
    async fn a_slow_future_trips_the_limit() -> TestResult {
        let error = check!(async_std::task::sleep(Duration::from_secs(30)))
            .completes_within(Duration::from_millis(10))
            .await
            .expect_err("a 30s sleep cannot finish in 10ms");
        let rendered = error.to_string();
        check!(rendered.contains("did not complete within")).satisfies(is_true())?;
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
        check!(polls.get()).satisfies(eq(3u32))?;
        Ok(())
    }

    #[async_std::test]
    async fn eventually_reports_elapsed_and_probe_count_on_timeout() -> TestResult {
        let error = eventually(Duration::from_millis(30), || async { false })
            .await
            .expect_err("a probe that is never true must time out");
        let rendered = error.to_string();
        check!(rendered.contains("was not met within")).satisfies(is_true())?;
        check!(rendered.contains("probe")).satisfies(is_true())?;
        Ok(())
    }
}
