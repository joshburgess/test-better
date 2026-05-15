//! Human-readable rendering of a [`TestError`].
//!
//! This is the *only* place a `TestError` is turned into text. Tooling never
//! parses this output; it reads [`TestError::to_structured`] instead.
//!
//! Rendering takes a `colorize` flag. `Display` always passes
//! `false`; `Debug` passes [`crate::color::color_enabled`], so the stock
//! `cargo test` harness gets color when the environment allows it.

use std::fmt;

use crate::error::{Payload, TestError};
use crate::trace::TraceEntry;

/// ANSI escape: red foreground, for the actual value and removed diff lines.
const RED: &str = "\x1b[31m";
/// ANSI escape: green foreground, for the expected value and added diff lines.
const GREEN: &str = "\x1b[32m";
/// ANSI escape: reset all attributes.
const RESET: &str = "\x1b[0m";

/// Renders `error` into `f`. Produces no trailing newline, so a rendered error
/// composes cleanly when indented inside a [`Payload::Multiple`].
///
/// `colorize` decides whether ANSI escapes are emitted; see [`crate::color`].
pub(crate) fn render(error: &TestError, f: &mut fmt::Formatter<'_>, colorize: bool) -> fmt::Result {
    match &error.message {
        Some(message) => writeln!(f, "{}: {message}", error.kind.headline())?,
        None => writeln!(f, "{}", error.kind.headline())?,
    }

    for frame in &error.context {
        writeln!(f, "  while {}", frame.message)?;
    }

    if !error.trace.is_empty() {
        writeln!(f, "  trace:")?;
        for entry in &error.trace {
            match entry {
                TraceEntry::Step(message) => writeln!(f, "    - {message}")?,
                TraceEntry::Kv { key, value } => writeln!(f, "    - {key} = {value}")?,
            }
        }
    }

    if let Some(payload) = error.payload.as_deref() {
        render_payload(payload, f, colorize)?;
    }

    write!(f, "  at {}", error.location)
}

fn render_payload(payload: &Payload, f: &mut fmt::Formatter<'_>, colorize: bool) -> fmt::Result {
    match payload {
        Payload::ExpectedActual {
            expected,
            actual,
            diff,
        } => {
            // The labels are padded so the two values line up.
            if colorize {
                writeln!(f, "  expected: {GREEN}{expected}{RESET}")?;
                writeln!(f, "    actual: {RED}{actual}{RESET}")?;
            } else {
                writeln!(f, "  expected: {expected}")?;
                writeln!(f, "    actual: {actual}")?;
            }
            if let Some(diff) = diff {
                for line in diff.lines() {
                    match (colorize, diff_line_color(line)) {
                        (true, Some(color)) => writeln!(f, "  {color}{line}{RESET}")?,
                        _ => writeln!(f, "  {line}")?,
                    }
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
                // `sub` renders via `Display`, which is always plain; indent
                // every line of it.
                let rendered = sub.to_string();
                for line in rendered.lines() {
                    writeln!(f, "      {line}")?;
                }
            }
        }
    }
    Ok(())
}

/// The ANSI color for a diff line, by its leading marker: `-` removed (red),
/// `+` added (green), anything else unchanged context (no color).
fn diff_line_color(line: &str) -> Option<&'static str> {
    match line.as_bytes().first() {
        Some(b'-') => Some(RED),
        Some(b'+') => Some(GREEN),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use crate::color::{ColorChoice, color_choice, set_color_choice};
    use crate::error::{ContextFrame, ErrorKind, Payload, TestError};
    use crate::{OrFail, TestResult, Trace};
    use test_better_matchers::{expect, is_false, is_true};

    #[test]
    fn render_has_no_trailing_newline() -> TestResult {
        let rendered = TestError::new(ErrorKind::Assertion).to_string();
        expect!(rendered.ends_with('\n')).to(is_false()).or_fail()?;
        Ok(())
    }

    #[test]
    fn nested_multiple_indents_each_line() -> TestResult {
        let inner = TestError::new(ErrorKind::Assertion)
            .with_message("inner")
            .with_context_frame(ContextFrame::new("inner context"));
        let outer =
            TestError::new(ErrorKind::Assertion).with_payload(Payload::Multiple(vec![inner]));
        let rendered = outer.to_string();
        expect!(rendered.contains("      assertion failed: inner"))
            .to(is_true())
            .or_fail()?;
        expect!(rendered.contains("      while inner context"))
            .to(is_true())
            .or_fail()?;
        Ok(())
    }

    #[test]
    fn trace_breadcrumbs_render_in_chronological_order() -> TestResult {
        let mut trace = Trace::new();
        trace.step("connecting to db");
        trace.kv("db_url", "postgres://localhost");
        trace.step("running the query");
        let error = TestError::new(ErrorKind::Assertion).with_message("query returned no rows");
        drop(trace);

        let rendered = error.to_string();
        let connect = rendered
            .find("- connecting to db")
            .or_fail_with("step rendered")?;
        let url = rendered
            .find("- db_url = postgres://localhost")
            .or_fail_with("kv rendered")?;
        let query = rendered
            .find("- running the query")
            .or_fail_with("second step rendered")?;
        expect!(connect < url).to(is_true()).or_fail()?;
        expect!(url < query).to(is_true()).or_fail()?;
        Ok(())
    }

    #[test]
    fn debug_colorizes_only_when_color_is_on() -> TestResult {
        let _guard = crate::color::TEST_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let original = color_choice();

        let error = TestError::new(ErrorKind::Assertion).with_payload(Payload::ExpectedActual {
            expected: "line one\nline two".to_string(),
            actual: "line one\nline 2".to_string(),
            diff: Some("-line two\n+line 2".to_string()),
        });

        // `Always`: `Debug` emits ANSI, including red removals and green adds.
        set_color_choice(ColorChoice::Always);
        let colored = format!("{error:?}");

        // `Never`: `Debug` stays plain.
        set_color_choice(ColorChoice::Never);
        let plain = format!("{error:?}");

        // `Display` is plain even with color forced on.
        set_color_choice(ColorChoice::Always);
        let display = format!("{error}");

        // Restore before any `?` to avoid skipping the restore on early return.
        set_color_choice(original);

        expect!(colored.contains("\x1b[31m"))
            .to(is_true())
            .or_fail()?;
        expect!(colored.contains("\x1b[32m"))
            .to(is_true())
            .or_fail()?;
        expect!(plain.contains('\x1b')).to(is_false()).or_fail()?;
        expect!(display.contains('\x1b')).to(is_false()).or_fail()?;
        Ok(())
    }
}
