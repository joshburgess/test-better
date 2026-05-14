//! `test-better-matchers`: the `Matcher` trait and standard matchers.
//!
//! A matcher is a reusable, composable expectation. This crate provides:
//!
//! - [`Matcher`], the trait every matcher implements, and its structured
//!   result types [`MatchResult`] and [`Mismatch`];
//! - [`Description`], the composable account of what a matcher expects;
//! - the primitive matchers [`eq`], [`ne`], [`lt`], [`le`], [`gt`], [`ge`],
//!   [`is_true`], [`is_false`], and the test fixtures [`always_matches`] and
//!   [`never_matches`];
//! - the logical combinators [`not`], [`all_of`], and [`any_of`];
//! - the [`Option`]/[`Result`] matchers [`some`], [`none`], [`ok`], and
//!   [`err`];
//! - the collection matchers [`have_len`], [`is_empty`], [`is_not_empty`],
//!   [`contains`], [`contains_all`], [`contains_in_order`], [`every`], and
//!   [`at_least_one`], generic over the [`Sequence`] trait;
//! - the string matchers [`contains_str`], [`starts_with`], [`ends_with`], and
//!   (behind the `regex` feature) `matches_regex`;
//! - the numeric matchers [`close_to`], [`between`], [`is_nan`], and
//!   [`is_finite`], generic over the sealed [`Float`] trait;
//! - the [`satisfies`] escape hatch, a matcher built from an arbitrary
//!   named predicate;
//! - the [`define_matcher!`](crate::define_matcher) macro, the declarative
//!   shortcut for the common custom-matcher case;
//! - the [`expect!`](crate::expect) macro and its [`Subject`] type, the entry point for an
//!   assertion; when the subject is a [`Future`], the `resolves_to` method
//!   awaits it and matches its output, and `to_complete_within` awaits it
//!   under a time limit (the latter behind a runtime feature: `tokio`,
//!   `async-std`, or `smol`); and `to_match_snapshot` compares a
//!   [`Display`](std::fmt::Display) value against a file-backed snapshot
//!   (`test-better-snapshot`);
//! - [`eventually`] and [`eventually_blocking`], which retry a `bool` probe on
//!   an exponential [`Backoff`] schedule until it passes or a deadline is hit,
//!   replacing `sleep + assert` flakiness;
//! - [`soft`] and its [`SoftAsserter`]/[`SoftScope`], which collect several
//!   failures in one test run instead of stopping at the first, with nestable
//!   context sub-scopes;
//! - the line-oriented [`diff_lines`] renderer, behind the default `diff`
//!   feature.
//!
//! Later iterations add the remaining matcher combinators
//! (PROJECT_BUILD_PLAN.md §8, Phase 3).

mod collections;
mod combinators;
mod define;
mod description;
#[cfg(feature = "diff")]
mod diff;
mod fixtures;
mod matcher;
mod numeric;
mod option_result;
mod primitives;
mod satisfies;
mod soft;
mod strings;
mod subject;

pub use collections::{
    ContainsAll, Sequence, at_least_one, contains, contains_all, contains_in_order, every,
    have_len, is_empty, is_not_empty,
};
pub use combinators::{MatcherTuple, all_of, any_of, not};
pub use description::Description;
#[cfg(feature = "diff")]
pub use diff::diff_lines;
pub use fixtures::{always_matches, never_matches};
pub use matcher::{MatchResult, Matcher, Mismatch};
pub use numeric::{Float, between, close_to, is_finite, is_nan};
pub use option_result::{err, none, ok, some};
pub use primitives::{eq, ge, gt, is_false, is_true, le, lt, ne};
pub use satisfies::satisfies;
pub use soft::{SoftAsserter, SoftScope, soft};
#[cfg(feature = "regex")]
pub use strings::matches_regex;
pub use strings::{contains_str, ends_with, starts_with};
pub use subject::Subject;
// Re-exported from `test-better-async`: `Elapsed` and `RuntimeAvailable` appear
// in `Subject::to_complete_within`'s signature, and the `eventually` family is
// the polling counterpart to the timeout assertion.
pub use test_better_async::{
    Backoff, Elapsed, RuntimeAvailable, eventually, eventually_blocking, eventually_blocking_with,
    eventually_with,
};
// `expect!` and `define_matcher!` are `#[macro_export]`, so they already live
// at the crate root.
