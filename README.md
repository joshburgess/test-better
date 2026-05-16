# test-better

[![CI](https://github.com/joshburgess/test-better/actions/workflows/ci.yml/badge.svg)](https://github.com/joshburgess/test-better/actions/workflows/ci.yml)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](#license)

> Result-returning tests with `?`: composable matchers, rich failure output, and never a `.unwrap()` in sight.

`test-better` is a testing library for Rust that treats `?` as the control-flow
operator of tests. Instead of panicking with `.unwrap()`, `.expect("...")`, and
`assert_eq!`, you write tests that return `Result` and use composable,
intention-revealing matchers. When something fails you get the expression that
failed, the expected and actual values, the source location, and the full
context chain, all rendered as a value rather than a panic.

It works with the stock `cargo test` harness (no runtime required), stays
runtime-agnostic for async code, and grows from primitive assertions up through
async, property, snapshot, and parameterized testing without changing how a
test is shaped.

## Quick start

Add the facade crate to your dev-dependencies:

```toml
[dev-dependencies]
test-better = "0.2"
```

Write a test that returns `TestResult` and reach for `?`:

```rust
use test_better::prelude::*;

#[test]
fn parses_a_valid_port() -> TestResult {
    let port = parse_port("8080").or_fail_with("8080 is a valid port")?;
    check!(port).satisfies(eq(8080))?;
    check!(port).violates(lt(1024))?;
    Ok(())
}
```

When `check!` fails, the message names the expression, both sides of the
comparison, and where it happened, with no backtrace through the harness:

```text
check failed: check!(port).satisfies(eq(8080))
  expected: 8080
  actual:   8000
  at tests/config.rs:12
```

## Why `?` instead of `panic!`

A panicking assertion throws away everything except a message. A
`Result`-returning test keeps the context:

- **Failures are values.** A `Matcher<T>` is a value you can pass around,
  negate, and combine, not a statement that aborts the thread.
- **Context is never erased.** Attach a human-readable note with
  `.context("...")` or `.or_fail_with("...")` and it travels with the error.
- **`?` composes.** Setup that can fail, the assertion itself, and teardown all
  use the same operator. No nested `match`, no `.unwrap()` to "just get past"
  the setup.
- **No required runtime.** Tests run under plain `cargo test`. A prettier
  grouped-output runner is available but never mandatory.

## What's in the box

```rust
use test_better::prelude::*;

// Composable matchers over any type
check!(name).satisfies(eq("Ada"))?;
check!(items).satisfies(contains(eq(3)))?;
check!(result).satisfies(ok(eq(8080)))?;

// Structural matching on structs and enums
check!(user).satisfies(matches_struct!(User { active: true, .. }))?;
check!(event).satisfies(matches_variant!(Event::Click { .. }))?;

// Soft assertions: collect several failures, report them together
soft(|s| {
    s.check(&a, eq(1));
    s.check(&b, eq(2));
})?;
```

Async, property, and snapshot testing are layered on the same `check!`/`?`
shape:

```rust
// Async timing assertions, runtime-agnostic
check!(fetch_user(id)).completes_within(Duration::from_millis(50)).await?;

// Property testing over generated inputs
property!(|xs: Vec<i64>| {
    check!(decode(&encode(&xs))).satisfies(eq(Ok(xs)))
})?;

// Inline and file snapshots, with redactions
check!(render_page(&ctx)).matches_inline_snapshot(r#"<h1>Hello</h1>"#)?;
```

## Documentation

- **The [`test-better` book](https://joshburgess.github.io/test-better/)** is
  the prose guide: Getting Started, Migrating from `assert!`, Writing Matchers,
  Async Testing, Property Testing, Snapshots, Fixtures, Performance, and
  Recipes. The sources live under [`book/`](./book/); build a local copy with
  `mdbook build book`.
- **The API reference** is the rustdoc: `cargo doc --open -p test-better`.
- **Runnable examples** live in [`examples/`](./examples/), each a small crate
  with its own test suite: `cargo test -p web-handler-tests-example` and
  friends.

## Workspace layout

`test-better` is the facade you depend on; it re-exports everything through its
`prelude`. The functionality is split across focused crates so you only compile
what you use.

| Crate | Purpose |
|-------|---------|
| `test-better` | Facade crate: re-exports and `prelude`. |
| `test-better-core` | `TestError`, `TestResult`, `ContextExt`, `OrFail`. |
| `test-better-matchers` | `Matcher` trait, standard matchers, `check!`. |
| `test-better-macros` | Procedural macros (`matches_struct!`, `#[test_case]`, fixtures). |
| `test-better-async` | Async and timing helpers (runtime-gated). |
| `test-better-property` | Property-testing bridge (`proptest`-backed). |
| `test-better-snapshot` | Snapshot testing, inline and file-based. |
| `test-better-runner` | Optional `cargo-test-better` pretty runner. |

## Contributing

Bug reports, documentation fixes, new matchers, and feature work are all
welcome. See [CONTRIBUTING.md](./CONTRIBUTING.md) for the design principles,
local check commands, and the definition of done.

## License

Dual-licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](./LICENSE-APACHE))
- MIT license ([LICENSE-MIT](./LICENSE-MIT))

at your option.
