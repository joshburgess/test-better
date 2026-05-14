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
pub use test_better_matchers::{
    Description, MatchResult, Matcher, Mismatch, Subject, always_matches, eq, expect, ge, gt,
    is_false, is_true, le, lt, ne, never_matches,
};

/// The one `use` a test file should need: `use test_better::prelude::*;`.
///
/// The prelude is deliberately small. It brings in the result type, the error
/// type, the extension traits whose methods (`context`, `or_fail`, ...) are
/// meant to be called without qualification, the `expect!` macro, and the
/// matcher constructors (`eq`, `lt`, ...). The structured-failure types and the
/// custom-matcher machinery (`Matcher`, `Description`, ...) stay out of it:
/// they are imported by name when needed, not in the body of every test.
///
/// # Re-exporting macros
///
/// `#[macro_export]` places a macro at the crate root, not inside the module it
/// is written in, so a glob import of this module would *not* pick it up unless
/// the macro is named here explicitly. That is why `expect` appears below with
/// `pub use crate::expect;`; later phases add their `#[macro_export]` macros the
/// same way.
pub mod prelude {
    pub use test_better_core::{ContextExt, OrFail, TestError, TestResult};
    pub use test_better_matchers::{
        always_matches, eq, ge, gt, is_false, is_true, le, lt, ne, never_matches,
    };

    pub use crate::expect;
}
