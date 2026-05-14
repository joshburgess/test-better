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
//! - the [`expect!`](crate::expect) macro and its [`Subject`] type, the entry point for an
//!   assertion;
//! - the line-oriented [`diff_lines`] renderer, behind the default `diff`
//!   feature.
//!
//! Later iterations add the remaining matcher combinators
//! (PROJECT_BUILD_PLAN.md §8, Phase 3).

mod combinators;
mod description;
#[cfg(feature = "diff")]
mod diff;
mod fixtures;
mod matcher;
mod option_result;
mod primitives;
mod subject;

pub use combinators::{MatcherTuple, all_of, any_of, not};
pub use description::Description;
#[cfg(feature = "diff")]
pub use diff::diff_lines;
pub use fixtures::{always_matches, never_matches};
pub use matcher::{MatchResult, Matcher, Mismatch};
pub use option_result::{err, none, ok, some};
pub use primitives::{eq, ge, gt, is_false, is_true, le, lt, ne};
pub use subject::Subject;
// `expect!` is `#[macro_export]`, so it already lives at the crate root.
