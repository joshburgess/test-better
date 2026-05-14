//! A `#[fixture]` function takes no parameters: a fixture is resolved by name,
//! not called with arguments.
use test_better::fixture;

#[fixture]
fn db(_url: &str) -> Result<i32, ()> {
    Ok(0)
}

fn main() {}
