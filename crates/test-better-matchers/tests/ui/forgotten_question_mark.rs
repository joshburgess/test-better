//! Dropping the `?` on an `expect!(..)` leaves the returned `Result` unused.
//! With `unused_must_use` denied, that becomes a hard error trybuild observes,
//! which proves the warning fires in ordinary builds too.
#![deny(unused_must_use)]

use test_better_matchers::{eq, expect};

fn main() {
    expect!(2 + 2).to(eq(4));
}
