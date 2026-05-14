//! Integration coverage for `expect!(fut).to_complete_within(..)` on the smol
//! runtime (PROJECT_BUILD_PLAN.md Iteration 5.2).
//!
//! This crate is excluded from the workspace so its `smol` runtime feature is
//! never unified with the `tokio`/`async-std` crates. It is run on its own:
//! `cargo test --manifest-path tests/timeout-smol/Cargo.toml`.
//!
//! smol has no `#[smol::test]` attribute, so each test drives its future with
//! `smol::block_on`.

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use test_better::prelude::*;

    #[test]
    fn a_fast_future_completes_within_the_limit() -> TestResult {
        smol::block_on(async {
            expect!(async { 1 + 1 })
                .to_complete_within(Duration::from_secs(5))
                .await?;
            Ok(())
        })
    }

    #[test]
    fn a_slow_future_trips_the_limit() -> TestResult {
        smol::block_on(async {
            let error = expect!(smol::Timer::after(Duration::from_secs(30)))
                .to_complete_within(Duration::from_millis(10))
                .await
                .expect_err("a 30s timer cannot finish in 10ms");
            let rendered = error.to_string();
            expect!(rendered.contains("did not complete within")).to(is_true())?;
            Ok(())
        })
    }
}
