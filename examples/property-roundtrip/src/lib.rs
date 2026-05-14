//! Property testing a roundtrip with `test-better`.
//!
//! The most reliable property to reach for is a *roundtrip*: if you encode a
//! value and decode the result, you should get the value back. It needs no
//! reference implementation, it holds for every input, and a counterexample
//! is a concrete bug.
//!
//! Here [`encode`] turns a `Vec<i64>` into a newline-separated string and
//! [`decode`] parses it back. The `property!` macro generates inputs (here,
//! `Vec<i64>` is `proptest::Arbitrary`, so the strategy is inferred from the
//! binding), runs the body against each, and on a failure shrinks the input
//! to a minimal counterexample.
//!
//! Run the suite with `cargo test -p property-roundtrip-example`.

/// Why [`decode`] could not parse its input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecodeError {
    /// The zero-based line that failed to parse.
    pub line: usize,
    /// The offending text.
    pub text: String,
}

/// Encodes a slice of integers as one decimal number per line.
///
/// The empty slice encodes to the empty string.
#[must_use]
pub fn encode(values: &[i64]) -> String {
    values
        .iter()
        .map(i64::to_string)
        .collect::<Vec<_>>()
        .join("\n")
}

/// Decodes the output of [`encode`] back into a `Vec<i64>`.
///
/// The empty string decodes to the empty vector. Any line that is not a valid
/// `i64` is a [`DecodeError`].
pub fn decode(encoded: &str) -> Result<Vec<i64>, DecodeError> {
    if encoded.is_empty() {
        return Ok(Vec::new());
    }
    encoded
        .split('\n')
        .enumerate()
        .map(|(line, text)| {
            text.parse::<i64>().map_err(|_| DecodeError {
                line,
                text: text.to_string(),
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use test_better::prelude::*;

    #[test]
    fn decoding_an_encoded_value_returns_it_for_every_input() -> TestResult {
        // The roundtrip property. `Vec<i64>` is `Arbitrary`, so `property!`
        // infers the strategy from the binding and runs the body against many
        // generated vectors; a failure would be shrunk to a minimal one.
        property!(|values: Vec<i64>| {
            let encoded = encode(&values);
            expect!(decode(&encoded)).to(eq(Ok(values)))
        })
    }

    #[test]
    fn a_single_value_in_an_explicit_range_roundtrips() -> TestResult {
        // The `using` clause supplies an explicit strategy instead of an
        // inferred one: an ordinary numeric range.
        property!(|n| {
            let encoded = encode(&[n]);
            expect!(decode(&encoded)).to(eq(Ok(vec![n])))
        } using -1_000_000i64..1_000_000)
    }

    #[test]
    fn the_empty_vector_roundtrips() -> TestResult {
        // The edge case the property generator will also reach, pinned down on
        // its own so it is obvious it is covered.
        expect!(encode(&[])).to(eq(String::new()))?;
        expect!(decode("")).to(eq(Ok(Vec::<i64>::new())))?;
        Ok(())
    }

    #[test]
    fn a_known_encoding_has_the_expected_shape() -> TestResult {
        expect!(encode(&[1, -2, 3])).to(eq(String::from("1\n-2\n3")))
    }

    #[test]
    fn a_non_numeric_line_is_a_decode_error() -> TestResult {
        let error = decode("1\noops\n3")
            .err()
            .or_fail_with("\"oops\" is not an i64")?;
        expect!(error.line).to(eq(1))?;
        expect!(error.text).to(eq(String::from("oops")))?;
        Ok(())
    }
}
