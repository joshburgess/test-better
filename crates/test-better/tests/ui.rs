//! Compile-fail tests for the structural matcher macros.
//!
//! `matches_struct!` without a trailing `..` expands to a struct pattern that
//! lists exactly the named fields, so a missing or unknown field is a hard
//! error from the destructure. A misplaced `..` is caught by the macro itself.

#[test]
fn structural_matcher_compile_errors() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/ui/missing_field_without_rest.rs");
    t.compile_fail("tests/ui/unknown_field.rs");
    t.compile_fail("tests/ui/rest_not_last.rs");
}
