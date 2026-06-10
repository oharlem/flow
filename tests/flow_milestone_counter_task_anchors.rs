// Workspace-level task anchor file for the milestone counter feature.
//
// T-001: `crates/flow-core/src/settings.rs` and
// `crates/flow-cli/tests/integration.rs` cover persistent `counter` settings
// and `flow set counter=<n>` validation.
//
// T-002: `crates/flow-core/src/parse/roadmap.rs`,
// `crates/flow-core/src/roadmap.rs`, and
// `crates/flow-cli/tests/start_milestone.rs` cover non-padded milestone IDs
// for lookup and closeout.
//
// T-003: `crates/flow-cli/tests/roadmap_cli.rs` covers roadmap allocation from
// the counter setting and counter advancement after roadmap finalize.
//
// T-004: `crates/flow-core/src/roadmap.rs` and
// `crates/flow-cli/tests/roadmap_cli.rs` cover strict ID validation and
// duplicate numeric collision protection.
//
// T-005: Documentation and embedded guidance updates are covered by text
// assertions in review plus this anchor for task traceability.
//
// T-006: The integration tests named above exercise the end-to-end milestone,
// counter reset, start linkage, and closeout behaviors.
