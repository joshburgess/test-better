//! The structured (plain-data) form of a [`TestError`].
//!
//! [`TestError`] holds borrowed data (`&'static Location`, `Cow<'static, str>`)
//! and a non-cloneable `Box<dyn Error>` payload, which makes it awkward to
//! serialize, compare, or send across a process boundary. [`StructuredError`]
//! is its fully-owned, `PartialEq`, optionally-`serde` mirror.
//!
//! Per PROJECT_BUILD_PLAN.md §3, this is the form tooling and the Phase 9 runner
//! consume: no consumer ever recovers structure by parsing rendered text.

use std::panic::Location;

use crate::error::{ErrorKind, Payload, TestError};

/// A source location, owned and serializable (the plain-data form of
/// [`std::panic::Location`]).
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SourceLocation {
    /// Source file path, as reported by the compiler.
    pub file: String,
    /// 1-based line number.
    pub line: u32,
    /// 1-based column number.
    pub column: u32,
}

impl SourceLocation {
    fn from_std(location: &Location<'_>) -> Self {
        Self {
            file: location.file().to_string(),
            line: location.line(),
            column: location.column(),
        }
    }
}

/// The plain-data form of [`crate::ContextFrame`].
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StructuredContextFrame {
    /// The "while doing X" description.
    pub message: String,
    /// Where the frame was attached, when known.
    pub location: Option<SourceLocation>,
}

/// The plain-data form of [`Payload`].
///
/// [`Payload::Other`] holds a `Box<dyn Error>`, which cannot be serialized; it
/// is flattened here into its `Display` string plus its source chain.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum StructuredPayload {
    /// Mirrors [`Payload::ExpectedActual`].
    ExpectedActual {
        /// `Debug`-rendered expected value.
        expected: String,
        /// `Debug`-rendered actual value.
        actual: String,
        /// Optional pre-rendered diff between the two.
        diff: Option<String>,
    },
    /// Mirrors [`Payload::Multiple`].
    Multiple(Vec<StructuredError>),
    /// Mirrors [`Payload::Other`], flattened to strings.
    Other {
        /// `Display` of the wrapped error.
        message: String,
        /// `Display` of each error in the wrapped error's source chain.
        chain: Vec<String>,
    },
}

impl StructuredPayload {
    fn from_payload(payload: &Payload) -> Self {
        match payload {
            Payload::ExpectedActual {
                expected,
                actual,
                diff,
            } => StructuredPayload::ExpectedActual {
                expected: expected.clone(),
                actual: actual.clone(),
                diff: diff.clone(),
            },
            Payload::Multiple(errors) => {
                StructuredPayload::Multiple(errors.iter().map(TestError::to_structured).collect())
            }
            Payload::Other(inner) => {
                let mut chain = Vec::new();
                let mut source = inner.source();
                while let Some(current) = source {
                    chain.push(current.to_string());
                    source = current.source();
                }
                StructuredPayload::Other {
                    message: inner.to_string(),
                    chain,
                }
            }
        }
    }
}

/// The plain-data, owned, serializable mirror of [`TestError`].
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StructuredError {
    /// The failure category.
    pub kind: ErrorKind,
    /// What failed, when stated concisely.
    pub message: Option<String>,
    /// Where the failure originated.
    pub location: SourceLocation,
    /// The context chain, outermost first.
    pub context: Vec<StructuredContextFrame>,
    /// Structured detail, when applicable.
    pub payload: Option<StructuredPayload>,
}

impl TestError {
    /// Converts this error into its structured, owned, serializable form.
    ///
    /// This is the boundary between `test-better` and any tooling that consumes
    /// failures (PROJECT_BUILD_PLAN.md §3): tooling reads the structured form,
    /// never the rendered text.
    #[must_use]
    pub fn to_structured(&self) -> StructuredError {
        StructuredError {
            kind: self.kind,
            message: self.message.as_ref().map(ToString::to_string),
            location: SourceLocation::from_std(self.location),
            context: self
                .context
                .iter()
                .map(|frame| StructuredContextFrame {
                    message: frame.message.to_string(),
                    location: frame.location.map(SourceLocation::from_std),
                })
                .collect(),
            payload: self.payload.as_ref().map(StructuredPayload::from_payload),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::ContextFrame;

    fn all_kinds() -> [ErrorKind; 6] {
        [
            ErrorKind::Assertion,
            ErrorKind::Setup,
            ErrorKind::Timeout,
            ErrorKind::Snapshot,
            ErrorKind::Property,
            ErrorKind::Custom,
        ]
    }

    #[test]
    fn every_kind_round_trips_through_structured() {
        for kind in all_kinds() {
            let error = TestError::new(kind).with_message("boom");
            let structured = error.to_structured();
            assert_eq!(structured.kind, kind);
            assert_eq!(structured.message.as_deref(), Some("boom"));
        }
    }

    #[test]
    fn structured_captures_location_and_context() {
        let error =
            TestError::new(ErrorKind::Assertion).with_context_frame(ContextFrame::new("step one"));
        let structured = error.to_structured();
        assert_eq!(structured.context.len(), 1);
        assert_eq!(structured.context[0].message, "step one");
        assert!(structured.location.file.ends_with("structured.rs"));
        assert!(structured.location.line > 0);
    }

    #[test]
    fn expected_actual_payload_round_trips() {
        let error = TestError::new(ErrorKind::Assertion).with_payload(Payload::ExpectedActual {
            expected: "1".to_string(),
            actual: "2".to_string(),
            diff: Some("- 1\n+ 2".to_string()),
        });
        match error.to_structured().payload {
            Some(StructuredPayload::ExpectedActual {
                expected,
                actual,
                diff,
            }) => {
                assert_eq!(expected, "1");
                assert_eq!(actual, "2");
                assert_eq!(diff.as_deref(), Some("- 1\n+ 2"));
            }
            other => panic!("expected ExpectedActual, got {other:?}"),
        }
    }

    #[test]
    fn multiple_payload_round_trips_recursively() {
        let error = TestError::new(ErrorKind::Assertion).with_payload(Payload::Multiple(vec![
            TestError::new(ErrorKind::Assertion).with_message("a"),
            TestError::new(ErrorKind::Setup).with_message("b"),
        ]));
        match error.to_structured().payload {
            Some(StructuredPayload::Multiple(subs)) => {
                assert_eq!(subs.len(), 2);
                assert_eq!(subs[0].message.as_deref(), Some("a"));
                assert_eq!(subs[1].kind, ErrorKind::Setup);
            }
            other => panic!("expected Multiple, got {other:?}"),
        }
    }

    #[test]
    fn other_payload_flattens_error_chain() {
        let io = std::io::Error::new(std::io::ErrorKind::NotFound, "missing");
        let error = TestError::new(ErrorKind::Custom).with_payload(Payload::Other(Box::new(io)));
        match error.to_structured().payload {
            Some(StructuredPayload::Other { message, chain }) => {
                assert_eq!(message, "missing");
                assert!(chain.is_empty());
            }
            other => panic!("expected Other, got {other:?}"),
        }
    }

    #[cfg(feature = "serde")]
    #[test]
    fn structured_error_json_round_trips() {
        let error = TestError::new(ErrorKind::Property)
            .with_message("shrunk input failed")
            .with_context_frame(ContextFrame::new("checking the round-trip property"))
            .with_payload(Payload::ExpectedActual {
                expected: "Ok(\"x\")".to_string(),
                actual: "Err(..)".to_string(),
                diff: None,
            });
        let structured = error.to_structured();
        let json = serde_json::to_string(&structured).expect("serialize");
        let back: StructuredError = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(structured, back);
    }
}
