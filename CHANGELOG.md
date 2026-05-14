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
- `test-better-matchers`: the logical combinators `not`, `all_of`, and
  `any_of`. `not(m)` inverts a matcher; `all_of`/`any_of` take a tuple of
  matchers (arities 2 through 8, via the `MatcherTuple` trait) under
  conjunction and disjunction. `all_of`'s failure is the first failing
  sub-matcher's, so it pinpoints the broken expectation; `any_of`'s describes
  the whole disjunction. Each combinator builds its `Description` from its
  children's through the `!`/`and`/`or` combinators on `Description`
  (Iteration 3.1).
- `test-better`: the facade crate re-exports the logical combinators (`not`,
  `all_of`, `any_of`, `MatcherTuple`); the prelude gains `not`, `all_of`, and
  `any_of` (Iteration 3.1).
- `test-better-matchers`: the `Option`/`Result` matchers `some`, `none`, `ok`,
  and `err`. `some`, `ok`, and `err` take an inner matcher and apply it to the
  wrapped value, so they nest (`some(ok(eq(42)))`); a nested failure wraps each
  layer's expectation in a `label:`-headed `Description`, rendering aligned,
  indented `some:`/`ok:` blocks (Iteration 3.2).
- `test-better`: the facade crate re-exports the `Option`/`Result` matchers
  (`some`, `none`, `ok`, `err`); the prelude gains them too (Iteration 3.2).
- `test-better-matchers`: the `Sequence` trait and the collection matchers
  `have_len`, `is_empty`, `is_not_empty`, `contains`, `contains_all`,
  `contains_in_order`, `every`, and `at_least_one`. `Sequence` is implemented
  for `[T]`, `[T; N]`, `Vec<T>`, `VecDeque<T>`, `BTreeSet<T>`, `HashSet<T>`,
  and `&S`. `contains_all` takes a tuple of matchers (arities 2 through 8, via
  the `ContainsAll` trait); `contains_in_order` takes an array. Failures name
  the index of the first item (or, for sets, the offending value) that broke
  the expectation (Iteration 3.3).
- `test-better`: the facade crate re-exports the collection matchers and the
  `Sequence`/`ContainsAll` traits; the prelude gains the matchers (Iteration
  3.3).
- `test-better-matchers`: the string matchers `contains_str`, `starts_with`,
  `ends_with`, and `matches_regex`. Each is generic over `T: AsRef<str>`, so it
  matches `&str`, `String`, and `str` alike; a multi-line mismatch carries a
  line-oriented diff. `matches_regex` is behind a new, non-default `regex`
  feature (backed by the `regex` crate); an invalid pattern is reported as a
  match failure rather than a panic, so the constructor stays infallible
  (Iteration 3.4).
- `test-better`: the facade crate re-exports the string matchers and gains a
  `regex` feature forwarding to `test-better-matchers/regex`; the prelude gains
  the string matchers (`matches_regex` only when `regex` is enabled)
  (Iteration 3.4).

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
- `Sequence` is *not* implemented for lazy iterators, a deliberate deviation
  from PROJECT_BUILD_PLAN.md §8 Iteration 3.3 ("iterators, eagerly collected").
  A blanket `impl<I: Iterator> Sequence for I` overlaps, under coherence, with
  the concrete `impl Sequence for Vec<T>` (and the other collections), so the
  two cannot coexist. `Sequence` borrows its items through `&self`, which a
  lazy iterator cannot offer anyway. Callers collect an iterator into a `Vec`
  first (`expect!(it.collect::<Vec<_>>())`), which is the "eager collection"
  the plan asked for, just at the call site. Recorded as an idea in
  `BACKLOG.md` in case a dedicated adapter is wanted later (Iteration 3.3).
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
