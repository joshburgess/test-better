//! [`Trace`]: in-test breadcrumbs.
//!
//! A `Trace` records a chronological list of steps and key/value pairs while a
//! test runs. The entries live in a thread-local for the trace's lifetime, so
//! every [`TestError`](crate::TestError) built while the trace is in scope
//! snapshots them automatically. A failure then renders the breadcrumbs that
//! led up to it, in the order they happened, with no need to thread the trace
//! value through the code under test.
//!
//! `cargo test` runs each test on its own thread, so a thread-local is per-test
//! in practice. The one caveat is async: if a runtime moves a task across
//! threads, a `TestError` constructed after the move snapshots the wrong
//! thread's trace (usually an empty one). Keep a `Trace` within a single
//! synchronous span, or within one async task that is not migrated.

use std::borrow::Cow;
use std::cell::RefCell;
use std::fmt;

/// One breadcrumb recorded on a [`Trace`].
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[non_exhaustive]
pub enum TraceEntry {
    /// A narrative step, recorded by [`Trace::step`].
    Step(Cow<'static, str>),
    /// A key/value pair, recorded by [`Trace::kv`].
    Kv {
        /// The key.
        key: Cow<'static, str>,
        /// The value, rendered to text when the breadcrumb was recorded.
        value: String,
    },
}

thread_local! {
    /// The active trace's entries for the current thread, or `None` when no
    /// `Trace` is in scope.
    static ACTIVE: RefCell<Option<Vec<TraceEntry>>> = const { RefCell::new(None) };
}

/// A scoped collector of in-test breadcrumbs.
///
/// Construct one at the top of a test; every [`TestError`](crate::TestError)
/// built before it is dropped carries a snapshot of the breadcrumbs recorded so
/// far, and renders them in the failure output.
///
/// ```
/// use test_better_core::Trace;
///
/// let mut trace = Trace::new();
/// trace.step("connecting to db");
/// trace.kv("db_url", "postgres://localhost/test");
/// trace.step("running the query");
/// // If an assertion fails here, these three breadcrumbs are attached to the
/// // resulting `TestError` and shown, in order, in the rendered failure.
/// ```
///
/// Dropping the `Trace` ends the scope. Nested traces compose: an inner
/// `Trace::new()` displaces the outer trace's entries and restores them on
/// drop, so the outer trace resumes intact.
pub struct Trace {
    /// The thread-local entries displaced by this `Trace`, restored on drop.
    /// `None` is the common case: no outer trace was in scope.
    previous: Option<Vec<TraceEntry>>,
}

impl Trace {
    /// Starts a trace, collecting breadcrumbs until it is dropped.
    #[must_use]
    pub fn new() -> Self {
        let previous = ACTIVE.with(|cell| cell.borrow_mut().replace(Vec::new()));
        Self { previous }
    }

    /// Records a narrative step.
    pub fn step(&mut self, message: impl Into<Cow<'static, str>>) {
        let entry = TraceEntry::Step(message.into());
        ACTIVE.with(|cell| {
            if let Some(entries) = cell.borrow_mut().as_mut() {
                entries.push(entry);
            }
        });
    }

    /// Records a key/value breadcrumb, rendering `value` with [`Display`] now,
    /// so the breadcrumb is not tied to the value's lifetime.
    ///
    /// [`Display`]: std::fmt::Display
    pub fn kv(&mut self, key: impl Into<Cow<'static, str>>, value: impl fmt::Display) {
        let entry = TraceEntry::Kv {
            key: key.into(),
            value: value.to_string(),
        };
        ACTIVE.with(|cell| {
            if let Some(entries) = cell.borrow_mut().as_mut() {
                entries.push(entry);
            }
        });
    }

    /// The breadcrumbs recorded in the active trace so far, oldest first.
    #[must_use]
    pub fn entries(&self) -> Vec<TraceEntry> {
        snapshot()
    }
}

impl Default for Trace {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for Trace {
    fn drop(&mut self) {
        ACTIVE.with(|cell| *cell.borrow_mut() = self.previous.take());
    }
}

/// Snapshots the active thread's trace entries, for [`TestError`] construction.
/// Empty when no `Trace` is in scope.
///
/// [`TestError`]: crate::TestError
pub(crate) fn snapshot() -> Vec<TraceEntry> {
    ACTIVE.with(|cell| cell.borrow().clone().unwrap_or_default())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ErrorKind, OrFail, TestError, TestResult};
    use test_better_matchers::{eq, expect, is_true};

    #[test]
    fn steps_and_kv_are_recorded_in_order() -> TestResult {
        let mut trace = Trace::new();
        trace.step("first");
        trace.kv("key", 42);
        trace.step("second");
        let entries = trace.entries();
        expect!(entries.len()).to(eq(3)).or_fail()?;
        expect!(entries[0].clone())
            .to(eq(TraceEntry::Step("first".into())))
            .or_fail()?;
        expect!(entries[1].clone())
            .to(eq(TraceEntry::Kv {
                key: "key".into(),
                value: "42".to_string(),
            }))
            .or_fail()?;
        expect!(entries[2].clone())
            .to(eq(TraceEntry::Step("second".into())))
            .or_fail()?;
        Ok(())
    }

    #[test]
    fn an_error_built_within_a_trace_snapshots_it() -> TestResult {
        let mut trace = Trace::new();
        trace.step("doing the thing");
        let error = TestError::new(ErrorKind::Assertion);
        expect!(error.trace.len()).to(eq(1)).or_fail()?;
        expect!(error.trace[0].clone())
            .to(eq(TraceEntry::Step("doing the thing".into())))
            .or_fail()?;
        Ok(())
    }

    #[test]
    fn an_error_built_with_no_trace_in_scope_has_an_empty_trace() -> TestResult {
        let error = TestError::new(ErrorKind::Assertion);
        expect!(error.trace.is_empty()).to(is_true()).or_fail()?;
        Ok(())
    }

    #[test]
    fn dropping_a_trace_ends_the_scope() -> TestResult {
        {
            let mut trace = Trace::new();
            trace.step("inside the scope");
        }
        // The trace is dropped; a later error captures nothing.
        let error = TestError::new(ErrorKind::Assertion);
        expect!(error.trace.is_empty()).to(is_true()).or_fail()?;
        Ok(())
    }

    #[test]
    fn nested_traces_compose_and_restore() -> TestResult {
        let mut outer = Trace::new();
        outer.step("outer step");
        {
            let mut inner = Trace::new();
            inner.step("inner step");
            expect!(inner.entries().len()).to(eq(1)).or_fail()?;
        }
        // The inner trace is gone; the outer trace's entry is back.
        let entries = outer.entries();
        expect!(entries.len()).to(eq(1)).or_fail()?;
        expect!(entries[0].clone())
            .to(eq(TraceEntry::Step("outer step".into())))
            .or_fail()?;
        Ok(())
    }
}
