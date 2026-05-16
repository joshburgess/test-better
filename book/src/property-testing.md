# Property Testing

A property test asserts that something holds for *every* input in a range,
rather than for a handful of hand-picked cases. `test-better`'s property layer
is a thin seam over [`proptest`]: you write the property as a closure that
returns `TestResult`, and a failure is shrunk to a minimal counterexample that
still carries the matcher failure that broke it.

## The `property!` macro

The everyday form is the `property!` macro. The closure binding's type names
the strategy: any type that is `proptest::Arbitrary` (most std types are) is
inferred from the annotation.

```rust
use test_better::prelude::*;

#[test]
fn incrementing_changes_the_value() -> TestResult {
    property!(|n: u32| {
        check!(n.wrapping_add(1)).satisfies(ne(n))
    })
}
```

The macro call *is* the test body: it returns the `TestResult` the `#[test]`
function returns.

To name an explicit strategy instead of inferring one, add a `using` clause.
The binding is then bare; its type and values come from the strategy. A numeric
range is a `proptest` strategy, so it works directly:

```rust
use test_better::prelude::*;

#[test]
fn values_in_range_stay_in_range() -> TestResult {
    property!(|n| {
        check!(n).satisfies(lt(10u64))
    } using 0u64..10)
}
```

## Shrinking and counterexamples

When a property fails, `proptest` shrinks the failing input toward the simplest
value that still fails, and `test-better` reports both the shrunk and the
original input, alongside the matcher failure:

```rust
use test_better::prelude::*;

#[test]
fn this_property_is_false() -> TestResult {
    // "every value in 0..1000 is below 500" is false; the run shrinks the
    // counterexample down to exactly 500.
    let error = property!(|n: u32| {
        check!(n).satisfies(lt(500u32))
    } using 0u32..1_000)
    .err()
    .or_fail_with("values at or above 500 exist in 0..1000")?;

    let rendered = error.to_string();
    check!(rendered.contains("the shrunk (minimal) input is 500")).satisfies(is_true())?;
    check!(rendered.contains("less than 500")).satisfies(is_true())
}
```

The point of carrying the matcher failure through shrinking is that the report
is not just "500 failed": it is the full `check!` failure for the minimal
input, so you see *what* about `500` broke the property.

## The function form: `for_all` and `for_all_with`

`property!` expands to a call to `for_all`. You can call it directly when you
want the `Result<(), PropertyFailure<T>>` as a value rather than as the test's
return:

```rust
use test_better::prelude::*;
use test_better::for_all;

#[test]
fn doubling_stays_in_bounds() -> TestResult {
    let outcome = for_all(0u32..1_000, |n| check!(n * 2).satisfies(lt(2_000u32)));
    check!(outcome.is_ok()).satisfies(is_true())
}
```

`PropertyFailure<T>` exposes the `shrunk` and `original` inputs and the carried
`failure: TestError`, so a test can assert on the counterexample itself.

`for_all_with` takes a `PropertyConfig` (the case count) and a `Runner` (seeded
deterministically or randomized), for when the defaults are not what you want:

```rust
use test_better::prelude::*;
use test_better::{PropertyConfig, Runner, for_all_with};

#[test]
fn run_more_cases() -> TestResult {
    let mut runner = Runner::randomized();
    let outcome = for_all_with(PropertyConfig { cases: 32 }, &mut runner, 0u64..10, |n| {
        check!(n).satisfies(lt(10u64))
    });
    check!(outcome.is_ok()).satisfies(is_true())
}
```

## Custom strategies

A `Strategy<T>` describes how to generate and shrink values of `T`. Any
`proptest` strategy is a `test-better` `Strategy` through a blanket impl, so
`proptest`'s combinators (`prop_map`, `prop_filter`, tuples, collections) are
available with no wrapper. `any::<T>()` is the default strategy for a type, the
same one `property!` infers.

There is also an optional `quickcheck` bridge behind the `quickcheck` feature:
`arbitrary::<T>()` turns a `quickcheck::Arbitrary` type into a `Strategy<T>`.
`proptest` is the primary backend; reach for the bridge only when you already
have `quickcheck::Arbitrary` impls you want to reuse.

[`proptest`]: https://docs.rs/proptest
