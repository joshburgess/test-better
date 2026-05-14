//! A `#[test_case]` whose argument count does not match the function's
//! parameter count is a compile error.
use test_better::test_case;

#[test_case(1, 2, 3 ; "too many")]
fn takes_one(_value: i32) {}

fn main() {}
