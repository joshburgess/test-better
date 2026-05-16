# Snapshots

A snapshot test asserts that a value still renders the way it did last time.
Instead of writing the expected output by hand, you let the test record it
once, commit that, and fail on any later change. It is the right tool for
output that is large, structured, or tedious to spell out: rendered HTML,
serialized payloads, formatted reports, error messages.

`test-better` has two flavours: file snapshots, stored in a `.snap` file next
to the test, and inline snapshots, stored in a string literal in the test
itself.

## File snapshots

`check!(value).matches_snapshot("name")` compares the value's `Display`
output against `tests/snapshots/<module_path>__<name>.snap`:

```rust
use test_better::prelude::*;

#[test]
fn the_home_page_renders() -> TestResult {
    let rendered = render_home_page();
    check!(rendered).matches_snapshot("home_page")
}
```

The first time this runs there is no `.snap` file, so the test fails with a
"missing snapshot" error. Record it by running with `UPDATE_SNAPSHOTS=1`:

```sh
UPDATE_SNAPSHOTS=1 cargo test
```

That writes the `.snap` file. Review it, commit it, and from then on the test
compares against it. When the output legitimately changes, re-run with
`UPDATE_SNAPSHOTS=1` and commit the updated file; when it changes
*unexpectedly*, the test fails with a diff.

## Inline snapshots

For short values, an inline snapshot keeps the expected output in the test:

```rust
use test_better::prelude::*;

#[test]
fn arithmetic_still_works() -> TestResult {
    check!(2 + 2).matches_inline_snapshot("4")
}
```

Multi-line values are written as a raw string; leading indentation is
normalized, so the literal can be indented to match the surrounding code:

```rust
use test_better::prelude::*;

#[test]
fn the_report_renders() -> TestResult {
    let report = ["name: alice", "score: 42", "status: active"].join("\n");
    check!(report).matches_inline_snapshot(
        r#"
        name: alice
        score: 42
        status: active
        "#,
    )
}
```

An inline snapshot starts empty. Run the test under `UPDATE_SNAPSHOTS=1` and it
records a *pending patch* rather than editing your source mid-run; apply the
pending patches with the `cargo test-better accept` companion (see the
[runner recipe](./recipes.md)).

## Redactions: ignoring the parts that always change

Real output often contains values that change every run (timestamps, UUIDs,
temp paths) but are not what the test is about. `Redactions` rewrites those to
a stable placeholder before the comparison:

```rust
use test_better::Redactions;
use test_better::prelude::*;

#[test]
fn the_audit_line_renders() -> TestResult {
    let line = format!("{} user=alice action=login", now_rfc3339());
    let redactions = Redactions::new()
        .redact_rfc3339_timestamps()
        .redact_uuids();
    check!(line).matches_snapshot_with("audit_line", &redactions)
}
```

`Redactions` is a builder: `redact_rfc3339_timestamps` and `redact_uuids` are
built in; `replace(needle, placeholder)` swaps a fixed string; `redact_with`
takes an arbitrary rewrite rule. `matches_snapshot_with` and
`matches_inline_snapshot_with` take the configured `Redactions`.

## When to snapshot, and when not

Snapshots are powerful but blunt: a snapshot test asserts on the *whole*
output, so it fails on any change, intended or not. Use one when the output is
genuinely too large or too structured to assert piece by piece. When you care
about one field, a targeted `check!` with `matches_struct!` or `contains_str`
says more about *what* matters and fails more precisely.
