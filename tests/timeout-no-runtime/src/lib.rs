//! Compile-fail coverage for `to_complete_within` without a runtime feature
//! (PROJECT_BUILD_PLAN.md Iteration 5.2).
//!
//! The crate intentionally has no library code. Its only content is the
//! `trybuild` test in `tests/ui.rs`, which confirms that calling
//! `to_complete_within` with no `tokio`/`async-std`/`smol` feature enabled is
//! a compile error whose diagnostic names those flags.
