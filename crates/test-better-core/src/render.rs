//! Human-readable rendering of a [`TestError`].
//!
//! This is the *only* place a `TestError` is turned into text. Tooling never
//! parses this output; it reads [`TestError::to_structured`] instead
//! (PROJECT_BUILD_PLAN.md §3).
//!
//! Phase 1 renders without color. Phase 2 (Iteration 2.4) adds an optional ANSI
//! layer here, driven by `set_color_choice`; `Display` stays plain while `Debug`
//! may colorize.

use std::fmt;

use crate::error::{Payload, TestError};

/// Renders `error` into `f`. Produces no trailing newline, so a rendered error
/// composes cleanly when indented inside a [`Payload::Multiple`].
pub(crate) fn render(error: &TestError, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match &error.message {
        Some(message) => writeln!(f, "{}: {message}", error.kind.headline())?,
        None => writeln!(f, "{}", error.kind.headline())?,
    }

    for frame in &error.context {
        writeln!(f, "  while {}", frame.message)?;
    }

    if let Some(payload) = &error.payload {
        render_payload(payload, f)?;
    }

    write!(f, "  at {}", error.location)
}

fn render_payload(payload: &Payload, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match payload {
        Payload::ExpectedActual {
            expected,
            actual,
            diff,
        } => {
            // The labels are padded so the two values line up.
            writeln!(f, "  expected: {expected}")?;
            writeln!(f, "    actual: {actual}")?;
            if let Some(diff) = diff {
                for line in diff.lines() {
                    writeln!(f, "  {line}")?;
                }
            }
        }
        Payload::Other(inner) => {
            writeln!(f, "  caused by: {inner}")?;
            let mut source = inner.source();
            while let Some(current) = source {
                writeln!(f, "    caused by: {current}")?;
                source = current.source();
            }
        }
        Payload::Multiple(errors) => {
            let count = errors.len();
            let noun = if count == 1 { "failure" } else { "failures" };
            writeln!(f, "  {count} {noun}:")?;
            for (index, sub) in errors.iter().enumerate() {
                writeln!(f, "  [{}]", index + 1)?;
                // `sub` renders via `Display`; indent every line of it.
                let rendered = sub.to_string();
                for line in rendered.lines() {
                    writeln!(f, "      {line}")?;
                }
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::error::{ContextFrame, ErrorKind, Payload, TestError};

    #[test]
    fn render_has_no_trailing_newline() {
        let rendered = TestError::new(ErrorKind::Assertion).to_string();
        assert!(!rendered.ends_with('\n'), "{rendered:?}");
    }

    #[test]
    fn nested_multiple_indents_each_line() {
        let inner = TestError::new(ErrorKind::Assertion)
            .with_message("inner")
            .with_context_frame(ContextFrame::new("inner context"));
        let outer =
            TestError::new(ErrorKind::Assertion).with_payload(Payload::Multiple(vec![inner]));
        let rendered = outer.to_string();
        assert!(
            rendered.contains("      assertion failed: inner"),
            "{rendered}"
        );
        assert!(rendered.contains("      while inner context"), "{rendered}");
    }
}
