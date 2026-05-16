//! Smoke test: a test file that depends only on `test-better` and imports only
//! `test_better::prelude::*` can write a `?`-driven test with `check!` and the
//! `?`-friendly conversions.
//!
//! This doubles as the reference for what a dogfooded test looks like from
//! outside the workspace: every assertion goes through `check!`.

use test_better::prelude::*;

/// A helper that fails, to give `?` something to propagate.
fn load_answer(present: bool) -> TestResult<i32> {
    let raw: Option<i32> = present.then_some(42);
    let answer = raw.or_fail_with("the answer should have been loaded")?;
    Ok(answer)
}

#[test]
fn prelude_supports_an_expect_driven_test() -> TestResult {
    let answer = load_answer(true).context("loading the answer")?;
    check!(answer).satisfies(eq(42))?;
    check!(answer).violates(lt(0))?;
    Ok(())
}

#[test]
fn expect_failure_names_the_expression_and_values() -> TestResult {
    let error = check!(2 + 2).satisfies(eq(5)).expect_err("2 + 2 is not 5");
    let rendered = error.to_string();
    check!(rendered.contains("2 + 2")).satisfies(is_true())?;
    check!(rendered.contains("equal to 5")).satisfies(is_true())?;
    Ok(())
}

#[test]
fn or_fail_failure_path_carries_context_and_message() -> TestResult {
    let failure = load_answer(false)
        .context("loading the answer")
        .expect_err("the missing answer should fail");

    let rendered = failure.to_string();
    check!(rendered.contains("the answer should have been loaded")).satisfies(is_true())?;
    check!(rendered.contains("while loading the answer")).satisfies(is_true())?;
    Ok(())
}
