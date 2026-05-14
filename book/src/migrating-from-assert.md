# Migrating from `assert!`

If you have an existing test suite, you do not have to rewrite it all at once.
`test-better` tests are ordinary `#[test]` functions; a `TestResult`-returning
test sits next to a panicking one in the same file. Convert a test when you
next touch it.

This chapter is the translation table.

## The shape of the function

A panicking test returns `()` and its assertions panic. A `test-better` test
returns `TestResult` and its assertions are `?`-propagated:

```rust
// Before
#[test]
fn before() {
    let user = load_user(1);
    assert_eq!(user.name, "alice");
}
```

```rust
use test_better::prelude::*;

// After
#[test]
fn after() -> TestResult {
    let user = load_user(1);
    expect!(user.name).to(eq("alice"))
}
```

## Assertion translation table

| Panicking                              | `test-better`                                  |
|-----------------------------------------|------------------------------------------------|
| `assert!(x)`                            | `expect!(x).to(is_true())?`                    |
| `assert!(!x)`                           | `expect!(x).to(is_false())?`                   |
| `assert_eq!(a, b)`                      | `expect!(a).to(eq(b))?`                        |
| `assert_ne!(a, b)`                      | `expect!(a).to(ne(b))?`                        |
| `assert!(a < b)`                        | `expect!(a).to(lt(b))?`                        |
| `assert!(a >= b)`                       | `expect!(a).to(ge(b))?`                        |
| `assert!(v.contains(&x))`               | `expect!(&v).to(contains(eq(x)))?`             |
| `assert!(v.is_empty())`                 | `expect!(&v).to(is_empty())?`                  |
| `assert!(s.contains("foo"))`            | `expect!(s).to(contains_str("foo"))?`          |
| `assert!(opt.is_some())`                | `expect!(opt).to(some(always_matches()))?` *   |
| `assert_eq!(opt, Some(x))`              | `expect!(opt).to(some(eq(x)))?`                |
| `assert!(res.is_ok())`                  | `expect!(res).to(ok(always_matches()))?` *     |
| `assert_eq!(res, Ok(x))`                | `expect!(res).to(ok(eq(x)))?`                  |

\* `some` and `ok` take an *inner* matcher for the contained value. To assert
only that the option or result is the right variant, pass `always_matches()`;
otherwise pass a matcher for the value you expect inside it.

## Replacing `.unwrap()` and `.expect()`

`.unwrap()` and `.expect("...")` panic. Their `?`-friendly replacements live on
the `OrFail` extension trait, in the prelude:

```rust
use test_better::prelude::*;
# fn config_path() -> Option<String> { Some("/etc/app.toml".into()) }
# fn read(_: &str) -> Result<String, std::io::Error> { Ok(String::new()) }

#[test]
fn loads_the_config() -> TestResult {
    // Before: let path = config_path().unwrap();
    let path = config_path().or_fail_with("a config path is configured")?;

    // Before: let body = read(&path).expect("config is readable");
    let body = read(&path).or_fail_with("the config file is readable")?;

    expect!(body.is_empty()).to(is_true())
}
```

- `or_fail()` uses a generic message; `or_fail_with("...")` lets you say what
  you expected. On a `Result` it preserves the underlying error as the cause,
  so the original error message is still in the output.
- Use these everywhere you would have reached for `.unwrap()` in test setup,
  not just on the value under test.

## Annotating *where* a failure happened: `context`

`.context("...")` (and `.with_context(|| ...)`, which builds its message only
on the failure path) attach a frame describing what the test was doing. They
work on any `Result` whose error implements `std::error::Error`, and on a
`TestResult` directly:

```rust
use test_better::prelude::*;
# fn open_db() -> Result<(), std::io::Error> { Ok(()) }
# fn run_migrations() -> Result<(), std::io::Error> { Ok(()) }

#[test]
fn the_database_is_ready() -> TestResult {
    open_db().context("opening the test database")?;
    run_migrations().context("running migrations")?;
    Ok(())
}
```

A failure inside `run_migrations` is reported "while running migrations", so
you do not have to reconstruct what step you were on from a line number.

## A pragmatic order of operations

1. Change the signature to `-> TestResult` and add `Ok(())` at the end.
2. Replace each `assert*!` with the `expect!` form from the table, `?` on each.
3. Replace `.unwrap()` / `.expect()` in the test's setup with `or_fail*`.
4. Add `.context(..)` where a bare failure would be ambiguous.

The result is a test that, when it fails, tells you what it was doing and what
it found, rather than just where the panic was caught.
