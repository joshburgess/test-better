#!/usr/bin/env bash
# Dogfood check: the workspace's own tests must use `expect!`, `TestResult`,
# and `or_fail` rather than the stock panic/assert API.
#
# `assert!`, `assert_eq!`, `assert_ne!`, `.unwrap()`, and `Result`/`Option`'s
# `.expect("...")` must not appear anywhere under a crate's `src/` — that covers
# both inline `#[cfg(test)]` modules and `///` doc examples. They remain allowed
# under `tests/` and `examples/`, which are exercised by the clippy
# `*-in-tests` allowances, not by this check.
#
# The `.expect` pattern is matched only when followed by a string literal
# (`.expect("`), the universal shape of the panic-on-`None`/`Err` call. This
# deliberately does not match `SoftAsserter::expect(&actual, matcher)`, which
# is the library's own soft-assertion API, not a panic operator. A non-test
# `.expect` with a non-literal message is still caught by the workspace's
# `clippy::expect_used = "deny"` lint.
#
# Run from the repository root. Exits non-zero (and prints the offenders) if
# any banned pattern is found.
set -euo pipefail

pattern='assert(_eq|_ne)?!|\.unwrap\(\)|\.expect\("'

if matches=$(grep -rnE "$pattern" crates/*/src/ 2>/dev/null); then
    echo "error: banned panic/assert API found under crates/*/src/"
    echo "       tests in src/ must dogfood expect!/TestResult/or_fail"
    echo
    echo "$matches"
    exit 1
fi

echo "ok: no assert!/unwrap()/expect() under crates/*/src/"
