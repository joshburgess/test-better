//! Color choice for rendered failures.
//!
//! Color is owned here, in `core`, not in `matchers` (PROJECT_BUILD_PLAN.md
//! §7, Iteration 2.4): the renderer that backs `TestError`'s `Debug`/`Display`
//! lives in `core`, so this is the one place that decides whether ANSI escapes
//! are emitted. `matchers` only ever produces structured, uncolored data.
//!
//! [`Display`](std::fmt::Display) is always plain. [`Debug`](std::fmt::Debug)
//! may colorize, because that is what the stock `cargo test` harness prints.

use std::io::IsTerminal;
use std::sync::atomic::{AtomicU8, Ordering};

/// When rendered failures should use ANSI color.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ColorChoice {
    /// Color when the output looks like a terminal and `NO_COLOR` is unset.
    /// This is the default.
    Auto,
    /// Always emit color, regardless of terminal detection or `NO_COLOR`.
    Always,
    /// Never emit color.
    Never,
}

const AUTO: u8 = 0;
const ALWAYS: u8 = 1;
const NEVER: u8 = 2;

/// The process-wide color choice, defaulting to [`ColorChoice::Auto`].
static CHOICE: AtomicU8 = AtomicU8::new(AUTO);

/// Serializes the color-sensitive tests across this crate (`color` here and
/// `debug_matches_display` in `error`), which would otherwise race on the
/// global [`CHOICE`].
#[cfg(test)]
pub(crate) static TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// Sets the process-wide [`ColorChoice`] for rendered failures.
pub fn set_color_choice(choice: ColorChoice) {
    let encoded = match choice {
        ColorChoice::Auto => AUTO,
        ColorChoice::Always => ALWAYS,
        ColorChoice::Never => NEVER,
    };
    CHOICE.store(encoded, Ordering::Relaxed);
}

/// Returns the process-wide [`ColorChoice`].
#[must_use]
pub fn color_choice() -> ColorChoice {
    match CHOICE.load(Ordering::Relaxed) {
        ALWAYS => ColorChoice::Always,
        NEVER => ColorChoice::Never,
        _ => ColorChoice::Auto,
    }
}

/// Resolves a [`ColorChoice`] against the environment into a yes/no decision.
///
/// Split out as a pure function so the `Auto` logic (including `NO_COLOR`) is
/// testable without touching global state or the real environment.
fn resolve(choice: ColorChoice, no_color: bool, is_terminal: bool) -> bool {
    match choice {
        ColorChoice::Always => true,
        ColorChoice::Never => false,
        ColorChoice::Auto => !no_color && is_terminal,
    }
}

/// Whether rendered `Debug` output should currently emit ANSI color.
pub(crate) fn color_enabled() -> bool {
    // `NO_COLOR`: set and non-empty disables color (https://no-color.org).
    let no_color = std::env::var_os("NO_COLOR").is_some_and(|value| !value.is_empty());
    resolve(color_choice(), no_color, std::io::stderr().is_terminal())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{OrFail, TestResult};
    use test_better_matchers::{eq, expect, is_false, is_true};

    #[test]
    fn resolve_handles_every_choice_and_no_color() -> TestResult {
        // Always wins over both `NO_COLOR` and terminal detection.
        expect!(resolve(ColorChoice::Always, true, false))
            .to(is_true())
            .or_fail()?;
        // Never loses to both.
        expect!(resolve(ColorChoice::Never, false, true))
            .to(is_false())
            .or_fail()?;
        // Auto needs a terminal and an unset `NO_COLOR`.
        expect!(resolve(ColorChoice::Auto, false, true))
            .to(is_true())
            .or_fail()?;
        expect!(resolve(ColorChoice::Auto, true, true))
            .to(is_false())
            .or_fail()?;
        expect!(resolve(ColorChoice::Auto, false, false))
            .to(is_false())
            .or_fail()?;
        Ok(())
    }

    #[test]
    fn choice_round_trips_through_the_global_slot() -> TestResult {
        let _guard = TEST_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let original = color_choice();

        set_color_choice(ColorChoice::Always);
        let after_always = color_choice();
        set_color_choice(ColorChoice::Never);
        let after_never = color_choice();
        set_color_choice(ColorChoice::Auto);
        let after_auto = color_choice();

        // Restore before any `?` to avoid skipping the restore on early return.
        set_color_choice(original);

        expect!(after_always)
            .to(eq(ColorChoice::Always))
            .or_fail()?;
        expect!(after_never).to(eq(ColorChoice::Never)).or_fail()?;
        expect!(after_auto).to(eq(ColorChoice::Auto)).or_fail()?;
        Ok(())
    }
}
