//! `test-better-core`: error and result types.
//!
//! This crate is the foundation of the `test-better` testing library. It makes
//! `?` viable as the single control-flow operator of a test by providing:
//!
//! - [`TestError`], the structured failure type, and its parts ([`ErrorKind`],
//!   [`ContextFrame`], [`Payload`]);
//! - [`TestResult`], the `?`-friendly return type for tests and helpers;
//! - [`ContextExt`], which attaches "while doing X" context to a fallible value;
//! - [`OrFail`], the `?`-friendly alternative to panicking on the error path;
//! - [`Trace`], which records in-test breadcrumbs attached to any failure;
//! - [`StructuredError`], the owned/serializable mirror that tooling consumes;
//! - [`RUNNER_ENV`]/[`STRUCTURED_MARKER`], the structured-output channel the
//!   optional `cargo test-better` runner consumes.
//!
//! See PROJECT_BUILD_PLAN.md §6 (Phase 1) and §13 (Phase 8).

mod color;
mod context;
mod error;
mod or_fail;
mod render;
mod result;
mod runner;
mod structured;
mod trace;

pub use color::{ColorChoice, color_choice, set_color_choice};
pub use context::ContextExt;
pub use error::{ContextFrame, ErrorKind, Payload, TestError};
pub use or_fail::OrFail;
pub use result::TestResult;
pub use runner::{RUNNER_ENV, STRUCTURED_MARKER};
pub use structured::{SourceLocation, StructuredContextFrame, StructuredError, StructuredPayload};
pub use trace::{Trace, TraceEntry};
