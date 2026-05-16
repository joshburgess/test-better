//! Acceptance tests for the best-effort `quickcheck` bridge, exercised
//! through the `test-better` facade. The whole file is gated on the
//! `quickcheck` feature: with it off, this compiles to an empty test binary.

#![cfg(feature = "quickcheck")]

use test_better::prelude::*;
use test_better::{arbitrary, for_all};

#[test]
fn for_all_passes_a_property_that_holds_for_every_arbitrary_value() -> TestResult {
    // `arbitrary::<i32>()` draws `quickcheck::Arbitrary` values through the
    // same seam a `proptest` strategy uses. Negating twice is the identity.
    let outcome = for_all(arbitrary::<i32>(), |n: i32| {
        check!(n.wrapping_neg().wrapping_neg()).satisfies(eq(n))
    });
    check!(outcome.is_ok()).satisfies(is_true())
}

#[test]
fn for_all_shrinks_a_quickcheck_counterexample() -> TestResult {
    // "every u32 is below 10" is false. The bridge maps `quickcheck`'s linear
    // `shrink` onto the seam's `simplify`/`complicate` protocol: the result is
    // still a valid counterexample (at or above the bound) and never larger
    // than the input that first failed. Unlike `proptest`'s integrated
    // shrinking, `quickcheck` does not promise the exact boundary value, so
    // the test asserts only what the reduced-fidelity bridge guarantees.
    let failure = for_all(arbitrary::<u32>(), |n: u32| check!(n).satisfies(lt(10u32)))
        .err()
        .or_fail_with("values at or above 10 exist")?;
    check!(failure.shrunk).satisfies(ge(10u32))?;
    check!(failure.shrunk).satisfies(le(failure.original))?;
    check!(failure.failure.to_string().contains("less than 10")).satisfies(is_true())
}
