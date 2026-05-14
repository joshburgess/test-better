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
- `test-better-matchers`: the numeric matchers `close_to`, `between`,
  `is_nan`, and `is_finite`, generic over a sealed `Float` trait implemented
  for `f32` and `f64`. `close_to`'s failure shows the tolerance and the actual
  difference; `NaN` is correctly not close to, between, or equal to anything
  (Iteration 3.5).
- `test-better`: the facade crate re-exports the numeric matchers and the
  `Float` trait; the prelude gains the matchers (Iteration 3.5).
- `test-better-matchers`: the `satisfies` escape hatch, a matcher built from an
  arbitrary `Fn(&T) -> bool` predicate. It takes a `name` so a failure reports
  the named expectation rather than the useless `<closure>` (Iteration 3.6).
- `test-better`: the facade crate re-exports `satisfies`; the prelude gains it
  (Iteration 3.6).
- `test-better-macros`: the structural matcher macros `matches_struct!`,
  `matches_tuple!`, and `matches_variant!`. Each takes a type (or `Enum::Variant`)
  and a brace/paren list of `field: matcher` (or positional `matcher`) entries,
  with an optional trailing `..` to ignore the rest; it expands to a `Matcher`
  for that shape. A field's failure is wrapped in a `field:`-headed `Description`.
  Without `..`, every field must be listed (a missing field is a compile error
  from the generated exhaustiveness check); an unknown field and a misplaced `..`
  are also compile errors (Iteration 3.7).
- `test-better`: the facade crate re-exports `matches_struct!`, `matches_tuple!`,
  and `matches_variant!`; the prelude gains them. The macros' generated code
  refers to `::test_better`, so they are usable through the facade only
  (Iteration 3.7).
- `test-better-matchers`: the `define_matcher!` declarative macro, the shortcut
  for the common custom-matcher case. It takes a name, optional constructor
  parameters, a target type, an `expects:` description, and a `matches:`
  predicate, and expands to a matcher type, its `Matcher` impl, and a
  constructor function. Anything richer (a structured diff, an inner matcher) is
  still written by hand as an `impl Matcher<T>` (Iteration 3.8).
- `test-better`: the facade crate re-exports `define_matcher!`; the prelude
  gains it. A new `cookbook` module documents how to write custom matchers, both
  with `define_matcher!` and by hand (Iteration 3.8).
- `examples/custom-matcher/`: a new workspace example crate, the runnable
  companion to the `cookbook` module: a `define_matcher!` matcher, a
  hand-written `impl Matcher<T>`, and a matcher that takes an inner matcher
  (Iteration 3.8).
- `test-better-matchers`: soft assertions, `soft` and `SoftAsserter`. `soft`
  runs a closure in a scope where `SoftAsserter::expect` and
  `SoftAsserter::check` *record* failures instead of returning early; on scope
  exit `soft` returns `Ok(())` or a single `TestError` collecting every
  recorded failure under `Payload::Multiple`, each sub-failure keeping its own
  source location (Iteration 4.1).
- `test-better`: the facade crate re-exports `soft` and `SoftAsserter`; the
  prelude gains `soft` (Iteration 4.1).
- `test-better-matchers`: `SoftAsserter::context` and the `SoftScope` it
  returns. `context` opens a sub-scope whose `expect`/`check` attach an extra
  context frame to every recorded failure; the frame is removed when the
  `SoftScope` is dropped, and nested sub-scopes stack their frames
  outermost-first (Iteration 4.2).
- `test-better`: the facade crate re-exports `SoftScope` (Iteration 4.2).
- `test-better-matchers`: `soft` is now panic-safe. The closure runs under
  `catch_unwind`; if it panics, any failures recorded before the panic are
  written to stderr and the panic is re-raised unchanged, so a panic inside a
  soft scope still fails the test as a panic rather than being swallowed
  (Iteration 4.3).
- `test-better-matchers`: `Subject::resolves_to`, the async counterpart of
  `to`. When the expression handed to `expect!` is a `Future`,
  `expect!(fut).resolves_to(matcher).await?` awaits it and matches its output.
  The method is `#[track_caller]` and returns a future (an `async fn` cannot be
  `#[track_caller]`), so the failure location is the call site, not the await
  point. It is runtime-agnostic: it only awaits, so it works under
  `#[tokio::test]`, `pollster::block_on`, and any other executor
  (Iteration 5.1).
- `test-better-core`: `TestError::with_location`, a builder that overrides the
  captured location. It backs the async `expect!` methods, which capture the
  caller's location synchronously and thread it through once the awaited
  assertion has a result (Iteration 5.1).
- `test-better-async`: the runtime-agnostic timeout layer. `run_within` awaits
  a future under a time limit and returns `Elapsed` if it overruns; the
  `RuntimeAvailable` marker trait gates it on a runtime feature. Concrete
  sleep backends are provided behind the `tokio`, `async-std`, and `smol`
  features (Iteration 5.2).
- `test-better-matchers`: `Subject::to_complete_within`, the async `expect!`
  method that fails a test when a future overruns a `Duration`. It needs a
  runtime feature (`tokio`, `async-std`, or `smol`); with none enabled the
  call is a compile error naming those flags. Like `resolves_to` it is
  `#[track_caller]` and returns a future, so the failure points at the call
  site (Iteration 5.2).
- `test-better`, `test-better-matchers`: `tokio`, `async-std`, and `smol`
  features, each forwarding down to `test-better-async`'s, plus re-exports of
  `Elapsed` and `RuntimeAvailable` (Iteration 5.2).
- `test-better-async`: `eventually` and `eventually_blocking`, the polling
  helpers that retry a `bool`-returning probe until it passes or a `Duration`
  deadline is reached, replacing `sleep + assert` flakiness. They sleep between
  probes on an exponential `Backoff` schedule (configurable via the
  `eventually_with` / `eventually_blocking_with` variants), and the failure
  reports how long they waited and how many times they probed. `eventually` is
  async and runtime-gated like `to_complete_within`; `eventually_blocking`
  sleeps with `std::thread::sleep` and needs no runtime feature
  (Iteration 5.3).
- `test-better`, `test-better-matchers`: re-exports of `eventually`,
  `eventually_blocking`, `eventually_with`, `eventually_blocking_with`, and
  `Backoff`. The prelude gains the two common-path functions, `eventually` and
  `eventually_blocking` (Iteration 5.3).
- `test-better-property`: the property-testing bridge crate. The `Strategy<T>`
  seam (with `ValueTree<T>`, `Runner`, `GenError`, and `ProptestTree`) is a
  deliberately small trait the runner is written against; `proptest` satisfies
  it through a blanket impl, so a property test names ordinary `proptest`
  strategies with no `proptest` import at the call site (Iteration 6.1a).
- `test-better-property`: the `check` runner. `check` (and `check_with`)
  generate cases from a `Strategy<T>`, run a `T -> TestResult` predicate, and on
  the first failure drive the `ValueTree` shrink protocol to a minimal
  counterexample, returning a `PropertyFailure<T>` that carries the original and
  shrunk inputs, the matcher failure, and the case count. `Config` sets the case
  count (256 by default, matching `proptest`); `check` is deterministic by
  default, `check_with` exposes an explicit `Runner` (Iteration 6.1b).
- `test-better`: the facade crate re-exports the property-testing surface
  (`check`, `check_with`, `Strategy`, `ValueTree`, `Runner`, `GenError`,
  `ProptestTree`, `PropertyFailure`, and `Config` renamed `PropertyConfig`), so
  a property test needs only the facade dependency (Iteration 6.1b).
- `test-better-property`, `test-better`: the best-effort `quickcheck` bridge,
  behind a new off-by-default `quickcheck` feature. `arbitrary::<T>()` turns any
  `quickcheck::Arbitrary` type into a seam `Strategy<T>` (`ArbitraryStrategy<T>`,
  with `QuickcheckTree<T>` adapting `quickcheck`'s linear `shrink` to the
  `simplify`/`complicate` protocol), so a property test can name
  `arbitrary::<MyType>()` wherever it would name a `proptest` strategy. The
  facade forwards the feature and re-exports `arbitrary`, `ArbitraryStrategy`,
  and `QuickcheckTree` (Iteration 6.1c).
- `test-better-property`, `test-better`: the `property!` macro, the test-facing
  front for property testing. It takes a closure with a typed binding and a
  block body returning `TestResult`, infers a `Strategy` from the binding's type
  (via the new `any::<T>()` strategy constructor) or takes one explicitly with a
  trailing `using` clause, runs it through `check`, and renders a counterexample
  as a `TestError`. It expands to an expression, so `property!(...)` is the body
  of an ordinary `#[test]` function. The facade re-exports `property!` and
  `any`, and the prelude gains `property!` (Iteration 6.2).
- `test-better-property`: a failing property now renders the original failing
  input, the shrunk minimal input, and the matcher's own structured
  description. `render_failure` keeps the matcher failure whole and wraps three
  context frames (the case count, the original input, the shrunk input) around
  it, promoting the kind to `ErrorKind::Property`. A golden-file test
  (`tests/shrink_output.rs`, with the golden under `tests/golden/`) pins the
  exact output (Iteration 6.3).
- `test-better-snapshot`: the file-backed snapshot store. `assert_snapshot`
  (and the directory-explicit `assert_snapshot_in`) compares a value against
  `tests/snapshots/<module-path>__<name>.snap`, or rewrites it under
  `SnapshotMode::Update` / `UPDATE_SNAPSHOTS=1`. `snapshot_path` exposes the
  path-naming rule and `SnapshotFailure` is the structured outcome (missing
  file, mismatch, or I/O error). The crate is `std`-only (Iteration 7.1).
- `test-better-matchers`: `Subject::to_match_snapshot(name)` asserts a
  `Display` value against its file-backed snapshot. A mismatch renders as an
  `ErrorKind::Snapshot` failure with an expected/actual payload and (with the
  `diff` feature on) a line-oriented diff; a missing snapshot points the reader
  at `UPDATE_SNAPSHOTS=1`. `expect!` now also captures `module_path!()` to name
  the snapshot file, so `Subject::new` takes a third argument (Iteration 7.1).
- `test-better`: the facade crate re-exports the snapshot surface
  (`assert_snapshot`, `assert_snapshot_in`, `snapshot_path`, `SnapshotMode`,
  `SnapshotFailure`); `to_match_snapshot` rides along on the re-exported
  `Subject` (Iteration 7.1).

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
- The structural matcher macros (Iteration 3.7) cannot name a struct's field
  types, and an impl's generic parameters cannot be unified with concrete types
  from a destructure. The generated code therefore uses a projection closure
  whose `Fn(&S) -> (&F0, &F1, ...)` type pins both the subject type and the
  field types; the closure is threaded through a generated generic constructor
  function carrying the `Fn` bound, which forces the higher-ranked inference a
  bare struct field would not get. This is the googletest-style projection
  pattern, a deviation from any single-struct sketch in PROJECT_BUILD_PLAN.md §8.
  The macros are re-exported through the `test-better` facade only, since their
  output names `::test_better` and a proc-macro crate's output can only name the
  consumer's direct dependencies. Compile-fail behavior is pinned by `trybuild`
  tests in `crates/test-better/tests/ui/` (Iteration 3.7).
- `scripts/check-test-api.sh` now matches `.expect("` (the panic call followed
  by its string-literal message) rather than a bare `.expect(`. The soft
  assertion API (Iteration 4.1) names a method `SoftAsserter::expect`, and a
  bare `.expect(` could not tell `s.expect(&actual, matcher)` apart from
  `Result::expect("...")`. A non-test `.expect` with a non-literal message is
  still denied by the workspace's `clippy::expect_used` lint (Iteration 4.1).
- The async `expect!` acceptance tests (Iteration 5.1) cover `pollster` and
  `tokio` but not `async-std`, which PROJECT_BUILD_PLAN.md §10 also names.
  `async-std` is unmaintained as of 2025; `resolves_to` only awaits the future
  and touches no runtime API, so two unrelated executors already demonstrate
  the runtime-agnosticism the plan asks for. A `trybuild` test
  (`tests/ui/sync_to_on_future.rs`) locks that the sync `to` cannot be pointed
  at a future-typed subject with an output matcher: that path must go through
  `resolves_to` (Iteration 5.1).
- `to_complete_within` (Iteration 5.2) is an inherent `Subject` method, kept in
  the same impl block as `resolves_to` per the §7.3 decision. To make that
  possible without `test-better-matchers` taking on optional runtime
  dependencies, the timeout machinery lives one layer down in
  `test-better-async` (which now depends only on `test-better-core`), and
  `test-better-matchers` depends on it. The dependency edges are
  `matchers -> async -> core`; `test-better-async` carries `test-better-matchers`
  as a dev-dependency for dogfooding, the same permitted dev-cycle as
  `test-better-core`.
- The "no runtime feature is a compile error" requirement
  (PROJECT_BUILD_PLAN.md §10 Iteration 5.2) is met with a deferred trait bound,
  not a literal `compile_error!`. A `compile_error!` in a function body fires
  whenever the crate is built, which would break `cargo build` with no runtime
  feature; and a `where` bound on a *concrete* type (`SelectedRuntime:
  Timeout`) is rejected at the definition, not deferred. Instead,
  `RuntimeAvailable` is a marker trait bound on the *generic* future type, so
  the check is deferred to the call site, and `#[diagnostic::on_unimplemented]`
  supplies the message naming the feature flags. The crate compiles cleanly
  with zero runtime features; only *calling* `to_complete_within` without one
  is the error.
- The three per-runtime acceptance crates (`tests/timeout-tokio`,
  `tests/timeout-async-std`, `tests/timeout-smol`) are excluded from the
  workspace. A workspace-wide `cargo test --all-features` unifies all three
  runtime features into one `test-better`, under which `cfg` priority picks
  `tokio` and the `async-std`/`smol` tests would run against the wrong runtime.
  Excluding them keeps each crate's runtime feature isolated; CI runs each with
  its own `cargo test --manifest-path`. The fourth crate,
  `tests/timeout-no-runtime`, enables no runtime feature and so is a safe
  workspace member; its `trybuild` test confirms the missing-runtime
  diagnostic (Iteration 5.2).
- `eventually` (Iteration 5.3) is a free function in `test-better-async`, not a
  `Subject` method: it polls a probe closure rather than asserting on a single
  captured value, so there is nothing for `expect!` to wrap. Its async form is
  runtime-gated the same way as `to_complete_within` (the deferred
  `RuntimeAvailable` bound, here on the probe closure type), since the
  inter-probe sleep is runtime-provided; `eventually_blocking` is the
  runtime-free escape hatch and carries no such bound. The
  `#[diagnostic::on_unimplemented]` message on `RuntimeAvailable` was broadened
  from naming `to_complete_within` specifically to "this async timing
  assertion", with an extra note pointing at `eventually_blocking`. The
  `eventually` acceptance tests live in the per-runtime crates (real runtimes
  drive the inter-probe sleep); `eventually_blocking` is covered by inline tests
  and a facade integration test, and `tests/timeout-no-runtime` gains a second
  `trybuild` case for the gated `eventually` (Iteration 5.3).
- `Backoff`'s `initial`/`ceiling`/`factor` knobs are exposed through the
  `eventually_with` / `eventually_blocking_with` variants rather than widening
  the two-argument signature PROJECT_BUILD_PLAN.md §10 Iteration 5.3 sketches.
  The plain `eventually` / `eventually_blocking` use `Backoff::default` (1ms
  initial, doubling, 100ms ceiling). Only the two default-schedule functions are
  in the prelude; `Backoff` and the `_with` variants are imported by name, in
  keeping with the deliberately small prelude (Iteration 5.3).
- `proptest` is the property-testing backend for v1.0 (BACKLOG.md §11.1,
  resolved in Iteration 6.1a): it ships integrated shrinking, the feature the
  `ValueTree` protocol is built on. It is depended on with
  `default-features = false, features = ["std"]`, which drops the `fork` /
  `timeout` machinery (and `rusty-fork` / `libc` / `wait-timeout`) the bridge
  does not use; `cargo deny` stays clean. A `quickcheck` bridge remains open in
  BACKLOG.md as a post-1.0 idea, not a 1.0 blocker.
- The `Strategy<T>` seam has one coherence limitation: a user type that is
  itself a `proptest::strategy::Strategy` is already covered by the blanket
  impl, so it cannot also carry a hand-written `Strategy<T>` impl. This is
  accepted: the seam exists so the runner does not name `proptest` directly, not
  so users reimplement strategies. Recorded in BACKLOG.md (Iteration 6.1a).
- `check` runs deterministically by default (`Runner::deterministic`) so a
  property test does not flake from run to run: the same strategy and predicate
  pass or fail identically every time. `check_with` exposes `Runner::randomized`
  for callers who want fresh entropy per run. An over-filtered strategy that
  cannot produce a value is skipped, not counted as a property failure
  (Iteration 6.1b).
- `Config` is re-exported from the facade as `PropertyConfig`: at the facade
  root, where one crate's surface meets eight others, a bare `Config` says too
  little (Iteration 6.1b).
- `property!` (Iteration 6.2) is an *expression* macro that expands to a
  `TestResult`, not an item macro that generates a `#[test] fn`. The
  PROJECT_BUILD_PLAN.md §11 6.2 sketch shows `property!(|s: String| { ... })`
  with no test name, which an item macro could not turn into a named function;
  an expression macro composes with `?` and is the body of a hand-named
  `#[test] fn`, which keeps test naming and attributes in the user's hands.
  `property!` routes through a `#[doc(hidden)]` `run_property` helper rather
  than re-exporting `check`'s rendering, so Iteration 6.3 can enrich the
  shrunk-failure output without touching the macro. The strategy-inference
  branch requires the binding type to be `proptest::arbitrary::Arbitrary`; the
  `using` clause sidesteps that for any seam `Strategy`.
- The shrunk-failure rendering `property!` produces is built from `TestError`
  context frames rather than a new `Payload` variant: the matcher's own failure
  is kept whole (its `message` and `ExpectedActual` payload carry the structured
  description), and the property metadata (case count, original input, shrunk
  input) is added as three context frames around it. A dedicated
  `Payload::Property` would have meant boxing the matcher failure inside it,
  since a `TestError` has one payload slot and the matcher's is already
  `ExpectedActual`; the structured form of a property failure is the typed
  `PropertyFailure<T>` that `check` returns, not a payload variant. The
  golden-file test (Iteration 6.3) pins the rendered output; it builds the
  `PropertyFailure` by hand so the golden file is deterministic and not coupled
  to the backend's RNG, and normalizes the environment-specific `  at` line.
  Regenerate it with `BLESS_GOLDEN=1`.
- The `quickcheck` bridge (Iteration 6.1c) ships at documented reduced fidelity
  rather than being deferred, which PROJECT_BUILD_PLAN.md §11 6.1c lists as an
  acceptable outcome. Two limitations are inherent to `quickcheck`'s model, not
  bugs: a `quickcheck::Gen` owns its RNG and cannot be seeded from the seam's
  `Runner`, so an `arbitrary()` strategy is *not* made reproducible by
  `Runner::deterministic` (only `proptest` strategies honor it); and shrinking
  is `quickcheck`'s flat `Arbitrary::shrink` iterator, which the
  `QuickcheckTree` maps onto the seam's `simplify`/`complicate` protocol
  faithfully but which does not promise the exact boundary value `proptest`'s
  integrated shrinking does. Both are documented on the `quickcheck_bridge`
  module. The `quickcheck` feature pulls a second `rand` major version into the
  graph; `cargo deny`'s `multiple-versions` is `warn`, so this is a warning, not
  a failure, and the feature is off by default.
- `test-better-snapshot` (Iteration 7.1) is deliberately `std`-only and does
  not depend on `test-better-core`: it returns the structured `SnapshotFailure`,
  and `test-better-matchers` owns the `SnapshotFailure` -> `TestError` rendering
  (it is the crate with the diff renderer, and `test-better-matchers` already
  depends on `test-better-snapshot` for the `to_match_snapshot` method). The
  snapshot directory is resolved from the current working directory, which
  `cargo test` sets to the package root; the directory-explicit
  `assert_snapshot_in` exists so the crate's own lifecycle test can drive a
  temporary directory rather than a committed fixture. The update-vs-compare
  decision is a `SnapshotMode` parameter, not read from the environment inside
  the core, so a test can exercise both modes without mutating process-global
  env state (`SnapshotMode::from_env` reads `UPDATE_SNAPSHOTS` only at the
  `assert_snapshot` boundary). Snapshot files store the value verbatim, with no
  added trailing newline, so comparison is exact. The facade `tests/snapshot.rs`
  asserts only the matching case: a mismatch asserted through `to_match_snapshot`
  would behave differently under `UPDATE_SNAPSHOTS=1`, so the mismatch and
  create/update paths are tested where they can be driven explicitly
  (`test-better-snapshot/tests/lifecycle.rs` and the `snapshot_error` unit
  tests).
