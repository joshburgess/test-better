//! [`OrFail`]: turn a `Result` or `Option` into a [`TestResult`] at the call
//! site, the `?`-friendly alternative to panicking on failure.
//!
//! Panicking on failure provides a location but no story; `.or_fail()?` produces a
//! [`TestError`] that names what was expected and carries the underlying
//! error's chain. In the happy path the two are
//! interchangeable; in the failure path `or_fail` is strictly more informative.

use std::borrow::Cow;
use std::error::Error;

use crate::context::coerce;
use crate::error::{ErrorKind, TestError};
use crate::result::TestResult;

/// Converts a fallible value into a [`TestResult`], producing a [`TestError`]
/// on the failure path.
pub trait OrFail<T> {
    /// Converts the failure path into a [`TestError`] with a default message.
    fn or_fail(self) -> TestResult<T>;

    /// Like [`or_fail`](OrFail::or_fail), but with a caller-supplied message.
    fn or_fail_with(self, message: impl Into<Cow<'static, str>>) -> TestResult<T>;
}

/// The error produced when [`OrFail::or_fail`] is called on a [`None`].
#[track_caller]
fn missing_value<T>() -> TestError {
    TestError::new(ErrorKind::Assertion).with_message(format!(
        "expected Some({}), got None",
        std::any::type_name::<T>()
    ))
}

impl<T> OrFail<T> for Option<T> {
    #[track_caller]
    fn or_fail(self) -> TestResult<T> {
        match self {
            Some(value) => Ok(value),
            None => Err(missing_value::<T>()),
        }
    }

    #[track_caller]
    fn or_fail_with(self, message: impl Into<Cow<'static, str>>) -> TestResult<T> {
        match self {
            Some(value) => Ok(value),
            None => Err(TestError::new(ErrorKind::Assertion).with_message(message)),
        }
    }
}

impl<T, E> OrFail<T> for Result<T, E>
where
    E: Error + Send + Sync + 'static,
{
    #[track_caller]
    fn or_fail(self) -> TestResult<T> {
        self.map_err(coerce)
    }

    /// The supplied message is attached as a context frame, so the underlying
    /// error's own message and source chain are preserved.
    #[track_caller]
    fn or_fail_with(self, message: impl Into<Cow<'static, str>>) -> TestResult<T> {
        match self {
            Ok(value) => Ok(value),
            Err(error) => {
                Err(coerce(error).with_context_frame(crate::error::ContextFrame::new(message)))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::Payload;
    use crate::{OrFail, TestResult};
    use test_better_matchers::{eq, check, is_true};

    fn io_error() -> std::io::Error {
        std::io::Error::new(std::io::ErrorKind::NotFound, "missing file")
    }

    #[test]
    fn or_fail_passes_through_some_like_unwrap() -> TestResult {
        let some: Option<i32> = Some(7);
        check!(some.or_fail()?).satisfies(eq(7)).or_fail()?;
        Ok(())
    }

    #[test]
    fn or_fail_passes_through_ok_like_unwrap() -> TestResult {
        let ok: Result<i32, std::io::Error> = Ok(7);
        check!(ok.or_fail()?).satisfies(eq(7)).or_fail()?;
        Ok(())
    }

    #[test]
    fn or_fail_on_none_names_the_expected_type_and_caller_location() -> TestResult {
        let missing: Option<i32> = None;
        let line = line!() + 1;
        let result = missing.or_fail();
        let error = result.expect_err("err path");
        check!(error.kind).satisfies(eq(ErrorKind::Assertion)).or_fail()?;
        check!(error.location.line()).satisfies(eq(line)).or_fail()?;
        let message = error.message.as_deref().or_fail_with("message present")?;
        check!(message.starts_with("expected Some("))
            .satisfies(is_true())
            .or_fail()?;
        check!(message.ends_with("i32), got None"))
            .satisfies(is_true())
            .or_fail()?;
        Ok(())
    }

    #[test]
    fn or_fail_with_on_none_uses_the_supplied_message() -> TestResult {
        let missing: Option<i32> = None;
        let error = missing
            .or_fail_with("the user should have been seeded")
            .expect_err("err path");
        check!(error.message.as_deref())
            .satisfies(eq(Some("the user should have been seeded")))
            .or_fail()?;
        Ok(())
    }

    #[test]
    fn or_fail_on_err_preserves_the_underlying_error() -> TestResult {
        let failing: Result<(), std::io::Error> = Err(io_error());
        let error = failing.or_fail().expect_err("err path");
        check!(error.kind).satisfies(eq(ErrorKind::Custom)).or_fail()?;
        match error.payload.as_deref() {
            Some(Payload::Other(inner)) => {
                check!(inner.to_string())
                    .satisfies(eq("missing file".to_string()))
                    .or_fail()?;
            }
            other => panic!("expected Other payload, got {other:?}"),
        }
        Ok(())
    }

    #[test]
    fn or_fail_does_not_double_wrap_a_test_error() -> TestResult {
        let original = TestError::assertion("values differ");
        let original_line = original.location.line();
        let failing: Result<(), TestError> = Err(original);
        let error = failing.or_fail().expect_err("err path");
        check!(error.kind).satisfies(eq(ErrorKind::Assertion)).or_fail()?;
        check!(error.location.line())
            .satisfies(eq(original_line))
            .or_fail()?;
        check!(error.message.as_deref())
            .satisfies(eq(Some("values differ")))
            .or_fail()?;
        check!(error.payload.is_none()).satisfies(is_true()).or_fail()?;
        Ok(())
    }

    #[test]
    fn or_fail_with_on_err_keeps_the_chain_and_adds_context() -> TestResult {
        let failing: Result<(), std::io::Error> = Err(io_error());
        let error = failing
            .or_fail_with("loading the config file")
            .expect_err("err path");
        check!(matches!(error.payload.as_deref(), Some(Payload::Other(_))))
            .satisfies(is_true())
            .or_fail()?;
        check!(error.context.len()).satisfies(eq(1)).or_fail()?;
        check!(error.context[0].message.as_ref())
            .satisfies(eq("loading the config file"))
            .or_fail()?;
        Ok(())
    }
}
