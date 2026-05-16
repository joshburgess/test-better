# Async Testing

`test-better` tests an async value in three ways: by awaiting it and asserting
on its output, by polling a condition until it becomes true, and by bounding
how long an operation may take. The first is runtime-agnostic; the last two
have a runtime-free form and a runtime-gated form.

## Asserting on a future's output: `resolves_to`

When the expression handed to `check!` is a `Future`, the `Subject` grows an
`await`-based method, `resolves_to`. It awaits the future and applies the
matcher to its output:

```rust
use test_better::prelude::*;

async fn doubled(n: i32) -> i32 {
    n + n
}

#[tokio::test]
async fn doubling_resolves_to_the_sum() -> TestResult {
    check!(doubled(21)).resolves_to(eq(42)).await?;
    Ok(())
}
```

`resolves_to` only awaits the future, so it is runtime-agnostic: the same
assertion works under `#[tokio::test]`, `#[async_std::test]`,
`pollster::block_on`, or any other executor. A mismatch is reported the same
way `satisfies` reports one: the expression (`doubled(21)`) and the actual
output.

## Polling until a condition holds: `eventually`

Some conditions become true *after* an operation, not synchronously: a
background task finishes, a file appears, a queue drains. `eventually` polls a
probe until it passes or a timeout elapses.

The runtime-free form is `eventually_blocking`. It needs no executor, so it is
an ordinary `#[test]`:

```rust
use std::time::Duration;
use test_better::prelude::*;

#[test]
fn the_worker_drains_the_queue() -> TestResult {
    let queue = start_worker();
    eventually_blocking(Duration::from_secs(5), || queue.is_empty())?;
    Ok(())
}
```

The async form is `eventually`: its probe is a future, and it sleeps on the
runtime between attempts. It is gated on a runtime feature of `test-better`
(`tokio`, `async-std`, or `smol`) being enabled, so the inter-probe sleep has
an executor to run on:

```rust
use std::time::Duration;
use test_better::prelude::*;

#[tokio::test]
async fn the_endpoint_comes_up() -> TestResult {
    let server = spawn_server();
    eventually(Duration::from_secs(5), || async { server.health().await.is_ok() }).await?;
    Ok(())
}
```

Both forms return the moment the probe passes, rather than always waiting out
the budget, and both report the elapsed time and probe count on a timeout. The
`eventually_with` / `eventually_blocking_with` variants take a `Backoff` to
control the inter-probe delay.

## Bounding how long an operation may take: `completes_within`

`completes_within` asserts that a future finishes inside a time limit. It
needs a real runtime to drive the timeout, so it is gated on one of
`test-better`'s runtime features and is only callable inside that runtime's
test:

```rust
use std::time::Duration;
use test_better::prelude::*;

#[tokio::test]
async fn the_cache_lookup_is_fast() -> TestResult {
    check!(cache_lookup("key"))
        .completes_within(Duration::from_millis(50))
        .await?;
    Ok(())
}
```

If the future does not complete in time, the failure is an `ErrorKind::Timeout`
naming the limit. Because the three runtime features are mutually exclusive in
a single build, pick the one matching your test runtime in `Cargo.toml`:

```toml
[dev-dependencies]
test-better = { version = "0.2", features = ["tokio"] }
```

## Choosing the right tool

- The value is a future and you want to assert on its output: `resolves_to`.
- A condition becomes true asynchronously and you want to wait for it:
  `eventually` (or `eventually_blocking` with no runtime).
- An operation must finish within a deadline: `completes_within`.
