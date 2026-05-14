//! `test-better-matchers`: the `Matcher` trait and standard matchers.
//!
//! A matcher is a reusable, composable expectation. This crate provides:
//!
//! - [`Matcher`], the trait every matcher implements, and its structured
//!   result types [`MatchResult`] and [`Mismatch`];
//! - [`Description`], the composable account of what a matcher expects.
//!
//! Later iterations add the standard matcher library and the `expect!` macro
//! (PROJECT_BUILD_PLAN.md §7-§8, Phases 2-3).

mod description;
mod matcher;

pub use description::Description;
pub use matcher::{MatchResult, Matcher, Mismatch};
