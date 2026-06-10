// Workspace-level task anchor file for
// Source-handling behavior is documented in docs/reference/commands.md and
// current work records live under the unified `flow/runs/` workspace.
//
// The Flow drift scanner walks `<repo>/tests/` for task IDs. The executable
// tests for this feature live under `crates/*`, so these comments anchor each
// task to the automated coverage that verifies it.
//
// - T-001 — `crates/flow-cli/tests/roadmap_cli.rs`
//     `t001_t002_roadmap_missing_path_like_source_errors_without_envelope`
//     covers the missing path-like source regression.
//
// - T-002 — `crates/flow-cli/src/cmd/roadmap.rs` and
//     `crates/flow-cli/tests/roadmap_cli.rs`
//     `t001_t002_roadmap_missing_path_like_source_errors_without_envelope`
//     verifies the command errors before envelope output, while the existing
//     inline and readable-file tests preserve accepted source forms.
//
// - T-003 — `assets/agents/roadmap.base.md` and
//     `crates/flow-core/src/assets.rs`
//     `t003_roadmap_prompt_requires_preview_even_when_confirmation_is_disabled`
//     verifies the embedded roadmap prompt keeps the preview step separate from
//     save confirmation behavior.
