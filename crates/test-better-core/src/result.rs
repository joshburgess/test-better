//! [`TestResult`]: the return type that makes `?` the single control-flow
//! operator of a test.
//!
//! A test (or any test helper) returns `TestResult`, so every fallible step
//! short-circuits with `?` and the first failure is the one reported.

use crate::error::TestError;

/// The result type returned by `test-better` tests and helpers.
///
/// It defaults its `Ok` type to `()`, the common case for a `#[test]` function,
/// while helpers that produce a value name it explicitly (`TestResult<User>`).
///
/// # Examples
///
/// ```
/// use test_better_core::{TestError, TestResult};
///
/// fn checked_div(numerator: i32, denominator: i32) -> TestResult<i32> {
///     if denominator == 0 {
///         return Err(TestError::assertion("denominator must be non-zero"));
///     }
///     Ok(numerator / denominator)
/// }
///
/// fn test_division() -> TestResult {
///     let quotient = checked_div(10, 2)?;
///     if quotient != 5 {
///         return Err(TestError::from_expected_actual(5, quotient));
///     }
///     Ok(())
/// }
///
/// fn main() -> TestResult {
///     test_division()?;
///     Ok(())
/// }
/// ```
pub type TestResult<T = ()> = Result<T, TestError>;
