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
//! The `property!` macro (Iteration 6.2) and rich shrunk-failure rendering
//! (Iteration 6.3) build on this surface.

mod check;
mod strategy;

pub use check::{Config, PropertyFailure, check, check_with};
pub use strategy::{GenError, ProptestTree, Runner, Strategy, ValueTree};
