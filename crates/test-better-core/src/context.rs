//! [`ContextExt`]: attach "while doing X" context to a fallible value.
//!
//! `ContextExt` is what makes `?` carry a story. A bare `?` propagates a
//! failure as-is; `.context("loading the fixture")?` propagates the same
//! failure with a frame explaining what the test was attempting.
//!
//! When the error path already holds a [`TestError`], the context frame is
//! pushed onto it directly: the original kind, location, and payload are kept,
//! and the error is *not* re-wrapped as a [`Payload::Other`].

use std::borrow::Cow;
use std::error::Error;

use crate::error::{ContextFrame, ErrorKind, Payload, TestError};
use crate::result::TestResult;

/// Attaches context to the failure path of a [`Result`] or the [`None`] of an
/// [`Option`].
pub trait ContextExt<T> {
    /// Adds a context frame describing the operation that was being attempted.
    ///
    /// On the success path the value is returned unchanged.
    fn context(self, message: impl Into<Cow<'static, str>>) -> TestResult<T>;

    /// Like [`context`](ContextExt::context), but the message is computed by
    /// `f`, which runs only on the failure path.
    fn with_context<F, S>(self, f: F) -> TestResult<T>
    where
        F: FnOnce() -> S,
        S: Into<Cow<'static, str>>;
}

/// Coerces an arbitrary error into a [`TestError`].
///
/// If `error` already *is* a `TestError` it is returned untouched (no
/// double-wrapping); otherwise it becomes the [`Payload::Other`] of a fresh
/// [`ErrorKind::Custom`] error, so its source chain stays walkable.
#[track_caller]
pub(crate) fn coerce<E>(error: E) -> TestError
where
    E: Error + Send + Sync + 'static,
{
    let boxed: Box<dyn Error + Send + Sync> = Box::new(error);
    match boxed.downcast::<TestError>() {
        Ok(test_error) => *test_error,
        Err(other) => TestError::new(ErrorKind::Custom).with_payload(Payload::Other(other)),
    }
}

/// The error produced when context is attached to a [`None`].
#[track_caller]
fn none_error() -> TestError {
    TestError::new(ErrorKind::Custom).with_message("value was None")
}

impl<T, E> ContextExt<T> for Result<T, E>
where
    E: Error + Send + Sync + 'static,
{
    #[track_caller]
    fn context(self, message: impl Into<Cow<'static, str>>) -> TestResult<T> {
        match self {
            Ok(value) => Ok(value),
            Err(error) => Err(coerce(error).with_context_frame(ContextFrame::new(message))),
        }
    }

    #[track_caller]
    fn with_context<F, S>(self, f: F) -> TestResult<T>
    where
        F: FnOnce() -> S,
        S: Into<Cow<'static, str>>,
    {
        match self {
            Ok(value) => Ok(value),
            Err(error) => Err(coerce(error).with_context_frame(ContextFrame::new(f()))),
        }
    }
}

impl<T> ContextExt<T> for Option<T> {
    #[track_caller]
    fn context(self, message: impl Into<Cow<'static, str>>) -> TestResult<T> {
        match self {
            Some(value) => Ok(value),
            None => Err(none_error().with_context_frame(ContextFrame::new(message))),
        }
    }

    #[track_caller]
    fn with_context<F, S>(self, f: F) -> TestResult<T>
    where
        F: FnOnce() -> S,
        S: Into<Cow<'static, str>>,
    {
        match self {
            Some(value) => Ok(value),
            None => Err(none_error().with_context_frame(ContextFrame::new(f()))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{OrFail, TestResult};
    use std::cell::Cell;
    use test_better_matchers::{eq, expect, is_true};

    fn io_error() -> std::io::Error {
        std::io::Error::new(std::io::ErrorKind::NotFound, "missing file")
    }

    #[test]
    fn context_passes_through_ok() -> TestResult {
        let value: TestResult<i32> = Ok::<i32, std::io::Error>(7).context("unused");
        expect!(value?).to(eq(7)).or_fail()?;
        Ok(())
    }

    #[test]
    fn context_passes_through_some() -> TestResult {
        let value: TestResult<i32> = Some(7).context("unused");
        expect!(value?).to(eq(7)).or_fail()?;
        Ok(())
    }

    #[test]
    fn context_wraps_foreign_error_as_other_payload() -> TestResult {
        let failing: Result<(), std::io::Error> = Err(io_error());
        let line = line!() + 1;
        let result = failing.context("reading the fixture");
        let error = result.expect_err("err path");
        expect!(error.kind).to(eq(ErrorKind::Custom)).or_fail()?;
        expect!(error.location.line()).to(eq(line)).or_fail()?;
        expect!(matches!(error.payload.as_deref(), Some(Payload::Other(_))))
            .to(is_true())
            .or_fail()?;
        expect!(error.context.len()).to(eq(1)).or_fail()?;
        expect!(error.context[0].message.as_ref())
            .to(eq("reading the fixture"))
            .or_fail()?;
        Ok(())
    }

    #[test]
    fn context_does_not_double_wrap_a_test_error() -> TestResult {
        let original = TestError::assertion("values differ");
        let original_line = original.location.line();
        let error = Err::<(), _>(original)
            .context("comparing the results")
            .expect_err("err path");
        // Kind, location, and the (absent) payload of the original are kept.
        expect!(error.kind).to(eq(ErrorKind::Assertion)).or_fail()?;
        expect!(error.location.line())
            .to(eq(original_line))
            .or_fail()?;
        expect!(error.payload.is_none()).to(is_true()).or_fail()?;
        expect!(error.message.as_deref())
            .to(eq(Some("values differ")))
            .or_fail()?;
        expect!(error.context.len()).to(eq(1)).or_fail()?;
        expect!(error.context[0].message.as_ref())
            .to(eq("comparing the results"))
            .or_fail()?;
        Ok(())
    }

    #[test]
    fn context_frames_accumulate_in_order() -> TestResult {
        let error = Err::<(), _>(io_error())
            .context("inner step")
            .context("outer step")
            .expect_err("err path");
        let messages: Vec<_> = error.context.iter().map(|f| f.message.as_ref()).collect();
        expect!(messages)
            .to(eq(vec!["inner step", "outer step"]))
            .or_fail()?;
        Ok(())
    }

    #[test]
    fn none_gains_context_and_caller_location() -> TestResult {
        let missing: Option<i32> = None;
        let line = line!() + 1;
        let result = missing.context("looking up the user");
        let error = result.expect_err("err path");
        expect!(error.kind).to(eq(ErrorKind::Custom)).or_fail()?;
        expect!(error.location.line()).to(eq(line)).or_fail()?;
        expect!(error.context[0].message.as_ref())
            .to(eq("looking up the user"))
            .or_fail()?;
        Ok(())
    }

    #[test]
    fn with_context_runs_the_closure_only_on_failure() -> TestResult {
        let calls = Cell::new(0);
        let ok: TestResult<i32> = Ok::<i32, std::io::Error>(1).with_context(|| {
            calls.set(calls.get() + 1);
            "unused"
        });
        expect!(ok?).to(eq(1)).or_fail()?;
        expect!(calls.get()).to(eq(0)).or_fail()?;

        let err = Err::<(), _>(io_error())
            .with_context(|| {
                calls.set(calls.get() + 1);
                "computed context"
            })
            .expect_err("err path");
        expect!(calls.get()).to(eq(1)).or_fail()?;
        expect!(err.context[0].message.as_ref())
            .to(eq("computed context"))
            .or_fail()?;
        Ok(())
    }
}
