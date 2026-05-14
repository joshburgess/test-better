//! The sync `to` must not be usable to match a future against an *output*
//! matcher: `expect!(fut).to(eq(4))` would silently match against the future
//! value itself, never awaiting it. `eq(4)` is a `Matcher<i32>`, not a
//! `Matcher<{the future type}>`, so the call fails to type-check. The async
//! path is `expect!(fut).resolves_to(eq(4)).await`.

use test_better::prelude::*;

fn main() {
    let _ = expect!(async { 4 }).to(eq(4));
}
