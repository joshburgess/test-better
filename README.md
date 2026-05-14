# test-better

[![CI](https://github.com/joshburgess/test-better/actions/workflows/ci.yml/badge.svg)](https://github.com/joshburgess/test-better/actions/workflows/ci.yml)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](#license)

> Result-returning tests with `?`: composable matchers, rich failure output, and never a `.unwrap()` in sight.

`test-better` is a generic Rust testing library that makes `Result`-returning
tests with `?` strictly better than panicking tests. It replaces `.unwrap()` and
`.expect("...")` with composable, intention-revealing helpers that produce rich
failure output and compose cleanly across async, property, snapshot, and
parameterized tests.

## Status

Early development. The canonical build plan lives in
[`PROJECT_BUILD_PLAN.md`](./PROJECT_BUILD_PLAN.md): it defines the mission,
design principles, and the phased iteration plan this repository is executed
against.

This repository is currently at **Phase 0 (Foundation & Scaffolding)**.

## Workspace layout

| Crate | Purpose |
|-------|---------|
| `test-better` | Facade crate: re-exports and `prelude`. |
| `test-better-core` | `TestError`, `TestResult`, `ContextExt`, `OrFail`. |
| `test-better-matchers` | `Matcher` trait, standard matchers, `expect!`. |
| `test-better-macros` | Procedural macros. |
| `test-better-async` | Async and timing helpers (runtime-gated). |
| `test-better-property` | Property-testing bridge. |
| `test-better-snapshot` | Snapshot testing. |
| `test-better-runner` | Optional `cargo-test-better` pretty runner. |

## License

Dual-licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](./LICENSE-APACHE))
- MIT license ([LICENSE-MIT](./LICENSE-MIT))

at your option.

## Contributing

See [CONTRIBUTING.md](./CONTRIBUTING.md). All work is organized around the
phases and iteration cycles in [`PROJECT_BUILD_PLAN.md`](./PROJECT_BUILD_PLAN.md).
Out-of-scope ideas surfaced mid-iteration go in [BACKLOG.md](./BACKLOG.md).
