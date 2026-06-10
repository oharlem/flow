// Workspace-level task anchor file for the control file model cleanup feature.
//
// T-001: `crates/flow-core/src/config.rs` and
// `crates/flow-core/src/settings.rs` cover config-backed confirmation and
// state-backed counter persistence.
//
// T-002: `crates/flow-cli/tests/integration.rs` covers `flow set
// confirmation=<value>`, `flow set counter=<n>`, invalid setting handling, and
// effective `flow settings` output across config/state storage.
//
// T-003: documentation and template updates are covered by text review plus
// this anchor so the task-to-verification relationship is visible.
//
// T-004: `crates/flow-cli/tests/roadmap_cli.rs`,
// `crates/flow-cli/tests/integration.rs`, and the full workspace verification
// cover persistence and documentation-reference behavior.
