//! Integration coverage for `expect!(fut).to_complete_within(..)` on the Tokio
//! runtime (PROJECT_BUILD_PLAN.md Iteration 5.2).
//!
//! This crate is excluded from the workspace so its `tokio` runtime feature is
//! never unified with the `async-std`/`smol` crates. It is run on its own:
//! `cargo test --manifest-path tests/timeout-tokio/Cargo.toml`.

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use test_better::prelude::*;

    #[tokio::test]
    async fn a_fast_future_completes_within_the_limit() -> TestResult {
        expect!(async { 1 + 1 })
            .to_complete_within(Duration::from_secs(5))
            .await?;
        Ok(())
    }

    #[tokio::test]
    async fn a_slow_future_trips_the_limit() -> TestResult {
        let error = expect!(tokio::time::sleep(Duration::from_secs(30)))
            .to_complete_within(Duration::from_millis(10))
            .await
            .expect_err("a 30s sleep cannot finish in 10ms");
        let rendered = error.to_string();
        expect!(rendered.contains("did not complete within")).to(is_true())?;
        Ok(())
    }

    #[tokio::test]
    async fn the_failure_names_the_expression_and_keeps_the_call_site() -> TestResult {
        let slow = tokio::time::sleep(Duration::from_secs(30));
        // The `to_complete_within` call site, captured here, must survive the
        // later `.await` on a different line.
        let line = line!() + 1;
        let pending = expect!(slow).to_complete_within(Duration::from_millis(10));
        let error = pending.await.expect_err("the sleep outlives the limit");
        // The `expect!`ed expression is echoed back in the message...
        expect!(error.to_string().contains("slow")).to(is_true())?;
        // ...and the failure points at the call site, not at the `.await`.
        expect!(error.location.line()).to(eq(line))?;
        Ok(())
    }
}
