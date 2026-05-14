//! A `;` in a `#[test_case]` must be followed by a string-literal label.
use test_better::test_case;

#[test_case(1 ;)]
fn takes_one(_value: i32) {}

fn main() {}
