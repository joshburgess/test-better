//! `test-better-runner`: optional pretty runner.
//!
//! Library half of the `cargo-test-better` subcommand: wraps `cargo test`,
//! consumes the structured `TestError` serialization, and groups failures by
//! context chain (PROJECT_BUILD_PLAN.md §14, Phase 9).
//!
//! Phase 0 scaffolding: intentionally empty.
