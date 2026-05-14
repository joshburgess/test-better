#!/usr/bin/env bash
# Enforces PROJECT_BUILD_PLAN.md Iteration 2.5 (the dogfood switchover): the
# workspace's own tests must use `expect!`, `TestResult`, and `or_fail` rather
# than the stock panic/assert API.
#
# `assert!`, `assert_eq!`, `assert_ne!`, `.unwrap()`, and `.expect(` must not
# appear anywhere under a crate's `src/` — that covers both inline
# `#[cfg(test)]` modules and `///` doc examples. They remain allowed under
# `tests/` and `examples/`, which are exercised by the clippy `*-in-tests`
# allowances, not by this check.
#
# Run from the repository root. Exits non-zero (and prints the offenders) if
# any banned pattern is found.
set -euo pipefail

pattern='assert(_eq|_ne)?!|\.unwrap\(\)|\.expect\('

if matches=$(grep -rnE "$pattern" crates/*/src/ 2>/dev/null); then
    echo "error: banned panic/assert API found under crates/*/src/"
    echo "       tests in src/ must dogfood expect!/TestResult/or_fail (PROJECT_BUILD_PLAN.md 2.5)"
    echo
    echo "$matches"
    exit 1
fi

echo "ok: no assert!/unwrap()/expect() under crates/*/src/"
