//! [`define_matcher!`], the declarative helper for the common custom-matcher
//! case: a named predicate over a concrete target type, with a human-readable
//! description.
//!
//! When a matcher needs more than a yes/no predicate (a structured diff, a
//! borrowed-through projection, an inner matcher) it is written by hand as an
//! `impl Matcher<T>`; see the cookbook in the `test-better` facade crate. This
//! macro covers the case that does not need any of that.

/// Defines a custom matcher from a predicate and a description.
///
/// This is the declarative shortcut for the most common custom matcher: one
/// that inspects a value of a concrete type and answers yes or no, with a
/// fixed human-readable account of what it expected. Anything richer (a diff,
/// an inner matcher, a borrowed projection) is written by hand as an
/// `impl Matcher<T>`.
///
/// # Syntax
///
/// (These examples name `test_better_matchers` directly because they are this
/// crate's own doc tests; a user crate writes `use test_better::prelude::*;`
/// and `use test_better::define_matcher;` instead.)
///
/// ```
/// use test_better_matchers::define_matcher;
///
/// define_matcher! {
///     /// Matches an even integer.
///     pub fn is_even for i32 {
///         expects: "an even integer",
///         matches: |n| n % 2 == 0,
///     }
/// }
/// ```
///
/// The matcher may take constructor parameters; each is in scope inside both
/// `expects` and `matches` as a value of its declared type, so the parameter
/// types must be [`Clone`]:
///
/// ```
/// use test_better_core::TestResult;
/// use test_better_matchers::{define_matcher, expect};
///
/// define_matcher! {
///     /// Matches a string that ends with `suffix`.
///     pub fn has_suffix(suffix: &'static str) for String {
///         expects: format!("a string ending in {suffix:?}"),
///         matches: |actual| actual.ends_with(suffix),
///     }
/// }
///
/// fn main() -> TestResult {
///     expect!(String::from("report.csv")).to(has_suffix(".csv"))?;
///     Ok(())
/// }
/// ```
///
/// # Requirements
///
/// - The target type must implement [`Debug`](std::fmt::Debug): a failure
///   reports the actual value through `{:?}`.
/// - `expects` is any expression that converts into the matcher's description
///   text (a string literal or a `format!` are the usual choices).
/// - `matches` is written like a closure, `|binding| expression`, but it is
///   not a real closure: `binding` names the borrowed actual value and the
///   expression is its `bool` body, evaluated with the constructor parameters
///   in scope.
#[macro_export]
macro_rules! define_matcher {
    // Public form, no constructor parameters. Normalizes to the `@build` arm
    // with an empty parameter list.
    (
        $(#[$meta:meta])*
        $vis:vis fn $name:ident for $target:ty {
            expects: $expects:expr,
            matches: | $actual:ident | $body:expr $(,)?
        }
    ) => {
        $crate::define_matcher! {
            @build
            $(#[$meta])*
            $vis fn $name () for $target {
                expects: $expects,
                matches: | $actual | $body
            }
        }
    };

    // Public form, with constructor parameters.
    (
        $(#[$meta:meta])*
        $vis:vis fn $name:ident ( $( $param:ident : $pty:ty ),* $(,)? ) for $target:ty {
            expects: $expects:expr,
            matches: | $actual:ident | $body:expr $(,)?
        }
    ) => {
        $crate::define_matcher! {
            @build
            $(#[$meta])*
            $vis fn $name ( $( $param : $pty ),* ) for $target {
                expects: $expects,
                matches: | $actual | $body
            }
        }
    };

    // Internal worker. The parameter list is always present (possibly empty),
    // so there is a single shape to expand.
    (
        @build
        $(#[$meta:meta])*
        $vis:vis fn $name:ident ( $( $param:ident : $pty:ty ),* ) for $target:ty {
            expects: $expects:expr,
            matches: | $actual:ident | $body:expr
        }
    ) => {
        $(#[$meta])*
        $vis fn $name ( $( $param : $pty ),* ) -> impl $crate::Matcher<$target> {
            struct __TbDefinedMatcher {
                $( $param : $pty , )*
            }

            impl $crate::Matcher<$target> for __TbDefinedMatcher {
                #[allow(unused_variables, clippy::clone_on_copy)]
                fn check(&self, __tb_actual: &$target) -> $crate::MatchResult {
                    $( let $param = ::core::clone::Clone::clone(&self.$param); )*
                    let $actual = __tb_actual;
                    if $body {
                        $crate::MatchResult::pass()
                    } else {
                        $crate::MatchResult::fail($crate::Mismatch::new(
                            $crate::Matcher::description(self),
                            ::std::format!("{:?}", __tb_actual),
                        ))
                    }
                }

                #[allow(unused_variables, clippy::clone_on_copy)]
                fn description(&self) -> $crate::Description {
                    $( let $param = ::core::clone::Clone::clone(&self.$param); )*
                    $crate::Description::text($expects)
                }
            }

            __TbDefinedMatcher {
                $( $param , )*
            }
        }
    };
}
