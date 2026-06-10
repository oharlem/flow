// Workspace-level task anchor file for confirmation settings.
//
// The real automated coverage for this feature lives in crate tests under
// `crates/flow-cli/tests/` and `crates/flow-core/src/`.
// the repository-level `tests/` directory, so this file anchors each task ID to
// the test that verifies it.
//
// - T-001 — `crates/flow-core/src/settings.rs`
//     `t006_missing_settings_default_confirmation_no`,
//     `t001_t005_settings_round_trip_confirmation_no`, and
//     strict invalid-value tests cover project settings defaults and
//     persistence.
//
// - T-002 — `crates/flow-cli/tests/integration.rs`
//     `t001_t002_flow_set_writes_project_confirmation_setting` and
//     `t002_t005_flow_set_rejects_invalid_settings_without_changes` cover the
//     `flow set confirmation=yes|no` command and invalid assignment handling.
//
// - T-003 — `crates/flow-cli/tests/integration.rs`
//     `t003_confirmation_no_suppresses_protected_branch_prompt` and
//     `t003_t005_confirmation_no_suppresses_finalize_confirmation_text` cover
//     confirmation suppression in prompt and finalization output paths.
//
// - T-004 — `crates/flow-cli/tests/integration.rs`
//     `help_lists_all_commands` covers root help output, and the regenerated
//     `docs/reference/cli.md` covers generated reference output.
//
// - T-005 — `crates/flow-cli/tests/integration.rs`
//     The `t002_t005_*` and `t003_t005_*` integration tests are the end-to-end
//     persisted-confirmation regression coverage.
//
// - T-006 — `crates/flow-cli/tests/integration.rs`
//     `t006_default_confirmation_no_suppresses_protected_branch_prompt` covers
//     the default `confirmation=no` behavior without a saved setting.
//
// - T-007 — `crates/flow-cli/tests/integration.rs`
//     `t007_t008_flow_settings_lists_defaults_and_saved_values` covers the
//     `flow settings` command.
//
// - T-008 — `crates/flow-cli/tests/integration.rs`
//     `help_lists_all_commands` and
//     `t007_t008_flow_settings_lists_defaults_and_saved_values` cover the CLI
//     help and regression behavior for default settings display.
