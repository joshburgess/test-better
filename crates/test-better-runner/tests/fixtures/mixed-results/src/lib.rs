//! A fixed mix of test outcomes (three pass, two fail, one ignored) so the
//! `cargo test-better` summary table has known counts to assert against
//! (PROJECT_BUILD_PLAN.md Iteration 9.3). The crate is never built for real
//! use; the failing tests fail on purpose.

#[test]
fn addition_holds() {
    assert_eq!(2 + 2, 4);
}

#[test]
fn subtraction_holds() {
    assert_eq!(9 - 4, 5);
}

#[test]
fn multiplication_holds() {
    assert_eq!(3 * 7, 21);
}

#[test]
fn division_is_wrong_on_purpose() {
    assert_eq!(10 / 2, 6, "deliberate failure for the summary fixture");
}

#[test]
fn comparison_is_wrong_on_purpose() {
    assert!(1 > 2, "deliberate failure for the summary fixture");
}

#[test]
#[ignore = "left unrun on purpose, so the summary reports one ignored test"]
fn skipped_for_now() {
    assert_eq!(0, 0);
}
