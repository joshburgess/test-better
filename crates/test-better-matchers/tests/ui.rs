//! Compile-fail tests for the `expect!` macro.
//!
//! `Subject::to` / `to_not` return a `#[must_use]` `Result`; the safety net is
//! that a forgotten `?` is at least a warning. These tests pin that down.

#[test]
fn forgotten_question_mark_is_a_warning() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/ui/forgotten_question_mark.rs");
}
