//! The `Strategy<T>` seam: the trait the property runner is written against,
//! and the `proptest` backend that satisfies it.
//!
//! The seam is deliberately small. A [`Strategy`] knows how to draw one
//! [`ValueTree`] from a [`Runner`]'s randomness; a `ValueTree` holds a current
//! value and can `simplify`/`complicate` it. That is exactly enough to drive
//! `proptest`'s integrated shrinking, and it leaves room for a second backend
//! later without promising one today.
//!
//! `proptest` plugs in through a blanket impl: every
//! `proptest::strategy::Strategy` *is* a [`Strategy`] here, so a property test
//! names ordinary `proptest` strategies and the runner never mentions
//! `proptest` in its own signatures.

use std::fmt;

/// An opaque error from a strategy that could not produce a value.
///
/// It wraps the backend's own generation error so callers do not depend on the
/// backend's type. In practice the simple strategies a property test uses
/// (`any::<T>()`, numeric ranges) never fail to generate; this surfaces only
/// for heavily filtered strategies that exhaust their rejection budget.
#[derive(Debug, Clone)]
pub struct GenError(proptest::test_runner::Reason);

impl fmt::Display for GenError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "strategy could not generate a value: {}", self.0)
    }
}

impl std::error::Error for GenError {}

/// The per-run state a [`Strategy`] draws from: the random number generator
/// and the backend's bookkeeping.
///
/// It wraps `proptest`'s `TestRunner`. The seam owns this type so the backend
/// does not appear in the property runner's public signatures.
pub struct Runner {
    inner: proptest::test_runner::TestRunner,
}

impl Runner {
    /// A runner with a fixed, reproducible seed: the same sequence of generated
    /// values on every run.
    ///
    /// This is the default behind [`check`](crate::check). A property test that
    /// is reproducible cannot flake from the RNG: it passes or fails the same
    /// way every time, in CI and on a laptop. A caller who wants a fresh seed
    /// each run constructs one with [`Runner::randomized`] and passes it to
    /// [`check_with`](crate::check_with).
    #[must_use]
    pub fn deterministic() -> Self {
        Self {
            inner: proptest::test_runner::TestRunner::deterministic(),
        }
    }

    /// A runner seeded from the environment, or randomly when the environment
    /// says nothing, like a stock `proptest` run.
    #[must_use]
    pub fn randomized() -> Self {
        Self {
            inner: proptest::test_runner::TestRunner::default(),
        }
    }

    /// The wrapped backend runner. Private: only the `proptest` adapter impl in
    /// this module reaches through the seam.
    fn backend(&mut self) -> &mut proptest::test_runner::TestRunner {
        &mut self.inner
    }
}

impl Default for Runner {
    /// The reproducible runner, the same as [`Runner::deterministic`].
    fn default() -> Self {
        Self::deterministic()
    }
}

/// A source of values of type `T`, with shrinking, for property testing.
///
/// This is the seam between `test-better`'s property runner
/// ([`check`](crate::check)) and a concrete generation/shrinking backend. The
/// crate ships exactly one backend, `proptest`: every `proptest::strategy::Strategy`
/// is a `Strategy` here through a blanket impl.
///
/// # The blanket impl and its one limitation
///
/// Because the blanket `impl<S: proptest::strategy::Strategy> Strategy for S`
/// covers every `proptest` strategy, a user type that *also* happens to be a
/// `proptest::strategy::Strategy` cannot carry a hand-written `Strategy` impl
/// (coherence cannot prove the two do not overlap). This is acceptable today:
/// `proptest` is the one backend and every strategy is a `proptest` strategy
/// already. The trait is a seam for a future backend, not a finished
/// portability layer.
pub trait Strategy<T> {
    /// The shrinkable, in-progress value this strategy produces.
    type Tree: ValueTree<T>;

    /// Draws one fresh value tree from `runner`'s randomness.
    ///
    /// # Errors
    ///
    /// Returns [`GenError`] if the strategy could not produce a value (a
    /// filtered strategy that exhausted its rejection budget).
    fn new_tree(&self, runner: &mut Runner) -> Result<Self::Tree, GenError>;
}

/// A single generated value that can be shrunk toward a simpler one.
///
/// After a failing case the runner calls [`simplify`](Self::simplify) to get a
/// smaller candidate; if a candidate shrank so far it stopped failing, the
/// runner calls [`complicate`](Self::complicate) to walk back. Together they
/// binary-search toward a minimal counterexample.
pub trait ValueTree<T> {
    /// The current value.
    fn current(&self) -> T;

    /// Replaces the current value with a simpler one. Returns `true` if it
    /// moved, `false` if the value is already as simple as the tree can make
    /// it.
    fn simplify(&mut self) -> bool;

    /// Walks back toward the last value that still failed, undoing a
    /// [`simplify`](Self::simplify) that shrank past the failure. Returns
    /// `true` if it moved.
    fn complicate(&mut self) -> bool;
}

/// Adapts a `proptest` value tree to the seam's [`ValueTree`].
///
/// Produced by the blanket [`Strategy`] impl; rarely named directly.
pub struct ProptestTree<VT>(VT);

impl<VT, T> ValueTree<T> for ProptestTree<VT>
where
    VT: proptest::strategy::ValueTree<Value = T>,
{
    fn current(&self) -> T {
        self.0.current()
    }

    fn simplify(&mut self) -> bool {
        self.0.simplify()
    }

    fn complicate(&mut self) -> bool {
        self.0.complicate()
    }
}

impl<S, T> Strategy<T> for S
where
    S: proptest::strategy::Strategy<Value = T>,
{
    type Tree = ProptestTree<S::Tree>;

    fn new_tree(&self, runner: &mut Runner) -> Result<Self::Tree, GenError> {
        proptest::strategy::Strategy::new_tree(self, runner.backend())
            .map(ProptestTree)
            .map_err(GenError)
    }
}

/// The default [`Strategy`] for a type: `proptest`'s `any::<T>()`, surfaced
/// through the seam.
///
/// This is what [`property!`](crate::property) uses when the closure binding
/// has a type annotation and no `using` clause. It is available for direct use
/// too: `check(any::<u32>(), |n| ...)` is the same as naming the `u32` strategy
/// inline. The `quickcheck` counterpart is
/// [`arbitrary`](crate::quickcheck_bridge::arbitrary).
///
/// ```
/// use test_better_core::TestResult;
/// use test_better_matchers::{expect, ge};
/// use test_better_property::{any, check};
///
/// # fn main() -> TestResult {
/// check(any::<u8>(), |n: u8| expect!(u16::from(n)).to(ge(0u16)))
///     .map_err(|f| f.failure)?;
/// # Ok(())
/// # }
/// ```
#[must_use]
pub fn any<T>() -> impl Strategy<T>
where
    T: proptest::arbitrary::Arbitrary,
{
    proptest::arbitrary::any::<T>()
}

#[cfg(test)]
mod tests {
    use super::*;

    use test_better_core::{OrFail, TestResult};
    use test_better_matchers::{expect, ge, is_true};

    #[test]
    fn a_proptest_strategy_is_a_seam_strategy() -> TestResult {
        // The blanket impl makes a numeric range usable through the seam with
        // no wrapping at the call site.
        let mut runner = Runner::deterministic();
        let tree = (0u32..10).new_tree(&mut runner).or_fail()?;
        expect!(tree.current() < 10).to(is_true())
    }

    #[test]
    fn simplify_shrinks_the_current_value_toward_its_origin() -> TestResult {
        // `proptest` shrinks integers toward zero, so simplifying repeatedly
        // never *grows* the value.
        let mut runner = Runner::deterministic();
        let mut tree = (5u32..1_000).new_tree(&mut runner).or_fail()?;
        let start = tree.current();
        while tree.simplify() {}
        expect!(start).to(ge(tree.current()))
    }
}
