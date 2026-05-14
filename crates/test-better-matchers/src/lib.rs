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
//! - the [`expect!`] macro and its [`Subject`] type, the entry point for an
//!   assertion.
//!
//! Later iterations add the matcher combinators (PROJECT_BUILD_PLAN.md
//! §7-§8, Phases 2-3).

mod description;
mod fixtures;
mod matcher;
mod primitives;
mod subject;

pub use description::Description;
pub use fixtures::{always_matches, never_matches};
pub use matcher::{MatchResult, Matcher, Mismatch};
pub use primitives::{eq, ge, gt, is_false, is_true, le, lt, ne};
pub use subject::Subject;
// `expect!` is `#[macro_export]`, so it already lives at the crate root.
