# Backlog

This is the parking lot for ideas and decisions surfaced mid-iteration that are
**out of scope for the current cycle**. Per PROJECT_BUILD_PLAN.md §16 step 3,
scope creep goes here instead of into the open PR.

Entries are not commitments. When an item is picked up, move it into the build
plan as an iteration cycle (or a new phase) and delete it from this file.

## Decisions to record here when made

These are explicitly deferred decision points called out by the build plan;
record the decision and its rationale here when the relevant phase begins.

- **Phase 2 (§7.3): RESOLVED (Iteration 2.3).** The async-`Subject` design is
  option (b): a single `Subject<T>` type. Phase 5 adds `await`-based methods to
  the existing `impl<T> Subject<T>` block with method-level `where T: Future`
  bounds and distinct names (e.g. `to_complete_within`). Rationale: a blanket
  `impl<T> Subject<T>` and an overlapping `impl<F: Future> Subject<F>` cannot
  coexist as inherent impls, and option (a)'s distinct `AsyncSubject<F>` would
  need `expect!` to dispatch on whether its argument is a future, which a
  syntactic macro cannot do. Phase 5 inherits this decision.
- **Phase 6 (§11.1): RESOLVED (Iteration 6.1a).** The primary property-testing
  backend is `proptest`. Rationale: its integrated shrinking (a `Strategy`
  produces a `ValueTree` that binary-searches toward a minimal counterexample)
  composes directly with `test-better`'s structured matcher output, and it is
  the de-facto standard for new Rust property testing. The `Strategy<T>` seam
  in `test-better-property` is a deliberately lowest-common-denominator trait
  (`new_tree` plus a `ValueTree` with `current`/`simplify`/`complicate`) that
  `proptest` satisfies through a blanket impl, so a property test names
  ordinary `proptest` strategies.
- **Phase 6 (§11 6.1c): RESOLVED (Iteration 6.1c).** The `quickcheck` bridge
  ships in 1.0 at documented reduced fidelity, behind an off-by-default
  `quickcheck` feature: `arbitrary::<T>()` turns a `quickcheck::Arbitrary` type
  into a seam `Strategy<T>`, and `QuickcheckTree` maps `quickcheck`'s linear
  `Arbitrary::shrink` iterator onto the `simplify`/`complicate` protocol. Two
  limitations are inherent to `quickcheck`'s model and documented on the
  `quickcheck_bridge` module: a `quickcheck::Gen` cannot be seeded from the
  seam's `Runner` (so `arbitrary()` strategies are not reproducible via
  `Runner::deterministic`), and shrinking does not promise the exact boundary
  value `proptest`'s integrated shrinking reaches. This is the "works at the
  documented reduced fidelity" outcome §11 6.1c lists as acceptable.
- **Phase 7 (§12 7.1): RESOLVED (Iteration 7.1).** The `<test_module>`
  component of a snapshot file name comes from the call site's `module_path!()`,
  captured by the `expect!` macro and carried on `Subject` (a third argument to
  `Subject::new`), rather than being derived at runtime from
  `Location::caller().file()`. `module_path!()` is the value the
  PROJECT_BUILD_PLAN.md path template literally names, it disambiguates two
  tests in different modules that pick the same snapshot name, and `Subject::new`
  had exactly one caller (the macro), so widening it was low-risk. The snapshot
  *directory* (`tests/snapshots`) is resolved from the working directory, which
  `cargo test` sets to the package root.
- **Phase 7 (§12 7.2): RESOLVED (Iteration 7.2).** `to_match_inline_snapshot` is
  a `#[track_caller]` method on `Subject`, not a macro: the build plan's shown
  API is method-shaped and the rest of `Subject` already is, so a macro would
  buy nothing but inconsistency. The runtime cannot rewrite its own source, so
  a mismatch under `UPDATE_SNAPSHOTS=1` only records a pending patch; the
  `test-better-accept` binary applies them later. The accept step lives in the
  library (`accept` module) rather than only in `src/bin`, so it is callable
  from an integration test against a fixture; the binary is a thin shell. `syn`
  is confined to the non-default `accept` feature so the crate stays
  dependency-free for ordinary runs. Pending patches are one-file-per-patch
  (`<pid>-<seq>.patch`) to avoid concurrent-append contention between parallel
  tests.
- **Phase 7 (§12 7.3): RESOLVED (Iteration 7.3).** Redactions are closure-based,
  not regex-based: `Redactions` is an ordered `Vec` of boxed
  `Fn(&str) -> String` rules, with hand-written byte scanners behind the
  built-in `redact_uuids` and `redact_rfc3339_timestamps`. The UUID and RFC 3339
  grammars are rigid enough to scan by hand, and doing so keeps
  `test-better-snapshot` dependency-free in its default build, preserving the
  std-only property `regex` would have broken for every user. Redaction is
  applied at the matcher boundary (`to_match_snapshot_with` /
  `to_match_inline_snapshot_with` redact the value, then delegate to the
  unchanged assert functions), so the comparison core stayed untouched. A
  regex-backed `redact_regex` behind a feature is the obvious next step if the
  built-ins plus the `redact_with` escape hatch are not enough; deferred until
  there is a concrete need.
- **Phase 9 (§9.1):** the structured-output channel the runner consumes
  (marker-wrapped JSON in captured output vs. a side-channel file under
  `target/`). Phase 9.2 depends on this. (Iteration 7.2 turned out not to: the
  inline-snapshot accept step uses its own per-patch files under `target/`,
  not the runner's structured-output channel.)

## Ideas

- **Iterator adapter for `Sequence` (surfaced in Iteration 3.3).** `Sequence`
  cannot be implemented for lazy iterators: a blanket `impl<I: Iterator>` would
  collide, under coherence, with the concrete collection impls, and `Sequence`
  borrows its items through `&self` anyway. Today the caller collects an
  iterator into a `Vec` before `expect!`. A dedicated newtype (e.g.
  `items(iter)` returning a `Sequence`-implementing wrapper that eagerly
  collects on construction) would let `expect!(items(some_iter))` work without
  a visible `.collect()`. Low priority; the `.collect()` workaround is fine.
