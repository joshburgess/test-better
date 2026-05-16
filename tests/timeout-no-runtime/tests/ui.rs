//! Confirms that the runtime-gated async assertions fail to compile, with a
//! diagnostic that names the runtime feature flags, when none of them is
//! enabled: `completes_within` and `eventually`.
//!
//! `trybuild` compiles the ui file in an isolated build, so the runtime
//! features that other test crates enable on `test-better` do not leak in
//! here: this build sees `test-better` with its default features only.

#[test]
fn completes_within_without_a_runtime_feature_is_a_clear_error() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/ui/missing_runtime_feature.rs");
}

#[test]
fn eventually_without_a_runtime_feature_is_a_clear_error() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/ui/eventually_missing_runtime.rs");
}
