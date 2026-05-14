//! Collection matchers and the [`Sequence`] trait they are generic over.
//!
//! [`Sequence`] is the crate's abstraction over an ordered run of items: it is
//! implemented for slices, arrays, `Vec`, `VecDeque`, `BTreeSet`, `HashSet`,
//! and `&S` for any `Sequence` `S`. The matchers in this module
//! ([`have_len`], [`is_empty`], [`is_not_empty`], [`contains`],
//! [`contains_all`], [`contains_in_order`], [`every`], [`at_least_one`]) work
//! for every one of those (PROJECT_BUILD_PLAN.md §8, Iteration 3.3).
//!
//! Failures name the index of the first item (or, for sets, the offending
//! value) that broke the expectation.

use std::collections::{BTreeSet, HashSet, VecDeque};
use std::fmt;

use crate::description::Description;
use crate::matcher::{MatchResult, Matcher, Mismatch};

/// An ordered run of items a collection matcher can inspect.
///
/// Implemented for `[T]`, `[T; N]`, `Vec<T>`, `VecDeque<T>`, `BTreeSet<T>`,
/// `HashSet<T>`, and `&S` for any `Sequence` `S`. Items are borrowed, not
/// cloned. A lazy iterator is not a `Sequence` (it cannot be inspected through
/// a shared borrow); collect it into a `Vec` first.
pub trait Sequence {
    /// The element type.
    type Item;

    /// Borrows every item, in order.
    fn sequence_items(&self) -> Vec<&Self::Item>;
}

impl<T> Sequence for [T] {
    type Item = T;

    fn sequence_items(&self) -> Vec<&T> {
        self.iter().collect()
    }
}

impl<T, const N: usize> Sequence for [T; N] {
    type Item = T;

    fn sequence_items(&self) -> Vec<&T> {
        self.iter().collect()
    }
}

impl<T> Sequence for Vec<T> {
    type Item = T;

    fn sequence_items(&self) -> Vec<&T> {
        self.iter().collect()
    }
}

impl<T> Sequence for VecDeque<T> {
    type Item = T;

    fn sequence_items(&self) -> Vec<&T> {
        self.iter().collect()
    }
}

impl<T> Sequence for BTreeSet<T> {
    type Item = T;

    fn sequence_items(&self) -> Vec<&T> {
        self.iter().collect()
    }
}

impl<T> Sequence for HashSet<T> {
    type Item = T;

    fn sequence_items(&self) -> Vec<&T> {
        self.iter().collect()
    }
}

impl<S: Sequence + ?Sized> Sequence for &S {
    type Item = S::Item;

    fn sequence_items(&self) -> Vec<&S::Item> {
        (**self).sequence_items()
    }
}

/// The matcher behind [`have_len`].
struct LenMatcher {
    expected: usize,
}

impl<C> Matcher<C> for LenMatcher
where
    C: Sequence + ?Sized,
{
    fn check(&self, actual: &C) -> MatchResult {
        let len = actual.sequence_items().len();
        if len == self.expected {
            MatchResult::pass()
        } else {
            MatchResult::fail(Mismatch::new(
                Description::text(format!("a sequence of length {}", self.expected)),
                format!("a sequence of length {len}"),
            ))
        }
    }

    fn description(&self) -> Description {
        Description::text(format!("a sequence of length {}", self.expected))
    }
}

/// Matches a sequence with exactly `n` items.
///
/// ```
/// use test_better_core::TestResult;
/// use test_better_matchers::{expect, have_len};
///
/// fn main() -> TestResult {
///     expect!(vec![1, 2, 3]).to(have_len(3))?;
///     Ok(())
/// }
/// ```
#[must_use]
pub fn have_len<C>(n: usize) -> impl Matcher<C>
where
    C: Sequence + ?Sized,
{
    LenMatcher { expected: n }
}

/// The matcher behind [`is_empty`] and [`is_not_empty`].
struct EmptyMatcher {
    want_empty: bool,
}

impl<C> Matcher<C> for EmptyMatcher
where
    C: Sequence + ?Sized,
{
    fn check(&self, actual: &C) -> MatchResult {
        let len = actual.sequence_items().len();
        if (len == 0) == self.want_empty {
            MatchResult::pass()
        } else if self.want_empty {
            MatchResult::fail(Mismatch::new(
                self.description_text(),
                format!("a sequence of length {len}"),
            ))
        } else {
            MatchResult::fail(Mismatch::new(self.description_text(), "an empty sequence"))
        }
    }

    fn description(&self) -> Description {
        self.description_text()
    }
}

impl EmptyMatcher {
    fn description_text(&self) -> Description {
        Description::text(if self.want_empty {
            "an empty sequence"
        } else {
            "a non-empty sequence"
        })
    }
}

/// Matches a sequence with no items.
///
/// ```
/// use test_better_core::TestResult;
/// use test_better_matchers::{expect, is_empty};
///
/// fn main() -> TestResult {
///     expect!(Vec::<i32>::new()).to(is_empty())?;
///     Ok(())
/// }
/// ```
#[must_use]
pub fn is_empty<C>() -> impl Matcher<C>
where
    C: Sequence + ?Sized,
{
    EmptyMatcher { want_empty: true }
}

/// Matches a sequence with at least one item.
///
/// ```
/// use test_better_core::TestResult;
/// use test_better_matchers::{expect, is_not_empty};
///
/// fn main() -> TestResult {
///     expect!(vec![1]).to(is_not_empty())?;
///     Ok(())
/// }
/// ```
#[must_use]
pub fn is_not_empty<C>() -> impl Matcher<C>
where
    C: Sequence + ?Sized,
{
    EmptyMatcher { want_empty: false }
}

/// The matcher behind [`contains`] and [`at_least_one`]: at least one item
/// satisfies the inner matcher.
struct AnyItemMatcher<M> {
    inner: M,
    /// The phrase that heads the expected description (`contains` and
    /// `at_least_one` read differently even though they check the same thing).
    header: &'static str,
}

impl<C, M> Matcher<C> for AnyItemMatcher<M>
where
    C: Sequence + ?Sized,
    C::Item: fmt::Debug,
    M: Matcher<C::Item>,
{
    fn check(&self, actual: &C) -> MatchResult {
        let items = actual.sequence_items();
        if items.iter().any(|item| self.inner.check(item).matched) {
            MatchResult::pass()
        } else {
            MatchResult::fail(Mismatch::new(
                Description::labeled(self.header, self.inner.description()),
                format!("{items:?}"),
            ))
        }
    }

    fn description(&self) -> Description {
        Description::labeled(self.header, self.inner.description())
    }
}

/// Matches a sequence that contains at least one item satisfying `matcher`.
///
/// ```
/// use test_better_core::TestResult;
/// use test_better_matchers::{contains, eq, expect};
///
/// fn main() -> TestResult {
///     expect!(vec![1, 2, 3]).to(contains(eq(2)))?;
///     Ok(())
/// }
/// ```
#[must_use]
pub fn contains<C, M>(matcher: M) -> impl Matcher<C>
where
    C: Sequence + ?Sized,
    C::Item: fmt::Debug,
    M: Matcher<C::Item>,
{
    AnyItemMatcher {
        inner: matcher,
        header: "a sequence containing an item that is",
    }
}

/// Matches a sequence in which at least one item satisfies `matcher`.
///
/// The check is the same as [`contains`]; the two exist because they read
/// differently at the call site.
///
/// ```
/// use test_better_core::TestResult;
/// use test_better_matchers::{at_least_one, expect, gt};
///
/// fn main() -> TestResult {
///     expect!(vec![1, 2, 3]).to(at_least_one(gt(2)))?;
///     Ok(())
/// }
/// ```
#[must_use]
pub fn at_least_one<C, M>(matcher: M) -> impl Matcher<C>
where
    C: Sequence + ?Sized,
    C::Item: fmt::Debug,
    M: Matcher<C::Item>,
{
    AnyItemMatcher {
        inner: matcher,
        header: "at least one item to satisfy",
    }
}

/// The matcher behind [`every`]: every item satisfies the inner matcher.
struct EveryMatcher<M> {
    inner: M,
}

impl<C, M> Matcher<C> for EveryMatcher<M>
where
    C: Sequence + ?Sized,
    C::Item: fmt::Debug,
    M: Matcher<C::Item>,
{
    fn check(&self, actual: &C) -> MatchResult {
        let items = actual.sequence_items();
        for (index, item) in items.iter().enumerate() {
            if let Some(failure) = self.inner.check(item).failure {
                return MatchResult::fail(Mismatch::new(
                    // `EveryMatcher<M>` implements `Matcher<C>` for a family of
                    // `C`, so `description` is spelled out to stay unambiguous.
                    Matcher::<C>::description(self),
                    format!("item at index {index} was {}", failure.actual),
                ));
            }
        }
        MatchResult::pass()
    }

    fn description(&self) -> Description {
        Description::labeled("every item to satisfy", self.inner.description())
    }
}

/// Matches a sequence in which *every* item satisfies `matcher`.
///
/// On failure the error names the index of the first item that did not match.
///
/// ```
/// use test_better_core::TestResult;
/// use test_better_matchers::{every, expect, gt};
///
/// fn main() -> TestResult {
///     expect!(vec![1, 2, 3]).to(every(gt(0)))?;
///     Ok(())
/// }
/// ```
#[must_use]
pub fn every<C, M>(matcher: M) -> impl Matcher<C>
where
    C: Sequence + ?Sized,
    C::Item: fmt::Debug,
    M: Matcher<C::Item>,
{
    EveryMatcher { inner: matcher }
}

/// The matcher behind [`contains_in_order`].
struct InOrderMatcher<M, const N: usize> {
    matchers: [M; N],
}

impl<C, M, const N: usize> Matcher<C> for InOrderMatcher<M, N>
where
    C: Sequence + ?Sized,
    C::Item: fmt::Debug,
    M: Matcher<C::Item>,
{
    fn check(&self, actual: &C) -> MatchResult {
        let items = actual.sequence_items();
        let mut next = 0;
        for item in &items {
            if next < N && self.matchers[next].check(item).matched {
                next += 1;
            }
        }
        if next == N {
            MatchResult::pass()
        } else {
            MatchResult::fail(Mismatch::new(
                Matcher::<C>::description(self),
                format!(
                    "a sequence matching {next} of {N} in order \
                     (no later item satisfied matcher at index {next}): {items:?}"
                ),
            ))
        }
    }

    fn description(&self) -> Description {
        let joined = self
            .matchers
            .iter()
            .map(|m| m.description().to_string())
            .collect::<Vec<_>>()
            .join(", then ");
        Description::text(format!("a sequence containing, in order: {joined}"))
    }
}

/// Matches a sequence that contains items satisfying `matchers` in order, not
/// necessarily contiguously.
///
/// On failure the error names the index of the first matcher that no remaining
/// item could satisfy.
///
/// ```
/// use test_better_core::TestResult;
/// use test_better_matchers::{contains_in_order, eq, expect};
///
/// fn main() -> TestResult {
///     expect!(vec![1, 2, 3, 4]).to(contains_in_order([eq(2), eq(4)]))?;
///     Ok(())
/// }
/// ```
#[must_use]
pub fn contains_in_order<C, M, const N: usize>(matchers: [M; N]) -> impl Matcher<C>
where
    C: Sequence + ?Sized,
    C::Item: fmt::Debug,
    M: Matcher<C::Item>,
{
    InOrderMatcher { matchers }
}

/// A tuple of matchers, all over the same `Item`, for [`contains_all`].
///
/// Implemented for tuples of arity 2 through 8 by a macro in this module; you
/// do not implement it yourself.
pub trait ContainsAll<Item> {
    /// The description of the first matcher that no item in `items` satisfies,
    /// or `None` if every matcher is satisfied.
    fn first_unsatisfied(&self, items: &[&Item]) -> Option<Description>;

    /// The conjunction (`a and b and ...`) of the tuple's descriptions.
    fn describe(&self) -> Description;
}

/// Implements [`ContainsAll`] for one tuple arity. The first type parameter is
/// split out so the description fold has a guaranteed first element.
macro_rules! impl_contains_all {
    ($first:ident, $($rest:ident),+) => {
        #[allow(non_snake_case)]
        impl<Item, $first, $($rest,)+> ContainsAll<Item> for ($first, $($rest,)+)
        where
            $first: Matcher<Item>,
            $($rest: Matcher<Item>,)+
        {
            fn first_unsatisfied(&self, items: &[&Item]) -> Option<Description> {
                let ($first, $($rest,)+) = self;
                if !items.iter().any(|item| $first.check(item).matched) {
                    return Some($first.description());
                }
                $(
                    if !items.iter().any(|item| $rest.check(item).matched) {
                        return Some($rest.description());
                    }
                )+
                None
            }

            fn describe(&self) -> Description {
                let ($first, $($rest,)+) = self;
                let desc = $first.description();
                $( let desc = desc.and($rest.description()); )+
                desc
            }
        }
    };
}

impl_contains_all!(M1, M2);
impl_contains_all!(M1, M2, M3);
impl_contains_all!(M1, M2, M3, M4);
impl_contains_all!(M1, M2, M3, M4, M5);
impl_contains_all!(M1, M2, M3, M4, M5, M6);
impl_contains_all!(M1, M2, M3, M4, M5, M6, M7);
impl_contains_all!(M1, M2, M3, M4, M5, M6, M7, M8);

/// The matcher behind [`contains_all`].
struct ContainsAllMatcher<Tup> {
    matchers: Tup,
}

impl<C, Tup> Matcher<C> for ContainsAllMatcher<Tup>
where
    C: Sequence + ?Sized,
    C::Item: fmt::Debug,
    Tup: ContainsAll<C::Item>,
{
    fn check(&self, actual: &C) -> MatchResult {
        let items = actual.sequence_items();
        match self.matchers.first_unsatisfied(&items) {
            None => MatchResult::pass(),
            Some(unsatisfied) => MatchResult::fail(Mismatch::new(
                Description::labeled("a sequence containing an item that is", unsatisfied),
                format!("{items:?}"),
            )),
        }
    }

    fn description(&self) -> Description {
        Description::labeled("a sequence containing all of", self.matchers.describe())
    }
}

/// Matches a sequence in which every matcher in the tuple is satisfied by some
/// item (each matcher independently; one item may satisfy several).
///
/// On failure the error names the first matcher that no item satisfied.
///
/// ```
/// use test_better_core::TestResult;
/// use test_better_matchers::{contains_all, eq, expect, gt};
///
/// fn main() -> TestResult {
///     expect!(vec![1, 2, 3]).to(contains_all((eq(1), gt(2))))?;
///     Ok(())
/// }
/// ```
#[must_use]
pub fn contains_all<C, Tup>(matchers: Tup) -> impl Matcher<C>
where
    C: Sequence + ?Sized,
    C::Item: fmt::Debug,
    Tup: ContainsAll<C::Item>,
{
    ContainsAllMatcher { matchers }
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeSet, HashSet, VecDeque};

    use test_better_core::{OrFail, TestResult};

    use super::*;
    use crate::{eq, expect, gt, is_false, is_true, lt};

    #[test]
    fn have_len_matches_the_exact_length() -> TestResult {
        expect!(have_len(3).check(&vec![1, 2, 3]).matched).to(is_true())?;
        let failure = have_len(3)
            .check(&vec![1, 2])
            .failure
            .or_fail_with("length 2 is not 3")?;
        expect!(failure.expected.to_string()).to(eq("a sequence of length 3".to_string()))?;
        expect!(failure.actual).to(eq("a sequence of length 2".to_string()))?;
        Ok(())
    }

    #[test]
    fn is_empty_and_is_not_empty_are_opposites() -> TestResult {
        expect!(is_empty().check(&Vec::<i32>::new()).matched).to(is_true())?;
        expect!(is_empty().check(&vec![1]).matched).to(is_false())?;
        expect!(is_not_empty().check(&vec![1]).matched).to(is_true())?;
        expect!(is_not_empty().check(&Vec::<i32>::new()).matched).to(is_false())?;
        Ok(())
    }

    #[test]
    fn contains_finds_a_matching_item() -> TestResult {
        expect!(contains(eq(2)).check(&vec![1, 2, 3]).matched).to(is_true())?;
        let failure = contains(eq(9))
            .check(&vec![1, 2, 3])
            .failure
            .or_fail_with("9 is not in the sequence")?;
        expect!(failure.actual).to(eq("[1, 2, 3]".to_string()))?;
        Ok(())
    }

    #[test]
    fn every_names_the_index_of_the_first_failure() -> TestResult {
        expect!(every(gt(0)).check(&vec![1, 2, 3]).matched).to(is_true())?;
        let failure = every(gt(0))
            .check(&vec![1, 2, -1, 4])
            .failure
            .or_fail_with("-1 is not greater than 0")?;
        expect!(failure.actual.contains("index 2")).to(is_true())?;
        Ok(())
    }

    #[test]
    fn at_least_one_matches_when_some_item_does() -> TestResult {
        expect!(at_least_one(gt(2)).check(&vec![1, 2, 3]).matched).to(is_true())?;
        expect!(at_least_one(gt(9)).check(&vec![1, 2, 3]).matched).to(is_false())?;
        Ok(())
    }

    #[test]
    fn contains_in_order_respects_order_but_not_adjacency() -> TestResult {
        expect!(
            contains_in_order([eq(2), eq(4)])
                .check(&vec![1, 2, 3, 4])
                .matched
        )
        .to(is_true())?;
        let failure = contains_in_order([eq(4), eq(2)])
            .check(&vec![1, 2, 3, 4])
            .failure
            .or_fail_with("2 does not come after 4")?;
        expect!(failure.actual.contains("matcher at index 1")).to(is_true())?;
        Ok(())
    }

    #[test]
    fn contains_all_requires_every_matcher_to_be_satisfied() -> TestResult {
        expect!(contains_all((eq(1), gt(2))).check(&vec![1, 2, 3]).matched).to(is_true())?;
        let failure = contains_all((eq(1), gt(9)))
            .check(&vec![1, 2, 3])
            .failure
            .or_fail_with("nothing is greater than 9")?;
        expect!(failure.expected.to_string().contains("greater than 9")).to(is_true())?;
        Ok(())
    }

    #[test]
    fn collection_matchers_work_across_collection_types() -> TestResult {
        let deque: VecDeque<i32> = VecDeque::from(vec![1, 2, 3]);
        expect!(have_len(3).check(&deque).matched).to(is_true())?;

        let btree: BTreeSet<i32> = BTreeSet::from([1, 2, 3]);
        expect!(contains(eq(2)).check(&btree).matched).to(is_true())?;

        let set: HashSet<i32> = HashSet::from([1, 2, 3]);
        expect!(every(gt(0)).check(&set).matched).to(is_true())?;

        let slice: &[i32] = &[10, 20, 30];
        expect!(contains_in_order([eq(10), eq(30)]).check(&slice).matched).to(is_true())?;

        let array = [1, 2, 3];
        expect!(every(lt(4)).check(&array).matched).to(is_true())?;
        Ok(())
    }
}
