//! Every test in this fixture crate passes, so `cargo test` and
//! `cargo test-better` must both exit `0` against it.

#[test]
fn arithmetic_holds() {
    assert_eq!(2 + 2, 4);
}

#[test]
fn string_lengths_are_what_they_look_like() {
    assert_eq!("ok".len(), 2);
}
