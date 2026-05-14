//! `eventually` needs a runtime feature (`tokio`, `async-std`, or `smol`) on
//! `test-better` for its inter-probe sleep. With none enabled, the probe
//! closure's `RuntimeAvailable` bound is unsatisfied and the call does not
//! compile. The runtime-free `eventually_blocking` is the escape hatch and
//! carries no such bound.

use std::time::Duration;

use test_better::prelude::*;

fn main() {
    let _ = eventually(Duration::from_secs(1), || async { true });
}
