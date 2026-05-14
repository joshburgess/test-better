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
    ColorChoice, ContextExt, ContextFrame, ErrorKind, OrFail, Payload, SourceLocation,
    StructuredContextFrame, StructuredError, StructuredPayload, TestError, TestResult,
    color_choice, set_color_choice,
};
pub use test_better_macros::{matches_struct, matches_tuple, matches_variant};
// The property-testing bridge (Phase 6). `Config` is renamed `PropertyConfig`
// here: at the facade root, where one crate's surface meets eight others, a
// bare `Config` says too little.
#[cfg(feature = "diff")]
pub use test_better_matchers::diff_lines;
#[cfg(feature = "regex")]
pub use test_better_matchers::matches_regex;
pub use test_better_matchers::{
    Backoff, ContainsAll, Description, Elapsed, Float, MatchResult, Matcher, MatcherTuple,
    Mismatch, RuntimeAvailable, Sequence, SoftAsserter, SoftScope, Subject, all_of, always_matches,
    any_of, at_least_one, between, close_to, contains, contains_all, contains_in_order,
    contains_str, define_matcher, ends_with, eq, err, eventually, eventually_blocking,
    eventually_blocking_with, eventually_with, every, expect, ge, gt, have_len, is_empty, is_false,
    is_finite, is_nan, is_not_empty, is_true, le, lt, ne, never_matches, none, not, ok, satisfies,
    soft, some, starts_with,
};
pub use test_better_property::{
    Config as PropertyConfig, GenError, PropertyFailure, ProptestTree, Runner, Strategy, ValueTree,
    any, check, check_with, property,
};
// The best-effort `quickcheck` bridge, behind the facade's `quickcheck`
// feature: `arbitrary::<T>()` turns a `quickcheck::Arbitrary` type into a
// `Strategy<T>`. Off by default; `proptest` is the primary backend.
#[cfg(feature = "quickcheck")]
pub use test_better_property::{ArbitraryStrategy, QuickcheckTree, arbitrary};

/// How to write a custom matcher: see the [`cookbook`] module.
pub mod cookbook;

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
/// the macro is named here explicitly. That is why `expect` and
/// `define_matcher` appear below with `pub use crate::...;`; later phases add
/// their `#[macro_export]` macros the same way.
///
/// Procedural macros (`matches_struct!`, `matches_tuple!`, `matches_variant!`)
/// are different: they are ordinary items of `test-better-macros`, so a plain
/// `pub use` re-exports them and they need no special treatment.
pub mod prelude {
    pub use test_better_core::{ContextExt, OrFail, TestError, TestResult};
    #[cfg(feature = "regex")]
    pub use test_better_matchers::matches_regex;
    pub use test_better_matchers::{
        all_of, always_matches, any_of, at_least_one, between, close_to, contains, contains_all,
        contains_in_order, contains_str, ends_with, eq, err, eventually, eventually_blocking,
        every, ge, gt, have_len, is_empty, is_false, is_finite, is_nan, is_not_empty, is_true, le,
        lt, ne, never_matches, none, not, ok, satisfies, soft, some, starts_with,
    };

    pub use test_better_macros::{matches_struct, matches_tuple, matches_variant};

    pub use crate::{define_matcher, expect, property};
}
