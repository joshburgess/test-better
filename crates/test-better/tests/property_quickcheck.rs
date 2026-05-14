//! Acceptance tests for the best-effort `quickcheck` bridge, exercised
//! through the `test-better` facade. The whole file is gated on the
//! `quickcheck` feature: with it off, this compiles to an empty test binary.

#![cfg(feature = "quickcheck")]

use test_better::prelude::*;
use test_better::{arbitrary, check};

#[test]
fn check_passes_a_property_that_holds_for_every_arbitrary_value() -> TestResult {
    // `arbitrary::<i32>()` draws `quickcheck::Arbitrary` values through the
    // same seam a `proptest` strategy uses. Negating twice is the identity.
    let outcome = check(arbitrary::<i32>(), |n: i32| {
        expect!(n.wrapping_neg().wrapping_neg()).to(eq(n))
    });
    expect!(outcome.is_ok()).to(is_true())
}

#[test]
fn check_shrinks_a_quickcheck_counterexample() -> TestResult {
    // "every u32 is below 10" is false. The bridge maps `quickcheck`'s linear
    // `shrink` onto the seam's `simplify`/`complicate` protocol: the result is
    // still a valid counterexample (at or above the bound) and never larger
    // than the input that first failed. Unlike `proptest`'s integrated
    // shrinking, `quickcheck` does not promise the exact boundary value, so
    // the test asserts only what the reduced-fidelity bridge guarantees.
    let failure = check(arbitrary::<u32>(), |n: u32| expect!(n).to(lt(10u32)))
        .err()
        .or_fail_with("values at or above 10 exist")?;
    expect!(failure.shrunk).to(ge(10u32))?;
    expect!(failure.shrunk).to(le(failure.original))?;
    expect!(failure.failure.to_string().contains("less than 10")).to(is_true())
}
