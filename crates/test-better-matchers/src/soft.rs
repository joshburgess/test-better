//! Soft assertions: [`soft`] and [`SoftAsserter`].
//!
//! A normal assertion returns its `TestError` through `?`, so the first failure
//! ends the test. [`soft`] opens a scope in which assertions are *recorded*
//! rather than propagated; when the scope closes, every recorded failure is
//! reported together under a single [`Payload::Multiple`], each sub-failure
//! keeping its own location (PROJECT_BUILD_PLAN.md §9, Iteration 4.1).

use test_better_core::{ErrorKind, Payload, TestError, TestResult};

use crate::matcher::Matcher;

/// Runs `f` in a soft-assertion scope.
///
/// Inside `f`, failures recorded on the [`SoftAsserter`] do not end the
/// closure. When `f` returns, `soft` returns `Ok(())` if nothing was recorded,
/// or a single [`TestError`] collecting every recorded failure.
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
    f(&mut asserter);
    asserter.into_result()
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
            self.errors
                .push(
                    TestError::new(ErrorKind::Assertion).with_payload(Payload::ExpectedActual {
                        expected: mismatch.expected.to_string(),
                        actual: mismatch.actual,
                        diff: mismatch.diff,
                    }),
                );
        }
    }

    /// Records the result of an arbitrary fallible step. An `Err` is collected
    /// with its original location and context intact; an `Ok` is ignored.
    #[track_caller]
    pub fn check(&mut self, result: TestResult) {
        if let Err(error) = result {
            self.errors.push(error);
        }
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

#[cfg(test)]
mod tests {
    use test_better_core::{Payload, TestError, TestResult};

    use super::*;
    use crate::{eq, expect, is_true};

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
}
