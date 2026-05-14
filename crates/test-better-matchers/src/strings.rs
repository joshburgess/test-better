//! String matchers: [`contains_str`], [`starts_with`], [`ends_with`], and
//! (behind the `regex` feature) [`matches_regex`].
//!
//! Each is generic over `T: AsRef<str>`, so it matches `&str`, `String`,
//! `&String`, and `str` alike. When a mismatch involves a multi-line string,
//! the failure carries a line-oriented diff (PROJECT_BUILD_PLAN.md §8,
//! Iteration 3.4).

use std::fmt;

use crate::description::Description;
use crate::matcher::{MatchResult, Matcher, Mismatch};

/// A line-oriented diff of `expected` against `actual`, but only when at least
/// one of them spans multiple lines: a diff of two single-line strings is just
/// noise. With the `diff` feature off this is always `None`.
#[cfg(feature = "diff")]
fn multi_line_str_diff(expected: &str, actual: &str) -> Option<String> {
    if expected.contains('\n') || actual.contains('\n') {
        Some(crate::diff::diff_lines(expected, actual))
    } else {
        None
    }
}

#[cfg(not(feature = "diff"))]
fn multi_line_str_diff(_expected: &str, _actual: &str) -> Option<String> {
    None
}

/// Generates a string matcher backed by a `str` predicate method (`contains`,
/// `starts_with`, `ends_with`). The `str_description` inherent method keeps the
/// description reachable from `check` without an ambiguous `self.description()`
/// (the matcher implements `Matcher<T>` for a family of `T`).
macro_rules! str_predicate_matcher {
    ($matcher:ident, $method:ident, $describe:literal) => {
        struct $matcher {
            needle: String,
        }

        impl $matcher {
            fn str_description(&self) -> Description {
                Description::text(format!(concat!($describe, " {:?}"), self.needle))
            }
        }

        impl<T> Matcher<T> for $matcher
        where
            T: AsRef<str> + fmt::Debug + ?Sized,
        {
            fn check(&self, actual: &T) -> MatchResult {
                let haystack = actual.as_ref();
                if haystack.$method(self.needle.as_str()) {
                    MatchResult::pass()
                } else {
                    let mut mismatch = Mismatch::new(self.str_description(), format!("{actual:?}"));
                    if let Some(diff) = multi_line_str_diff(&self.needle, haystack) {
                        mismatch = mismatch.with_diff(diff);
                    }
                    MatchResult::fail(mismatch)
                }
            }

            fn description(&self) -> Description {
                self.str_description()
            }
        }
    };
}

str_predicate_matcher!(ContainsStrMatcher, contains, "a string containing");
str_predicate_matcher!(StartsWithMatcher, starts_with, "a string starting with");
str_predicate_matcher!(EndsWithMatcher, ends_with, "a string ending with");

/// Matches a string that contains `needle` as a substring.
///
/// ```
/// use test_better_core::TestResult;
/// use test_better_matchers::{contains_str, expect};
///
/// fn main() -> TestResult {
///     expect!("hello, world").to(contains_str("o, w"))?;
///     expect!(String::from("hello")).to_not(contains_str("bye"))?;
///     Ok(())
/// }
/// ```
pub fn contains_str<T>(needle: impl Into<String>) -> impl Matcher<T>
where
    T: AsRef<str> + fmt::Debug + ?Sized,
{
    ContainsStrMatcher {
        needle: needle.into(),
    }
}

/// Matches a string that starts with `prefix`.
///
/// ```
/// use test_better_core::TestResult;
/// use test_better_matchers::{expect, starts_with};
///
/// fn main() -> TestResult {
///     expect!("hello, world").to(starts_with("hello"))?;
///     Ok(())
/// }
/// ```
pub fn starts_with<T>(prefix: impl Into<String>) -> impl Matcher<T>
where
    T: AsRef<str> + fmt::Debug + ?Sized,
{
    StartsWithMatcher {
        needle: prefix.into(),
    }
}

/// Matches a string that ends with `suffix`.
///
/// ```
/// use test_better_core::TestResult;
/// use test_better_matchers::{ends_with, expect};
///
/// fn main() -> TestResult {
///     expect!("hello, world").to(ends_with("world"))?;
///     Ok(())
/// }
/// ```
pub fn ends_with<T>(suffix: impl Into<String>) -> impl Matcher<T>
where
    T: AsRef<str> + fmt::Debug + ?Sized,
{
    EndsWithMatcher {
        needle: suffix.into(),
    }
}

/// The matcher behind [`matches_regex`]. The pattern is compiled eagerly in
/// the constructor; a compilation error is held and surfaced as an ordinary
/// match failure, so the constructor needs no `Result` and the call site needs
/// no `?` on it.
#[cfg(feature = "regex")]
struct RegexMatcher {
    pattern: String,
    compiled: Result<regex::Regex, regex::Error>,
}

#[cfg(feature = "regex")]
impl RegexMatcher {
    fn regex_description(&self) -> Description {
        Description::text(format!("a string matching the regex {:?}", self.pattern))
    }
}

#[cfg(feature = "regex")]
impl<T> Matcher<T> for RegexMatcher
where
    T: AsRef<str> + fmt::Debug + ?Sized,
{
    fn check(&self, actual: &T) -> MatchResult {
        match &self.compiled {
            Err(error) => MatchResult::fail(Mismatch::new(
                self.regex_description(),
                format!("<invalid regex {:?}: {error}>", self.pattern),
            )),
            Ok(regex) => {
                if regex.is_match(actual.as_ref()) {
                    MatchResult::pass()
                } else {
                    MatchResult::fail(Mismatch::new(
                        self.regex_description(),
                        format!("{actual:?}"),
                    ))
                }
            }
        }
    }

    fn description(&self) -> Description {
        self.regex_description()
    }
}

/// Matches a string that the regular expression `pattern` finds a match in.
///
/// An invalid `pattern` is not a panic: it is held and reported as a match
/// failure when the matcher runs, so the constructor returns a plain matcher.
///
/// Behind the `regex` feature, which is off by default.
///
/// ```
/// use test_better_core::TestResult;
/// use test_better_matchers::{expect, matches_regex};
///
/// fn main() -> TestResult {
///     expect!("order #1234").to(matches_regex(r"#\d+"))?;
///     Ok(())
/// }
/// ```
#[cfg(feature = "regex")]
pub fn matches_regex<T>(pattern: impl Into<String>) -> impl Matcher<T>
where
    T: AsRef<str> + fmt::Debug + ?Sized,
{
    let pattern = pattern.into();
    let compiled = regex::Regex::new(&pattern);
    RegexMatcher { pattern, compiled }
}

#[cfg(test)]
mod tests {
    use test_better_core::{OrFail, TestResult};

    use super::*;
    use crate::{eq, expect, is_false, is_true};

    #[test]
    fn contains_str_matches_a_substring() -> TestResult {
        expect!(contains_str("ell").check("hello").matched).to(is_true())?;
        expect!(contains_str("xyz").check("hello").matched).to(is_false())?;
        // Works for `String` as well as `&str`.
        expect!(contains_str("ell").check(&String::from("hello")).matched).to(is_true())?;
        Ok(())
    }

    #[test]
    fn starts_with_and_ends_with_check_the_ends() -> TestResult {
        expect!(starts_with("he").check("hello").matched).to(is_true())?;
        expect!(starts_with("lo").check("hello").matched).to(is_false())?;
        expect!(ends_with("lo").check("hello").matched).to(is_true())?;
        expect!(ends_with("he").check("hello").matched).to(is_false())?;
        Ok(())
    }

    #[test]
    fn contains_str_failure_describes_the_needle_and_renders_the_actual() -> TestResult {
        let failure = contains_str("xyz")
            .check("hello")
            .failure
            .or_fail_with("hello does not contain xyz")?;
        expect!(failure.expected.to_string()).to(eq("a string containing \"xyz\"".to_string()))?;
        expect!(failure.actual).to(eq("\"hello\"".to_string()))?;
        Ok(())
    }

    #[cfg(feature = "diff")]
    #[test]
    fn multi_line_string_mismatch_carries_a_diff() -> TestResult {
        let actual = "line one\nline two\nline three";
        let failure = starts_with("line one\nline 2")
            .check(actual)
            .failure
            .or_fail_with("the multi-line prefix does not match")?;
        let diff = failure
            .diff
            .or_fail_with("a multi-line string mismatch should carry a diff")?;
        expect!(diff.contains("line 2")).to(is_true())?;
        expect!(diff.contains("line two")).to(is_true())?;
        Ok(())
    }

    #[cfg(feature = "regex")]
    #[test]
    fn matches_regex_matches_and_reports() -> TestResult {
        expect!(matches_regex(r"\d+").check("abc123").matched).to(is_true())?;
        expect!(matches_regex(r"^\d+$").check("abc123").matched).to(is_false())?;
        Ok(())
    }

    #[cfg(feature = "regex")]
    #[test]
    fn matches_regex_reports_an_invalid_pattern_as_a_failure() -> TestResult {
        let failure = matches_regex(r"(unclosed")
            .check("anything")
            .failure
            .or_fail_with("an invalid pattern fails the match")?;
        expect!(failure.actual.contains("invalid regex")).to(is_true())?;
        Ok(())
    }
}
