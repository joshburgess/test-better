//! Golden-file test for the diff renderer: a multi-line string mismatch
//! produces a stable, line-oriented diff.
//!
//! The whole file is gated on the `diff` feature, so `--no-default-features`
//! builds compile it away cleanly rather than failing on a missing item.
#![cfg(feature = "diff")]

use test_better_matchers::diff_lines;

/// The golden output lives in a file so a reviewer sees the exact rendered
/// diff in the PR, not an inline string literal. To refresh it after an
/// intentional change, run with `TEST_BETTER_BLESS=1`.
#[test]
fn multi_line_string_mismatch_matches_the_golden_file() {
    let expected = "the quick brown fox\njumps over the lazy dog\nand then keeps running";
    let actual = "the quick brown fox\nleaps over the lazy cat\nand then keeps running";

    let rendered = diff_lines(expected, actual);

    let golden_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/golden/multiline_string_diff.txt"
    );
    if std::env::var_os("TEST_BETTER_BLESS").is_some() {
        std::fs::write(golden_path, format!("{rendered}\n")).expect("write golden file");
        return;
    }

    let golden = std::fs::read_to_string(golden_path).expect("read golden file");
    assert_eq!(
        rendered,
        golden.trim_end_matches('\n'),
        "diff drifted from golden"
    );
}
