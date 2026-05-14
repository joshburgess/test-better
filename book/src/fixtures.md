# Fixtures

A fixture is a named, reusable piece of test setup. Instead of repeating the
same "open a database, run migrations, insert a user" preamble in every test,
you write it once as a fixture and name it as a parameter of the tests that
need it.

The design goal is that **a fixture failure is setup, not an assertion miss**.
If the database will not open, the test that needed it fails with an
`ErrorKind::Setup` error naming the fixture, not a confusing assertion failure
deep in the body.

## Defining a fixture

A fixture is a `fn` returning `TestResult<T>`, marked `#[fixture]`:

```rust
use test_better::prelude::*;

#[fixture]
fn answer() -> TestResult<i32> {
    Ok(42)
}
```

The body does whatever setup is needed and returns the value (or an error,
which becomes the `Setup` failure). Real fixtures build connections, temp
directories, seeded data: anything a test would otherwise construct inline.

## Using fixtures in a test

A `#[test_with_fixtures]` test names fixtures as parameters. Each is resolved
before the body runs, and the resolved value is passed in:

```rust
use test_better::prelude::*;
# #[fixture]
# fn answer() -> TestResult<i32> { Ok(42) }

#[test_with_fixtures]
fn the_answer_reaches_the_test(answer: i32) -> TestResult {
    expect!(answer).to(eq(42))
}
```

The parameter name must match the fixture's function name; the parameter type
is the `T` the fixture produces. Several fixtures are resolved left to right:

```rust
use test_better::prelude::*;

#[fixture]
fn name() -> TestResult<String> {
    Ok(String::from("alice"))
}

#[fixture]
fn age() -> TestResult<u32> {
    Ok(30)
}

#[test_with_fixtures]
fn both_fixtures_are_available(name: String, age: u32) -> TestResult {
    expect!(name.len() as u32).to(le(age))
}
```

## Fixture scope

By default a fixture runs once *per test* that names it: each test gets its own
fresh value. For expensive setup that is safe to share, declare module scope,
and the body runs once and every test gets a clone:

```rust
use test_better::prelude::*;

#[fixture(scope = "module")]
fn shared_config() -> TestResult<String> {
    Ok(String::from("loaded-once"))
}

#[test_with_fixtures]
fn one_test_sees_the_config(shared_config: String) -> TestResult {
    expect!(shared_config.as_str()).to(eq("loaded-once"))
}

#[test_with_fixtures]
fn another_test_sees_the_same_config(shared_config: String) -> TestResult {
    expect!(shared_config.is_empty()).to(is_false())
}
```

Use per-test scope (the default) when tests must not see each other's mutations;
use module scope when the value is read-only and the setup is worth doing once.

## When a fixture fails

A fixture that returns `Err` (or whose `?` propagates one) makes every test
that depends on it fail with an `ErrorKind::Setup` error. The failure names the
fixture and preserves the original error's detail, so the report points at the
broken setup rather than at whatever assertion happened to run first:

```rust
use test_better::prelude::*;

#[fixture]
fn broken_db() -> TestResult<i32> {
    Err(TestError::custom("could not connect to the database"))
}
```

Any `#[test_with_fixtures]` test taking `broken_db` fails before its body runs,
and the failure is re-categorized as `Setup`: it renders "test setup failed",
names "setting up fixture `broken_db`", and still includes the original "could
not connect to the database" detail. In practice a fixture rarely constructs an
error by hand: it propagates a real one with `?`, using `.context(..)` or
`.or_fail_with(..)` exactly as a test body would. That separation, setup
failure versus assertion failure, is the whole point of the fixture system.
