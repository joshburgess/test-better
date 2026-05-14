//! Tokens after the `; "label"` of a `#[test_case]` are rejected by the macro.
use test_better::test_case;

#[test_case(1 ; "a label" and then some)]
fn takes_one(_value: i32) {}

fn main() {}
