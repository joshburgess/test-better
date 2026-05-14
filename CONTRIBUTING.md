# Contributing to test-better

Thanks for your interest in `test-better`. This project is built iteratively
against a canonical plan, and contributions are expected to fit that structure.

## Read the plan first

[`PROJECT_BUILD_PLAN.md`](./PROJECT_BUILD_PLAN.md) is the source of truth. It
defines:

- the **mission** and **design principles** (§1) that every change must respect;
- the **phases** and **iteration cycles** (§4 onward) that work is decomposed into;
- the **iteration cycle workflow** and **Definition of Done** (§16).

Before opening a PR, find the iteration cycle it belongs to. If your change does
not map to one, it probably belongs in [`BACKLOG.md`](./BACKLOG.md) first, so it
can be turned into a proper cycle.

## Design principles (the short version)

These are non-negotiable; see §1 of the plan for the full text.

1. `?` is the control-flow operator of tests. Fallible operations return
   `Result<T, TestError>`.
2. Context is never erased: failures carry a human-readable context chain and a
   source location.
3. Assertions are values (`Matcher<T>`), not statements.
4. No required runtime: works with the stock `cargo test` harness.
5. Runtime-agnostic async: no `tokio`/`async-std` in core crates.
6. Dogfood relentlessly: from Phase 2 on, the library tests itself with itself.
7. Stability over surface area: small, orthogonal public API.
8. **Zero panic in user code paths.** No `.unwrap()`, `.expect()`, or `panic!`
   in any crate's `src/`. This is enforced by `[workspace.lints]` in `Cargo.toml`
   plus `clippy.toml`, which allow them only in test code.

## Local checks

Run the full local check before opening a PR (PROJECT_BUILD_PLAN.md §16 step 4):

```sh
cargo fmt --all --check
cargo clippy --workspace --all-features --all-targets -- -D warnings
cargo test --workspace --all-features
cargo doc --workspace --no-deps
```

`cargo deny check` is also run in CI; install it with `cargo install cargo-deny`
to run it locally.

### Formatting note

`rustfmt.toml` pins `max_width = 100`, which stable `cargo fmt` enforces. The
import-grouping style ("group imports by crate") relies on nightly-only rustfmt
options that are kept commented in `rustfmt.toml`; apply them with
`cargo +nightly fmt` if you want them locally. CI only enforces the stable
subset.

## Definition of Done (every cycle)

A PR is not done until all of the following hold (PROJECT_BUILD_PLAN.md §16):

- [ ] All acceptance criteria for the iteration cycle are met.
- [ ] No new `unwrap`/`expect`/`panic` in `src/` (CI-enforced).
- [ ] Public items are documented; doc-tests where useful.
- [ ] Tests are written using the library itself (Phase 2 onward).
- [ ] `CHANGELOG.md` is updated under `## [Unreleased]` for every public API change.
- [ ] CI is green across the full matrix.

## Commits and PRs

- Use [Conventional Commits](https://www.conventionalcommits.org/).
- Title PRs `phase-N.M: <short description>` and reference the iteration cycle.
- Include the acceptance test commands in the PR description, and a snippet of
  failure-message output where the change is user-facing.
- PRs are squash-merged after CI is green and review is approved.

## Licensing

By contributing, you agree that your contributions are dual-licensed under the
[MIT](./LICENSE-MIT) and [Apache-2.0](./LICENSE-APACHE) licenses, consistent with
the rest of the project.
