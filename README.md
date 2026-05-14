# test-better

[![CI](https://github.com/joshburgess/test-better/actions/workflows/ci.yml/badge.svg)](https://github.com/joshburgess/test-better/actions/workflows/ci.yml)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](#license)

> Result-returning tests with `?`: composable matchers, rich failure output, and never a `.unwrap()` in sight.

`test-better` is a generic Rust testing library that makes `Result`-returning
tests with `?` strictly better than panicking tests. It replaces `.unwrap()` and
`.expect("...")` with composable, intention-revealing helpers that produce rich
failure output and compose cleanly across async, property, snapshot, and
parameterized tests.

```rust
use test_better::prelude::*;

#[test]
fn parses_a_valid_port() -> TestResult {
    let port = parse_port("8080").or_fail_with("8080 is a valid port")?;
    expect!(port).to(eq(8080))?;
    expect!(port).to_not(lt(1024))?;
    Ok(())
}
```

A failure renders the expression that failed, the expected and actual values,
the source location, and any context attached on the way down: no panic, no
`assert_eq!`, no backtrace through the test harness.

## Documentation

- The prose guide is the **`test-better` book** under [`book/`](./book/):
  Getting Started, Migrating from `assert!`, Writing Matchers, Async Testing,
  Property Testing, Snapshots, Fixtures, and Recipes. Build it with
  `mdbook build book`.
- The API reference is the rustdoc: `cargo doc --open -p test-better`.

## Status

In development. The canonical build plan lives in
[`PROJECT_BUILD_PLAN.md`](./PROJECT_BUILD_PLAN.md): it defines the mission,
design principles, and the phased iteration plan this repository is executed
against. The library surface (core, matchers, macros, async, property,
snapshot, runner) is implemented; the project is in Phase 10, the
documentation and release pass.

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
