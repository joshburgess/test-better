//! The `quickcheck` bridge: a best-effort second backend behind the
//! `quickcheck` feature.
//!
//! [`arbitrary`] turns any `quickcheck::Arbitrary` type into a seam
//! [`Strategy<T>`], so a property test can name `arbitrary::<MyType>()` wherever
//! it would name a `proptest` strategy. This exists to prove the [`Strategy`]
//! trait is a real seam and not a `proptest`-shaped hole.
//!
//! # Reduced fidelity
//!
//! The bridge is honest about two limitations, both inherent to `quickcheck`'s
//! model rather than bugs:
//!
//! - **Generation is not seeded by the [`Runner`].** `quickcheck::Gen` owns its
//!   own RNG and cannot be constructed from an external seed, so an
//!   `arbitrary()` strategy draws fresh entropy every run. [`Runner::deterministic`]
//!   does *not* make a `quickcheck`-backed property reproducible; only
//!   `proptest`-backed strategies honor it.
//! - **Shrinking is `quickcheck`'s linear `shrink`, not integrated shrinking.**
//!   `quickcheck::Arbitrary::shrink` yields a flat iterator of smaller
//!   candidates with no `complicate` step. The bridge maps that onto the seam's
//!   `simplify`/`complicate` protocol faithfully (every sibling candidate is
//!   tried, and an accepted candidate is recursively shrunk), but it is still
//!   `quickcheck`'s search, not `proptest`'s.
//!
//! For a property that must be reproducible, prefer a `proptest` strategy.

use std::marker::PhantomData;

use quickcheck::Arbitrary;

use crate::strategy::{GenError, Runner, Strategy, ValueTree};

/// The default `quickcheck::Gen` size, the generator's notion of "how big".
///
/// `quickcheck`'s own test harness uses 100; the bridge matches it so a type's
/// `Arbitrary` impl behaves the same through the seam as it does under
/// `quickcheck` directly.
const DEFAULT_GEN_SIZE: usize = 100;

/// Bridges a `quickcheck::Arbitrary` type into the seam as a [`Strategy<T>`].
///
/// Use it anywhere a strategy is expected:
///
/// ```
/// use test_better_core::TestResult;
/// use test_better_matchers::{check, lt};
/// use test_better_property::{arbitrary, for_all};
///
/// # fn main() -> TestResult {
/// // A `u8` is `quickcheck::Arbitrary`; doubling one in `u16` never overflows.
/// for_all(arbitrary::<u8>(), |n: u8| {
///     check!(u16::from(n) * 2).satisfies(lt(512u16))
/// })
/// .map_err(|f| f.failure)?;
/// # Ok(())
/// # }
/// ```
#[must_use]
pub fn arbitrary<T: Arbitrary>() -> ArbitraryStrategy<T> {
    ArbitraryStrategy {
        size: DEFAULT_GEN_SIZE,
        _marker: PhantomData,
    }
}

/// A [`Strategy`] that generates values of `T` through `quickcheck::Arbitrary`.
///
/// Created by [`arbitrary`]; rarely named directly.
#[derive(Debug, Clone, Copy)]
pub struct ArbitraryStrategy<T> {
    size: usize,
    // `fn() -> T` so the marker is `Send`/`Sync` regardless of `T` and does not
    // claim to own a `T`.
    _marker: PhantomData<fn() -> T>,
}

impl<T: Arbitrary> Strategy<T> for ArbitraryStrategy<T> {
    type Tree = QuickcheckTree<T>;

    fn new_tree(&self, _runner: &mut Runner) -> Result<Self::Tree, GenError> {
        // `_runner` is deliberately unused: `quickcheck::Gen` owns its RNG and
        // cannot be seeded from the seam's `Runner`. See the module docs.
        let mut generator = quickcheck::Gen::new(self.size);
        let value = T::arbitrary(&mut generator);
        Ok(QuickcheckTree::new(value))
    }
}

/// Adapts `quickcheck`'s linear `shrink` iterator to the seam's [`ValueTree`].
///
/// `quickcheck` has no `complicate` step: `Arbitrary::shrink` is a flat iterator
/// of smaller candidates. This tree maps that onto the `simplify`/`complicate`
/// protocol the runner drives. `accepted` is the simplest value seen to still
/// fail; `siblings` is its remaining shrink candidates; `candidate` is the one
/// handed to the runner but not yet judged.
///
/// - [`simplify`](ValueTree::simplify), called again after the runner adopted a
///   `candidate`, promotes that candidate to `accepted` and shrinks *it*; then
///   it hands out the next sibling. The runner calling `simplify` again is the
///   signal that the last `candidate` reproduced the failure.
/// - [`complicate`](ValueTree::complicate), called when a `candidate` did *not*
///   reproduce the failure, discards it and tries the next sibling instead.
///   This is how every candidate in `quickcheck`'s flat iterator gets a turn.
pub struct QuickcheckTree<T> {
    accepted: T,
    siblings: Box<dyn Iterator<Item = T>>,
    candidate: Option<T>,
}

impl<T: Arbitrary> QuickcheckTree<T> {
    fn new(value: T) -> Self {
        let siblings = value.shrink();
        Self {
            accepted: value,
            siblings,
            candidate: None,
        }
    }
}

impl<T: Arbitrary> ValueTree<T> for QuickcheckTree<T> {
    fn current(&self) -> T {
        self.candidate
            .clone()
            .unwrap_or_else(|| self.accepted.clone())
    }

    fn simplify(&mut self) -> bool {
        // A pending `candidate` here means the runner is calling `simplify`
        // again, which it only does after the candidate reproduced the failure:
        // adopt it as the new baseline and shrink from there.
        if let Some(accepted) = self.candidate.take() {
            self.siblings = accepted.shrink();
            self.accepted = accepted;
        }
        match self.siblings.next() {
            Some(next) => {
                self.candidate = Some(next);
                true
            }
            None => false,
        }
    }

    fn complicate(&mut self) -> bool {
        // The current `candidate` did not reproduce the failure: drop it and
        // try the next sibling shrink of the accepted baseline.
        match self.siblings.next() {
            Some(next) => {
                self.candidate = Some(next);
                true
            }
            None => {
                self.candidate = None;
                false
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use test_better_core::{OrFail, TestResult};
    use test_better_matchers::{check, eq, is_true, le};

    #[test]
    fn an_arbitrary_type_is_a_seam_strategy() -> TestResult {
        // `u32` is `quickcheck::Arbitrary`; drawing through the seam yields a
        // value with no wrapping at the call site.
        let mut runner = Runner::deterministic();
        let tree = arbitrary::<u32>().new_tree(&mut runner).or_fail()?;
        // Every `u32` is `<= u32::MAX`; the point is only that a value came out.
        check!(tree.current()).satisfies(le(u32::MAX))
    }

    #[test]
    fn simplify_walks_quickcheck_shrink_toward_zero() -> TestResult {
        // `quickcheck` shrinks integers toward zero. Starting from a known
        // value and simplifying as far as the tree will go must not grow it.
        let mut tree = QuickcheckTree::new(500u32);
        let start = tree.current();
        while tree.simplify() {}
        check!(tree.current() <= start).satisfies(is_true())
    }

    #[test]
    fn complicate_advances_to_the_next_sibling_candidate() -> TestResult {
        // From a small value, `simplify` hands out the first shrink candidate;
        // `complicate` (the candidate "passed") must move to a different one,
        // not stall on the first.
        let mut tree = QuickcheckTree::new(8u32);
        check!(tree.simplify()).satisfies(is_true())?;
        let first = tree.current();
        // `complicate` either advances to another sibling or reports the
        // iterator is exhausted; if it advanced, the value must have changed.
        if tree.complicate() {
            check!(tree.current() != first).satisfies(is_true())?;
        }
        // The accepted baseline is untouched by an unaccepted candidate.
        check!(tree.accepted).satisfies(eq(8u32))
    }
}
