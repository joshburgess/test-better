//! The sync `satisfies` must not be usable to match a future against an
//! *output* matcher: `check!(fut).satisfies(eq(4))` would silently match
//! against the future value itself, never awaiting it. `eq(4)` is a
//! `Matcher<i32>`, not a `Matcher<{the future type}>`, so the call fails to
//! type-check. The async path is `check!(fut).resolves_to(eq(4)).await`.

use test_better::prelude::*;

fn main() {
    let _ = check!(async { 4 }).satisfies(eq(4));
}
