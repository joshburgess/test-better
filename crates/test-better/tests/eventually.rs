//! Acceptance tests for the runtime-free `eventually_blocking` polling helper,
//! exercised through the `test-better` facade.
//!
//! The async `eventually` is covered by the per-runtime crates under
//! `tests/timeout-*`, where a real runtime drives its inter-probe sleep.
//! `eventually_blocking` needs no runtime, so it is tested here as an ordinary
//! `#[test]`.

use std::cell::Cell;
use std::time::{Duration, Instant};

use test_better::prelude::*;

#[test]
fn eventually_blocking_returns_the_moment_the_probe_passes() -> TestResult {
    let polls = Cell::new(0u32);
    eventually_blocking(Duration::from_secs(5), || {
        polls.set(polls.get() + 1);
        polls.get() >= 4
    })?;
    // A generous 5s budget, but the probe passes on its fourth call, so polling
    // stops there rather than running to the deadline.
    check!(polls.get()).satisfies(eq(4u32))?;
    Ok(())
}

#[test]
fn eventually_blocking_failure_names_the_elapsed_time_and_probe_count() -> TestResult {
    let polls = Cell::new(0u32);
    let started = Instant::now();
    let error = eventually_blocking(Duration::from_millis(50), || {
        polls.set(polls.get() + 1);
        false
    })
    .expect_err("a probe that never passes must time out");
    // It did not give up early: the whole budget was spent.
    check!(started.elapsed() >= Duration::from_millis(50)).satisfies(is_true())?;
    let rendered = error.to_string();
    // The failure reports both how long it waited and how many times it probed.
    check!(rendered.contains("was not met within")).satisfies(is_true())?;
    check!(rendered.contains(&format!("{} probe", polls.get()))).satisfies(is_true())?;
    Ok(())
}
