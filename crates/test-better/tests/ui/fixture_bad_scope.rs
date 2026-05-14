//! The `scope` argument of `#[fixture]` accepts only "test" or "module".
use test_better::fixture;

#[fixture(scope = "global")]
fn db() -> Result<i32, ()> {
    Ok(0)
}

fn main() {}
