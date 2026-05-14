//! Custom matcher cookbook.
//!
//! The built-in matchers (`eq`, `contains`, `matches_struct!`, and the rest)
//! cover most assertions, but a test suite for a real domain eventually wants
//! its own vocabulary: `is_freezing()`, `is_a_valid_iban()`, `settled()`. A
//! custom matcher is reusable, composes with the combinators (`not`, `all_of`,
//! `some`, ...), and produces a failure message written in domain terms rather
//! than in terms of raw field values.
//!
//! This module is documentation only. It contains no code; the runnable
//! version of everything below lives in `examples/custom-matcher/`.
//!
//! # The two ways to write one
//!
//! 1. [`define_matcher!`](crate::define_matcher) is the declarative shortcut.
//!    Reach for it when the matcher is a predicate plus a description and
//!    nothing more. It is the right tool for the large majority of cases.
//! 2. A hand-written `impl Matcher<T>` gives full control: a structured diff,
//!    an inner matcher applied to a projection, a description assembled from
//!    parts. Reach for it when the shortcut is not enough.
//!
//! Before writing either, check whether a built-in already fits. To assert on
//! the *shape* of a struct, tuple, or enum variant, the structural macros
//! ([`matches_struct!`](crate::matches_struct),
//! [`matches_tuple!`](crate::matches_tuple),
//! [`matches_variant!`](crate::matches_variant)) compose existing matchers and
//! need no new type. To wrap an ad-hoc closure once, without naming it,
//! [`satisfies`](crate::satisfies) is lighter still.
//!
//! # The `Matcher` trait
//!
//! A matcher is any type that implements [`Matcher<T>`](crate::Matcher):
//!
//! ```ignore
//! pub trait Matcher<T: ?Sized> {
//!     fn check(&self, actual: &T) -> MatchResult;
//!     fn description(&self) -> Description;
//! }
//! ```
//!
//! - `check` inspects the borrowed value and returns a
//!   [`MatchResult`](crate::MatchResult): either
//!   [`MatchResult::pass()`](crate::MatchResult::pass) or
//!   [`MatchResult::fail(mismatch)`](crate::MatchResult::fail).
//! - `description` returns a [`Description`](crate::Description), the composable
//!   account of what the matcher expects. It is used both in the failure
//!   message and by combinators that wrap this matcher.
//!
//! A failure carries a [`Mismatch`](crate::Mismatch): the `expected`
//! description, the `actual` value rendered with `{:?}`, and an optional
//! pre-rendered `diff`. The usual way to build one is
//! [`Mismatch::new(expected, actual)`](crate::Mismatch::new), optionally
//! followed by [`.with_diff(...)`](crate::Mismatch::with_diff).
//!
//! # The declarative shortcut
//!
//! [`define_matcher!`](crate::define_matcher) writes the type, the trait impl,
//! and the constructor for you. The matcher inspects a value of a concrete
//! type and answers yes or no, with a fixed description:
//!
//! ```
//! use test_better::prelude::*;
//! use test_better::define_matcher;
//!
//! define_matcher! {
//!     /// Matches a temperature, in degrees Celsius, at or below freezing.
//!     pub fn is_freezing for f64 {
//!         expects: "a temperature at or below 0\u{b0}C",
//!         matches: |celsius| *celsius <= 0.0,
//!     }
//! }
//!
//! fn main() -> TestResult {
//!     expect!(-4.0_f64).to(is_freezing())?;
//!     expect!(20.0_f64).to_not(is_freezing())?;
//!     Ok(())
//! }
//! ```
//!
//! Constructor parameters are supported; each is in scope inside `expects` and
//! `matches` as a value of its declared type (so the parameter types must be
//! [`Clone`]):
//!
//! ```
//! use test_better::prelude::*;
//! use test_better::define_matcher;
//!
//! define_matcher! {
//!     /// Matches a vector whose length is a multiple of `n`.
//!     pub fn len_multiple_of(n: usize) for Vec<i32> {
//!         expects: format!("a vector whose length is a multiple of {n}"),
//!         matches: |v| v.len() % n == 0,
//!     }
//! }
//!
//! fn main() -> TestResult {
//!     expect!(vec![1, 2, 3, 4]).to(len_multiple_of(2))?;
//!     Ok(())
//! }
//! ```
//!
//! The target type must implement [`Debug`](std::fmt::Debug), since a failure
//! reports the actual value through `{:?}`.
//!
//! # The hand-written form
//!
//! When the matcher needs more than a predicate, implement the trait directly.
//! The pattern is a (usually small) struct, an `impl Matcher<T>` for it, and a
//! lower-case constructor function returning `impl Matcher<T>` so callers never
//! name the struct:
//!
//! ```
//! use test_better::prelude::*;
//! use test_better::{Description, MatchResult, Matcher, Mismatch};
//!
//! /// A temperature reading, in degrees Celsius.
//! #[derive(Debug)]
//! struct Temperature(f64);
//!
//! struct IsFreezing;
//!
//! impl Matcher<Temperature> for IsFreezing {
//!     fn check(&self, actual: &Temperature) -> MatchResult {
//!         if actual.0 <= 0.0 {
//!             MatchResult::pass()
//!         } else {
//!             MatchResult::fail(Mismatch::new(
//!                 self.description(),
//!                 format!("{:.1}\u{b0}C, which is above freezing", actual.0),
//!             ))
//!         }
//!     }
//!
//!     fn description(&self) -> Description {
//!         Description::text("a temperature at or below 0\u{b0}C")
//!     }
//! }
//!
//! /// Matches a [`Temperature`] at or below freezing.
//! fn is_freezing() -> impl Matcher<Temperature> {
//!     IsFreezing
//! }
//!
//! fn main() -> TestResult {
//!     expect!(Temperature(-4.0)).to(is_freezing())?;
//!     Ok(())
//! }
//! ```
//!
//! Two things the hand-written form unlocks that the shortcut does not:
//!
//! - **A description built from parts.** `Description` composes with `and`,
//!   `or`, `labeled`, and `!`; a matcher that takes an inner matcher can fold
//!   the inner matcher's own `description()` into its own, so a nested failure
//!   reads as nested.
//! - **A control over the `actual` rendering and the diff.** `check` decides
//!   exactly what string the failure shows, and may attach a diff with
//!   [`Mismatch::with_diff`](crate::Mismatch::with_diff).
//!
//! # Matchers that take an inner matcher
//!
//! A matcher generic over `M: Matcher<Inner>` applies `M` to some projection of
//! the actual value. This is how `some`, `ok`, and the structural macros are
//! built. The key move is to wrap the inner matcher's failure in a
//! [`labeled`](crate::Description::labeled) description so the output keeps the
//! layer that failed:
//!
//! ```
//! use test_better::prelude::*;
//! use test_better::{Description, MatchResult, Matcher, Mismatch};
//!
//! #[derive(Debug)]
//! struct Celsius(f64);
//!
//! struct AsFloat<M>(M);
//!
//! impl<M: Matcher<f64>> Matcher<Celsius> for AsFloat<M> {
//!     fn check(&self, actual: &Celsius) -> MatchResult {
//!         let inner = self.0.check(&actual.0);
//!         match inner.failure {
//!             None => MatchResult::pass(),
//!             Some(mismatch) => MatchResult::fail(Mismatch {
//!                 expected: Description::labeled("degrees", mismatch.expected),
//!                 ..mismatch
//!             }),
//!         }
//!     }
//!
//!     fn description(&self) -> Description {
//!         Description::labeled("degrees", self.0.description())
//!     }
//! }
//!
//! /// Applies `inner` to a [`Celsius`] reading's underlying value.
//! fn as_float<M: Matcher<f64>>(inner: M) -> impl Matcher<Celsius> {
//!     AsFloat(inner)
//! }
//!
//! fn main() -> TestResult {
//!     expect!(Celsius(3.5)).to(as_float(gt(0.0)))?;
//!     Ok(())
//! }
//! ```
//!
//! # Where to go next
//!
//! `examples/custom-matcher/` is a small crate that builds and tests every
//! pattern above. It is the place to copy from when starting a real custom
//! matcher.
