# Contributing to test-better

Thanks for your interest in `test-better`. Bug reports, documentation fixes,
new matchers, and feature work are all welcome.

## Before you start

For anything larger than a bug fix or a doc tweak, open an issue first so the
design can be discussed before you write code.

## Design principles

These shape every change to the library:

1. `?` is the control-flow operator of tests. Fallible operations return
   `Result<T, TestError>`.
2. Context is never erased: failures carry a human-readable context chain and a
   source location.
3. Assertions are values (`Matcher<T>`), not statements.
4. No required runtime: works with the stock `cargo test` harness.
5. Runtime-agnostic async: no `tokio`/`async-std` in core crates.
6. Dogfood relentlessly: the library tests itself with itself.
7. Stability over surface area: a small, orthogonal public API.
8. **Zero panic in user code paths.** No `.unwrap()`, `.expect()`, or `panic!`
   in any crate's `src/`. This is enforced by `[workspace.lints]` in `Cargo.toml`
   plus `clippy.toml`, which allow them only in test code.

## Local checks

Run the full local check before opening a PR:

```sh
cargo fmt --all --check
cargo clippy --workspace --all-features --all-targets -- -D warnings
cargo test --workspace --all-features
cargo doc --workspace --no-deps
```

CI additionally runs `cargo deny check`, the dogfood scan
(`scripts/check-test-api.sh`), the public-API drift check
(`scripts/check-public-api.sh`), and an `mdbook build` of the guide. You can run
each locally; install `cargo-deny`, `cargo-public-api`, and `mdbook` with
`cargo install`.

### Formatting note

`rustfmt.toml` pins `max_width = 100`, which stable `cargo fmt` enforces. The
import-grouping style ("group imports by crate") relies on nightly-only rustfmt
options that are kept commented in `rustfmt.toml`; apply them with
`cargo +nightly fmt` if you want them locally. CI only enforces the stable
subset.

## Definition of Done

A PR is not done until all of the following hold:

- [ ] No new `unwrap`/`expect`/`panic` in `src/` (CI-enforced).
- [ ] Public items are documented; doc-tests where useful.
- [ ] Tests are written using the library itself.
- [ ] `CHANGELOG.md` is updated under `## [Unreleased]` for every public API
      change.
- [ ] The public-API snapshots under `public-api/` are regenerated if the
      surface changed (`scripts/check-public-api.sh --write`).
- [ ] CI is green across the full matrix.

## Commits and PRs

- Use [Conventional Commits](https://www.conventionalcommits.org/).
- Keep the PR description concrete: what changed, why, and the acceptance test
  commands. Include a snippet of failure-message output where the change is
  user-facing.
- PRs are squash-merged after CI is green and review is approved.

## Licensing

By contributing, you agree that your contributions are dual-licensed under the
[MIT](./LICENSE-MIT) and [Apache-2.0](./LICENSE-APACHE) licenses, consistent with
the rest of the project.
