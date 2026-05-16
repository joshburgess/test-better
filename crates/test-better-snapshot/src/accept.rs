//! The accept step: applying pending inline-snapshot patches back to source.
//! Behind the `accept` feature,
//! since this is the only part of the crate that needs `syn`.
//!
//! A pending patch (see [`crate::inline`]) names a source file, the call-site
//! line and column of a `matches_inline_snapshot` call, and the new snapshot
//! value. Applying it means finding that call's string-literal argument and
//! splicing a literal for the new value in its place, leaving the rest of the
//! file byte-for-byte untouched. The literal's exact extent comes from `syn`'s
//! span information; everything else is a string splice.

use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

use syn::spanned::Spanned;

use crate::{parse_pending_patch, pending_patch_dir};

/// One source file rewritten by [`apply_patches_from`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Applied {
    /// The source file that was rewritten.
    pub file: PathBuf,
    /// How many inline-snapshot literals in it were updated.
    pub patches: usize,
}

/// Why applying inline-snapshot patches failed.
#[derive(Debug)]
pub enum AcceptError {
    /// Reading the pending-patch directory, a patch file, or a source file
    /// (or writing a source file back) failed.
    Io {
        /// The path the operation concerned.
        path: PathBuf,
        /// The underlying I/O error.
        source: std::io::Error,
    },
    /// A pending-patch file was not in the expected format.
    MalformedPatch {
        /// The patch file.
        path: PathBuf,
        /// What was wrong with it.
        message: String,
    },
    /// A source file did not parse as Rust.
    Parse {
        /// The source file.
        path: PathBuf,
        /// The parser's message.
        message: String,
    },
    /// No `matches_inline_snapshot` call was found covering the patched line.
    NoCall {
        /// The source file.
        path: PathBuf,
        /// The 1-based line the patch pointed at.
        line: u32,
    },
    /// The `matches_inline_snapshot` call's argument was not a string literal,
    /// so there is nothing to rewrite.
    NotAStringLiteral {
        /// The source file.
        path: PathBuf,
        /// The 1-based line of the call.
        line: u32,
    },
}

impl fmt::Display for AcceptError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AcceptError::Io { path, source } => {
                write!(f, "I/O error at {}: {source}", path.display())
            }
            AcceptError::MalformedPatch { path, message } => {
                write!(f, "malformed pending patch {}: {message}", path.display())
            }
            AcceptError::Parse { path, message } => {
                write!(f, "could not parse {}: {message}", path.display())
            }
            AcceptError::NoCall { path, line } => write!(
                f,
                "no `matches_inline_snapshot` call at {}:{line}",
                path.display()
            ),
            AcceptError::NotAStringLiteral { path, line } => write!(
                f,
                "the `matches_inline_snapshot` call at {}:{line} has a non-literal argument",
                path.display()
            ),
        }
    }
}

impl std::error::Error for AcceptError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            AcceptError::Io { source, .. } => Some(source),
            _ => None,
        }
    }
}

/// Rewrites the inline-snapshot literal at `line`/`column` in `source` to a
/// literal standing for `new_value`, returning the new source text.
///
/// `path` is used only for error messages. The call is located by finding the
/// `matches_inline_snapshot` method call whose span covers `line`; its
/// string-literal argument is replaced, and nothing else in the file moves.
///
/// # Errors
///
/// Returns [`AcceptError::Parse`] if `source` is not valid Rust,
/// [`AcceptError::NoCall`] if no matching call covers `line`, or
/// [`AcceptError::NotAStringLiteral`] if that call's argument is not a string
/// literal.
pub fn apply_inline_patch(
    path: &Path,
    source: &str,
    line: u32,
    column: u32,
    new_value: &str,
) -> Result<String, AcceptError> {
    let file = syn::parse_file(source).map_err(|error| AcceptError::Parse {
        path: path.to_path_buf(),
        message: error.to_string(),
    })?;

    let mut finder = CallFinder {
        target_line: line,
        target_column: column,
        best: None,
    };
    syn::visit::visit_file(&mut finder, &file);

    let found = finder.best.ok_or(AcceptError::NoCall {
        path: path.to_path_buf(),
        line,
    })?;
    let literal = found.literal.ok_or(AcceptError::NotAStringLiteral {
        path: path.to_path_buf(),
        line,
    })?;

    let start = line_col_to_byte(source, literal.start_line, literal.start_column).ok_or(
        AcceptError::NoCall {
            path: path.to_path_buf(),
            line,
        },
    )?;
    let end = line_col_to_byte(source, literal.end_line, literal.end_column).ok_or(
        AcceptError::NoCall {
            path: path.to_path_buf(),
            line,
        },
    )?;

    // The indentation to format a multi-line literal against: the leading
    // whitespace of the line the literal starts on.
    let indent = line_indent(source, literal.start_line);
    let replacement = format_inline_literal(new_value, &indent);

    let mut rewritten = String::with_capacity(source.len() + replacement.len());
    rewritten.push_str(&source[..start]);
    rewritten.push_str(&replacement);
    rewritten.push_str(&source[end..]);
    Ok(rewritten)
}

/// Reads every `*.patch` file under `pending_dir`, rewrites the source files
/// they reference (resolved relative to `workspace_root`), and deletes each
/// patch file once applied. Returns one [`Applied`] per source file touched.
///
/// Patches for the same file are applied by re-parsing between each, so byte
/// offsets never go stale. A missing `pending_dir` is not an error: it just
/// means there is nothing to do, and an empty `Vec` is returned.
///
/// # Errors
///
/// Returns [`AcceptError`] on an I/O failure, a malformed patch file, or a
/// source file that does not parse or lacks the expected call.
pub fn apply_patches_from(
    pending_dir: &Path,
    workspace_root: &Path,
) -> Result<Vec<Applied>, AcceptError> {
    if !pending_dir.exists() {
        return Ok(Vec::new());
    }

    // Collect every staged patch, grouped by the source file it targets.
    let mut by_file: Vec<(PathBuf, Vec<StagedPatch>)> = Vec::new();
    let entries = fs::read_dir(pending_dir).map_err(|source| AcceptError::Io {
        path: pending_dir.to_path_buf(),
        source,
    })?;
    for entry in entries {
        let entry = entry.map_err(|source| AcceptError::Io {
            path: pending_dir.to_path_buf(),
            source,
        })?;
        let patch_path = entry.path();
        if patch_path.extension().and_then(|e| e.to_str()) != Some("patch") {
            continue;
        }
        let body = fs::read_to_string(&patch_path).map_err(|source| AcceptError::Io {
            path: patch_path.clone(),
            source,
        })?;
        let (location, value) =
            parse_pending_patch(&body).map_err(|error| AcceptError::MalformedPatch {
                path: patch_path.clone(),
                message: error.to_string(),
            })?;
        let source_file = workspace_root.join(&location.file);
        let index = match by_file.iter().position(|(file, _)| *file == source_file) {
            Some(index) => index,
            None => {
                by_file.push((source_file, Vec::new()));
                by_file.len() - 1
            }
        };
        by_file[index].1.push(StagedPatch {
            patch_file: patch_path,
            line: location.line,
            column: location.column,
            value,
        });
    }

    let mut applied = Vec::new();
    for (source_file, patches) in by_file {
        let mut source = fs::read_to_string(&source_file).map_err(|source| AcceptError::Io {
            path: source_file.clone(),
            source,
        })?;
        for patch in &patches {
            source = apply_inline_patch(
                &source_file,
                &source,
                patch.line,
                patch.column,
                &patch.value,
            )?;
        }
        fs::write(&source_file, &source).map_err(|source_err| AcceptError::Io {
            path: source_file.clone(),
            source: source_err,
        })?;
        for patch in &patches {
            // The rewrite succeeded; drop the spent patch. A failure to remove
            // it is not worth aborting the run over.
            let _ = fs::remove_file(&patch.patch_file);
        }
        applied.push(Applied {
            file: source_file,
            patches: patches.len(),
        });
    }
    Ok(applied)
}

/// One pending patch staged for application by [`apply_patches_from`]: the
/// pending-patch file it was read from, the call-site line and column, and the
/// new snapshot value.
struct StagedPatch {
    patch_file: PathBuf,
    line: u32,
    column: u32,
    value: String,
}

/// The CWD-derived convenience over [`apply_patches_from`]: the pending
/// directory comes from [`pending_patch_dir`], and the workspace root is the
/// nearest ancestor of the current directory containing a `Cargo.lock`.
///
/// # Errors
///
/// Returns [`AcceptError`] on the same conditions as [`apply_patches_from`],
/// plus an I/O error if the working directory or workspace root cannot be
/// resolved.
pub fn apply_pending_patches() -> Result<Vec<Applied>, AcceptError> {
    let pending_dir = pending_patch_dir().map_err(|source| AcceptError::Io {
        path: PathBuf::from("target/test-better-pending"),
        source,
    })?;
    let root = workspace_root().map_err(|source| AcceptError::Io {
        path: PathBuf::from("."),
        source,
    })?;
    apply_patches_from(&pending_dir, &root)
}

/// The nearest ancestor of the current directory containing a `Cargo.lock`.
fn workspace_root() -> std::io::Result<PathBuf> {
    let start = std::env::current_dir()?;
    let mut dir: &Path = &start;
    loop {
        if dir.join("Cargo.lock").is_file() {
            return Ok(dir.to_path_buf());
        }
        match dir.parent() {
            Some(parent) => dir = parent,
            None => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "no Cargo.lock found in any ancestor of the current directory",
                ));
            }
        }
    }
}

/// The span of an inline-snapshot string literal, in `syn`'s 1-based-line,
/// 0-based-column coordinates.
struct LiteralSpan {
    start_line: usize,
    start_column: usize,
    end_line: usize,
    end_column: usize,
}

/// The best `matches_inline_snapshot` call found so far for a target line.
struct FoundCall {
    /// `None` if the call's argument was not a string literal.
    literal: Option<LiteralSpan>,
    /// How far the call's start column is from the patch column, for picking
    /// between two calls that share a line.
    column_distance: usize,
}

/// Visits a parsed file looking for the `matches_inline_snapshot` call that
/// covers `target_line`, keeping the one closest to `target_column`.
struct CallFinder {
    target_line: u32,
    target_column: u32,
    best: Option<FoundCall>,
}

impl<'ast> syn::visit::Visit<'ast> for CallFinder {
    fn visit_expr_method_call(&mut self, node: &'ast syn::ExprMethodCall) {
        if node.method == "matches_inline_snapshot" {
            let span = node.span();
            let start = span.start();
            let end = span.end();
            let covers =
                start.line <= self.target_line as usize && self.target_line as usize <= end.line;
            if covers {
                let column_distance =
                    (start.column as isize - self.target_column as isize).unsigned_abs();
                let better = self
                    .best
                    .as_ref()
                    .is_none_or(|best| column_distance < best.column_distance);
                if better {
                    let literal = match node.args.first() {
                        Some(syn::Expr::Lit(syn::ExprLit {
                            lit: syn::Lit::Str(lit_str),
                            ..
                        })) => {
                            let lit_span = lit_str.span();
                            let lit_start = lit_span.start();
                            let lit_end = lit_span.end();
                            Some(LiteralSpan {
                                start_line: lit_start.line,
                                start_column: lit_start.column,
                                end_line: lit_end.line,
                                end_column: lit_end.column,
                            })
                        }
                        _ => None,
                    };
                    self.best = Some(FoundCall {
                        literal,
                        column_distance,
                    });
                }
            }
        }
        // Recurse so nested calls are still found.
        syn::visit::visit_expr_method_call(self, node);
    }
}

/// Converts a 1-based line and 0-based (char) column to a byte offset in
/// `source`. Returns `None` if the position is past the end of the file.
fn line_col_to_byte(source: &str, line: usize, column: usize) -> Option<usize> {
    let mut offset = 0usize;
    let mut current = 1usize;
    for raw_line in source.split_inclusive('\n') {
        if current == line {
            let content = raw_line.strip_suffix('\n').unwrap_or(raw_line);
            let content = content.strip_suffix('\r').unwrap_or(content);
            let byte_in_line = content
                .char_indices()
                .nth(column)
                .map(|(byte, _)| byte)
                .unwrap_or(content.len());
            return Some(offset + byte_in_line);
        }
        offset += raw_line.len();
        current += 1;
    }
    // A position one past the last line, column 0, is the end of the file.
    if current == line && column == 0 {
        Some(offset)
    } else {
        None
    }
}

/// The leading whitespace of the 1-based `line` in `source`.
fn line_indent(source: &str, line: usize) -> String {
    source
        .split_inclusive('\n')
        .nth(line.saturating_sub(1))
        .map(|raw| {
            raw.chars()
                .take_while(|c| *c == ' ' || *c == '\t')
                .collect()
        })
        .unwrap_or_default()
}

/// Formats `value` as a Rust raw string literal. A single-line value is written
/// inline; a multi-line value is written with a leading newline and each line
/// indented past `indent`, which `normalize_inline_literal` undoes on the next
/// run.
fn format_inline_literal(value: &str, indent: &str) -> String {
    let hashes = "#".repeat(hash_count(value));
    if value.contains('\n') {
        let mut out = format!("r{hashes}\"\n");
        for content_line in value.split('\n') {
            if content_line.is_empty() {
                out.push('\n');
            } else {
                out.push_str(indent);
                out.push_str("    ");
                out.push_str(content_line);
                out.push('\n');
            }
        }
        out.push_str(indent);
        out.push('"');
        out.push_str(&hashes);
        out
    } else {
        format!("r{hashes}\"{value}\"{hashes}")
    }
}

/// The number of `#`s a raw string around `value` needs: one more than the
/// longest run of `#`s that immediately follows a `"` in `value`.
fn hash_count(value: &str) -> usize {
    let bytes = value.as_bytes();
    let mut longest = 0usize;
    let mut index = 0usize;
    while index < bytes.len() {
        if bytes[index] == b'"' {
            let mut run = 0usize;
            let mut after = index + 1;
            while after < bytes.len() && bytes[after] == b'#' {
                run += 1;
                after += 1;
            }
            longest = longest.max(run);
        }
        index += 1;
    }
    longest + 1
}

#[cfg(test)]
mod tests {
    use test_better_core::{OrFail, TestResult};
    use test_better_matchers::{eq, check, is_true};

    use super::*;

    #[test]
    fn hash_count_outgrows_embedded_quote_hash_runs() -> TestResult {
        check!(hash_count("plain")).satisfies(eq(1usize))?;
        check!(hash_count("has \"# inside")).satisfies(eq(2usize))?;
        check!(hash_count("has \"## inside")).satisfies(eq(3usize))
    }

    #[test]
    fn format_inline_literal_writes_a_single_line_value_inline() -> TestResult {
        check!(format_inline_literal("hello", "    ")).satisfies(eq("r#\"hello\"#".to_string()))
    }

    #[test]
    fn format_inline_literal_round_trips_through_normalization() -> TestResult {
        let value = "first line\nsecond line";
        let literal = format_inline_literal(value, "    ");
        // Strip the `r#"` ... `"#` wrapper to recover the literal's string
        // value, then normalize it the way the runtime comparison does.
        let inner = literal
            .strip_prefix("r#\"")
            .and_then(|rest| rest.strip_suffix("\"#"))
            .or_fail_with("formatted literal should be `r#\"...\"#`")?;
        check!(crate::normalize_inline_literal(inner)).satisfies(eq(value.to_string()))
    }

    #[test]
    fn apply_inline_patch_rewrites_the_literal_in_place() -> TestResult {
        let source = "fn t() {\n    check!(x).matches_inline_snapshot(r#\"old\"#)?;\n}\n";
        let rewritten = apply_inline_patch(Path::new("t.rs"), source, 2, 4, "new").or_fail()?;
        check!(rewritten.contains("r#\"new\"#")).satisfies(is_true())?;
        check!(rewritten.contains("old")).satisfies(eq(false))?;
        // Everything outside the literal is untouched.
        check!(rewritten.starts_with("fn t() {\n")).satisfies(is_true())?;
        check!(rewritten.ends_with("?;\n}\n")).satisfies(is_true())
    }

    #[test]
    fn apply_inline_patch_reports_a_missing_call() -> TestResult {
        let source = "fn t() {\n    let y = 1;\n}\n";
        let outcome = apply_inline_patch(Path::new("t.rs"), source, 2, 4, "new");
        let error = outcome.err().or_fail_with("there is no call to rewrite")?;
        check!(matches!(error, AcceptError::NoCall { .. })).satisfies(is_true())
    }
}
