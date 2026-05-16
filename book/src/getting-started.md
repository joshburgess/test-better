# Getting Started

## Add the dependency

`test-better` is a dev-dependency: it is only used by your tests.

```toml
[dev-dependencies]
test-better = "0.2"
```

That single crate is a facade: it re-exports the whole library, so a test file
needs one dependency and one import.

## The one import

```rust
use test_better::prelude::*;
```

The prelude brings in everything an everyday test uses: the `TestResult` type,
the `check!` macro, the matcher constructors (`eq`, `lt`, `contains`, ...),
and the `?`-friendly extension methods (`context`, `or_fail`). Less common
items (the custom-matcher machinery, the structured-failure types) are imported
by name when you need them, so they stay out of the body of every test.

## Your first test

A `test-better` test returns [`TestResult`], which is an alias for
`Result<(), TestError>`. The body uses `?` on each assertion and ends with
`Ok(())`:

```rust
use test_better::prelude::*;

fn parse_port(input: &str) -> Option<u16> {
    input.parse().ok()
}

#[test]
fn parses_a_valid_port() -> TestResult {
    let port = parse_port("8080").or_fail_with("8080 is a valid port")?;
    check!(port).satisfies(eq(8080))?;
    Ok(())
}
```

Three things are happening:

- `or_fail_with` replaces `.unwrap()`. On `None` it produces a `TestError`
  whose message is the string you gave it; the `?` returns it.
- `check!(port)` captures both the value *and the source text* `port`, so a
  failure names the expression.
- `.satisfies(eq(8080))` returns a `TestResult`. The `?` propagates a mismatch; on a
  match it is `Ok(())` and execution continues.

The trailing `Ok(())` is the test passing. If the last line of the test is
itself an assertion, you can return it directly and drop the `Ok(())`:

```rust
use test_better::prelude::*;
# fn parse_port(input: &str) -> Option<u16> { input.parse().ok() }

#[test]
fn parses_a_valid_port() -> TestResult {
    let port = parse_port("8080").or_fail_with("8080 is a valid port")?;
    check!(port).satisfies(eq(8080))
}
```

## What a failure looks like

When `check!(port).satisfies(eq(8080))` fails, the test does not panic with
`assertion failed: left == right`. It returns a `TestError` that renders the
expression, what was expected, and what was found:

```text
assertion failed

  check!(port).satisfies(eq(8080))
  expected: equal to 8080
  actual: 9090

  at tests/config.rs:11:5
```

If you attached context with `.context(..)` on the way down, that chain is
printed too. The next chapter, [Migrating from `assert!`](./migrating-from-assert.md),
covers the rest of the everyday vocabulary.

## Negation and multiple matchers

`violates` is the negation of `satisfies`:

```rust
use test_better::prelude::*;

#[test]
fn a_fresh_cart_is_empty_and_has_no_total() -> TestResult {
    let cart: Vec<u32> = Vec::new();
    check!(&cart).satisfies(is_empty())?;
    check!(cart.iter().sum::<u32>()).violates(gt(0))?;
    Ok(())
}
```

To assert several things about one value in a single `check!`, combine
matchers with `all_of` (see [Recipes](./recipes.md)); to keep going after the
first failure and report *all* of them, use `soft` (also in Recipes).

[`TestResult`]: https://docs.rs/test-better/latest/test_better/type.TestResult.html
