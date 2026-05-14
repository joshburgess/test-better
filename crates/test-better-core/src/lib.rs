//! `test-better-core`: error and result types.
//!
//! This crate is the foundation of the `test-better` testing library. It makes
//! `?` viable as the single control-flow operator of a test by providing:
//!
//! - [`TestError`], the structured failure type, and its parts ([`ErrorKind`],
//!   [`ContextFrame`], [`Payload`]);
//! - [`StructuredError`], the owned/serializable mirror that tooling consumes;
//!
//! Later iterations add `TestResult`, `ContextExt`, and `OrFail` to this crate.
//!
//! See PROJECT_BUILD_PLAN.md §6 (Phase 1).

mod error;
mod render;
mod structured;

pub use error::{ContextFrame, ErrorKind, Payload, TestError};
pub use structured::{SourceLocation, StructuredContextFrame, StructuredError, StructuredPayload};
