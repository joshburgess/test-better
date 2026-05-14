//! `test-better-core`: error and result types.
//!
//! This crate is the foundation of the `test-better` testing library. It makes
//! `?` viable as the single control-flow operator of a test by providing:
//!
//! - [`TestError`], the structured failure type, and its parts ([`ErrorKind`],
//!   [`ContextFrame`], [`Payload`]);
//! - [`TestResult`], the `?`-friendly return type for tests and helpers;
//! - [`ContextExt`], which attaches "while doing X" context to a fallible value;
//! - [`StructuredError`], the owned/serializable mirror that tooling consumes;
//!
//! Later iterations add `OrFail` to this crate.
//!
//! See PROJECT_BUILD_PLAN.md §6 (Phase 1).

mod context;
mod error;
mod render;
mod result;
mod structured;

pub use context::ContextExt;
pub use error::{ContextFrame, ErrorKind, Payload, TestError};
pub use result::TestResult;
pub use structured::{SourceLocation, StructuredContextFrame, StructuredError, StructuredPayload};
