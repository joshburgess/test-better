//! The property runner: generate cases, run the predicate, shrink on failure.
//!
//! [`check`] is the whole of Iteration 6.1b's user-facing surface. It draws
//! values from a [`Strategy`], runs a `T -> TestResult` predicate against each,
//! and, on the first failure, drives the [`ValueTree`] shrink protocol to a
//! minimal counterexample. The `property!` macro (Iteration 6.2) is a thin
//! syntactic wrapper over this; the shrunk-failure *rendering* is Iteration
//! 6.3's job, so a [`PropertyFailure`] here is plain structured data.

use test_better_core::{TestError, TestResult};

use crate::strategy::{Runner, Strategy, ValueTree};

/// How a property run is configured.
#[derive(Debug, Clone, Copy)]
pub struct Config {
    /// How many generated cases to try before concluding the property holds.
    pub cases: u32,
}

impl Default for Config {
    /// 256 cases, matching `proptest`'s own default.
    fn default() -> Self {
        Self { cases: 256 }
    }
}

/// A property that did not hold.
///
/// It carries the counterexample twice: `original` is the first generated
/// input that failed, `shrunk` is the minimal failing input the shrink search
/// reached. `failure` is the [`TestError`] the shrunk input produced, and
/// `cases` is how many inputs ran (including the failing one) before shrinking
/// began.
#[derive(Debug)]
pub struct PropertyFailure<T> {
    /// The first generated input that failed the property.
    pub original: T,
    /// The minimal failing input the shrink search reached.
    pub shrunk: T,
    /// The failure produced by `shrunk`.
    pub failure: TestError,
    /// How many cases ran (including the failing one) before shrinking began.
    pub cases: u32,
}

/// Checks `property` against values from `strategy`, using [`Config::default`]
/// and a reproducible [`Runner`].
///
/// Returns `Ok(())` if every generated case satisfies `property`, or a
/// [`PropertyFailure`] carrying the shrunk counterexample otherwise. The run is
/// deterministic: the same strategy and property pass or fail the same way
/// every time (see [`Runner::deterministic`]). For an explicit case count or a
/// randomized runner, use [`check_with`].
///
/// ```
/// use test_better_core::TestResult;
/// use test_better_matchers::{expect, lt};
/// use test_better_property::check;
///
/// # fn main() -> TestResult {
/// // Holds for every `u8`: doubling in `u16` never overflows.
/// check(0u8..=255, |n| {
///     let doubled = u16::from(n) * 2;
///     expect!(doubled).to(lt(512u16))
/// })
/// .map_err(|f| f.failure)?;
/// # Ok(())
/// # }
/// ```
pub fn check<T, S, F>(strategy: S, property: F) -> Result<(), PropertyFailure<T>>
where
    S: Strategy<T>,
    T: Clone,
    F: FnMut(T) -> TestResult,
{
    check_with(
        Config::default(),
        &mut Runner::deterministic(),
        strategy,
        property,
    )
}

/// Checks `property` against values from `strategy` with an explicit [`Config`]
/// and [`Runner`].
///
/// This is [`check`] with its two defaults exposed: pass a [`Config`] to change
/// the case count, and a [`Runner`] (for example [`Runner::randomized`]) to
/// change the seeding.
pub fn check_with<T, S, F>(
    config: Config,
    runner: &mut Runner,
    strategy: S,
    mut property: F,
) -> Result<(), PropertyFailure<T>>
where
    S: Strategy<T>,
    T: Clone,
    F: FnMut(T) -> TestResult,
{
    for case in 0..config.cases {
        // A strategy that cannot produce a value (an over-filtered strategy)
        // is not a property failure; skip the case and try another draw.
        let Ok(mut tree) = strategy.new_tree(runner) else {
            continue;
        };
        let value = tree.current();
        let Err(failure) = property(value.clone()) else {
            continue;
        };
        // `value` failed: shrink toward a minimal counterexample.
        let (shrunk, failure) = shrink(&mut tree, value.clone(), failure, &mut property);
        return Err(PropertyFailure {
            original: value,
            shrunk,
            failure,
            cases: case + 1,
        });
    }
    Ok(())
}

/// Drives the [`ValueTree`] shrink protocol from a known-failing value.
///
/// The protocol: `simplify` to a smaller candidate and test it. If it still
/// fails, adopt it and `simplify` again. If it stopped failing, `complicate`
/// back toward the last failure and test *that* candidate, repeating until
/// `complicate` can move no further. The inner loop is what makes the search
/// converge: every value `complicate` produces is re-tested, not skipped over
/// by a premature `simplify`. `minimal` always holds the simplest value seen
/// to still fail, so it is correct to return even though the tree's own
/// `current()` may sit on a passing value when the search ends.
fn shrink<T, VT, F>(
    tree: &mut VT,
    mut minimal: T,
    mut minimal_failure: TestError,
    property: &mut F,
) -> (T, TestError)
where
    VT: ValueTree<T>,
    T: Clone,
    F: FnMut(T) -> TestResult,
{
    while tree.simplify() {
        loop {
            let candidate = tree.current();
            match property(candidate.clone()) {
                // Simpler and still failing: adopt it, then `simplify` again.
                Err(failure) => {
                    minimal = candidate;
                    minimal_failure = failure;
                    break;
                }
                // Simplified past the failure: walk back. If `complicate` can
                // still move, test the value it lands on; if it cannot, the
                // search is exhausted.
                Ok(()) => {
                    if !tree.complicate() {
                        return (minimal, minimal_failure);
                    }
                }
            }
        }
    }
    (minimal, minimal_failure)
}

#[cfg(test)]
mod tests {
    use super::*;

    use test_better_core::{OrFail, TestResult};
    use test_better_matchers::{eq, expect, ge, is_true, lt};

    #[test]
    fn a_property_that_always_holds_passes() -> TestResult {
        let outcome = check(0u32..1_000, |n| expect!(n).to(lt(1_000u32)));
        expect!(outcome.is_ok()).to(is_true())
    }

    #[test]
    fn a_failing_property_shrinks_to_the_minimal_counterexample() -> TestResult {
        // "every u32 is below 100" is false; the smallest counterexample is
        // exactly 100, and `proptest` shrinks integers toward zero, so the
        // shrink search must land on it.
        let failure = check(proptest::num::u32::ANY, |n| expect!(n).to(lt(100u32)))
            .err()
            .or_fail_with("a property that is false for most u32 must fail")?;
        expect!(failure.shrunk).to(eq(100u32))?;
        // The original counterexample was some value at or above the bound...
        expect!(failure.original).to(ge(100u32))?;
        // ...and at least one case ran to find it.
        expect!(failure.cases).to(ge(1u32))
    }

    #[test]
    fn the_shrunk_failure_is_the_one_the_minimal_input_produces() -> TestResult {
        let failure = check(proptest::num::i64::ANY, |n| expect!(n).to(lt(0i64)))
            .err()
            .or_fail_with("non-negative i64 values exist")?;
        // The minimal non-negative i64 is 0.
        expect!(failure.shrunk).to(eq(0i64))?;
        // The carried `TestError` is the failure 0 itself produces.
        let rendered = failure.failure.to_string();
        expect!(rendered.contains("less than 0")).to(is_true())
    }

    #[test]
    fn check_with_honors_a_smaller_case_count() -> TestResult {
        // With a single case and an always-true property, exactly one draw is
        // taken and the run still passes.
        let mut runner = Runner::deterministic();
        let outcome = check_with(Config { cases: 1 }, &mut runner, 0u32..10, |_| {
            TestResult::Ok(())
        });
        expect!(outcome.is_ok()).to(is_true())
    }
}
