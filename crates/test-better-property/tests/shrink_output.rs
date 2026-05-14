//! Golden-file test for the shrunk-failure rendering (PROJECT_BUILD_PLAN.md
//! Iteration 6.3).
//!
//! `render_failure` is exactly what `property!` uses to turn a counterexample
//! into a `TestError`, so pinning its output here pins the user-visible
//! shrunk-failure message: the original input, the shrunk input, and the
//! matcher's own structured description.
//!
//! The `PropertyFailure` is hand-built rather than produced by `check`, so the
//! golden file is fully deterministic and not coupled to the backend's RNG.
//! Run with `BLESS_GOLDEN=1` to regenerate the golden file after an
//! intentional rendering change.

use std::fs;
use std::path::PathBuf;

use test_better_core::{ErrorKind, OrFail, Payload, TestError, TestResult};
use test_better_matchers::{eq, expect};
use test_better_property::{PropertyFailure, render_failure};

#[test]
fn a_shrunk_property_failure_renders_to_the_golden_file() -> TestResult {
    // A failure shaped like a real one: `expect!(n).to(lt(100))` against the
    // shrunk input 100, after the search walked down from a large original.
    let matcher_failure = TestError::new(ErrorKind::Assertion)
        .with_message("expect!(n)")
        .with_payload(Payload::ExpectedActual {
            expected: "less than 100".to_string(),
            actual: "100".to_string(),
            diff: None,
        });
    let failure = PropertyFailure {
        original: 3_000_000_000u32,
        shrunk: 100u32,
        failure: matcher_failure,
        cases: 47,
    };

    let rendered = render_failure(failure).to_string();
    // The trailing `  at <path>:<line>:<col>` line is environment-specific;
    // normalize it so the golden file pins the rendering, not the file layout.
    let normalized = normalize_location(&rendered);

    let golden_path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/golden/shrunk_failure.txt");
    if std::env::var_os("BLESS_GOLDEN").is_some() {
        fs::write(&golden_path, format!("{normalized}\n")).or_fail()?;
        return Ok(());
    }
    let golden = fs::read_to_string(&golden_path).or_fail()?;
    expect!(normalized.as_str()).to(eq(golden.trim_end_matches('\n')))
}

/// Replaces the final `  at ...` line with a stable placeholder so the golden
/// file does not depend on this test's path or line numbers.
fn normalize_location(rendered: &str) -> String {
    let mut lines: Vec<&str> = rendered.lines().collect();
    if let Some(last) = lines.last_mut()
        && last.trim_start().starts_with("at ")
    {
        *last = "  at <location>";
    }
    lines.join("\n")
}
