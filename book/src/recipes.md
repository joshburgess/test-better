# Recipes

Shorter answers to common questions, each independent of the others.

## Assert several things about one value

`all_of` combines matchers: the value must satisfy every one. `any_of` is the
or-form. Both take a tuple of matchers:

```rust
use test_better::prelude::*;

#[test]
fn the_score_is_in_a_sensible_range() -> TestResult {
    let score = 73_u32;
    expect!(score).to(all_of((ge(0), le(100), ne(50))))?;
    Ok(())
}
```

## Keep going after the first failure: `soft`

A `?` on a failed `expect!` returns immediately, so a test stops at its first
failure. When you want to see *every* failure in one run (checking each field
of a response, say), `soft` collects them:

```rust
use test_better::prelude::*;

#[test]
fn every_field_is_checked() -> TestResult {
    soft(|s| {
        s.expect(&1, eq(1));
        s.expect(&"alice", eq("alice"));
        s.expect(&true, is_true());
    })
}
```

`soft` returns `Ok(())` if every soft assertion passed, or a single `TestError`
that renders all of them, each with its own source location. Inside the
closure, `s.expect(&value, matcher)` is the soft form of `expect!`, and
`s.context("...")` opens a labeled scope for the assertions that follow.

## Match the shape of a struct, tuple, or enum

The structural macros assert on shape without a custom matcher. Each field
position holds a matcher, and `..` ignores the rest:

```rust
use test_better::prelude::*;
use test_better::{matches_struct, matches_tuple, matches_variant};

# #[derive(Debug)]
# struct User { name: String, age: u32, email: String }
# #[derive(Debug)]
# struct Point(i32, i32);
# #[derive(Debug)]
# enum Shape { Circle { radius: f64 } }
#[test]
fn structural_matchers() -> TestResult {
    let user = User { name: "alice".into(), age: 30, email: "alice@example.com".into() };
    expect!(user).to(matches_struct!(User {
        name: eq(String::from("alice")),
        age: gt(18u32),
        ..
    }))?;

    expect!(Point(3, 4)).to(matches_tuple!(Point(gt(0), lt(100))))?;

    expect!(Shape::Circle { radius: 2.0 })
        .to(matches_variant!(Shape::Circle { radius: gt(0.0) }))?;
    Ok(())
}
```

On a failure, the message names the field or position that did not match. The
matchers nest: an inner `matches_struct!` is just another matcher expression.

## Assert on collections

`contains` takes a matcher and checks at least one element satisfies it;
`every` checks all of them; `have_len`, `is_empty`, and `is_not_empty` check
size. `contains_in_order` checks a subsequence:

```rust
use test_better::prelude::*;

#[test]
fn collection_matchers() -> TestResult {
    let scores = vec![10, 20, 30, 40];
    expect!(&scores).to(have_len(4))?;
    expect!(&scores).to(contains(eq(30)))?;
    expect!(&scores).to(every(gt(0)))?;
    expect!(&scores).to(contains_in_order([eq(10), eq(40)]))?;
    Ok(())
}
```

## Parameterized tests with `#[test_case]`

`#[test_case]` turns one function into many generated `#[test]`s, one per
attribute line. Each line is the argument list, optionally followed by
`; "label"`:

```rust
use test_better::prelude::*;
use test_better::test_case;

#[test_case(2, 2, 4)]
#[test_case(10, 5, 15 ; "bigger numbers")]
fn addition_works(a: i32, b: i32, sum: i32) -> TestResult {
    expect!(a + b).to(eq(sum))
}
```

The generated tests are gathered into a module named for the function, so the
second case above runs as `addition_works::bigger_numbers`. Import `test_case`
by name: it is deliberately kept out of the prelude because `std` exports an
attribute of the same name.

## Match a string

`contains_str`, `starts_with`, and `ends_with` are the substring matchers; with
the `regex` feature, `matches_regex` takes a pattern:

```rust
use test_better::prelude::*;

#[test]
fn string_matchers() -> TestResult {
    let greeting = "Hello, alice!";
    expect!(greeting).to(starts_with("Hello"))?;
    expect!(greeting).to(contains_str("alice"))?;
    expect!(greeting).to(ends_with("!"))?;
    Ok(())
}
```

## The `cargo test-better` runner

`test-better-runner` provides an optional `cargo-test-better` binary: a thin
wrapper around `cargo test` that groups failures by their context area and
prints a run summary. Install it and run it in place of `cargo test`:

```sh
cargo install test-better-runner
cargo test-better
```

It is opt-in tooling: your tests do not depend on it, and a plain `cargo test`
behaves exactly as before. The same crate's `cargo test-better accept`
subcommand applies the pending patches that inline snapshots record under
`UPDATE_SNAPSHOTS=1`.

## Control colored output

Failure rendering is colored when the output is a terminal. To force it on or
off (in CI logs, or when capturing output for a test), set the color choice:

```rust
use test_better::{ColorChoice, set_color_choice};

# fn main() {
set_color_choice(ColorChoice::Never);
# }
```

`ColorChoice` is `Always`, `Never`, or `Auto`; `color_choice()` reads the
current setting.

## Inspect a failure as data

For tooling, `TestError::to_structured()` produces a `StructuredError`: an
owned, `Clone`-able, `serde`-serializable (behind the `serde` feature) form of
the failure, with the kind, message, location, context chain, and payload. It
is what the `cargo-test-better` runner consumes; a test that needs to assert on
the *structure* of a failure rather than its rendered text can use it directly.
