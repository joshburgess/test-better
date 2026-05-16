//! `test-better`: `Result`-returning Rust tests with `?`.
//!
//! `test-better` makes a test that returns [`TestResult`] and uses `?` strictly
//! better than a panicking one: a failure is a value that carries the
//! expression that failed, the values involved, the source location, and the
//! context you attached on the way down.
//!
//! This is the facade crate, the one users depend on. It re-exports the public
//! surface of the workspace's focused crates (`test-better-core`,
//! `test-better-matchers`, and so on) so a test file needs a single
//! dependency and, ideally, a single `use`:
//!
//! ```
//! use test_better::prelude::*;
//!
//! fn parse_port(input: &str) -> Option<u16> {
//!     input.parse().ok()
//! }
//!
//! # fn main() -> TestResult {
//! // `or_fail_with` is the `?`-friendly stand-in for a panicking unwrap;
//! // `check!` captures the expression text so a failure names `port`,
//! // not just its value.
//! let port = parse_port("8080").or_fail_with("8080 is a valid port")?;
//! check!(port).satisfies(eq(8080))?;
//! check!(port).violates(lt(1024))?;
//! # Ok(())
//! # }
//! ```
//!
//! In a real test file the body above is a `#[test] fn ... -> TestResult`
//! ending in `Ok(())`. See the [`prelude`] for the one import a test file
//! needs, and the [`cookbook`] for writing custom matchers. The prose guide
//! (Getting Started, migrating from the stock assertion macros, async,
//! property, snapshot, and fixture testing) is the `test-better` book under
//! `book/`.

pub use test_better_core::{
    ColorChoice, ContextExt, ContextFrame, ErrorKind, OrFail, Payload, RUNNER_ENV,
    STRUCTURED_MARKER, SourceLocation, StructuredContextFrame, StructuredError, StructuredPayload,
    TestError, TestResult, Trace, TraceEntry, color_choice, set_color_choice,
};
pub use test_better_macros::{
    fixture, matches_struct, matches_tuple, matches_variant, test_case, test_with_fixtures,
};
// The property-testing bridge. `Config` is renamed `PropertyConfig` here: at
// the facade root, where one crate's surface meets eight others, a bare
// `Config` says too little.
#[cfg(feature = "diff")]
pub use test_better_matchers::diff_lines;
#[cfg(feature = "regex")]
pub use test_better_matchers::matches_regex;
pub use test_better_matchers::{
    Backoff, ContainsAll, Description, Elapsed, Float, Items, MatchResult, Matcher, MatcherTuple,
    Mismatch, RuntimeAvailable, Sequence, SoftAsserter, SoftScope, Subject, all_of, always_matches,
    any_of, at_least_one, between, check, close_to, contains, contains_all, contains_in_order,
    contains_str, define_matcher, ends_with, eq, err, eventually, eventually_blocking,
    eventually_blocking_with, eventually_with, every, ge, gt, have_len, is_empty, is_false,
    is_finite, is_nan, is_not_empty, is_true, items, le, lt, ne, never_matches, none, not, ok,
    predicate, soft, some, starts_with,
};
pub use test_better_property::{
    Config as PropertyConfig, GenError, PropertyFailure, ProptestTree, Runner, Strategy, ValueTree,
    any, for_all, for_all_with, property,
};
// The best-effort `quickcheck` bridge, behind the facade's `quickcheck`
// feature: `arbitrary::<T>()` turns a `quickcheck::Arbitrary` type into a
// `Strategy<T>`. Off by default; `proptest` is the primary backend.
#[cfg(feature = "quickcheck")]
pub use test_better_property::{ArbitraryStrategy, QuickcheckTree, arbitrary};
// The snapshot store. The everyday entry points are the
// `check!(value).matches_snapshot("name")` and `.matches_inline_snapshot(..)`
// methods (on the re-exported `Subject`); these are the lower-level pieces they
// are built on, for callers that need an explicit directory or mode, or that
// drive the `test-better-accept` companion binary.
pub use test_better_snapshot::{
    InlineLocation, InlineSnapshotFailure, Redactions, SnapshotFailure, SnapshotMode,
    assert_inline_snapshot, assert_snapshot, assert_snapshot_in, normalize_inline_literal,
    parse_pending_patch, pending_patch_dir, snapshot_path,
};

/// How to write a custom matcher: see the [`cookbook`] module.
pub mod cookbook;

/// The one `use` a test file should need: `use test_better::prelude::*;`.
///
/// The prelude is deliberately small. It brings in the result type, the error
/// type, the extension traits whose methods (`context`, `or_fail`, ...) are
/// meant to be called without qualification, the `check!` macro, and the
/// matcher constructors (`eq`, `lt`, ...). The structured-failure types and the
/// custom-matcher machinery (`Matcher`, `Description`, ...) stay out of it:
/// they are imported by name when needed, not in the body of every test.
///
/// # Re-exporting macros
///
/// `#[macro_export]` places a macro at the crate root, not inside the module it
/// is written in, so a glob import of this module would *not* pick it up unless
/// the macro is named here explicitly. That is why `check` and
/// `define_matcher` appear below with `pub use crate::...;`; later phases add
/// their `#[macro_export]` macros the same way.
///
/// Procedural macros (`matches_struct!`, `matches_tuple!`, `matches_variant!`,
/// and the `#[fixture]` / `#[test_with_fixtures]` attribute pair) are different:
/// they are ordinary items of `test-better-macros`, so a plain `pub use`
/// re-exports them and they need no special treatment. `#[fixture]` and
/// `#[test_with_fixtures]` are in the prelude (unlike `#[test_case]`, they do
/// not collide with anything in `std`'s prelude).
///
/// The one exception is `#[test_case]`: it lives at the facade root
/// (`test_better::test_case`) but is kept *out* of the prelude, because `std`'s
/// prelude exports a `test_case` attribute of its own and two glob imports of
/// one name are ambiguous. Import it by name.
pub mod prelude {
    pub use test_better_core::{ContextExt, OrFail, TestError, TestResult};
    #[cfg(feature = "regex")]
    pub use test_better_matchers::matches_regex;
    pub use test_better_matchers::{
        all_of, always_matches, any_of, at_least_one, between, close_to, contains, contains_all,
        contains_in_order, contains_str, ends_with, eq, err, eventually, eventually_blocking,
        every, ge, gt, have_len, is_empty, is_false, is_finite, is_nan, is_not_empty, is_true,
        items, le, lt, ne, never_matches, none, not, ok, predicate, soft, some, starts_with,
    };

    // `test_case` is deliberately *not* re-exported here. `std`'s own prelude
    // already exports a `test_case` attribute (the unstable custom-test-
    // frameworks one), and two glob imports of the same name are ambiguous at
    // the use site. It is available as `test_better::test_case`; import it
    // explicitly.
    pub use test_better_macros::{
        fixture, matches_struct, matches_tuple, matches_variant, test_with_fixtures,
    };

    pub use crate::{check, define_matcher, property};
}
