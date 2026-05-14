//! `#[fixture]` and `#[test_with_fixtures]` end-to-end through the
//! `test-better` facade (PROJECT_BUILD_PLAN.md Iteration 8.3).
//!
//! A fixture is a `fn() -> TestResult<T>` of reusable setup; a
//! `#[test_with_fixtures]` test names fixtures as parameters and they are
//! resolved before the body runs. The point of the design is that a fixture
//! failure surfaces as `ErrorKind::Setup`, never as an assertion miss. The
//! tests that exercise that error path drive a failing fixture on purpose and
//! then inspect the captured error, so the suite still passes.

use test_better::ErrorKind;
use test_better::prelude::*;

#[fixture]
fn answer() -> TestResult<i32> {
    Ok(42)
}

#[test_with_fixtures]
fn a_fixture_value_reaches_the_test(answer: i32) -> TestResult {
    expect!(answer).to(eq(42))
}

#[fixture]
fn name() -> TestResult<String> {
    Ok(String::from("alice"))
}

#[fixture]
fn age() -> TestResult<u32> {
    Ok(30)
}

#[test_with_fixtures]
fn several_fixtures_are_resolved_left_to_right(name: String, age: u32) -> TestResult {
    expect!(name.len() as u32).to(le(age))
}

// A module-scoped fixture: the body runs once and every test gets a clone.
#[fixture(scope = "module")]
fn shared_config() -> TestResult<String> {
    Ok(String::from("loaded-once"))
}

#[test_with_fixtures]
fn a_module_fixture_is_shared(shared_config: String) -> TestResult {
    expect!(shared_config.as_str()).to(eq("loaded-once"))
}

#[test_with_fixtures]
fn a_module_fixture_is_shared_again(shared_config: String) -> TestResult {
    expect!(shared_config.is_empty()).to(is_false())
}

// A fixture that fails: its error must come through as `Setup`.
#[fixture]
fn broken_db() -> TestResult<i32> {
    Err(TestError::custom("could not connect to the database"))
}

#[test_with_fixtures]
#[ignore = "deliberately fails to exercise the fixture Setup error path"]
fn uses_broken_db(broken_db: i32) -> TestResult {
    expect!(broken_db).to(eq(1))
}

#[test]
fn a_fixture_failure_is_a_setup_error() -> TestResult {
    // `uses_broken_db` is generated as an (ignored) `#[test]`; call it directly
    // to capture what it would have reported.
    let failure = uses_broken_db().err().or_fail()?;
    expect!(failure.kind).to(eq(ErrorKind::Setup))?;

    let rendered = format!("{failure}");
    expect!(rendered.contains("test setup failed")).to(is_true())?;
    expect!(rendered.contains("setting up fixture `broken_db`")).to(is_true())?;
    // The original failure detail is preserved, just re-categorized.
    expect!(rendered.contains("could not connect to the database")).to(is_true())
}

// The same, module-scoped: the cached `Err` is reported as a fresh `Setup`
// failure that carries the original's rendered text.
#[fixture(scope = "module")]
fn broken_shared() -> TestResult<String> {
    Err(TestError::custom("config file is missing"))
}

#[test]
fn a_module_fixture_failure_is_a_setup_error() -> TestResult {
    let failure = broken_shared().err().or_fail()?;
    expect!(failure.kind).to(eq(ErrorKind::Setup))?;

    let rendered = format!("{failure}");
    expect!(rendered.contains("test setup failed")).to(is_true())?;
    expect!(rendered.contains("module-scoped fixture `broken_shared` failed")).to(is_true())?;
    expect!(rendered.contains("config file is missing")).to(is_true())
}
