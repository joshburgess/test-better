//! Compile-fail coverage for `completes_within` without a runtime feature.
//!
//! The crate intentionally has no library code. Its only content is the
//! `trybuild` test in `tests/ui.rs`, which confirms that calling
//! `completes_within` with no `tokio`/`async-std`/`smol` feature enabled is
//! a compile error whose diagnostic names those flags.
