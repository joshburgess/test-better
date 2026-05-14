//! Compile-fail tests.
//!
//! `matches_struct!` without a trailing `..` expands to a struct pattern that
//! lists exactly the named fields, so a missing or unknown field is a hard
//! error from the destructure. A misplaced `..` is caught by the macro itself.
//!
//! The async `expect!` case (Iteration 5.1) locks that the sync `to` cannot be
//! pointed at a future with an output matcher: that path must go through
//! `resolves_to`.

#[test]
fn structural_matcher_compile_errors() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/ui/missing_field_without_rest.rs");
    t.compile_fail("tests/ui/unknown_field.rs");
    t.compile_fail("tests/ui/rest_not_last.rs");
}

#[test]
fn sync_to_is_not_callable_on_a_future_subject() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/ui/sync_to_on_future.rs");
}
