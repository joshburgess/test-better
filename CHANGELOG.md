# Changelog

All notable changes to `test-better` are documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and the project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

Per PROJECT_BUILD_PLAN.md §3 and §16, **every public API change is recorded under
`## [Unreleased]` before the PR that introduces it is merged.** All crates are
versioned in lockstep until 1.0.

## [Unreleased]

### Added

- Phase 0 scaffolding: workspace with eight member crates, pinned toolchain
  (`1.95.0`), lint and format configuration, CI matrix, dual licensing, and
  contribution docs. No public API yet.
- `test-better-core`: `TestError` failure type with `ErrorKind`, `ContextFrame`,
  and `Payload`. `Display`/`Debug` render a pretty failure message and the
  `std::error::Error` impl walks the wrapped-error source chain (Iteration 1.1).
- `test-better-core`: `StructuredError` (with `SourceLocation`,
  `StructuredContextFrame`, `StructuredPayload`) and `TestError::to_structured`,
  the owned/serializable form tooling consumes. An optional `serde` feature
  derives `Serialize`/`Deserialize` on the structured types (Iteration 1.1).
- `test-better-core`: `TestResult<T = ()>`, the `?`-friendly result alias
  returned by tests and helpers (Iteration 1.2).
- `test-better-core`: `TestError` convenience constructors `assertion`,
  `custom`, and `from_expected_actual`, each `#[track_caller]` so the captured
  location is the caller's (Iteration 1.2).
- `test-better-core`: `ContextExt`, implemented for `Result<T, E>` and
  `Option<T>`, with `context` and `with_context` (the latter computes its
  message only on the failure path). Both are `#[track_caller]`. A `Result`
  whose error already is a `TestError` is not double-wrapped: the context
  frame is pushed onto it directly (Iteration 1.3).
- `test-better-core`: `OrFail`, the `?`-friendly replacement for `.unwrap()`,
  implemented for `Result<T, E>` and `Option<T>`, with `or_fail` and
  `or_fail_with`. `or_fail` on `None` names the expected type; on `Err` it
  preserves the underlying error chain. All methods are `#[track_caller]`
  (Iteration 1.4).
- `test-better`: the facade crate now re-exports the public surface of
  `test-better-core` and exposes a `prelude` module, so a test file needs one
  dependency and one `use test_better::prelude::*;`. A `serde` feature forwards
  to `test-better-core`'s. The prelude documents the macro re-export pattern
  Phase 2 will slot into (Iteration 1.5).
- `test-better-matchers`: the `Matcher<T: ?Sized>` trait with its structured
  result types `MatchResult` and `Mismatch`, and `Description`, the composable
  account of a matcher's expectation (`text`, `and`, `or`, `labeled`, and
  `std::ops::Not`). No public matchers yet (Iteration 2.1).
- `test-better-matchers`: the primitive matchers `eq`, `ne`, `lt`, `le`, `gt`,
  `ge`, `is_true`, `is_false`, and the test fixtures `always_matches` and
  `never_matches` (Iteration 2.2).
- `test-better-matchers`: the `expect!` macro and its `Subject<T>` type, with
  `to` and `to_not` returning `TestResult` (`#[track_caller]`, and `#[must_use]`
  via `Result` so a forgotten `?` is a warning). `expect!` captures the source
  text of its argument, so a failure names the expression, not just its value.
  A `trybuild` test pins down the forgotten-`?` warning (Iteration 2.3).
- `test-better`: the facade crate now re-exports the matcher surface and the
  `expect!` macro; the prelude gains `expect!` and the matcher constructors
  (Iteration 2.3).
- `test-better-core`: `ColorChoice` (`Auto`/`Always`/`Never`), `set_color_choice`,
  and `color_choice`. The `TestError` renderer now takes a colorize flag:
  `Debug` may emit ANSI color (honoring `NO_COLOR` and terminal detection under
  `Auto`), while `Display` stays plain. Color ownership lives here, not in
  `matchers` (Iteration 2.4).
- `test-better-matchers`: the `diff_lines` line-oriented diff renderer, behind a
  new default `diff` feature (backed by `similar`). `eq` now attaches a diff to
  its mismatch when the values' pretty (`{:#?}`) representations span multiple
  lines; `matchers` produces only the structured, uncolored diff text
  (Iteration 2.4).
- `test-better`: the facade crate gains a default `diff` feature forwarding to
  `test-better-matchers/diff`, and re-exports the color configuration
  (`ColorChoice`, `set_color_choice`, `color_choice`) and `diff_lines`
  (Iteration 2.4).

### Notes

- `TestError` carries a dedicated `message: Option<Cow<'static, str>>` field, a
  deliberate deviation from the struct sketched in PROJECT_BUILD_PLAN.md §6.1:
  the message answers *what* failed, context frames answer *while doing what*.
  See the type's rustdoc for rationale.
- `clippy.toml` gained `allow-panic-in-tests = true`, completing the
  "allowed in tests" intent of PROJECT_BUILD_PLAN.md §3 (Phase 0 set only the
  unwrap/expect equivalents).
- The async-`Subject` design (PROJECT_BUILD_PLAN.md §7.3) is resolved: a single
  `Subject<T>` type, with Phase 5's `await`-based methods added to the same impl
  block under method-level `where T: Future` bounds. Rationale in `BACKLOG.md`.
- `TestError::payload` is `Option<Box<Payload>>` rather than `Option<Payload>`.
  `TestError` is returned by value through every `?`, so it is kept small; the
  large `Payload::ExpectedActual` variant lives behind the box. The public
  `Payload` enum and `with_payload` signature are unchanged (Iteration 1.3).
- Dogfood switchover (Iteration 2.5): every test in the workspace now uses
  `expect!` and `TestResult` instead of `assert!`/`assert_eq!`/`.unwrap()`/
  `.expect()`, enforced by `scripts/check-test-api.sh` (a new `dogfood` CI job)
  scanning `crates/*/src/`. No public API changed. Two implementation notes:
  `test-better-core` carries `test-better-matchers` as a dev-dependency (a
  dev-dependency cycle, which Cargo permits) so its own tests can use `expect!`;
  and because that cycle compiles `test-better-core` twice, its inline tests
  bridge a matcher result into the test's `TestResult` with a trailing
  `.or_fail()?` rather than a bare `?`. Tests in dependent crates and in
  `tests/` directories use the plain `expect!(..).to(..)?` form.
