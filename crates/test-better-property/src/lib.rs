//! `test-better-property`: property-testing bridge.
//!
//! A property test states a fact that should hold for *every* input ("parsing
//! then serializing round-trips"), and the runner tries to break it with
//! generated inputs, shrinking any counterexample to its simplest form.
//!
//! This crate is the bridge between `test-better`'s `expect!` idiom and a
//! property-testing backend. It has two pieces:
//!
//! - the [`Strategy`] seam (with [`ValueTree`], [`Runner`], [`GenError`]): a
//!   deliberately small trait the runner is written against. v1.0's backend is
//!   `proptest`, which satisfies it through a blanket impl, so a property test
//!   names ordinary `proptest` strategies (BACKLOG.md, Iteration 6.1a);
//! - the runner: [`check`] (and [`check_with`]) generate cases, run a
//!   `T -> TestResult` predicate, and on failure return a [`PropertyFailure`]
//!   carrying the shrunk counterexample.
//!
//! Behind the off-by-default `quickcheck` feature, [`arbitrary`] bridges a
//! `quickcheck::Arbitrary` type into the same seam: a best-effort second
//! backend that proves [`Strategy`] is a real seam (Iteration 6.1c). It is
//! honest about its reduced fidelity; see the [`quickcheck_bridge`] module docs.
//!
//! The [`property!`] macro is the test-facing front for all of this: it takes
//! a closure, infers a [`Strategy`] from the binding's type (or takes one with
//! a `using` clause), and runs it through [`check`]. Rich shrunk-failure
//! rendering (Iteration 6.3) builds on the same surface.

mod check;
mod property;
#[cfg(feature = "quickcheck")]
pub mod quickcheck_bridge;
mod strategy;

pub use check::{Config, PropertyFailure, check, check_with};
pub use property::run_property;
#[cfg(feature = "quickcheck")]
pub use quickcheck_bridge::{ArbitraryStrategy, QuickcheckTree, arbitrary};
pub use strategy::{GenError, ProptestTree, Runner, Strategy, ValueTree, any};
