# Backlog

This is the parking lot for ideas and decisions surfaced mid-iteration that are
**out of scope for the current cycle**. Per PROJECT_BUILD_PLAN.md §16 step 3,
scope creep goes here instead of into the open PR.

Entries are not commitments. When an item is picked up, move it into the build
plan as an iteration cycle (or a new phase) and delete it from this file.

## Decisions to record here when made

These are explicitly deferred decision points called out by the build plan;
record the decision and its rationale here when the relevant phase begins.

- **Phase 2 (§7.3):** the async-`Subject` design (distinct `AsyncSubject<F>` vs.
  a separately-named `Future`-bounded method). Phase 5 inherits whatever is
  chosen here.
- **Phase 6 (§11.1):** the primary property-testing backend (recommended:
  `proptest`) and whether the `quickcheck` bridge ships in 1.0 or is deferred.
- **Phase 9 (§9.1):** the structured-output channel the runner consumes
  (marker-wrapped JSON in captured output vs. a side-channel file under
  `target/`). Phases 7.2 and 9.2 both depend on this.

## Ideas

_(none yet)_
