//! Redactions: stabilizing non-deterministic content before a snapshot
//! comparison.
//!
//! A snapshot is only worth keeping if it is stable run to run, but rendered
//! values often carry content that is not: a freshly minted UUID, a wall-clock
//! timestamp. A [`Redactions`] set rewrites those spans to a fixed placeholder
//! *before* the value is compared or stored, so the run-to-run noise never
//! reaches the snapshot file. Because the same redactions run on every
//! comparison, the placeholder is what lives in the `.snap` file and what later
//! runs compare against.
//!
//! The built-in rules are hand-written scanners ([`redact_uuids`] for the
//! 8-4-4-4-12 hex form, [`redact_rfc3339_timestamps`] for ISO-8601-style
//! date-times), so this crate stays `std`-only. [`replace`] handles a known
//! literal, and [`redact_with`] is the escape hatch for anything the built-ins
//! do not cover.
//!
//! [`redact_uuids`]: Redactions::redact_uuids
//! [`redact_rfc3339_timestamps`]: Redactions::redact_rfc3339_timestamps
//! [`replace`]: Redactions::replace
//! [`redact_with`]: Redactions::redact_with

use std::fmt;

/// One redaction rule: maps the running text to its rewritten form. Boxed so a
/// literal replacement, a built-in scanner, and a user-supplied function are
/// all the same type.
type RedactionRule = Box<dyn Fn(&str) -> String + Send + Sync>;

/// An ordered set of text rewrites applied to a value before it is compared
/// against (or written to) a snapshot.
///
/// Build one with the chained methods, then hand it to
/// `check!(value).matches_snapshot_with(name, &redactions)` (or the inline
/// variant). Rules run in the order they were added, each on the output of the
/// last.
///
/// ```
/// use test_better_core::TestResult;
/// use test_better_matchers::{eq, check};
/// use test_better_snapshot::Redactions;
///
/// # fn main() -> TestResult {
/// let redactions = Redactions::new().redact_uuids();
/// let rendered = "user 550e8400-e29b-41d4-a716-446655440000 logged in";
/// check!(redactions.apply(rendered))
///     .satisfies(eq("user [uuid] logged in".to_string()))?;
/// # Ok(())
/// # }
/// ```
#[derive(Default)]
pub struct Redactions {
    /// The rules, run in order, each on the output of the last.
    rules: Vec<RedactionRule>,
}

impl fmt::Debug for Redactions {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // The rules are closures, so there is nothing useful to print but how
        // many there are.
        f.debug_struct("Redactions")
            .field("rules", &self.rules.len())
            .finish()
    }
}

impl Redactions {
    /// An empty set: [`apply`](Self::apply) returns its input unchanged until a
    /// rule is added.
    #[must_use]
    pub fn new() -> Self {
        Self { rules: Vec::new() }
    }

    /// Replaces every occurrence of the literal `needle` with `placeholder`.
    ///
    /// This is the rule for a value you already know, e.g. a generated id you
    /// captured earlier in the test. An empty `needle` is a no-op rule rather
    /// than a rule that matches everywhere.
    #[must_use]
    pub fn replace(mut self, needle: impl Into<String>, placeholder: impl Into<String>) -> Self {
        let needle = needle.into();
        let placeholder = placeholder.into();
        self.rules.push(Box::new(move |input| {
            if needle.is_empty() {
                input.to_string()
            } else {
                input.replace(needle.as_str(), placeholder.as_str())
            }
        }));
        self
    }

    /// Replaces every UUID (the canonical 8-4-4-4-12 hex form, either case)
    /// with `[uuid]`.
    #[must_use]
    pub fn redact_uuids(mut self) -> Self {
        self.rules
            .push(Box::new(|input| scan_replace(input, "[uuid]", uuid_at)));
        self
    }

    /// Replaces every RFC 3339 / ISO 8601 date-time (e.g.
    /// `2026-05-14T12:34:56Z`, with optional fractional seconds and either a
    /// `Z` or a `±hh:mm` offset) with `[timestamp]`.
    #[must_use]
    pub fn redact_rfc3339_timestamps(mut self) -> Self {
        self.rules.push(Box::new(|input| {
            scan_replace(input, "[timestamp]", rfc3339_at)
        }));
        self
    }

    /// Adds an arbitrary rewriting rule: the escape hatch for content the
    /// built-ins do not cover.
    ///
    /// The closure is handed the running text and returns its rewritten form.
    #[must_use]
    pub fn redact_with(mut self, rule: impl Fn(&str) -> String + Send + Sync + 'static) -> Self {
        self.rules.push(Box::new(rule));
        self
    }

    /// Runs every rule, in order, and returns the rewritten text. With no rules
    /// added this returns `input` unchanged.
    #[must_use]
    pub fn apply(&self, input: &str) -> String {
        let mut text = input.to_string();
        for rule in &self.rules {
            text = rule(&text);
        }
        text
    }

    /// Whether any rule has been added. An empty set is worth skipping: its
    /// [`apply`](Self::apply) is an allocation that changes nothing.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.rules.is_empty()
    }
}

/// Walks `input` left to right, replacing every span `matcher` accepts with
/// `placeholder` and copying everything else through verbatim.
///
/// `matcher` is called with the remaining tail; it returns the byte length of
/// a match starting at the front, or `None`. A match advances past the whole
/// span, so matches never overlap.
fn scan_replace(input: &str, placeholder: &str, matcher: impl Fn(&str) -> Option<usize>) -> String {
    let mut out = String::with_capacity(input.len());
    let mut rest = input;
    while !rest.is_empty() {
        if let Some(len) = matcher(rest) {
            out.push_str(placeholder);
            rest = &rest[len..];
        } else {
            match rest.chars().next() {
                Some(ch) => {
                    out.push(ch);
                    rest = &rest[ch.len_utf8()..];
                }
                // `rest` is non-empty, so this arm is unreachable; breaking
                // rather than panicking keeps the function total.
                None => break,
            }
        }
    }
    out
}

/// If `s` starts with a canonical UUID, returns its byte length (always 36).
///
/// The shape is five hex groups of lengths 8, 4, 4, 4, 12 joined by `-`. A
/// trailing hex digit is rejected so a longer hex run is not chopped in half.
fn uuid_at(s: &str) -> Option<usize> {
    const GROUPS: [usize; 5] = [8, 4, 4, 4, 12];
    let bytes = s.as_bytes();
    let mut pos = 0usize;
    for (index, &len) in GROUPS.iter().enumerate() {
        for _ in 0..len {
            if !bytes.get(pos)?.is_ascii_hexdigit() {
                return None;
            }
            pos += 1;
        }
        if index < GROUPS.len() - 1 {
            if bytes.get(pos) != Some(&b'-') {
                return None;
            }
            pos += 1;
        }
    }
    // Don't swallow the front of an even longer hex string.
    if bytes.get(pos).is_some_and(u8::is_ascii_hexdigit) {
        return None;
    }
    Some(pos)
}

/// If `s` starts with an RFC 3339 date-time, returns its byte length.
fn rfc3339_at(s: &str) -> Option<usize> {
    let bytes = s.as_bytes();
    let mut pos = 0usize;

    // date: yyyy-mm-dd
    take_digits(bytes, &mut pos, 4)?;
    take_byte(bytes, &mut pos, b'-')?;
    take_digits(bytes, &mut pos, 2)?;
    take_byte(bytes, &mut pos, b'-')?;
    take_digits(bytes, &mut pos, 2)?;

    // date-time separator: `T`, `t`, or a space (RFC 3339 §5.6 allows all).
    match bytes.get(pos) {
        Some(b'T' | b't' | b' ') => pos += 1,
        _ => return None,
    }

    // time: hh:mm:ss
    take_digits(bytes, &mut pos, 2)?;
    take_byte(bytes, &mut pos, b':')?;
    take_digits(bytes, &mut pos, 2)?;
    take_byte(bytes, &mut pos, b':')?;
    take_digits(bytes, &mut pos, 2)?;

    // optional fractional seconds: a dot followed by one or more digits.
    if bytes.get(pos) == Some(&b'.') {
        pos += 1;
        let frac_start = pos;
        while bytes.get(pos).is_some_and(u8::is_ascii_digit) {
            pos += 1;
        }
        if pos == frac_start {
            return None;
        }
    }

    // offset: `Z`/`z` or `±hh:mm`.
    match bytes.get(pos) {
        Some(b'Z' | b'z') => pos += 1,
        Some(b'+' | b'-') => {
            pos += 1;
            take_digits(bytes, &mut pos, 2)?;
            take_byte(bytes, &mut pos, b':')?;
            take_digits(bytes, &mut pos, 2)?;
        }
        _ => return None,
    }

    Some(pos)
}

/// Advances `pos` past exactly `count` ASCII digits, or returns `None` and
/// leaves `pos` untouched.
fn take_digits(bytes: &[u8], pos: &mut usize, count: usize) -> Option<()> {
    for offset in 0..count {
        if !bytes.get(*pos + offset)?.is_ascii_digit() {
            return None;
        }
    }
    *pos += count;
    Some(())
}

/// Advances `pos` past `expected`, or returns `None` and leaves `pos`
/// untouched.
fn take_byte(bytes: &[u8], pos: &mut usize, expected: u8) -> Option<()> {
    if bytes.get(*pos) == Some(&expected) {
        *pos += 1;
        Some(())
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use test_better_core::TestResult;
    use test_better_matchers::{eq, check, is_true};

    use super::*;

    #[test]
    fn an_empty_set_returns_its_input_unchanged() -> TestResult {
        let redactions = Redactions::new();
        check!(redactions.is_empty()).satisfies(is_true())?;
        check!(redactions.apply("untouched")).satisfies(eq("untouched".to_string()))
    }

    #[test]
    fn redact_uuids_replaces_every_canonical_uuid() -> TestResult {
        let redactions = Redactions::new().redact_uuids();
        let input = "from 550E8400-E29B-41D4-A716-446655440000 to \
                     00000000-0000-0000-0000-000000000000";
        check!(redactions.apply(input)).satisfies(eq("from [uuid] to [uuid]".to_string()))
    }

    #[test]
    fn redact_uuids_leaves_a_near_miss_alone() -> TestResult {
        let redactions = Redactions::new().redact_uuids();
        // One hex digit short in the last group, and a non-hex character: both
        // must pass through verbatim.
        let input = "550e8400-e29b-41d4-a716-44665544000 and zzze8400-e29b";
        check!(redactions.apply(input)).satisfies(eq(input.to_string()))
    }

    #[test]
    fn redact_rfc3339_timestamps_handles_z_and_offset_and_fractions() -> TestResult {
        let redactions = Redactions::new().redact_rfc3339_timestamps();
        let input = "at 2026-05-14T12:34:56Z and 2026-01-02T03:04:05.678-05:00 done";
        check!(redactions.apply(input)).satisfies(eq("at [timestamp] and [timestamp] done".to_string()))
    }

    #[test]
    fn rules_run_in_order_and_compose() -> TestResult {
        let redactions = Redactions::new()
            .redact_uuids()
            .replace("[uuid]", "<id>")
            .redact_with(|text| text.to_uppercase());
        check!(redactions.apply("id 550e8400-e29b-41d4-a716-446655440000"))
            .satisfies(eq("ID <ID>".to_string()))
    }

    #[test]
    fn replace_ignores_an_empty_needle() -> TestResult {
        let redactions = Redactions::new().replace("", "X");
        check!(redactions.apply("abc")).satisfies(eq("abc".to_string()))
    }

    #[test]
    fn a_uuid_glued_to_more_hex_is_not_redacted() -> TestResult {
        // A 37th hex digit means this is not a bare UUID; leave it alone rather
        // than emit `[uuid]f`.
        let redactions = Redactions::new().redact_uuids();
        let input = "550e8400-e29b-41d4-a716-446655440000f";
        check!(redactions.apply(input)).satisfies(eq(input.to_string()))?;
        // The Debug impl reports the rule count.
        check!(format!("{redactions:?}").contains("rules: 1")).satisfies(is_true())
    }
}
