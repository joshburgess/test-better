//! The [`check!`](crate::check) macro and its [`Subject`] type: the entry point for writing
//! an assertion.
//!
//! `check!(value)` captures the value *and the source text of the expression
//! it came from*, so a failure can name `2 + 2`, not just `4`. The resulting
//! [`Subject`] is consumed by [`Subject::satisfies`] / [`Subject::violates`],
//! each of which returns a [`TestResult`] so the assertion chains with `?`.
//!
//! Every method on `Subject` reads as a present-tense factual claim about the
//! value: "x satisfies the matcher", "x matches the snapshot", "the future
//! completes within 50ms". That shape is the convention for the whole crate.
//!
//! # Async
//!
//! When the expression handed to `check!` is a [`Future`], the resulting
//! `Subject` grows an `await`-based method, [`Subject::resolves_to`]. The
//! design is a single `Subject<T>` with that method added to *this* impl block
//! under a method-level `where T: Future` bound and a distinct name: a blanket
//! `impl<T> Subject<T>` and an overlapping `impl<F: Future> Subject<F>` cannot
//! coexist as inherent impls.
//!
//! `resolves_to` is runtime-agnostic: it just awaits the future, so it works
//! under `#[tokio::test]`, `#[async_std::test]`, `pollster::block_on`, or any
//! other executor.

use std::fmt::Display;
use std::future::Future;
use std::panic::Location;
use std::time::Duration;

use test_better_async::{Elapsed, RuntimeAvailable, run_within};
use test_better_core::{ErrorKind, Payload, TestError, TestResult};
use test_better_snapshot::{
    InlineLocation, InlineSnapshotFailure, Redactions, SnapshotFailure, SnapshotMode,
};

use crate::description::Description;
use crate::matcher::{Matcher, Mismatch};

/// A value under test, paired with the source text of the expression that
/// produced it.
///
/// `Subject` owns its value (the [`check!`](crate::check) macro hands it over by value) and
/// borrows nothing, so it carries no lifetime parameter.
pub struct Subject<T> {
    actual: T,
    expr: &'static str,
    module_path: &'static str,
}

impl<T> Subject<T> {
    /// Pairs `actual` with the source text it came from and the `module_path!()`
    /// of the call site. Called by [`check!`](crate::check); rarely
    /// constructed directly.
    ///
    /// `module_path` is only consulted by [`matches_snapshot`](Self::matches_snapshot),
    /// which uses it to name the snapshot file; every other method ignores it.
    #[must_use]
    pub fn new(actual: T, expr: &'static str, module_path: &'static str) -> Self {
        Self {
            actual,
            expr,
            module_path,
        }
    }

    /// Asserts that the value satisfies `matcher`.
    ///
    /// Returns `Ok(())` on a match and a [`TestError`] otherwise. The result is
    /// `#[must_use]` (it is a `Result`), so a forgotten `?` is a compiler
    /// warning rather than a silently-passing assertion.
    #[track_caller]
    pub fn satisfies<M>(self, matcher: M) -> TestResult
    where
        M: Matcher<T>,
    {
        match matcher.check(&self.actual).failure {
            None => Ok(()),
            Some(mismatch) => Err(mismatch_error(self.expr, mismatch)),
        }
    }

    /// Asserts that the value does *not* satisfy `matcher`.
    ///
    /// Returns `Ok(())` when the matcher does not match, and a [`TestError`]
    /// when it unexpectedly does. Equivalent to
    /// [`satisfies`](Self::satisfies)`(`[`not`](crate::not)`(matcher))`; pick
    /// whichever reads better at the call site.
    #[track_caller]
    pub fn violates<M>(self, matcher: M) -> TestResult
    where
        M: Matcher<T>,
    {
        if matcher.check(&self.actual).matched {
            Err(unexpected_match_error(self.expr, matcher.description()))
        } else {
            Ok(())
        }
    }

    /// Awaits the future and asserts that its output satisfies `matcher`.
    ///
    /// This is the async counterpart of [`satisfies`](Self::satisfies): reach
    /// for it when the expression handed to [`check!`](crate::check) is a
    /// [`Future`]. The matcher runs against the future's *output*, so
    /// `check!(fut).resolves_to(eq(4))` is exactly
    /// `check!(fut.await).satisfies(eq(4))` without the intermediate binding.
    ///
    /// The method itself is *not* `async`: it is `#[track_caller]` and returns
    /// a future. The call-site location is captured synchronously when
    /// `resolves_to` is called (an `async fn` could not be `#[track_caller]`),
    /// then carried into the failure once the returned future is awaited.
    ///
    /// ```
    /// use test_better_core::TestResult;
    /// use test_better_matchers::{check, eq};
    ///
    /// # fn main() -> TestResult {
    /// pollster::block_on(async {
    ///     check!(async { 2 + 2 }).resolves_to(eq(4)).await?;
    ///     TestResult::Ok(())
    /// })
    /// # }
    /// ```
    #[track_caller]
    pub fn resolves_to<M>(self, matcher: M) -> impl Future<Output = TestResult>
    where
        T: Future,
        M: Matcher<T::Output>,
    {
        // Captured here, synchronously, before the returned future is ever
        // polled: this is the user's `check!(..).resolves_to(..)` call site.
        let location = Location::caller();
        async move {
            let output = self.actual.await;
            match matcher.check(&output).failure {
                None => Ok(()),
                Some(mismatch) => Err(mismatch_error(self.expr, mismatch).with_location(location)),
            }
        }
    }

    /// Awaits the future, but fails if it does not finish within `limit`.
    ///
    /// Like [`resolves_to`](Self::resolves_to), this is for a future-typed
    /// subject and returns a future rather than being `async` itself, so the
    /// `#[track_caller]` location is the call site. Unlike `resolves_to`, it
    /// does not look at the output: the assertion is purely about *time*.
    ///
    /// The timeout needs a runtime to provide its sleep, selected by a cargo
    /// feature on `test-better`: `tokio`, `async-std`, or `smol`. With none
    /// enabled, the `T: RuntimeAvailable` bound is unsatisfied and the call is
    /// a compile error naming those flags.
    ///
    /// ```ignore
    /// use std::time::Duration;
    /// use test_better::prelude::*;
    ///
    /// # async fn run() -> TestResult {
    /// check!(some_future())
    ///     .completes_within(Duration::from_millis(50))
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    #[track_caller]
    pub fn completes_within(self, limit: Duration) -> impl Future<Output = TestResult>
    where
        T: Future + RuntimeAvailable,
    {
        let location = Location::caller();
        async move {
            match run_within(limit, self.actual).await {
                Ok(_) => Ok(()),
                Err(elapsed) => Err(timeout_error(self.expr, elapsed).with_location(location)),
            }
        }
    }

    /// Asserts that the value matches the snapshot stored under `name`.
    ///
    /// The snapshot lives at `tests/snapshots/<module-path>__<name>.snap` in the
    /// package under test, with `<module-path>` taken from the call site's
    /// `module_path!()`. On a normal run the value is compared against that
    /// file; a difference (or a missing file) is a `Snapshot` failure, and the
    /// difference is rendered with the standard line-oriented diff. Rerun the
    /// test with `UPDATE_SNAPSHOTS=1` to create the file the first time, or to
    /// accept an intentional change.
    ///
    /// The value only has to be [`Display`]: that is what gets written to and
    /// compared against the file.
    ///
    /// ```no_run
    /// use test_better_core::TestResult;
    /// use test_better_matchers::check;
    ///
    /// # fn main() -> TestResult {
    /// // Run once with `UPDATE_SNAPSHOTS=1` to record the snapshot; later runs
    /// // compare against `tests/snapshots/<module>__homepage.snap`.
    /// check!("<h1>Hello</h1>").matches_snapshot("homepage")?;
    /// # Ok(())
    /// # }
    /// ```
    #[track_caller]
    pub fn matches_snapshot(self, name: &str) -> TestResult
    where
        T: Display,
    {
        self.matches_snapshot_with(name, &Redactions::new())
    }

    /// Like [`matches_snapshot`](Self::matches_snapshot), but runs `redactions`
    /// over the value first.
    ///
    /// Use this when the rendered value carries content that is not stable run
    /// to run (a generated UUID, a timestamp): the redactions rewrite those
    /// spans to fixed placeholders, so the snapshot file stays meaningful.
    /// Because the redactions run on every comparison, the placeholder is what
    /// is stored and what later runs compare against.
    ///
    /// ```no_run
    /// use test_better_core::TestResult;
    /// use test_better_matchers::check;
    /// use test_better_snapshot::Redactions;
    ///
    /// # fn main() -> TestResult {
    /// let rendered = format!("created {}", uuid_of_new_record());
    /// check!(rendered)
    ///     .matches_snapshot_with("record", &Redactions::new().redact_uuids())?;
    /// # Ok(())
    /// # }
    /// # fn uuid_of_new_record() -> &'static str { "00000000-0000-0000-0000-000000000000" }
    /// ```
    #[track_caller]
    pub fn matches_snapshot_with(self, name: &str, redactions: &Redactions) -> TestResult
    where
        T: Display,
    {
        let actual = redactions.apply(&self.actual.to_string());
        match test_better_snapshot::assert_snapshot(self.module_path, name, &actual) {
            Ok(()) => Ok(()),
            Err(failure) => Err(snapshot_error(self.expr, name, failure)),
        }
    }

    /// Asserts that the value matches the inline snapshot literal `expected`.
    ///
    /// Unlike [`matches_snapshot`](Self::matches_snapshot), the snapshot lives
    /// in the test source: `expected` *is* the snapshot. The literal is
    /// normalized before comparison (a leading newline and the common
    /// indentation are cosmetic), so it can be indented to match the
    /// surrounding code.
    ///
    /// On a mismatch with `UPDATE_SNAPSHOTS` unset this fails like any
    /// assertion. With `UPDATE_SNAPSHOTS=1` it records a *pending patch* under
    /// `target/test-better-pending/` and passes; the `test-better-accept`
    /// companion binary rewrites the literal in the source from those patches.
    /// A proc macro could not do this rewrite (it runs before the test), so the
    /// work is split: this method captures the call-site location with
    /// `#[track_caller]`, the accept binary edits the file.
    ///
    /// ```no_run
    /// use test_better_core::TestResult;
    /// use test_better_matchers::check;
    ///
    /// # fn main() -> TestResult {
    /// check!(2 + 2).matches_inline_snapshot("4")?;
    /// # Ok(())
    /// # }
    /// ```
    #[track_caller]
    pub fn matches_inline_snapshot(self, expected: &str) -> TestResult
    where
        T: Display,
    {
        self.matches_inline_snapshot_with(expected, &Redactions::new())
    }

    /// Like [`matches_inline_snapshot`](Self::matches_inline_snapshot), but
    /// runs `redactions` over the value first.
    ///
    /// The inline-snapshot counterpart of
    /// [`matches_snapshot_with`](Self::matches_snapshot_with): redaction
    /// rewrites non-deterministic spans to fixed placeholders before the
    /// comparison, so `UPDATE_SNAPSHOTS=1` records the *redacted* value as the
    /// literal and later runs stay green.
    ///
    /// ```no_run
    /// use test_better_core::TestResult;
    /// use test_better_matchers::check;
    /// use test_better_snapshot::Redactions;
    ///
    /// # fn main() -> TestResult {
    /// let rendered = format!("at {}", now_rfc3339());
    /// check!(rendered).matches_inline_snapshot_with(
    ///     "at [timestamp]",
    ///     &Redactions::new().redact_rfc3339_timestamps(),
    /// )?;
    /// # Ok(())
    /// # }
    /// # fn now_rfc3339() -> &'static str { "2026-05-14T12:34:56Z" }
    /// ```
    #[track_caller]
    pub fn matches_inline_snapshot_with(self, expected: &str, redactions: &Redactions) -> TestResult
    where
        T: Display,
    {
        let actual = redactions.apply(&self.actual.to_string());
        let caller = Location::caller();
        let location = InlineLocation {
            file: caller.file().to_string(),
            line: caller.line(),
            column: caller.column(),
        };
        match test_better_snapshot::assert_inline_snapshot(
            &actual,
            expected,
            &location,
            SnapshotMode::from_env(),
        ) {
            Ok(()) => Ok(()),
            Err(failure) => Err(inline_snapshot_error(self.expr, failure)),
        }
    }
}

/// Builds the error for a matcher that did not match: the expected/actual pair
/// goes into the payload, the source expression into the message.
#[track_caller]
fn mismatch_error(expr: &str, mismatch: Mismatch) -> TestError {
    TestError::new(ErrorKind::Assertion)
        .with_message(format!("check!({expr})"))
        .with_payload(Payload::ExpectedActual {
            expected: mismatch.expected.to_string(),
            actual: mismatch.actual,
            diff: mismatch.diff,
        })
}

/// Builds the error for `violates` when the matcher matched but should not
/// have. There is no `Mismatch` in this case, so the message carries the whole
/// story.
#[track_caller]
fn unexpected_match_error(expr: &str, description: Description) -> TestError {
    TestError::new(ErrorKind::Assertion).with_message(format!(
        "check!({expr}): expected it not to be {description}, but it was"
    ))
}

/// Builds the error for `completes_within` when the future ran past its
/// limit. This is a timing failure, not a value mismatch, so it carries only
/// a message, no payload.
#[track_caller]
fn timeout_error(expr: &str, elapsed: Elapsed) -> TestError {
    TestError::new(ErrorKind::Assertion).with_message(format!(
        "check!({expr}): did not complete within {:?}",
        elapsed.limit
    ))
}

/// Renders a [`SnapshotFailure`] from `test-better-snapshot` into a `TestError`.
///
/// A mismatch becomes an `ExpectedActual` payload, exactly like a value
/// matcher's mismatch, so the standard renderer shows it (and its diff) the
/// same way. A missing file or an I/O error has no two sides to compare, so it
/// carries only a message.
#[track_caller]
fn snapshot_error(expr: &str, name: &str, failure: SnapshotFailure) -> TestError {
    match failure {
        SnapshotFailure::Mismatch {
            path,
            expected,
            actual,
        } => {
            let diff = snapshot_diff(&expected, &actual);
            TestError::new(ErrorKind::Snapshot)
                .with_message(format!(
                    "check!({expr}): snapshot {name:?} at {} does not match",
                    path.display()
                ))
                .with_payload(Payload::ExpectedActual {
                    expected,
                    actual,
                    diff,
                })
        }
        SnapshotFailure::Missing { path } => {
            TestError::new(ErrorKind::Snapshot).with_message(format!(
                "check!({expr}): snapshot {name:?} does not exist at {}; \
                 rerun with UPDATE_SNAPSHOTS=1 to create it",
                path.display()
            ))
        }
        SnapshotFailure::Io {
            path,
            action,
            source,
        } => TestError::new(ErrorKind::Snapshot).with_message(format!(
            "check!({expr}): snapshot {name:?} I/O error {action} ({}): {source}",
            path.display()
        )),
    }
}

/// Renders an [`InlineSnapshotFailure`] into a `TestError`, the inline-snapshot
/// counterpart of [`snapshot_error`]. Both sides go into an `ExpectedActual`
/// payload so the standard renderer (and its diff) show the change.
#[track_caller]
fn inline_snapshot_error(expr: &str, failure: InlineSnapshotFailure) -> TestError {
    let InlineSnapshotFailure { expected, actual } = failure;
    let diff = snapshot_diff(&expected, &actual);
    TestError::new(ErrorKind::Snapshot)
        .with_message(format!(
            "check!({expr}): inline snapshot does not match; \
             rerun with UPDATE_SNAPSHOTS=1 to update it"
        ))
        .with_payload(Payload::ExpectedActual {
            expected,
            actual,
            diff,
        })
}

/// The line-oriented diff for a snapshot mismatch. Unlike a value matcher,
/// which only diffs multi-line values, a snapshot is text and a diff is always
/// the readable way to show what changed. With the `diff` feature off this is
/// `None` and the failure still renders, just without the diff.
#[cfg(feature = "diff")]
fn snapshot_diff(expected: &str, actual: &str) -> Option<String> {
    Some(crate::diff::diff_lines(expected, actual))
}

#[cfg(not(feature = "diff"))]
fn snapshot_diff(_expected: &str, _actual: &str) -> Option<String> {
    None
}

/// Captures an expression and its source text for assertion with a matcher.
///
/// ```
/// use test_better_core::TestResult;
/// use test_better_matchers::{check, eq};
///
/// fn main() -> TestResult {
///     check!(2 + 2).satisfies(eq(4))?;
///     Ok(())
/// }
/// ```
#[macro_export]
macro_rules! check {
    ($actual:expr) => {
        $crate::Subject::new($actual, ::core::stringify!($actual), ::core::module_path!())
    };
}

#[cfg(test)]
mod tests {
    use test_better_core::TestResult;

    use crate::{eq, is_true};

    #[test]
    fn satisfies_returns_ok_on_a_match() -> TestResult {
        let result = check!(2 + 2).satisfies(eq(4));
        check!(result.is_ok()).satisfies(is_true())?;
        Ok(())
    }

    #[test]
    fn satisfies_failure_mentions_the_expression_and_the_expected_value() -> TestResult {
        let error = check!(2 + 2).satisfies(eq(5)).expect_err("2 + 2 is not 5");
        let rendered = error.to_string();
        check!(rendered.contains("2 + 2")).satisfies(is_true())?;
        check!(rendered.contains("equal to 5")).satisfies(is_true())?;
        check!(rendered.contains("actual: 4")).satisfies(is_true())?;
        Ok(())
    }

    #[test]
    fn satisfies_failure_captures_the_caller_location() -> TestResult {
        let line = line!() + 1;
        let error = check!(2 + 2).satisfies(eq(5)).expect_err("2 + 2 is not 5");
        check!(error.location.line()).satisfies(eq(line))?;
        check!(error.location.file().ends_with("subject.rs")).satisfies(is_true())?;
        Ok(())
    }

    #[test]
    fn violates_returns_ok_when_the_matcher_does_not_match() -> TestResult {
        let result = check!(2 + 2).violates(eq(5));
        check!(result.is_ok()).satisfies(is_true())?;
        Ok(())
    }

    #[test]
    fn violates_failure_mentions_the_expression_and_the_matcher() -> TestResult {
        let error = check!(true).violates(is_true()).expect_err("true is true");
        let rendered = error.to_string();
        check!(rendered.contains("check!(true)")).satisfies(is_true())?;
        check!(rendered.contains("not to be true")).satisfies(is_true())?;
        Ok(())
    }

    #[test]
    fn violates_captures_the_caller_location() -> TestResult {
        let line = line!() + 1;
        let error = check!(true).violates(is_true()).expect_err("true is true");
        check!(error.location.line()).satisfies(eq(line))?;
        Ok(())
    }

    #[test]
    fn resolves_to_returns_ok_when_the_output_matches() -> TestResult {
        pollster::block_on(async {
            let result = check!(async { 2 + 2 }).resolves_to(eq(4)).await;
            check!(result.is_ok()).satisfies(is_true())
        })
    }

    #[test]
    fn resolves_to_failure_mentions_the_expression_and_the_output() -> TestResult {
        pollster::block_on(async {
            let error = check!(async { 2 + 2 })
                .resolves_to(eq(5))
                .await
                .expect_err("2 + 2 does not resolve to 5");
            let rendered = error.to_string();
            check!(rendered.contains("async { 2 + 2 }")).satisfies(is_true())?;
            check!(rendered.contains("equal to 5")).satisfies(is_true())?;
            check!(rendered.contains("actual: 4")).satisfies(is_true())
        })
    }

    #[test]
    fn resolves_to_failure_captures_the_call_site_not_the_await() -> TestResult {
        // The location is captured where `resolves_to` is *called*, even
        // though the future is awaited on a later line.
        pollster::block_on(async {
            let line = line!() + 1;
            let pending = check!(async { 2 + 2 }).resolves_to(eq(5));
            let error = pending.await.expect_err("2 + 2 does not resolve to 5");
            check!(error.location.line()).satisfies(eq(line))?;
            check!(error.location.file().ends_with("subject.rs")).satisfies(is_true())
        })
    }

    #[test]
    fn snapshot_mismatch_renders_as_a_snapshot_error_with_a_diff() -> TestResult {
        use std::path::PathBuf;

        use test_better_core::ErrorKind;
        use test_better_snapshot::SnapshotFailure;

        let failure = SnapshotFailure::Mismatch {
            path: PathBuf::from("tests/snapshots/m__page.snap"),
            expected: "line one\nline two".to_string(),
            actual: "line one\nline TWO".to_string(),
        };
        let error = super::snapshot_error("page", "page", failure);
        check!(error.kind == ErrorKind::Snapshot).satisfies(is_true())?;
        let rendered = error.to_string();
        check!(rendered.contains("snapshot \"page\"")).satisfies(is_true())?;
        // The expected/actual payload renders, and `diff` is on by default.
        check!(rendered.contains("line one")).satisfies(is_true())?;
        #[cfg(feature = "diff")]
        check!(rendered.contains("-line two")).satisfies(is_true())?;
        Ok(())
    }

    #[test]
    fn snapshot_missing_renders_as_a_snapshot_error_pointing_at_update() -> TestResult {
        use std::path::PathBuf;

        use test_better_core::ErrorKind;
        use test_better_snapshot::SnapshotFailure;

        let failure = SnapshotFailure::Missing {
            path: PathBuf::from("tests/snapshots/m__page.snap"),
        };
        let error = super::snapshot_error("page", "page", failure);
        check!(error.kind == ErrorKind::Snapshot).satisfies(is_true())?;
        check!(error.to_string().contains("UPDATE_SNAPSHOTS=1")).satisfies(is_true())?;
        Ok(())
    }

    #[test]
    fn inline_snapshot_mismatch_renders_as_a_snapshot_error_with_a_diff() -> TestResult {
        use test_better_core::ErrorKind;
        use test_better_snapshot::InlineSnapshotFailure;

        let failure = InlineSnapshotFailure {
            expected: "one\ntwo".to_string(),
            actual: "one\nTWO".to_string(),
        };
        let error = super::inline_snapshot_error("value", failure);
        check!(error.kind == ErrorKind::Snapshot).satisfies(is_true())?;
        let rendered = error.to_string();
        check!(rendered.contains("inline snapshot does not match")).satisfies(is_true())?;
        check!(rendered.contains("UPDATE_SNAPSHOTS=1")).satisfies(is_true())?;
        #[cfg(feature = "diff")]
        check!(rendered.contains("-two")).satisfies(is_true())?;
        Ok(())
    }
}
