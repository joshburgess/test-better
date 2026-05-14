//! Smoke test: a test file that depends only on `test-better` and imports only
//! `test_better::prelude::*` can write a `?`-driven test with `expect!` and the
//! `?`-friendly conversions.
//!
//! This is the acceptance check for PROJECT_BUILD_PLAN.md Iterations 1.5 and
//! 2.3 (the facade wiring of `expect!` and the matchers).

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
    expect!(answer).to(eq(42))?;
    expect!(answer).to_not(lt(0))?;
    Ok(())
}

#[test]
fn expect_failure_names_the_expression_and_values() {
    let error = expect!(2 + 2).to(eq(5)).expect_err("2 + 2 is not 5");
    let rendered = error.to_string();
    assert!(rendered.contains("2 + 2"), "{rendered}");
    assert!(rendered.contains("equal to 5"), "{rendered}");
}

#[test]
fn or_fail_failure_path_carries_context_and_message() {
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
