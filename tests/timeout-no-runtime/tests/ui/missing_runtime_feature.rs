//! `to_complete_within` needs a runtime feature (`tokio`, `async-std`, or
//! `smol`) on `test-better`. With none enabled, the `RuntimeAvailable` bound
//! is unsatisfied and the call does not compile.

use std::time::Duration;

use test_better::prelude::*;

fn main() {
    let _ = expect!(async {}).to_complete_within(Duration::from_secs(1));
}
