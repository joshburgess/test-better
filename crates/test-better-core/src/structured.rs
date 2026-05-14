//! The structured (plain-data) form of a [`TestError`].
//!
//! [`TestError`] holds borrowed data (`&'static Location`, `Cow<'static, str>`)
//! and a non-cloneable `Box<dyn Error>` payload, which makes it awkward to
//! serialize, compare, or send across a process boundary. [`StructuredError`]
//! is its fully-owned, `PartialEq`, optionally-`serde` mirror.
//!
//! This is the form tooling and the Phase 9 runner consume: no consumer ever
//! recovers structure by parsing rendered text.

use std::panic::Location;

use crate::error::{ErrorKind, Payload, TestError};
use crate::trace::TraceEntry;

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
    /// The in-test breadcrumbs active when the error was built, oldest first.
    /// [`TraceEntry`] is already plain data, so it is its own structured form.
    pub trace: Vec<TraceEntry>,
    /// Structured detail, when applicable.
    pub payload: Option<StructuredPayload>,
}

impl TestError {
    /// Converts this error into its structured, owned, serializable form.
    ///
    /// This is the boundary between `test-better` and any tooling that consumes
    /// failures: tooling reads the structured form, never the rendered text.
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
            trace: self.trace.clone(),
            payload: self.payload.as_deref().map(StructuredPayload::from_payload),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::ContextFrame;
    use crate::{OrFail, TestResult, Trace};
    use test_better_matchers::{eq, expect, is_true};

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
    fn every_kind_round_trips_through_structured() -> TestResult {
        for kind in all_kinds() {
            let error = TestError::new(kind).with_message("boom");
            let structured = error.to_structured();
            expect!(structured.kind).to(eq(kind)).or_fail()?;
            expect!(structured.message.as_deref())
                .to(eq(Some("boom")))
                .or_fail()?;
        }
        Ok(())
    }

    #[test]
    fn structured_captures_location_and_context() -> TestResult {
        let error =
            TestError::new(ErrorKind::Assertion).with_context_frame(ContextFrame::new("step one"));
        let structured = error.to_structured();
        expect!(structured.context.len()).to(eq(1)).or_fail()?;
        expect!(structured.context[0].message.as_str())
            .to(eq("step one"))
            .or_fail()?;
        expect!(structured.location.file.ends_with("structured.rs"))
            .to(is_true())
            .or_fail()?;
        expect!(structured.location.line > 0)
            .to(is_true())
            .or_fail()?;
        Ok(())
    }

    #[test]
    fn structured_carries_the_trace() -> TestResult {
        let mut trace = Trace::new();
        trace.step("step one");
        trace.kv("answer", 42);
        let error = TestError::new(ErrorKind::Assertion);
        drop(trace);

        let structured = error.to_structured();
        expect!(structured.trace.len()).to(eq(2)).or_fail()?;
        expect!(structured.trace[0].clone())
            .to(eq(TraceEntry::Step("step one".into())))
            .or_fail()?;
        Ok(())
    }

    #[test]
    fn expected_actual_payload_round_trips() -> TestResult {
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
                expect!(expected).to(eq("1".to_string())).or_fail()?;
                expect!(actual).to(eq("2".to_string())).or_fail()?;
                expect!(diff.as_deref())
                    .to(eq(Some("- 1\n+ 2")))
                    .or_fail()?;
            }
            other => panic!("expected ExpectedActual, got {other:?}"),
        }
        Ok(())
    }

    #[test]
    fn multiple_payload_round_trips_recursively() -> TestResult {
        let error = TestError::new(ErrorKind::Assertion).with_payload(Payload::Multiple(vec![
            TestError::new(ErrorKind::Assertion).with_message("a"),
            TestError::new(ErrorKind::Setup).with_message("b"),
        ]));
        match error.to_structured().payload {
            Some(StructuredPayload::Multiple(subs)) => {
                expect!(subs.len()).to(eq(2)).or_fail()?;
                expect!(subs[0].message.as_deref())
                    .to(eq(Some("a")))
                    .or_fail()?;
                expect!(subs[1].kind).to(eq(ErrorKind::Setup)).or_fail()?;
            }
            other => panic!("expected Multiple, got {other:?}"),
        }
        Ok(())
    }

    #[test]
    fn other_payload_flattens_error_chain() -> TestResult {
        let io = std::io::Error::new(std::io::ErrorKind::NotFound, "missing");
        let error = TestError::new(ErrorKind::Custom).with_payload(Payload::Other(Box::new(io)));
        match error.to_structured().payload {
            Some(StructuredPayload::Other { message, chain }) => {
                expect!(message).to(eq("missing".to_string())).or_fail()?;
                expect!(chain.is_empty()).to(is_true()).or_fail()?;
            }
            other => panic!("expected Other, got {other:?}"),
        }
        Ok(())
    }

    #[cfg(feature = "serde")]
    #[test]
    fn structured_error_json_round_trips() -> TestResult {
        let error = TestError::new(ErrorKind::Property)
            .with_message("shrunk input failed")
            .with_context_frame(ContextFrame::new("checking the round-trip property"))
            .with_payload(Payload::ExpectedActual {
                expected: "Ok(\"x\")".to_string(),
                actual: "Err(..)".to_string(),
                diff: None,
            });
        let structured = error.to_structured();
        let json = serde_json::to_string(&structured).or_fail_with("serialize")?;
        let back: StructuredError = serde_json::from_str(&json).or_fail_with("deserialize")?;
        expect!(structured).to(eq(back)).or_fail()?;
        Ok(())
    }
}
