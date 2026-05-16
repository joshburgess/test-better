# Writing Matchers

The built-in matchers cover most assertions, but a test suite for a real domain
eventually wants its own vocabulary: `is_freezing()`, `is_a_valid_iban()`,
`settled()`. A custom matcher is reusable, composes with the combinators
(`not`, `all_of`, `some`, ...), and produces a failure message written in
domain terms rather than in raw field values.

There are two ways to write one. The runnable companion to this chapter is the
`examples/custom-matcher/` crate in the repository, and the `test_better::cookbook`
module in the rustdoc.

## Before writing one: check the built-ins

To assert on the *shape* of a struct, tuple, or enum variant, the structural
macros (`matches_struct!`, `matches_tuple!`, `matches_variant!`) compose
existing matchers and need no new type. To wrap an ad-hoc closure once, without
naming it, `satisfies` is lighter still:

```rust
use test_better::prelude::*;

#[test]
fn the_id_is_even() -> TestResult {
    let id = 4096_u32;
    check!(id).satisfies(satisfies("an even id", |n| n % 2 == 0))
}
```

Reach for a real matcher when the predicate is reused, or when the failure
message needs to be better than "did not satisfy ...".

## 1. `define_matcher!`: the declarative shortcut

When the matcher is a predicate plus a description and nothing more,
`define_matcher!` writes the matcher type, its `Matcher` impl, and the
constructor function for you:

```rust
use test_better::define_matcher;

define_matcher! {
    /// Matches a temperature, in degrees Celsius, at or below freezing.
    pub fn is_freezing for f64 {
        expects: "a temperature at or below 0°C",
        matches: |celsius| *celsius <= 0.0,
    }
}
```

The matcher can take parameters; the `expects` description can be computed from
them:

```rust
use test_better::define_matcher;

define_matcher! {
    /// Matches a temperature strictly warmer than `floor` degrees Celsius.
    pub fn warmer_than(floor: f64) for f64 {
        expects: format!("a temperature warmer than {floor}°C"),
        matches: |celsius| *celsius > floor,
    }
}
```

Both are used like any built-in matcher: `check!(reading).satisfies(is_freezing())`,
`check!(reading).satisfies(warmer_than(18.0))`. This is the right tool for the large
majority of cases.

## 2. A hand-written `impl Matcher<T>`: full control

When the shortcut is not enough (you want a structured diff, an inner matcher
applied to a projection, or a failure message phrased for the domain type),
implement `Matcher<T>` directly. The trait has two methods:

```rust
use test_better::{Description, MatchResult, Matcher, Mismatch};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Temperature(pub f64);

struct IsFreezingReading;

impl Matcher<Temperature> for IsFreezingReading {
    fn check(&self, actual: &Temperature) -> MatchResult {
        if actual.0 <= 0.0 {
            MatchResult::pass()
        } else {
            MatchResult::fail(Mismatch::new(
                self.description(),
                format!("{:.1}°C, which is above freezing", actual.0),
            ))
        }
    }

    fn description(&self) -> Description {
        Description::text("a temperature at or below 0°C")
    }
}

/// Matches a `Temperature` reading at or below freezing.
#[must_use]
pub fn is_freezing_reading() -> impl Matcher<Temperature> {
    IsFreezingReading
}
```

- `check` returns `MatchResult::pass()` or `MatchResult::fail(mismatch)`. The
  `Mismatch` carries the `Description` of what was expected and a string for
  what was actually found.
- `description` returns the matcher's expectation. It is what `not` negates and
  what combinators compose, so keep it a noun phrase ("a temperature at or
  below 0°C"), not a sentence.

The convention is to keep the matcher type private and expose a constructor
function. Mark the constructor `#[must_use]`: a matcher that is built and
dropped is a bug.

## 3. A matcher that adapts an inner matcher

The most composable shape takes an inner `Matcher<U>` and applies it to a
projection of `T`. This lets every numeric matcher (`gt`, `between`,
`close_to`, ...) work on your domain type without a dedicated matcher for each:

```rust
use test_better::{Description, MatchResult, Matcher, Mismatch};
# #[derive(Debug, Clone, Copy, PartialEq)]
# pub struct Temperature(pub f64);

struct AsCelsius<M>(M);

impl<M: Matcher<f64>> Matcher<Temperature> for AsCelsius<M> {
    fn check(&self, actual: &Temperature) -> MatchResult {
        let inner = self.0.check(&actual.0);
        match inner.failure {
            None => MatchResult::pass(),
            Some(mismatch) => MatchResult::fail(Mismatch {
                expected: Description::labeled("degrees Celsius", mismatch.expected),
                ..mismatch
            }),
        }
    }

    fn description(&self) -> Description {
        Description::labeled("degrees Celsius", self.0.description())
    }
}

/// Applies `inner` to the underlying degrees-Celsius value of a `Temperature`.
pub fn as_celsius<M: Matcher<f64>>(inner: M) -> impl Matcher<Temperature> {
    AsCelsius(inner)
}
```

`Description::labeled` wraps the inner description with a header, so a nested
failure keeps the layer that failed: the output shows `degrees Celsius` and,
underneath it, whatever the inner matcher expected.

## Describing expectations

`Description` is the composable account of what a matcher expects:

- `Description::text("...")` is a leaf.
- `Description::labeled(header, child)` nests a description under a header.
- `a.and(b)` / `a.or(b)` combine two descriptions; `!d` negates one.

Building the description out of these, rather than formatting a string, is what
lets `not`, `all_of`, and `any_of` produce a sensible message when they wrap
your matcher.
