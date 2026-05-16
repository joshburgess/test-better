# Changelog

All notable changes to `test-better` are documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and the project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

**Every public API change is recorded under `## [Unreleased]` before the PR that
introduces it is merged.** All crates are versioned in lockstep.

## [Unreleased]

### Added

- `test-better-matchers`: `items(iter)`, an eager `Sequence` wrapper that
  collects an iterator into a `Vec` so any collection matcher (`have_len`,
  `contains`, `contains_in_order`, `every`, ...) accepts a lazy iterator
  without the caller writing `.collect()` first. The returned `Items<T>` is
  re-exported from the facade and `items` is added to the prelude.

## [0.1.0] - 2026-05-15

The initial public release. `test-better` makes a `Result`-returning test that
uses `?` strictly better than a panicking one: a failure is a value carrying
the expression that failed, the values involved, the source location, and the
context attached on the way down. This release covers the full surface: the
core error and result types, the `Matcher` trait with the standard matcher
library and `expect!`, the structural and parameterized macros, async and
timing helpers, the property-testing bridge, snapshot testing, and the
optional `cargo-test-better` runner. The prose guide is the `test-better`
book, published at <https://joshburgess.github.io/test-better/>.

### Added

- Initial scaffolding: workspace with eight member crates, pinned toolchain
  (`1.95.0`), lint and format configuration, CI matrix, dual licensing, and
  contribution docs. No public API yet.
- `test-better-core`: `TestError` failure type with `ErrorKind`, `ContextFrame`,
  and `Payload`. `Display`/`Debug` render a pretty failure message and the
  `std::error::Error` impl walks the wrapped-error source chain.
- `test-better-core`: `StructuredError` (with `SourceLocation`,
  `StructuredContextFrame`, `StructuredPayload`) and `TestError::to_structured`,
  the owned/serializable form tooling consumes. An optional `serde` feature
  derives `Serialize`/`Deserialize` on the structured types.
- `test-better-core`: `TestResult<T = ()>`, the `?`-friendly result alias
  returned by tests and helpers.
- `test-better-core`: `TestError` convenience constructors `assertion`,
  `custom`, and `from_expected_actual`, each `#[track_caller]` so the captured
  location is the caller's.
- `test-better-core`: `ContextExt`, implemented for `Result<T, E>` and
  `Option<T>`, with `context` and `with_context` (the latter computes its
  message only on the failure path). Both are `#[track_caller]`. A `Result`
  whose error already is a `TestError` is not double-wrapped: the context
  frame is pushed onto it directly.
- `test-better-core`: `OrFail`, the `?`-friendly replacement for `.unwrap()`,
  implemented for `Result<T, E>` and `Option<T>`, with `or_fail` and
  `or_fail_with`. `or_fail` on `None` names the expected type; on `Err` it
  preserves the underlying error chain. All methods are `#[track_caller]`.
- `test-better`: the facade crate now re-exports the public surface of
  `test-better-core` and exposes a `prelude` module, so a test file needs one
  dependency and one `use test_better::prelude::*;`. A `serde` feature forwards
  to `test-better-core`'s. The prelude documents the macro re-export pattern
  matchers will slot into.
- `test-better-matchers`: the `Matcher<T: ?Sized>` trait with its structured
  result types `MatchResult` and `Mismatch`, and `Description`, the composable
  account of a matcher's expectation (`text`, `and`, `or`, `labeled`, and
  `std::ops::Not`). No public matchers yet.
- `test-better-matchers`: the primitive matchers `eq`, `ne`, `lt`, `le`, `gt`,
  `ge`, `is_true`, `is_false`, and the test fixtures `always_matches` and
  `never_matches`.
- `test-better-matchers`: the `expect!` macro and its `Subject<T>` type, with
  `to` and `to_not` returning `TestResult` (`#[track_caller]`, and `#[must_use]`
  via `Result` so a forgotten `?` is a warning). `expect!` captures the source
  text of its argument, so a failure names the expression, not just its value.
  A `trybuild` test pins down the forgotten-`?` warning.
- `test-better`: the facade crate now re-exports the matcher surface and the
  `expect!` macro; the prelude gains `expect!` and the matcher constructors.
- `test-better-core`: `ColorChoice` (`Auto`/`Always`/`Never`), `set_color_choice`,
  and `color_choice`. The `TestError` renderer now takes a colorize flag:
  `Debug` may emit ANSI color (honoring `NO_COLOR` and terminal detection under
  `Auto`), while `Display` stays plain. Color ownership lives here, not in
  `matchers`.
- `test-better-matchers`: the `diff_lines` line-oriented diff renderer, behind a
  new default `diff` feature (backed by `similar`). `eq` now attaches a diff to
  its mismatch when the values' pretty (`{:#?}`) representations span multiple
  lines; `matchers` produces only the structured, uncolored diff text.
- `test-better`: the facade crate gains a default `diff` feature forwarding to
  `test-better-matchers/diff`, and re-exports the color configuration
  (`ColorChoice`, `set_color_choice`, `color_choice`) and `diff_lines`.
- `test-better-matchers`: the logical combinators `not`, `all_of`, and
  `any_of`. `not(m)` inverts a matcher; `all_of`/`any_of` take a tuple of
  matchers (arities 2 through 8, via the `MatcherTuple` trait) under
  conjunction and disjunction. `all_of`'s failure is the first failing
  sub-matcher's, so it pinpoints the broken expectation; `any_of`'s describes
  the whole disjunction. Each combinator builds its `Description` from its
  children's through the `!`/`and`/`or` combinators on `Description`.
- `test-better`: the facade crate re-exports the logical combinators (`not`,
  `all_of`, `any_of`, `MatcherTuple`); the prelude gains `not`, `all_of`, and
  `any_of`.
- `test-better-matchers`: the `Option`/`Result` matchers `some`, `none`, `ok`,
  and `err`. `some`, `ok`, and `err` take an inner matcher and apply it to the
  wrapped value, so they nest (`some(ok(eq(42)))`); a nested failure wraps each
  layer's expectation in a `label:`-headed `Description`, rendering aligned,
  indented `some:`/`ok:` blocks.
- `test-better`: the facade crate re-exports the `Option`/`Result` matchers
  (`some`, `none`, `ok`, `err`); the prelude gains them too.
- `test-better-matchers`: the `Sequence` trait and the collection matchers
  `have_len`, `is_empty`, `is_not_empty`, `contains`, `contains_all`,
  `contains_in_order`, `every`, and `at_least_one`. `Sequence` is implemented
  for `[T]`, `[T; N]`, `Vec<T>`, `VecDeque<T>`, `BTreeSet<T>`, `HashSet<T>`,
  and `&S`. `contains_all` takes a tuple of matchers (arities 2 through 8, via
  the `ContainsAll` trait); `contains_in_order` takes an array. Failures name
  the index of the first item (or, for sets, the offending value) that broke
  the expectation.
- `test-better`: the facade crate re-exports the collection matchers and the
  `Sequence`/`ContainsAll` traits; the prelude gains the matchers.
- `test-better-matchers`: the string matchers `contains_str`, `starts_with`,
  `ends_with`, and `matches_regex`. Each is generic over `T: AsRef<str>`, so it
  matches `&str`, `String`, and `str` alike; a multi-line mismatch carries a
  line-oriented diff. `matches_regex` is behind a new, non-default `regex`
  feature (backed by the `regex` crate); an invalid pattern is reported as a
  match failure rather than a panic, so the constructor stays infallible.
- `test-better`: the facade crate re-exports the string matchers and gains a
  `regex` feature forwarding to `test-better-matchers/regex`; the prelude gains
  the string matchers (`matches_regex` only when `regex` is enabled).
- `test-better-matchers`: the numeric matchers `close_to`, `between`,
  `is_nan`, and `is_finite`, generic over a sealed `Float` trait implemented
  for `f32` and `f64`. `close_to`'s failure shows the tolerance and the actual
  difference; `NaN` is correctly not close to, between, or equal to anything.
- `test-better`: the facade crate re-exports the numeric matchers and the
  `Float` trait; the prelude gains the matchers.
- `test-better-matchers`: the `satisfies` escape hatch, a matcher built from an
  arbitrary `Fn(&T) -> bool` predicate. It takes a `name` so a failure reports
  the named expectation rather than the useless `<closure>`.
- `test-better`: the facade crate re-exports `satisfies`; the prelude gains it.
- `test-better-macros`: the structural matcher macros `matches_struct!`,
  `matches_tuple!`, and `matches_variant!`. Each takes a type (or `Enum::Variant`)
  and a brace/paren list of `field: matcher` (or positional `matcher`) entries,
  with an optional trailing `..` to ignore the rest; it expands to a `Matcher`
  for that shape. A field's failure is wrapped in a `field:`-headed `Description`.
  Without `..`, every field must be listed (a missing field is a compile error
  from the generated exhaustiveness check); an unknown field and a misplaced `..`
  are also compile errors.
- `test-better`: the facade crate re-exports `matches_struct!`, `matches_tuple!`,
  and `matches_variant!`; the prelude gains them. The macros' generated code
  refers to `::test_better`, so they are usable through the facade only.
- `test-better-matchers`: the `define_matcher!` declarative macro, the shortcut
  for the common custom-matcher case. It takes a name, optional constructor
  parameters, a target type, an `expects:` description, and a `matches:`
  predicate, and expands to a matcher type, its `Matcher` impl, and a
  constructor function. Anything richer (a structured diff, an inner matcher) is
  still written by hand as an `impl Matcher<T>`.
- `test-better`: the facade crate re-exports `define_matcher!`; the prelude
  gains it. A new `cookbook` module documents how to write custom matchers, both
  with `define_matcher!` and by hand.
- `examples/custom-matcher/`: a new workspace example crate, the runnable
  companion to the `cookbook` module: a `define_matcher!` matcher, a
  hand-written `impl Matcher<T>`, and a matcher that takes an inner matcher.
- `test-better-matchers`: soft assertions, `soft` and `SoftAsserter`. `soft`
  runs a closure in a scope where `SoftAsserter::expect` and
  `SoftAsserter::check` *record* failures instead of returning early; on scope
  exit `soft` returns `Ok(())` or a single `TestError` collecting every
  recorded failure under `Payload::Multiple`, each sub-failure keeping its own
  source location.
- `test-better`: the facade crate re-exports `soft` and `SoftAsserter`; the
  prelude gains `soft`.
- `test-better-matchers`: `SoftAsserter::context` and the `SoftScope` it
  returns. `context` opens a sub-scope whose `expect`/`check` attach an extra
  context frame to every recorded failure; the frame is removed when the
  `SoftScope` is dropped, and nested sub-scopes stack their frames
  outermost-first.
- `test-better`: the facade crate re-exports `SoftScope`.
- `test-better-matchers`: `soft` is now panic-safe. The closure runs under
  `catch_unwind`; if it panics, any failures recorded before the panic are
  written to stderr and the panic is re-raised unchanged, so a panic inside a
  soft scope still fails the test as a panic rather than being swallowed.
- `test-better-matchers`: `Subject::resolves_to`, the async counterpart of
  `to`. When the expression handed to `expect!` is a `Future`,
  `expect!(fut).resolves_to(matcher).await?` awaits it and matches its output.
  The method is `#[track_caller]` and returns a future (an `async fn` cannot be
  `#[track_caller]`), so the failure location is the call site, not the await
  point. It is runtime-agnostic: it only awaits, so it works under
  `#[tokio::test]`, `pollster::block_on`, and any other executor.
- `test-better-core`: `TestError::with_location`, a builder that overrides the
  captured location. It backs the async `expect!` methods, which capture the
  caller's location synchronously and thread it through once the awaited
  assertion has a result.
- `test-better-async`: the runtime-agnostic timeout layer. `run_within` awaits
  a future under a time limit and returns `Elapsed` if it overruns; the
  `RuntimeAvailable` marker trait gates it on a runtime feature. Concrete
  sleep backends are provided behind the `tokio`, `async-std`, and `smol`
  features.
- `test-better-matchers`: `Subject::to_complete_within`, the async `expect!`
  method that fails a test when a future overruns a `Duration`. It needs a
  runtime feature (`tokio`, `async-std`, or `smol`); with none enabled the
  call is a compile error naming those flags. Like `resolves_to` it is
  `#[track_caller]` and returns a future, so the failure points at the call
  site.
- `test-better`, `test-better-matchers`: `tokio`, `async-std`, and `smol`
  features, each forwarding down to `test-better-async`'s, plus re-exports of
  `Elapsed` and `RuntimeAvailable`.
- `test-better-async`: `eventually` and `eventually_blocking`, the polling
  helpers that retry a `bool`-returning probe until it passes or a `Duration`
  deadline is reached, replacing `sleep + assert` flakiness. They sleep between
  probes on an exponential `Backoff` schedule (configurable via the
  `eventually_with` / `eventually_blocking_with` variants), and the failure
  reports how long they waited and how many times they probed. `eventually` is
  async and runtime-gated like `to_complete_within`; `eventually_blocking`
  sleeps with `std::thread::sleep` and needs no runtime feature.
- `test-better`, `test-better-matchers`: re-exports of `eventually`,
  `eventually_blocking`, `eventually_with`, `eventually_blocking_with`, and
  `Backoff`. The prelude gains the two common-path functions, `eventually` and
  `eventually_blocking`.
- `test-better-property`: the property-testing bridge crate. The `Strategy<T>`
  seam (with `ValueTree<T>`, `Runner`, `GenError`, and `ProptestTree`) is a
  deliberately small trait the runner is written against; `proptest` satisfies
  it through a blanket impl, so a property test names ordinary `proptest`
  strategies with no `proptest` import at the call site.
- `test-better-property`: the `check` runner. `check` (and `check_with`)
  generate cases from a `Strategy<T>`, run a `T -> TestResult` predicate, and on
  the first failure drive the `ValueTree` shrink protocol to a minimal
  counterexample, returning a `PropertyFailure<T>` that carries the original and
  shrunk inputs, the matcher failure, and the case count. `Config` sets the case
  count (256 by default, matching `proptest`); `check` is deterministic by
  default, `check_with` exposes an explicit `Runner`.
- `test-better`: the facade crate re-exports the property-testing surface
  (`check`, `check_with`, `Strategy`, `ValueTree`, `Runner`, `GenError`,
  `ProptestTree`, `PropertyFailure`, and `Config` renamed `PropertyConfig`), so
  a property test needs only the facade dependency.
- `test-better-property`, `test-better`: the best-effort `quickcheck` bridge,
  behind a new off-by-default `quickcheck` feature. `arbitrary::<T>()` turns any
  `quickcheck::Arbitrary` type into a seam `Strategy<T>` (`ArbitraryStrategy<T>`,
  with `QuickcheckTree<T>` adapting `quickcheck`'s linear `shrink` to the
  `simplify`/`complicate` protocol), so a property test can name
  `arbitrary::<MyType>()` wherever it would name a `proptest` strategy. The
  facade forwards the feature and re-exports `arbitrary`, `ArbitraryStrategy`,
  and `QuickcheckTree`.
- `test-better-property`, `test-better`: the `property!` macro, the test-facing
  front for property testing. It takes a closure with a typed binding and a
  block body returning `TestResult`, infers a `Strategy` from the binding's type
  (via the new `any::<T>()` strategy constructor) or takes one explicitly with a
  trailing `using` clause, runs it through `check`, and renders a counterexample
  as a `TestError`. It expands to an expression, so `property!(...)` is the body
  of an ordinary `#[test]` function. The facade re-exports `property!` and
  `any`, and the prelude gains `property!`.
- `test-better-property`: a failing property now renders the original failing
  input, the shrunk minimal input, and the matcher's own structured
  description. `render_failure` keeps the matcher failure whole and wraps three
  context frames (the case count, the original input, the shrunk input) around
  it, promoting the kind to `ErrorKind::Property`. A golden-file test
  (`tests/shrink_output.rs`, with the golden under `tests/golden/`) pins the
  exact output.
- `test-better-snapshot`: the file-backed snapshot store. `assert_snapshot`
  (and the directory-explicit `assert_snapshot_in`) compares a value against
  `tests/snapshots/<module-path>__<name>.snap`, or rewrites it under
  `SnapshotMode::Update` / `UPDATE_SNAPSHOTS=1`. `snapshot_path` exposes the
  path-naming rule and `SnapshotFailure` is the structured outcome (missing
  file, mismatch, or I/O error). The crate is `std`-only.
- `test-better-matchers`: `Subject::to_match_snapshot(name)` asserts a
  `Display` value against its file-backed snapshot. A mismatch renders as an
  `ErrorKind::Snapshot` failure with an expected/actual payload and (with the
  `diff` feature on) a line-oriented diff; a missing snapshot points the reader
  at `UPDATE_SNAPSHOTS=1`. `expect!` now also captures `module_path!()` to name
  the snapshot file, so `Subject::new` takes a third argument.
- `test-better`: the facade crate re-exports the snapshot surface
  (`assert_snapshot`, `assert_snapshot_in`, `snapshot_path`, `SnapshotMode`,
  `SnapshotFailure`); `to_match_snapshot` rides along on the re-exported
  `Subject`.
- `test-better-snapshot`: inline snapshots, where the snapshot literal lives in
  the test source. `normalize_inline_literal` undoes the cosmetic indentation
  of a literal, `assert_inline_snapshot` compares against it, and on a mismatch
  under `UPDATE_SNAPSHOTS=1` records a *pending patch* under
  `target/test-better-pending/` (`pending_patch_dir`, `parse_pending_patch`).
  Behind the new `accept` feature, the `test-better-accept` companion binary
  reads those patches and rewrites the literals in place: `apply_inline_patch`,
  `apply_patches_from`, and `apply_pending_patches`, with `Applied` and
  `AcceptError` as their result types.
- `test-better-matchers`: `Subject::to_match_inline_snapshot(literal)` asserts a
  `Display` value against a snapshot literal in the test source, capturing the
  call site via `#[track_caller]`. A mismatch renders as an
  `ErrorKind::Snapshot` failure with an expected/actual payload and diff,
  pointing the reader at `UPDATE_SNAPSHOTS=1`.
- `test-better`: the facade crate re-exports the inline-snapshot surface
  (`InlineLocation`, `InlineSnapshotFailure`, `assert_inline_snapshot`,
  `normalize_inline_literal`, `parse_pending_patch`, `pending_patch_dir`);
  `to_match_inline_snapshot` rides along on the re-exported `Subject`.
- `test-better-snapshot`: `Redactions`, an ordered set of text rewrites applied
  to a value before it is compared against (or stored as) a snapshot. Built-in
  rules `redact_uuids` and `redact_rfc3339_timestamps` stabilize the two most
  common sources of run-to-run noise; `replace` handles a known literal and
  `redact_with` is the escape hatch. The built-ins are hand-written scanners,
  so the crate stays `std`-only.
- `test-better-matchers`: `Subject::to_match_snapshot_with` and
  `to_match_inline_snapshot_with`, the redaction-aware variants of the snapshot
  methods. They run a `Redactions` set over the value before the comparison, so
  the placeholder (not the noise) is what is stored and matched against.
- `test-better`: the facade crate re-exports `Redactions`; the `*_with` snapshot
  methods ride along on the re-exported `Subject`.
- `test-better-macros`: the `#[test_case]` attribute. Stacking
  `#[test_case(args... ; "label")]` lines on a `fn` generates one `#[test]` per
  case, gathered into a module named for the function (so a case is addressable
  as `the_fn::the_label`, or `the_fn::case_N` when unlabeled). For a
  value-returning test each generated case wraps the call in failure context
  carrying the label and the rendered arguments. Other attributes on the
  function (`#[ignore]`, doc comments) are forwarded onto every generated test.
- `test-better`: the facade crate re-exports `test_case` at its root.
- `test-better-core`: `Trace` and `TraceEntry`, for in-test breadcrumbs.
  `Trace::new()` opens a scope; `step` and `kv` record narrative steps and
  key/value pairs into a thread-local. Every `TestError` built while the trace
  is in scope snapshots the breadcrumbs, and the rendered failure shows them in
  chronological order. `TestError` gains a public `trace: Vec<TraceEntry>`
  field, and `StructuredError` a matching one, so tooling sees the trail too.
- `test-better`: the facade crate re-exports `Trace` and `TraceEntry` at its
  root.
- `test-better-macros`: the `#[fixture]` and `#[test_with_fixtures]` attribute
  pair. `#[fixture]` turns a `fn() -> TestResult<T>` into a fixture whose
  failures are re-categorized as `ErrorKind::Setup` (with a context frame naming
  the fixture), so a setup problem never reads as an assertion miss. Fixtures
  are per-test by default; `#[fixture(scope = "module")]` runs the body once via
  a `LazyLock` and hands every test a clone (`T: Clone + Send + Sync + 'static`).
  `#[test_with_fixtures]` rewrites a parameterized test into a zero-argument
  `#[test]` that resolves each parameter `name: T` by calling the same-named
  fixture `fn name()` and `?`-propagating it, left to right.
- `test-better-core`: `TestError::with_kind`, which overrides an error's kind,
  consuming and returning `self`. The `#[fixture]` macro uses it to re-stamp a
  fixture failure as `ErrorKind::Setup`.
- `test-better`: the facade crate re-exports `fixture` and `test_with_fixtures`
  at its root and, unlike `test_case`, in the prelude.
- `test-better-runner`: the `cargo-test-better` binary, invoked as
  `cargo test-better`. It wraps `cargo test`, dropping the cargo-injected
  `test-better` subcommand argument, forwarding every other argument verbatim,
  inheriting stdio, and propagating the exit code (a signal death maps to
  `101`, as cargo itself does). The library exposes `cargo_test_command` and
  `run`, plus `RUNNER_ENV` and `STRUCTURED_MARKER`: the names of the
  environment variable and output sentinel of the structured-output channel
  the runner will consume later.
- `test-better-core`: the emitting side of the structured-output channel. When
  `RUNNER_ENV` is set in the environment (the runner sets it), `TestError`'s
  `Debug` appends one `STRUCTURED_MARKER`-wrapped JSON line carrying the
  `StructuredError`, after the human-readable render. `RUNNER_ENV` and
  `STRUCTURED_MARKER` are now defined here and re-exported through the facade;
  the JSON payload needs the `serde` feature, which now also pulls in
  `serde_json`.
- `test-better-runner`: `run` now pipes the wrapped `cargo test`'s stdout,
  tees every non-marker line through unchanged, and groups the structured
  failures it finds by their top context frame, printing a `GroupedReport`
  after the build exits. A failure with no marker line (a plain `panic!`, or
  non-`test-better` code) is listed ungrouped and labelled "unstructured",
  never parsed. New public surface: `scan_output`, `GroupedReport`,
  `ContextGroup`, `StructuredFailure`.
- `test-better-runner`: `run` now prints a one-line summary table after the
  wrapped build (passed/failed/ignored counts, plus the wall-clock duration the
  runner measures itself), and shows a live `running: done/total` counter on
  stderr while the build runs, gated on stderr being a TTY. New public surface:
  `RunSummary`, `ProgressEvent`, `progress_event`, and a `summary` field on
  `GroupedReport`.
- Public API review: `#[must_use]` was added to the matcher constructors (`eq`,
  `contains`, `all_of`, ...), the property `Strategy` constructor `any`, and
  `test-better-runner`'s `scan_output`: each returns a pure value that is a bug
  to discard. No items were added or removed.
- Documentation: the prose guide is now an mdBook under `book/`, with an
  Introduction and eight chapters (Getting Started, migrating from the stock
  assertion macros, Writing Matchers, Async Testing, Property Testing,
  Snapshots, Fixtures, and Recipes). A `book` CI job builds it with mdBook on
  every push. `README.md` gained a usage example and a Documentation section,
  and the facade crate's landing rustdoc was expanded with a runnable example.
- Examples: four worked examples join `examples/custom-matcher`, each a runnable
  workspace crate with a dogfooded test suite: `web-handler-tests` (testing
  request handlers with structural matchers and `soft`), `state-machine`
  (transition functions and `matches_variant!`), `property-roundtrip` (a
  `property!` roundtrip with an inferred strategy), and `snapshot-html`
  (inline-snapshotting a small HTML renderer).
- Benchmarks: `crates/test-better/benches/expect_overhead.rs`, a
  `harness = false` benchmark that times `expect!` against the stock assert
  macros on a hot loop. For a passing primitive matcher, `expect!` stays within
  an order of magnitude of `assert_eq!` (a single-digit-nanosecond per-call
  cost). The book gains a "Performance" chapter writing up the result.

### Notes

- `TestError` carries a dedicated `message: Option<Cow<'static, str>>` field:
  the message answers *what* failed, context frames answer *while doing what*.
  See the type's rustdoc for rationale.
- `clippy.toml` gained `allow-panic-in-tests = true`, completing the
  "allowed in tests" intent of the workspace lints (the initial setup added
  only the unwrap/expect equivalents).
- The async-`Subject` design is resolved: a single `Subject<T>` type, with the
  `await`-based methods added to the same impl block under method-level
  `where T: Future` bounds. A blanket `impl<T> Subject<T>` and an overlapping
  `impl<F: Future> Subject<F>` cannot coexist as inherent impls, and a separate
  `AsyncSubject<F>` would force `expect!` to dispatch on whether its argument
  is a future, which a syntactic macro cannot do.
- `TestError::payload` is `Option<Box<Payload>>` rather than `Option<Payload>`.
  `TestError` is returned by value through every `?`, so it is kept small; the
  large `Payload::ExpectedActual` variant lives behind the box. The public
  `Payload` enum and `with_payload` signature are unchanged.
- `Sequence` is *not* implemented for lazy iterators.
  A blanket `impl<I: Iterator> Sequence for I` overlaps, under coherence, with
  the concrete `impl Sequence for Vec<T>` (and the other collections), so the
  two cannot coexist. `Sequence` borrows its items through `&self`, which a
  lazy iterator cannot offer anyway. The dedicated adapter is `items(iter)`,
  which collects into a `Vec<T>` up front and wraps it in `Items<T>`; callers
  who already have a `Vec` can pass it directly.
- Dogfood switchover: every test in the workspace now uses
  `expect!` and `TestResult` instead of `assert!`/`assert_eq!`/`.unwrap()`/
  `.expect()`, enforced by `scripts/check-test-api.sh` (a new `dogfood` CI job)
  scanning `crates/*/src/`. No public API changed. Two implementation notes:
  `test-better-core` carries `test-better-matchers` as a dev-dependency (a
  dev-dependency cycle, which Cargo permits) so its own tests can use `expect!`;
  and because that cycle compiles `test-better-core` twice, its inline tests
  bridge a matcher result into the test's `TestResult` with a trailing
  `.or_fail()?` rather than a bare `?`. Tests in dependent crates and in
  `tests/` directories use the plain `expect!(..).to(..)?` form.
- The structural matcher macros (`matches_struct!`, `matches_tuple!`, `matches_variant!`) cannot name a struct's field
  types, and an impl's generic parameters cannot be unified with concrete types
  from a destructure. The generated code therefore uses a projection closure
  whose `Fn(&S) -> (&F0, &F1, ...)` type pins both the subject type and the
  field types; the closure is threaded through a generated generic constructor
  function carrying the `Fn` bound, which forces the higher-ranked inference a
  bare struct field would not get. This is the googletest-style projection
  pattern. The macros are re-exported through the `test-better` facade only, since their
  output names `::test_better` and a proc-macro crate's output can only name the
  consumer's direct dependencies. Compile-fail behavior is pinned by `trybuild`
  tests in `crates/test-better/tests/ui/`.
- `scripts/check-test-api.sh` now matches `.expect("` (the panic call followed
  by its string-literal message) rather than a bare `.expect(`. The soft
  assertion API names a method `SoftAsserter::expect`, and a
  bare `.expect(` could not tell `s.expect(&actual, matcher)` apart from
  `Result::expect("...")`. A non-test `.expect` with a non-literal message is
  still denied by the workspace's `clippy::expect_used` lint.
- The async `expect!` acceptance tests cover `pollster` and
  `tokio` but not `async-std`. `async-std` is unmaintained as of 2025;
  `resolves_to` only awaits the future and touches no runtime API, so two
  unrelated executors already demonstrate that it is runtime-agnostic. A
  `trybuild` test
  (`tests/ui/sync_to_on_future.rs`) locks that the sync `to` cannot be pointed
  at a future-typed subject with an output matcher: that path must go through
  `resolves_to`.
- `to_complete_within` is an inherent `Subject` method, kept in
  the same impl block as `resolves_to`. To make that
  possible without `test-better-matchers` taking on optional runtime
  dependencies, the timeout machinery lives one layer down in
  `test-better-async` (which now depends only on `test-better-core`), and
  `test-better-matchers` depends on it. The dependency edges are
  `matchers -> async -> core`; `test-better-async` carries `test-better-matchers`
  as a dev-dependency for dogfooding, the same permitted dev-cycle as
  `test-better-core`.
- The "no runtime feature is a compile error" requirement is met with a
  deferred trait bound, not a literal `compile_error!`. A `compile_error!` in a
  function body fires
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
  diagnostic.
- `eventually` is a free function in `test-better-async`, not a
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
  `trybuild` case for the gated `eventually`.
- `Backoff`'s `initial`/`ceiling`/`factor` knobs are exposed through the
  `eventually_with` / `eventually_blocking_with` variants rather than widening
  the two-argument `eventually` signature. The plain `eventually` /
  `eventually_blocking` use `Backoff::default` (1ms
  initial, doubling, 100ms ceiling). Only the two default-schedule functions are
  in the prelude; `Backoff` and the `_with` variants are imported by name, in
  keeping with the deliberately small prelude.
- `proptest` is the property-testing backend for the initial release: it ships
  integrated shrinking, the feature the `ValueTree` protocol is built on. It
  is depended on with `default-features = false, features = ["std"]`, which
  drops the `fork` / `timeout` machinery (and `rusty-fork` / `libc` /
  `wait-timeout`) the bridge does not use; `cargo deny` stays clean.
- The `Strategy<T>` seam has one coherence limitation: a user type that is
  itself a `proptest::strategy::Strategy` is already covered by the blanket
  impl, so it cannot also carry a hand-written `Strategy<T>` impl. This is
  accepted: the seam exists so the runner does not name `proptest` directly,
  not so users reimplement strategies.
- `check` runs deterministically by default (`Runner::deterministic`) so a
  property test does not flake from run to run: the same strategy and predicate
  pass or fail identically every time. `check_with` exposes `Runner::randomized`
  for callers who want fresh entropy per run. An over-filtered strategy that
  cannot produce a value is skipped, not counted as a property failure.
- `Config` is re-exported from the facade as `PropertyConfig`: at the facade
  root, where one crate's surface meets eight others, a bare `Config` says too
  little.
- `property!` is an *expression* macro that expands to a
  `TestResult`, not an item macro that generates a `#[test] fn`. An item macro
  fed `property!(|s: String| { ... })` with no test name could not turn it into
  a named function;
  an expression macro composes with `?` and is the body of a hand-named
  `#[test] fn`, which keeps test naming and attributes in the user's hands.
  `property!` routes through a `#[doc(hidden)]` `run_property` helper rather
  than re-exporting `check`'s rendering, so the shrunk-failure rendering can be
  enriched without touching the macro. The strategy-inference
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
  golden-file test pins the rendered output; it builds the
  `PropertyFailure` by hand so the golden file is deterministic and not coupled
  to the backend's RNG, and normalizes the environment-specific `  at` line.
  Regenerate it with `BLESS_GOLDEN=1`.
- The `quickcheck` bridge ships at documented reduced fidelity
  rather than being deferred. Two limitations are inherent to `quickcheck`'s
  model, not
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
- `test-better-snapshot` is deliberately `std`-only and does
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
- Inline snapshots are split in two, mirroring `insta`: a proc
  macro cannot rewrite the file it expands from, so the runtime half
  (`to_match_inline_snapshot`, `std`-only) only *records* a pending patch on a
  mismatch under `UPDATE_SNAPSHOTS=1`, and the separate `test-better-accept`
  binary applies those patches afterwards. The accept step is the one place
  that needs `syn`, so it (and the binary) sit behind a non-default `accept`
  feature, keeping the crate dependency-free for ordinary test runs. A patch is
  written as its own line-oriented file under `target/test-better-pending/`
  (named `<pid>-<seq>.patch`) so parallel tests never contend on a shared file.
  `assert_inline_snapshot` drops a single trailing newline from the value before
  comparing: literal normalization trims trailing whitespace, so a value
  rendered with a trailing newline could otherwise never match its accepted
  literal. The accept step's literal placement comes from `syn`'s span
  information (`proc-macro2`'s `span-locations`), so everything outside the
  rewritten literal stays byte-for-byte unchanged; it is covered end-to-end by
  `test-better-snapshot/tests/accept.rs` against a scratch fixture file. As with
  file-backed snapshots, the facade `tests/snapshot.rs` asserts only the
  matching inline case.
- Redactions are deliberately closure-based rather than
  regex-based: `Redactions` holds an ordered `Vec` of boxed `Fn(&str) -> String`
  rules, and the built-in `redact_uuids` / `redact_rfc3339_timestamps` are
  hand-written byte scanners. The UUID and RFC 3339 grammars are rigid enough to
  scan directly, and writing them by hand keeps `test-better-snapshot`
  dependency-free in its default build (it gained `syn` only behind the
  non-default `accept` feature; pulling in `regex` for redaction would
  have undone the std-only property for everyone). A regex-backed
  `redact_regex` could be added later behind a feature if the built-ins and the
  `redact_with` escape hatch prove too limited. Redaction is applied at the
  matcher boundary (`Subject::to_match_snapshot_with` and the inline variant
  redact the value, then delegate to the unchanged `assert_snapshot` /
  `assert_inline_snapshot`), so the storage-and-comparison core never had to
  learn about redactions; `to_match_snapshot` is now `to_match_snapshot_with`
  with an empty `Redactions`.
- `#[test_case]` is kept *out* of `test_better::prelude`.
  `std`'s own prelude exports a `test_case` attribute
  (the unstable custom-test-frameworks one), and two glob imports of the same
  name are ambiguous at the use site. `test_case` lives at the facade root and
  is imported by name: `use test_better::test_case;`. The macro is implemented
  so the topmost `#[test_case]` does all the work: it parses its own arguments,
  then drains the function's remaining attributes, splitting further
  `#[test_case]` lines from attributes to forward (`#[ignore]`, doc comments).
  Generated tests are `pub(super)` so a deliberately failing case can be marked
  `#[ignore]` and then driven directly by path from an ordinary `#[test]`, which
  is how `crates/test-better/tests/test_case.rs` exercises the failure-context
  path without failing the suite.
- `Trace` is captured through a `thread_local!`, not a true
  task-local: `std` has no task-local, and `cargo test` runs each test on its
  own thread, so a thread-local is per-test in practice. The honest limitation,
  documented on the type, is async: a runtime that migrates a task across
  threads can have a later `TestError` snapshot the wrong thread's (usually
  empty) trace. `TestError::at` (the single internal constructor all the
  `#[track_caller]` constructors funnel through) takes the snapshot, so every
  error path picks the trace up automatically. Adding the `trace` field to the
  all-public-fields `TestError` and `StructuredError` structs is a pre-1.0
  source break for any code that built them with an exhaustive struct literal;
  inside the workspace only the internal `TestError::at` did, and it was
  updated.
- The `#[fixture]` system is two cooperating attribute macros
  rather than a single `#[fixture]`: a
  `#[test_with_fixtures]` is needed because a plain `#[test]` cannot take
  parameters, so the parameter-to-fixture wiring needs its own attribute. Both
  go in the prelude (they collide with nothing in `std`, unlike `#[test_case]`).
  The convention is name-based: a parameter `db: Db` is filled by `fn db()`, so
  the fixture function name *is* the dependency name. Per-test scope moves the
  value straight out, so `T` need not be `Clone`; module scope caches in a
  `LazyLock` and clones, and because a cached `Err` cannot be moved out, the
  module-scope error path synthesizes a fresh `Setup` error carrying the
  original's rendered text rather than the original `TestError` itself. Fixture
  compile-fail behavior (a `#[fixture]` with parameters, an unknown `scope`) is
  pinned by `trybuild` tests in `crates/test-better/tests/ui/`.
- The runner's structured-output channel is marker-wrapped JSON
  in the test's own captured output, not a side-channel file under `target/`.
  Captured stdout is the one byte stream every test harness already routes
  through `cargo test`, so the runner reads it without coordinating on a file
  path that the binary, the harness, and the runner would all have to agree
  on. The exit-code-parity acceptance is
  pinned by two fixture crates under
  `crates/test-better-runner/tests/fixtures/`; each carries an empty
  `[workspace]` table so it is its own workspace root (out of the parent
  workspace) and has no dependencies, so the nested `cargo test` the parity
  test drives builds in a moment. The runner inherits stdio, so the nested
  `cargo test` against the has-failures fixture prints its own `FAILED` lines
  into the parent run's output; that noise is expected, and the parent suite
  still exits `0`.
- The runner wires the emitting and consuming ends of the structured-output channel and no longer inherits stdout: it pipes stdout so the marker lines can
  be picked out, teeing every other line through unchanged (stderr is still
  inherited). The marker is emitted from `TestError`'s `Debug`, so it rides
  along whenever the stock harness prints a returned `Err` (or any other
  `{:?}` of a `TestError`) while `RUNNER_ENV` is set; an ordinary `cargo test`
  never sets that variable, so its output is byte-for-byte unchanged. Grouping
  uses the *top* context frame (`context[0]`, the first one attached), so a
  test names its feature area with a single outer `.context(..)`. The
  end-to-end acceptance is pinned by a third fixture workspace,
  `structured-failures`, which depends on `test-better-core` (built with
  `serde`) and fails in two context areas plus one plain `panic!`.
- The runner's summary counts are read from libtest's own `test result:`
  lines, not from the structured channel: passing and ignored tests emit no
  marker, so their counts have no other source. This is not a violation of the
  "never parse rendered failure text" rule, which is about not re-parsing
  `test-better`'s *own* renderer; the `test result:` line is libtest's stable
  output. The run duration is measured by the runner around the child process
  rather than summed from the per-binary `finished in` values, which would
  double-count tests that ran in parallel. The summary-counts acceptance is
  pinned by a fourth fixture workspace, `mixed-results` (three passing, two
  failing, one ignored, dependency-free). The live progress counter's TTY path
  is not covered by an automated test (it requires a pseudo-terminal); its pure
  pieces, `progress_event` and `parse_result_line`, are unit-tested instead.
- The public API review found no internal-but-public items to hide:
  `run_property`/`render_failure` were already `#[doc(hidden)]`, the `Float`
  trait is already sealed, and `Subject::new` stays public-but-documented as
  the `expect!` entry point. The only changes were the `#[must_use]`
  attributes noted above.
- The worked examples are workspace member crates with `#[cfg(test)]`
  suites, not `examples/*.rs` target files, matching the existing
  `examples/custom-matcher` layout. They are therefore exercised by
  `cargo test --workspace` (the CI `test` job) rather than
  `cargo test --examples`, which only runs `[[example]]` targets. A member crate
  gives each example its own
  `Cargo.toml`, dependency set, and a real test suite a reader can run with
  `cargo test -p <name>-example`.
- The benchmark uses `harness = false`: a plain `fn main` timed with
  `std::time::Instant`, not a `criterion` (or other framework) bench. This
  keeps the dependency tree empty of a benchmark framework (`criterion` interop
  is deferred to a later release) and lets the bench build and
  run on stable. A side effect is that `cargo test` runs the bench's `main`, so
  the loop count is kept modest (10M/loop) to stay fast in the suite. The
  measured ratios are machine-dependent; the chapter and CHANGELOG state the
  order-of-magnitude bound, not a fixed number.

[Unreleased]: https://github.com/joshburgess/test-better/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/joshburgess/test-better/releases/tag/v0.1.0
