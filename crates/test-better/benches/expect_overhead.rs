//! Benchmark: the cost of `expect!` versus the stock `assert_eq!` on a hot
//! loop (PROJECT_BUILD_PLAN.md Iteration 10.4).
//!
//! This is a `harness = false` benchmark: it is an ordinary `fn main` that
//! times two loops with `std::time::Instant` and prints a comparison table. It
//! needs no benchmark framework, so it builds and runs on stable and adds no
//! dependency to the tree. `cargo test` runs it (the iteration count is kept
//! small enough for that); `cargo bench` runs it too.
//!
//! What it establishes: for a passing primitive matcher, `expect!` stays
//! within an order of magnitude of `assert_eq!`. Both are far below the cost
//! of anything a test actually does (I/O, allocation, spawning a runtime), so
//! the difference never shows up in a real suite. The numbers behind that
//! claim are written up in the book's "Performance" chapter.

use std::hint::black_box;
use std::time::{Duration, Instant};

use test_better::prelude::*;

/// Iterations per measured loop. Kept modest so `cargo test` can run this
/// binary as part of the suite without a noticeable pause; still large enough
/// that per-iteration timing is stable.
const ITERS: u64 = 10_000_000;

/// Times `body` over `ITERS` iterations after a warmup pass of the same size.
fn measure(mut body: impl FnMut()) -> Duration {
    for _ in 0..ITERS {
        body();
    }
    let start = Instant::now();
    for _ in 0..ITERS {
        body();
    }
    start.elapsed()
}

/// Nanoseconds per iteration, as an `f64` for the ratio arithmetic.
fn per_iter_ns(elapsed: Duration) -> f64 {
    elapsed.as_nanos() as f64 / ITERS as f64
}

fn main() {
    // A passing equality check: `assert_eq!` versus `expect!(..).to(eq(..))`.
    let baseline_eq = measure(|| {
        let a = black_box(8080_u32);
        let b = black_box(8080_u32);
        assert_eq!(a, b);
    });
    let expect_eq = measure(|| {
        let a = black_box(8080_u32);
        let b = black_box(8080_u32);
        // The `?`-free form: the `TestResult` is consumed by `black_box` so the
        // optimizer cannot drop the call.
        let _ = black_box(expect!(a).to(eq(b)));
    });

    // A passing ordering check: `assert!(a < b)` versus `expect!(a).to(lt(b))`.
    let baseline_lt = measure(|| {
        let a = black_box(1023_u32);
        let b = black_box(1024_u32);
        assert!(a < b);
    });
    let expect_lt = measure(|| {
        let a = black_box(1023_u32);
        let b = black_box(1024_u32);
        let _ = black_box(expect!(a).to(lt(b)));
    });

    let rows = [
        ("eq", per_iter_ns(baseline_eq), per_iter_ns(expect_eq)),
        ("lt", per_iter_ns(baseline_lt), per_iter_ns(expect_lt)),
    ];

    println!("expect! overhead vs the stock assert macros ({ITERS} iters/loop)");
    println!(
        "{:<8} {:>14} {:>14} {:>10}",
        "matcher", "assert (ns)", "expect (ns)", "ratio"
    );
    for (name, baseline, expect) in rows {
        let ratio = if baseline > 0.0 {
            expect / baseline
        } else {
            0.0
        };
        println!("{name:<8} {baseline:>14.2} {expect:>14.2} {ratio:>9.1}x");
    }
}
