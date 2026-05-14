//! A field name that the struct does not have is rejected by the generated
//! destructure, even with a trailing `..`.
use test_better::prelude::*;
use test_better::matches_struct;

struct Point {
    x: i32,
    y: i32,
}

fn main() {
    let point = Point { x: 1, y: 2 };
    let _ = expect!(point).to(matches_struct!(Point { x: eq(1), z: eq(0), .. }));
}
