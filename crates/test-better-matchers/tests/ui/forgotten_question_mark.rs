//! Dropping the `?` on an `check!(..)` leaves the returned `Result` unused.
//! With `unused_must_use` denied, that becomes a hard error trybuild observes,
//! which proves the warning fires in ordinary builds too.
#![deny(unused_must_use)]

use test_better_matchers::{eq, check};

fn main() {
    check!(2 + 2).satisfies(eq(4));
}
