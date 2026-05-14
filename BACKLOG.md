# Backlog

This is the parking lot for ideas that are **out of scope for now**, plus a
record of design decisions and their rationale. Entries here are not
commitments.

## Resolved design decisions

These were genuine forks in the road. Recording the decision and why it was
made keeps the reasoning available the next time the same area is touched.

- **Async `Subject` design: RESOLVED.** There is a single `Subject<T>` type.
  The `await`-based methods are added to the existing `impl<T> Subject<T>` block
  with method-level `where T: Future` bounds and distinct names (e.g.
  `to_complete_within`). Rationale: a blanket `impl<T> Subject<T>` and an
  overlapping `impl<F: Future> Subject<F>` cannot coexist as inherent impls, and
  a distinct `AsyncSubject<F>` would need `expect!` to dispatch on whether its
  argument is a future, which a syntactic macro cannot do.
- **Primary property-testing backend: RESOLVED.** The backend is `proptest`.
  Rationale: its integrated shrinking (a `Strategy` produces a `ValueTree` that
  binary-searches toward a minimal counterexample) composes directly with
  `test-better`'s structured matcher output, and it is the de-facto standard for
  new Rust property testing. The `Strategy<T>` seam in `test-better-property` is
  a deliberately lowest-common-denominator trait (`new_tree` plus a `ValueTree`
  with `current`/`simplify`/`complicate`) that `proptest` satisfies through a
  blanket impl, so a property test names ordinary `proptest` strategies.
- **`quickcheck` bridge: RESOLVED.** The `quickcheck` bridge ships at documented
  reduced fidelity, behind an off-by-default `quickcheck` feature:
  `arbitrary::<T>()` turns a `quickcheck::Arbitrary` type into a seam
  `Strategy<T>`, and `QuickcheckTree` maps `quickcheck`'s linear
  `Arbitrary::shrink` iterator onto the `simplify`/`complicate` protocol. Two
  limitations are inherent to `quickcheck`'s model and documented on the
  `quickcheck_bridge` module: a `quickcheck::Gen` cannot be seeded from the
  seam's `Runner` (so `arbitrary()` strategies are not reproducible via
  `Runner::deterministic`), and shrinking does not promise the exact boundary
  value `proptest`'s integrated shrinking reaches.
- **Snapshot file naming: RESOLVED.** The `<test_module>` component of a
  snapshot file name comes from the call site's `module_path!()`, captured by
  the `expect!` macro and carried on `Subject` (a third argument to
  `Subject::new`), rather than being derived at runtime from
  `Location::caller().file()`. `module_path!()` disambiguates two tests in
  different modules that pick the same snapshot name, and `Subject::new` had
  exactly one caller (the macro), so widening it was low-risk. The snapshot
  *directory* (`tests/snapshots`) is resolved from the working directory, which
  `cargo test` sets to the package root.
- **Inline snapshots are a method, not a macro: RESOLVED.**
  `to_match_inline_snapshot` is a `#[track_caller]` method on `Subject`: the rest
  of `Subject` is method-shaped, so a macro would buy nothing but inconsistency.
  The runtime cannot rewrite its own source, so a mismatch under
  `UPDATE_SNAPSHOTS=1` only records a pending patch; the `test-better-accept`
  binary applies them later. The accept step lives in the library (`accept`
  module) rather than only in `src/bin`, so it is callable from an integration
  test against a fixture; the binary is a thin shell. `syn` is confined to the
  non-default `accept` feature so the crate stays dependency-free for ordinary
  runs. Pending patches are one-file-per-patch (`<pid>-<seq>.patch`) to avoid
  concurrent-append contention between parallel tests.
- **Redactions are closure-based: RESOLVED.** `Redactions` is an ordered `Vec`
  of boxed `Fn(&str) -> String` rules, with hand-written byte scanners behind
  the built-in `redact_uuids` and `redact_rfc3339_timestamps`. The UUID and
  RFC 3339 grammars are rigid enough to scan by hand, and doing so keeps
  `test-better-snapshot` dependency-free in its default build, preserving the
  std-only property `regex` would have broken for every user. Redaction is
  applied at the matcher boundary (`to_match_snapshot_with` /
  `to_match_inline_snapshot_with` redact the value, then delegate to the
  unchanged assert functions), so the comparison core stayed untouched. A
  regex-backed `redact_regex` behind a feature is the obvious next step if the
  built-ins plus the `redact_with` escape hatch are not enough; deferred until
  there is a concrete need.
- **Runner structured-output channel: RESOLVED.** The structured-output channel
  the runner consumes is **marker-wrapped JSON in the test's own captured
  output**, not a side-channel file under `target/`. The runner exports
  `TEST_BETTER_RUNNER=1` (`test_better_runner::RUNNER_ENV`) into the `cargo
  test` it spawns; when that is set, a failing `test-better` test prints one
  line bracketed by `test_better_runner::STRUCTURED_MARKER` carrying the
  serde-serialized `StructuredError`. Rationale: `cargo test` already captures
  test output and replays it for *failing* tests, which is exactly when the
  runner needs it, so there is no side-channel file to create, clean up, or
  detect staleness on, and no `--nocapture` requirement. A side-channel file
  would also have to survive contention from parallel test binaries and threads.
  A failure with no marker line (a plain `panic!`, non-`test-better` code) is
  shown ungrouped and labelled "unstructured"; the runner never parses prose.

## Ideas

- **Iterator adapter for `Sequence`.** `Sequence` cannot be implemented for lazy
  iterators: a blanket `impl<I: Iterator>` would collide, under coherence, with
  the concrete collection impls, and `Sequence` borrows its items through
  `&self` anyway. Today the caller collects an iterator into a `Vec` before
  `expect!`. A dedicated newtype (e.g. `items(iter)` returning a
  `Sequence`-implementing wrapper that eagerly collects on construction) would
  let `expect!(items(some_iter))` work without a visible `.collect()`. Low
  priority; the `.collect()` workaround is fine.
- **`redact_regex` behind a feature.** A regex-backed redaction rule, gated so
  the default `test-better-snapshot` build stays dependency-free. Deferred until
  the built-in scanners plus `redact_with` prove insufficient.
