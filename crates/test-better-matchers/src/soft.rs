//! Soft assertions: [`soft`], [`SoftAsserter`], and [`SoftScope`].
//!
//! A normal assertion returns its `TestError` through `?`, so the first failure
//! ends the test. [`soft`] opens a scope in which assertions are *recorded*
//! rather than propagated; when the scope closes, every recorded failure is
//! reported together under a single [`Payload::Multiple`], each sub-failure
//! keeping its own location (PROJECT_BUILD_PLAN.md §9, Iteration 4.1).
//!
//! [`SoftAsserter::context`] opens a sub-scope: failures recorded through the
//! returned [`SoftScope`] carry an extra context frame, and nested sub-scopes
//! stack their frames outermost-first (Iteration 4.2).
//!
//! A panic inside the [`soft`] closure does not mask the failures recorded
//! before it: [`soft`] runs the closure under [`catch_unwind`], reports the
//! collected failures, and re-raises the panic afterward (Iteration 4.3).
//!
//! [`catch_unwind`]: std::panic::catch_unwind

use std::borrow::Cow;

use test_better_core::{ContextFrame, ErrorKind, Payload, TestError, TestResult};

use crate::matcher::Matcher;

/// Runs `f` in a soft-assertion scope.
///
/// Inside `f`, failures recorded on the [`SoftAsserter`] do not end the
/// closure. When `f` returns, `soft` returns `Ok(())` if nothing was recorded,
/// or a single [`TestError`] collecting every recorded failure.
///
/// If `f` *panics*, the panic does not swallow what was already recorded:
/// `soft` runs `f` under [`catch_unwind`](std::panic::catch_unwind), prints the
/// collected soft failures to standard error, and then re-raises the panic so
/// the test still fails on it.
///
/// ```
/// use test_better_core::TestResult;
/// use test_better_matchers::{eq, soft};
///
/// fn main() -> TestResult {
///     soft(|s| {
///         s.expect(&2, eq(2));
///         s.expect(&"alice", eq("alice"));
///     })?;
///     Ok(())
/// }
/// ```
#[track_caller]
pub fn soft<F>(f: F) -> TestResult
where
    F: FnOnce(&mut SoftAsserter),
{
    let mut asserter = SoftAsserter::new();

    // `f` captures `&mut asserter`, and `&mut T` is not `UnwindSafe`. Asserting
    // unwind-safety is sound here: the only state `f` mutates is
    // `asserter.errors` and `asserter.context`, both plain `Vec`s. A panic can
    // leave them partially populated, but a partially-filled `Vec` is still a
    // valid, fully-readable value — there is no torn invariant for the code
    // after the `catch_unwind` to observe.
    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| f(&mut asserter)));

    let result = asserter.into_result();

    match outcome {
        Ok(()) => result,
        Err(panic) => {
            // A panic cut the closure short. Surface the failures recorded
            // before it — otherwise the panic would mask them — then re-raise
            // so the test still fails on the panic itself.
            if let Err(ref soft_failures) = result {
                eprintln!("soft assertions recorded before the panic:\n{soft_failures}");
            }
            std::panic::resume_unwind(panic);
        }
    }
}

/// The recorder passed to a [`soft`] closure.
///
/// Every `expect`/`check` that fails is pushed onto an internal list instead of
/// returning early; [`soft`] turns that list into one [`TestError`] on scope
/// exit. Callers rarely construct or name this type directly: it arrives as the
/// argument of the [`soft`] closure.
#[derive(Default)]
pub struct SoftAsserter {
    errors: Vec<TestError>,
    /// The context frames currently in effect, outermost first. Pushed by
    /// [`SoftAsserter::context`] and popped when the returned [`SoftScope`] is
    /// dropped.
    context: Vec<ContextFrame>,
}

impl SoftAsserter {
    /// Creates an empty recorder. Most callers use [`soft`] rather than
    /// constructing this directly.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Records whether `actual` satisfies `matcher`. A miss is collected, not
    /// propagated, so the closure keeps running.
    ///
    /// The recorded failure captures *this* call site, so each soft failure
    /// reports the line it came from.
    #[track_caller]
    pub fn expect<T, M>(&mut self, actual: &T, matcher: M)
    where
        T: ?Sized,
        M: Matcher<T>,
    {
        if let Some(mismatch) = matcher.check(actual).failure {
            self.record(TestError::new(ErrorKind::Assertion).with_payload(
                Payload::ExpectedActual {
                    expected: mismatch.expected.to_string(),
                    actual: mismatch.actual,
                    diff: mismatch.diff,
                },
            ));
        }
    }

    /// Records the result of an arbitrary fallible step. An `Err` is collected
    /// with its original location and context intact; an `Ok` is ignored.
    #[track_caller]
    pub fn check(&mut self, result: TestResult) {
        if let Err(error) = result {
            self.record(error);
        }
    }

    /// Opens a context sub-scope. Failures recorded through the returned
    /// [`SoftScope`] carry `message` as a context frame; the frame is removed
    /// when the `SoftScope` is dropped. Sub-scopes nest: a `SoftScope` can open
    /// further sub-scopes, and their frames stack outermost-first.
    #[track_caller]
    pub fn context(&mut self, message: impl Into<Cow<'static, str>>) -> SoftScope<'_> {
        self.context.push(ContextFrame::new(message));
        SoftScope { asserter: self }
    }

    /// Collects a failure, wrapping it in the context frames currently in
    /// effect. The scope frames are the *outer* circumstance, so they precede
    /// the error's own frames, which `TestError` already orders outermost-first.
    fn record(&mut self, mut error: TestError) {
        if !self.context.is_empty() {
            let mut frames = self.context.clone();
            frames.append(&mut error.context);
            error.context = frames;
        }
        self.errors.push(error);
    }

    /// Consumes the recorder, folding the collected failures into one result:
    /// `Ok(())` when nothing was recorded, otherwise a single [`TestError`]
    /// whose [`Payload::Multiple`] holds every failure.
    #[track_caller]
    fn into_result(self) -> TestResult {
        if self.errors.is_empty() {
            return Ok(());
        }
        let count = self.errors.len();
        let noun = if count == 1 {
            "soft assertion"
        } else {
            "soft assertions"
        };
        Err(TestError::new(ErrorKind::Assertion)
            .with_message(format!("{count} {noun} failed"))
            .with_payload(Payload::Multiple(self.errors)))
    }
}

/// A context sub-scope of a [`SoftAsserter`], returned by
/// [`SoftAsserter::context`].
///
/// Recording through a `SoftScope` behaves exactly like recording through the
/// underlying [`SoftAsserter`], except every recorded failure also carries the
/// scope's context frame (and the frames of any enclosing scopes). Dropping the
/// `SoftScope` removes its frame, so the context applies only to failures
/// recorded *while the scope is alive*.
pub struct SoftScope<'a> {
    asserter: &'a mut SoftAsserter,
}

impl SoftScope<'_> {
    /// Records whether `actual` satisfies `matcher`, attaching this scope's
    /// context to a miss. See [`SoftAsserter::expect`].
    #[track_caller]
    pub fn expect<T, M>(&mut self, actual: &T, matcher: M)
    where
        T: ?Sized,
        M: Matcher<T>,
    {
        self.asserter.expect(actual, matcher);
    }

    /// Records the result of a fallible step, attaching this scope's context to
    /// an `Err`. See [`SoftAsserter::check`].
    #[track_caller]
    pub fn check(&mut self, result: TestResult) {
        self.asserter.check(result);
    }

    /// Opens a nested context sub-scope. Its frame stacks *under* this scope's,
    /// so failures recorded through it carry both. See
    /// [`SoftAsserter::context`].
    #[track_caller]
    pub fn context(&mut self, message: impl Into<Cow<'static, str>>) -> SoftScope<'_> {
        self.asserter.context(message)
    }
}

impl Drop for SoftScope<'_> {
    fn drop(&mut self) {
        self.asserter.context.pop();
    }
}

#[cfg(test)]
mod tests {
    use test_better_core::{Payload, TestError, TestResult};

    use super::*;
    use crate::{contains_str, eq, expect, is_true};

    #[test]
    fn soft_with_no_failures_returns_ok() -> TestResult {
        let result = soft(|s| {
            s.expect(&2, eq(2));
            s.check(Ok(()));
        });
        expect!(result.is_ok()).to(is_true())?;
        Ok(())
    }

    #[test]
    fn soft_collects_every_failure_each_with_its_own_location() -> TestResult {
        let result = soft(|s| {
            s.expect(&1, eq(2));
            s.expect(&3, eq(4));
            s.expect(&5, eq(6));
        });
        let error = result.expect_err("three soft assertions failed");

        let rendered = error.to_string();
        expect!(rendered.contains("3 soft assertions failed")).to(is_true())?;
        expect!(rendered.contains("3 failures")).to(is_true())?;

        match error.payload.as_deref() {
            Some(Payload::Multiple(errors)) => {
                expect!(errors.len()).to(eq(3))?;
                // The three `expect` calls are on consecutive lines, so the
                // captured locations are all distinct.
                let lines: Vec<u32> = errors.iter().map(|e| e.location.line()).collect();
                expect!(lines[0] != lines[1] && lines[1] != lines[2] && lines[0] != lines[2])
                    .to(is_true())?;
            }
            _ => return Err(TestError::assertion("expected a Multiple payload")),
        }
        Ok(())
    }

    #[test]
    fn soft_check_records_an_err_and_ignores_ok() -> TestResult {
        let result = soft(|s| {
            s.check(Ok(()));
            s.check(Err(TestError::assertion("a recorded failure")));
        });
        let error = result.expect_err("one recorded check failed");
        expect!(error.to_string().contains("a recorded failure")).to(is_true())?;
        Ok(())
    }

    #[test]
    fn soft_check_preserves_the_recorded_error_location() -> TestResult {
        let recorded = TestError::assertion("from elsewhere");
        let recorded_line = recorded.location.line();
        let result = soft(|s| s.check(Err(recorded)));
        let error = result.expect_err("one recorded check failed");
        match error.payload.as_deref() {
            Some(Payload::Multiple(errors)) => {
                expect!(errors[0].location.line()).to(eq(recorded_line))?;
            }
            _ => return Err(TestError::assertion("expected a Multiple payload")),
        }
        Ok(())
    }

    #[test]
    fn context_scope_attaches_a_frame_to_recorded_failures() -> TestResult {
        let result = soft(|s| {
            let mut scope = s.context("while validating the user");
            scope.expect(&1, eq(2));
        });
        let error = result.expect_err("one soft assertion failed");
        match error.payload.as_deref() {
            Some(Payload::Multiple(errors)) => {
                let frames: Vec<&str> = errors[0]
                    .context
                    .iter()
                    .map(|frame| frame.message.as_ref())
                    .collect();
                expect!(frames).to(eq(vec!["while validating the user"]))?;
            }
            _ => return Err(TestError::assertion("expected a Multiple payload")),
        }
        Ok(())
    }

    #[test]
    fn context_scope_ends_when_the_scope_is_dropped() -> TestResult {
        let result = soft(|s| {
            {
                let mut scope = s.context("inside the scope");
                scope.expect(&1, eq(2));
            }
            // The scope has been dropped; this failure carries no context.
            s.expect(&3, eq(4));
        });
        let error = result.expect_err("two soft assertions failed");
        match error.payload.as_deref() {
            Some(Payload::Multiple(errors)) => {
                expect!(errors[0].context.len()).to(eq(1usize))?;
                expect!(errors[1].context.len()).to(eq(0usize))?;
            }
            _ => return Err(TestError::assertion("expected a Multiple payload")),
        }
        Ok(())
    }

    #[test]
    fn nested_context_scopes_stack_outermost_first() -> TestResult {
        let result = soft(|s| {
            let mut outer = s.context("while validating the user");
            outer.expect(&1, eq(2));
            let mut inner = outer.context("while checking the email");
            inner.expect(&"bad", contains_str("@"));
        });
        let error = result.expect_err("two soft assertions failed");
        let rendered = error.to_string();
        expect!(rendered.contains("while validating the user")).to(is_true())?;
        expect!(rendered.contains("while checking the email")).to(is_true())?;

        match error.payload.as_deref() {
            Some(Payload::Multiple(errors)) => {
                let outer_frames: Vec<&str> = errors[0]
                    .context
                    .iter()
                    .map(|frame| frame.message.as_ref())
                    .collect();
                let inner_frames: Vec<&str> = errors[1]
                    .context
                    .iter()
                    .map(|frame| frame.message.as_ref())
                    .collect();
                expect!(outer_frames).to(eq(vec!["while validating the user"]))?;
                expect!(inner_frames).to(eq(vec![
                    "while validating the user",
                    "while checking the email",
                ]))?;
            }
            _ => return Err(TestError::assertion("expected a Multiple payload")),
        }
        Ok(())
    }
}
