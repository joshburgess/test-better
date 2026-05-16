//! Without a trailing `..`, every field must be listed: omitting `y` makes the
//! generated struct pattern incomplete, which rustc rejects.
use test_better::prelude::*;
use test_better::matches_struct;

struct Point {
    x: i32,
    y: i32,
}

fn main() {
    let point = Point { x: 1, y: 2 };
    let _ = check!(point).satisfies(matches_struct!(Point { x: eq(1) }));
}
