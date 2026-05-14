//! Line-oriented diff rendering, behind the default `diff` feature.
//!
//! This module produces the *structured, uncolored* diff text that lands in a
//! [`Mismatch`](crate::Mismatch)'s `diff` field. Color is applied later, and
//! only by `test-better-core`'s renderer (PROJECT_BUILD_PLAN.md §7, Iteration
//! 2.4): `matchers` never emits ANSI escapes.
//!
//! The output is a unified-style diff: each line is prefixed with ` ` for
//! unchanged context, `-` for a line present in `expected` but not `actual`,
//! and `+` for a line present in `actual` but not `expected`. The renderer in
//! `core` keys its red/green coloring off exactly those markers.

use similar::{ChangeTag, TextDiff};

/// Renders a line-oriented diff between `expected` and `actual`.
///
/// The result has no trailing newline, so it composes when the renderer
/// indents each line.
///
/// ```
/// use test_better_matchers::diff_lines;
///
/// let diff = diff_lines("a\nb\nc", "a\nB\nc");
/// assert_eq!(diff, " a\n-b\n+B\n c");
/// ```
#[must_use]
pub fn diff_lines(expected: &str, actual: &str) -> String {
    let diff = TextDiff::from_lines(expected, actual);
    let mut out = String::new();
    for change in diff.iter_all_changes() {
        let marker = match change.tag() {
            ChangeTag::Delete => '-',
            ChangeTag::Insert => '+',
            ChangeTag::Equal => ' ',
        };
        out.push(marker);
        let value = change.value();
        out.push_str(value);
        // `from_lines` keeps line endings, but the final line may lack one.
        if !value.ends_with('\n') {
            out.push('\n');
        }
    }
    // Drop the trailing newline so the diff composes cleanly when indented.
    if out.ends_with('\n') {
        out.pop();
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn equal_input_is_all_context_lines() {
        assert_eq!(diff_lines("one\ntwo", "one\ntwo"), " one\n two");
    }

    #[test]
    fn a_changed_line_becomes_a_delete_then_an_insert() {
        let diff = diff_lines("keep\nold\nkeep", "keep\nnew\nkeep");
        assert_eq!(diff, " keep\n-old\n+new\n keep");
    }

    #[test]
    fn has_no_trailing_newline() {
        let diff = diff_lines("a\n", "b\n");
        assert!(!diff.ends_with('\n'), "{diff:?}");
    }
}
