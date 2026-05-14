//! `test-better`: facade crate.
//!
//! This is the crate users depend on. It re-exports the public surface of the
//! `test-better` testing library so a test file needs a single dependency and,
//! ideally, a single `use`:
//!
//! ```
//! use test_better::prelude::*;
//! ```
//!
//! The workspace is split into focused crates (`test-better-core`,
//! `test-better-matchers`, and so on, per PROJECT_BUILD_PLAN.md §2); this crate
//! is the seam that hides that split from users.

pub use test_better_core::{
    ContextExt, ContextFrame, ErrorKind, OrFail, Payload, SourceLocation, StructuredContextFrame,
    StructuredError, StructuredPayload, TestError, TestResult,
};

/// The one `use` a test file should need: `use test_better::prelude::*;`.
///
/// The prelude is deliberately small. It brings in the result type, the error
/// type, and the extension traits whose methods (`context`, `or_fail`, ...) are
/// meant to be called without qualification. The structured-failure types stay
/// out of it: they are for tooling, not for the body of a test.
///
/// # Re-exporting macros
///
/// `#[macro_export]` places a macro at the crate root, not inside the module it
/// is written in, so a glob import of this module would *not* pick it up unless
/// the macro is named here explicitly. Phase 2's `expect!` and friends will be
/// listed below with `pub use crate::expect;`; the pattern is established now so
/// that later phases only have to add a line.
pub mod prelude {
    pub use test_better_core::{ContextExt, OrFail, TestError, TestResult};
    // Phase 2+: `pub use crate::expect;` and other `#[macro_export]` macros go
    // here so `use test_better::prelude::*;` brings them into scope.
}
