//! [`ContextExt`]: attach "while doing X" context to a fallible value.
//!
//! `ContextExt` is what makes `?` carry a story. A bare `?` propagates a
//! failure as-is; `.context("loading the fixture")?` propagates the same
//! failure with a frame explaining what the test was attempting
//! (PROJECT_BUILD_PLAN.md §6).
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
fn coerce<E>(error: E) -> TestError
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
    use std::cell::Cell;

    fn io_error() -> std::io::Error {
        std::io::Error::new(std::io::ErrorKind::NotFound, "missing file")
    }

    #[test]
    fn context_passes_through_ok() {
        let value: TestResult<i32> = Ok::<i32, std::io::Error>(7).context("unused");
        assert_eq!(value.expect("ok path"), 7);
    }

    #[test]
    fn context_passes_through_some() {
        let value: TestResult<i32> = Some(7).context("unused");
        assert_eq!(value.expect("some path"), 7);
    }

    #[test]
    fn context_wraps_foreign_error_as_other_payload() {
        let failing: Result<(), std::io::Error> = Err(io_error());
        let line = line!() + 1;
        let result = failing.context("reading the fixture");
        let error = result.expect_err("err path");
        assert_eq!(error.kind, ErrorKind::Custom);
        assert_eq!(error.location.line(), line);
        assert!(matches!(error.payload.as_deref(), Some(Payload::Other(_))));
        assert_eq!(error.context.len(), 1);
        assert_eq!(error.context[0].message, "reading the fixture");
    }

    #[test]
    fn context_does_not_double_wrap_a_test_error() {
        let original = TestError::assertion("values differ");
        let original_line = original.location.line();
        let error = Err::<(), _>(original)
            .context("comparing the results")
            .expect_err("err path");
        // Kind, location, and the (absent) payload of the original are kept.
        assert_eq!(error.kind, ErrorKind::Assertion);
        assert_eq!(error.location.line(), original_line);
        assert!(error.payload.is_none());
        assert_eq!(error.message.as_deref(), Some("values differ"));
        assert_eq!(error.context.len(), 1);
        assert_eq!(error.context[0].message, "comparing the results");
    }

    #[test]
    fn context_frames_accumulate_in_order() {
        let error = Err::<(), _>(io_error())
            .context("inner step")
            .context("outer step")
            .expect_err("err path");
        let messages: Vec<_> = error.context.iter().map(|f| f.message.as_ref()).collect();
        assert_eq!(messages, ["inner step", "outer step"]);
    }

    #[test]
    fn none_gains_context_and_caller_location() {
        let missing: Option<i32> = None;
        let line = line!() + 1;
        let result = missing.context("looking up the user");
        let error = result.expect_err("err path");
        assert_eq!(error.kind, ErrorKind::Custom);
        assert_eq!(error.location.line(), line);
        assert_eq!(error.context[0].message, "looking up the user");
    }

    #[test]
    fn with_context_runs_the_closure_only_on_failure() {
        let calls = Cell::new(0);
        let ok: TestResult<i32> = Ok::<i32, std::io::Error>(1).with_context(|| {
            calls.set(calls.get() + 1);
            "unused"
        });
        assert_eq!(ok.expect("ok path"), 1);
        assert_eq!(calls.get(), 0, "closure must not run on the success path");

        let err = Err::<(), _>(io_error())
            .with_context(|| {
                calls.set(calls.get() + 1);
                "computed context"
            })
            .expect_err("err path");
        assert_eq!(calls.get(), 1, "closure must run once on the failure path");
        assert_eq!(err.context[0].message, "computed context");
    }
}
