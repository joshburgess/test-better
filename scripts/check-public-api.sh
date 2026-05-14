#!/usr/bin/env bash
# Enforces PROJECT_BUILD_PLAN.md Iteration 10.1 (the public API review): every
# crate's public surface is captured as a committed `public-api/<crate>.txt`
# snapshot, and this check fails if the live surface has drifted from it.
#
# A drift is not necessarily a bug: it just means the public API changed. The
# fix is to review the diff, and if the change is intended, regenerate the
# snapshots with `--write` and commit them alongside the code change.
#
# Needs `cargo-public-api` (`cargo install cargo-public-api`) and a `nightly`
# toolchain available to rustup (cargo-public-api drives nightly rustdoc to
# emit the JSON it reads).
#
# Run from the repository root.
#   scripts/check-public-api.sh           # check; non-zero on drift
#   scripts/check-public-api.sh --write   # regenerate the committed snapshots
set -euo pipefail

crates=(
    test-better-core
    test-better-matchers
    test-better-macros
    test-better-async
    test-better-property
    test-better-snapshot
    test-better-runner
    test-better
)

mode="check"
if [[ "${1:-}" == "--write" ]]; then
    mode="write"
fi

mkdir -p public-api
status=0

for crate in "${crates[@]}"; do
    snapshot="public-api/${crate}.txt"
    if [[ "$mode" == "write" ]]; then
        cargo public-api -p "$crate" --all-features --simplified > "$snapshot"
        echo "wrote $snapshot"
        continue
    fi

    live=$(cargo public-api -p "$crate" --all-features --simplified)
    if ! diff -u "$snapshot" <(printf '%s\n' "$live") > /dev/null 2>&1; then
        echo "error: public API of $crate has drifted from $snapshot"
        diff -u "$snapshot" <(printf '%s\n' "$live") || true
        status=1
    fi
done

if [[ "$mode" == "write" ]]; then
    exit 0
fi

if [[ "$status" -ne 0 ]]; then
    echo
    echo "       the public API changed. review the diff above; if intended,"
    echo "       run 'scripts/check-public-api.sh --write' and commit the result."
    exit 1
fi

echo "ok: public API matches the committed public-api/ snapshots"
