//! One test in this fixture crate fails on purpose, so `cargo test` exits
//! non-zero against it. `cargo test-better` must propagate that same code.

#[test]
fn this_one_passes() {
    assert_eq!(1 + 1, 2);
}

#[test]
fn this_one_fails() {
    assert_eq!(
        1 + 1,
        3,
        "deliberate failure, exercising the runner's error exit code",
    );
}
