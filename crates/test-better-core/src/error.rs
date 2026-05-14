//! The [`TestError`] data model: the single source of truth for a test failure.
//!
//! A `TestError` carries structured data, never pre-rendered text. Two consumers
//! read it (PROJECT_BUILD_PLAN.md §3):
//!
//! - the human renderer ([`Display`]/[`Debug`], see [`crate::render`]);
//! - tooling and the Phase 9 runner, via [`TestError::to_structured`].

use std::borrow::Cow;
use std::fmt;
use std::panic::Location;

/// The category of a [`TestError`].
///
/// The kind selects the headline of the rendered failure and lets tooling group
/// failures (a setup failure is not the same as an assertion miss).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[non_exhaustive]
pub enum ErrorKind {
    /// An assertion did not hold (the common case).
    Assertion,
    /// Test setup failed before the assertions could run (fixtures, Phase 8).
    Setup,
    /// An operation did not complete within its deadline (Phase 5).
    Timeout,
    /// A snapshot did not match its stored value (Phase 7).
    Snapshot,
    /// A property failed for some generated input (Phase 6).
    Property,
    /// A failure that does not fit the other kinds, including errors propagated
    /// from non-`test-better` code via `?`.
    Custom,
}

impl ErrorKind {
    /// The headline shown on the first line of a rendered failure.
    #[must_use]
    pub fn headline(self) -> &'static str {
        match self {
            ErrorKind::Assertion => "assertion failed",
            ErrorKind::Setup => "test setup failed",
            ErrorKind::Timeout => "timed out",
            ErrorKind::Snapshot => "snapshot mismatch",
            ErrorKind::Property => "property failed",
            ErrorKind::Custom => "test failed",
        }
    }
}

/// One human-readable frame in a [`TestError`]'s context chain.
///
/// Frames render in the order they were added, so the chain reads from the
/// outermost circumstance to the innermost.
#[derive(Debug, Clone)]
pub struct ContextFrame {
    /// The "while doing X" description.
    pub message: Cow<'static, str>,
    /// Where the frame was attached, when known.
    pub location: Option<&'static Location<'static>>,
}

impl ContextFrame {
    /// Creates a frame, capturing the caller's location.
    #[track_caller]
    #[must_use]
    pub fn new(message: impl Into<Cow<'static, str>>) -> Self {
        Self {
            message: message.into(),
            location: Some(Location::caller()),
        }
    }
}

/// Structured detail attached to a [`TestError`] beyond its message.
#[derive(Debug)]
#[non_exhaustive]
pub enum Payload {
    /// A comparison failure carrying the expected and actual values, and an
    /// optional structural diff (the diff is populated from Phase 2 onward).
    ExpectedActual {
        /// `Debug`-rendered expected value.
        expected: String,
        /// `Debug`-rendered actual value.
        actual: String,
        /// Optional pre-rendered diff between the two.
        diff: Option<String>,
    },
    /// Several failures collected together (soft assertions, Phase 4).
    Multiple(Vec<TestError>),
    /// An error propagated from outside `test-better`, preserved so its source
    /// chain stays walkable.
    Other(Box<dyn std::error::Error + Send + Sync>),
}

/// A test failure.
///
/// Every fallible `test-better` operation produces a `TestError` on the error
/// path, so `?` is the single control-flow operator of a test
/// (PROJECT_BUILD_PLAN.md §1).
///
/// # Note on the `message` field
///
/// PROJECT_BUILD_PLAN.md §6.1 sketches `TestError` without a top-level
/// `message`. A dedicated `message` field is kept here instead of overloading
/// the first context frame: the message answers *what* failed, while context
/// frames answer *while doing what*. This deviation is recorded in
/// `CHANGELOG.md`.
pub struct TestError {
    /// The failure category.
    pub kind: ErrorKind,
    /// What failed, when there is a concise statement of it.
    pub message: Option<Cow<'static, str>>,
    /// Where the failure originated (`#[track_caller]` capture).
    pub location: &'static Location<'static>,
    /// The context chain, outermost first.
    pub context: Vec<ContextFrame>,
    /// Structured detail, when applicable.
    ///
    /// Boxed so `TestError` stays small: it is returned by value through every
    /// `?` in a test, and [`Payload::ExpectedActual`] would otherwise inline
    /// three `String`s into the struct.
    pub payload: Option<Box<Payload>>,
}

impl TestError {
    /// Builds a bare error at an explicit location. Internal: the public
    /// surface is the `#[track_caller]` constructors, which capture the
    /// caller's location for themselves.
    pub(crate) fn at(kind: ErrorKind, location: &'static Location<'static>) -> Self {
        Self {
            kind,
            message: None,
            location,
            context: Vec::new(),
            payload: None,
        }
    }

    /// Creates a bare error of the given `kind`, capturing the caller's location.
    #[track_caller]
    #[must_use]
    pub fn new(kind: ErrorKind) -> Self {
        Self::at(kind, Location::caller())
    }

    /// Creates an [`ErrorKind::Assertion`] error with the given message.
    ///
    /// This is the common path for a hand-written failure: `return
    /// Err(TestError::assertion("..."))`.
    #[track_caller]
    #[must_use]
    pub fn assertion(message: impl Into<Cow<'static, str>>) -> Self {
        Self::at(ErrorKind::Assertion, Location::caller()).with_message(message)
    }

    /// Creates an [`ErrorKind::Custom`] error with the given message, for a
    /// failure that does not fit a more specific kind.
    #[track_caller]
    #[must_use]
    pub fn custom(message: impl Into<Cow<'static, str>>) -> Self {
        Self::at(ErrorKind::Custom, Location::caller()).with_message(message)
    }

    /// Creates an [`ErrorKind::Assertion`] error from a mismatched
    /// expected/actual pair, capturing each value's `Debug` representation into
    /// a [`Payload::ExpectedActual`].
    #[track_caller]
    #[must_use]
    pub fn from_expected_actual(expected: impl fmt::Debug, actual: impl fmt::Debug) -> Self {
        Self::at(ErrorKind::Assertion, Location::caller()).with_payload(Payload::ExpectedActual {
            expected: format!("{expected:?}"),
            actual: format!("{actual:?}"),
            diff: None,
        })
    }

    /// Sets the [`message`](Self::message), consuming and returning `self`.
    #[must_use]
    pub fn with_message(mut self, message: impl Into<Cow<'static, str>>) -> Self {
        self.message = Some(message.into());
        self
    }

    /// Sets the [`payload`](Self::payload), consuming and returning `self`.
    #[must_use]
    pub fn with_payload(mut self, payload: Payload) -> Self {
        self.payload = Some(Box::new(payload));
        self
    }

    /// Appends a context frame, consuming and returning `self`.
    #[must_use]
    pub fn with_context_frame(mut self, frame: ContextFrame) -> Self {
        self.context.push(frame);
        self
    }

    /// Appends a context frame in place.
    pub fn push_context(&mut self, frame: ContextFrame) {
        self.context.push(frame);
    }
}

impl fmt::Display for TestError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        crate::render::render(self, f)
    }
}

impl fmt::Debug for TestError {
    /// Renders the full pretty failure message, so the stock `cargo test`
    /// harness (which prints returned errors with `{:?}`) is already useful
    /// (PROJECT_BUILD_PLAN.md §6.1). Phase 2 adds optional ANSI color here;
    /// `Display` stays plain.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        crate::render::render(self, f)
    }
}

impl std::error::Error for TestError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self.payload.as_deref() {
            Some(Payload::Other(inner)) => Some(inner.as_ref()),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[track_caller]
    fn sample_assertion() -> TestError {
        TestError::new(ErrorKind::Assertion).with_message("values differ")
    }

    #[test]
    fn new_captures_caller_location() {
        let line = line!() + 1;
        let error = TestError::new(ErrorKind::Assertion);
        assert_eq!(error.location.line(), line);
        assert!(error.location.file().ends_with("error.rs"));
    }

    #[test]
    fn display_includes_headline_message_and_location() {
        let rendered = sample_assertion().to_string();
        assert!(
            rendered.contains("assertion failed: values differ"),
            "{rendered}"
        );
        assert!(rendered.contains("  at "), "{rendered}");
    }

    #[test]
    fn debug_matches_display() {
        let error = sample_assertion();
        assert_eq!(format!("{error:?}"), format!("{error}"));
    }

    #[test]
    fn context_frames_render_in_order() {
        let error = sample_assertion()
            .with_context_frame(ContextFrame::new("creating user"))
            .with_context_frame(ContextFrame::new("loading profile"));
        let rendered = error.to_string();
        let first = rendered.find("creating user").expect("first frame present");
        let second = rendered
            .find("loading profile")
            .expect("second frame present");
        assert!(first < second, "frames out of order:\n{rendered}");
    }

    #[test]
    fn error_source_walks_into_payload_other() {
        let io = std::io::Error::new(std::io::ErrorKind::NotFound, "missing file");
        let error = TestError::new(ErrorKind::Custom).with_payload(Payload::Other(Box::new(io)));
        let source = std::error::Error::source(&error).expect("source is the wrapped io error");
        assert_eq!(source.to_string(), "missing file");
    }

    #[test]
    fn expected_actual_payload_renders_both_values() {
        let error = TestError::new(ErrorKind::Assertion).with_payload(Payload::ExpectedActual {
            expected: "4".to_string(),
            actual: "5".to_string(),
            diff: None,
        });
        let rendered = error.to_string();
        assert!(rendered.contains("expected: 4"), "{rendered}");
        assert!(rendered.contains("actual: 5"), "{rendered}");
    }

    #[test]
    fn multiple_payload_renders_every_sub_failure() {
        let error = TestError::new(ErrorKind::Assertion).with_payload(Payload::Multiple(vec![
            TestError::new(ErrorKind::Assertion).with_message("first"),
            TestError::new(ErrorKind::Assertion).with_message("second"),
        ]));
        let rendered = error.to_string();
        assert!(rendered.contains("first"), "{rendered}");
        assert!(rendered.contains("second"), "{rendered}");
    }

    #[test]
    fn assertion_constructor_sets_kind_message_and_caller_location() {
        let line = line!() + 1;
        let error = TestError::assertion("values differ");
        assert_eq!(error.kind, ErrorKind::Assertion);
        assert_eq!(error.message.as_deref(), Some("values differ"));
        assert_eq!(error.location.line(), line);
        assert!(error.location.file().ends_with("error.rs"));
    }

    #[test]
    fn custom_constructor_sets_kind_message_and_caller_location() {
        let line = line!() + 1;
        let error = TestError::custom("something off");
        assert_eq!(error.kind, ErrorKind::Custom);
        assert_eq!(error.message.as_deref(), Some("something off"));
        assert_eq!(error.location.line(), line);
        assert!(error.location.file().ends_with("error.rs"));
    }

    #[test]
    fn from_expected_actual_captures_debug_values_and_caller_location() {
        let line = line!() + 1;
        let error = TestError::from_expected_actual(4, 5);
        assert_eq!(error.kind, ErrorKind::Assertion);
        assert_eq!(error.location.line(), line);
        match error.payload.map(|payload| *payload) {
            Some(Payload::ExpectedActual {
                expected,
                actual,
                diff,
            }) => {
                assert_eq!(expected, "4");
                assert_eq!(actual, "5");
                assert!(diff.is_none());
            }
            other => panic!("expected ExpectedActual, got {other:?}"),
        }
    }
}
