# Performance

The short version: `check!` is slower than `assert_eq!` per call, by a
single-digit multiple, and it does not matter.

## What the benchmark measures

`crates/test-better/benches/expect_overhead.rs` is a `harness = false`
benchmark: an ordinary `fn main` that times two hot loops with
`std::time::Instant` and prints a table. It compares a passing primitive
assertion written two ways:

- `assert_eq!(a, b)` and `assert!(a < b)`, the stock macros;
- `check!(a).satisfies(eq(b))` and `check!(a).satisfies(lt(b))`, the `test-better` form.

Run it with `cargo bench -p test-better --bench expect_overhead`. A typical
run on a developer laptop:

```text
check! overhead vs the stock assert macros (10000000 iters/loop)
matcher     assert (ns)    expect (ns)      ratio
eq                 0.57           4.02       7.1x
lt                 0.44           3.51       8.0x
```

The exact numbers move with the machine, but the shape holds: a passing
`check!` on a primitive matcher costs a few nanoseconds, a single-digit
multiple of the stock macro. That is comfortably **within an order of
magnitude** of `assert_eq!`.

## Where the overhead comes from

`assert_eq!` on two `u32`s compiles down to a compare and a branch. `check!`
does a little more on the passing path:

- it constructs a `Subject` wrapping a reference to the value;
- it constructs the matcher (`eq(b)` is a small value holding `b`);
- it calls `Matcher::check`, which returns `MatchResult::pass()`.

None of that allocates. The matcher's `Description`, the expected/actual
rendering, the source-location capture: those are built only on the *failure*
path, which a passing test never takes. So the per-call cost is a few struct
moves and a non-inlined call or two, not heap traffic.

## Why it does not matter

A few nanoseconds per assertion disappears next to anything a real test does.
Parsing a string, touching the filesystem, allocating a `Vec`, spawning an
async runtime: each is hundreds to millions of times more expensive than the
gap between `assert_eq!` and `check!`. A test suite's wall time is dominated
by its setup and its I/O, never by the assertion macro.

The one case where assertion cost could be visible is a property test running
the same `check!` across many thousands of generated inputs. Even there the
matcher call is dwarfed by the strategy's value generation and shrinking
machinery. If you ever do find an assertion in a genuine hot loop, the fix is
the same as it would be with `assert_eq!`: hoist it out of the loop, or assert
on the aggregate instead of each element.

`test-better` buys a great deal at that single-digit-nanosecond price: a
failure that is a value rather than a panic, the expression text, the
expected and actual sides, the source location, and the context chain. The
trade is heavily in your favor.
