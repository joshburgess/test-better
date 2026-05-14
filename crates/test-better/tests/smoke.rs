//! Phase 1 smoke test: a test file that depends only on `test-better` and
//! imports only `test_better::prelude::*` can write a `?`-driven test.
//!
//! This is the acceptance check for PROJECT_BUILD_PLAN.md Iteration 1.5.

use test_better::prelude::*;

/// A helper that fails, to give `?` something to propagate.
fn load_answer(present: bool) -> TestResult<i32> {
    let raw: Option<i32> = present.then_some(42);
    let answer = raw.or_fail_with("the answer should have been loaded")?;
    Ok(answer)
}

#[test]
fn prelude_supports_a_question_mark_driven_test() -> TestResult {
    let answer = load_answer(true).context("loading the answer")?;
    if answer != 42 {
        return Err(TestError::from_expected_actual(42, answer));
    }
    Ok(())
}

#[test]
fn failure_path_carries_context_and_message() {
    let failure = load_answer(false)
        .context("loading the answer")
        .expect_err("the missing answer should fail");

    let rendered = failure.to_string();
    assert!(
        rendered.contains("the answer should have been loaded"),
        "{rendered}"
    );
    assert!(rendered.contains("while loading the answer"), "{rendered}");
}
