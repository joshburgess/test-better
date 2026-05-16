# Introduction

`test-better` is a Rust testing library built around one idea: a test that
returns `Result` and uses `?` is strictly better than a test that panics.

A panicking test stops at the first failure, throws away everything it knew
about *why* it failed, and gives you a backtrace through the test harness
instead of a description of what went wrong. A `Result`-returning test keeps
the failure as a value: it carries the expression that failed, the values
involved, the source location, and any context you attached on the way down.

```rust
use test_better::prelude::*;

#[test]
fn the_answer_is_right() -> TestResult {
    let answer = compute_answer();
    check!(answer).satisfies(eq(42))?;
    Ok(())
}
```

When that assertion fails, the message names the *expression* (`answer`), not
just its value, and the comparison it expected. There is no `.unwrap()`, no
`assert_eq!`, and no panic: the `?` turns the failure into an early return that
the test harness reports.

## What you get

- **`check!` and matchers.** `check!(value).satisfies(matcher)` is the single
  assertion form. Matchers (`eq`, `lt`, `contains`, `some`, ...) compose with
  combinators (`not`, `all_of`, `any_of`) and you can write your own.
- **`?`-friendly conversions.** `or_fail` replaces `.unwrap()`; `context`
  annotates a failure with where you were in the test when it happened.
- **Rich failure output.** Failures render the expression, the expected and
  actual values, a diff for multi-line text, the source location, and the
  context chain.
- **One surface across test kinds.** Async assertions, property tests,
  snapshot tests, and fixture-driven tests all return the same `TestResult`
  and compose with the same `?`.

## How this book is organized

[Getting Started](./getting-started.md) gets a test file compiling. [Migrating
from `assert!`](./migrating-from-assert.md) is the translation table if you
have an existing suite. The remaining chapters each take one area in depth:
[writing your own matchers](./writing-matchers.md), [async](./async-testing.md),
[property testing](./property-testing.md), [snapshots](./snapshots.md), and
[fixtures](./fixtures.md). [Recipes](./recipes.md) collects shorter answers to
common questions.

The full API reference is the [rustdoc]; this book is the prose companion to
it.

[rustdoc]: https://docs.rs/test-better
