// Workspace-level task anchor file for the Phase B docs re-layout.
//
// This file is not a Cargo target. It records which automated tests cover the
// Phase B task IDs so the relationship stays visible in the repo.
//
// Anchored IDs and the test file that covers each:
//
// - T-001 — `crates/flow-cli/tests/cli_drift.rs`
//     The shared `cli_help::render_full()` helper is exercised by
//     `t010_no_drift_warning_when_cli_md_is_fresh`, which writes its output
//     to disk and asserts the doctor warning is absent.
//
// - T-002 — `crates/flow-cli/tests/docs_layout.rs`
//     `t011_new_reference_files_exist` asserts `docs/reference/cli.md` is
//     present; `t011_old_doc_paths_are_gone` asserts `docs/cli-reference.md`
//     is gone.
//
// - T-003 — `crates/flow-cli/tests/docs_layout.rs`
//     `t011_new_reference_files_exist` asserts `docs/reference/commands.md`
//     is present.
//
// - T-004 — `crates/flow-cli/tests/docs_layout.rs`
//     `t011_new_reference_files_exist` asserts `docs/reference/artifacts.md`
//     is present.
//
// - T-005 — `crates/flow-cli/tests/docs_layout.rs`
//     `t011_old_doc_paths_are_gone` asserts `docs/commands/` and
//     `docs/artifacts/` are gone.
//
// - T-006 — `crates/flow-cli/tests/docs_layout.rs`
//     `t011_no_links_to_deleted_doc_paths` walks every Markdown file and
//     fails on any reference to the deleted paths.
//
// - T-007 — `crates/flow-cli/tests/docs_layout.rs`
//     The link-guard scanned the record map before that page was retired;
//     stale routing-table cells would have failed the guard.
//
// - T-008 — `crates/flow-cli/tests/docs_layout.rs`
//     The link-guard also scans `docs/SUMMARY.md`; stale TOC entries would
//     fail the guard.
//
// - T-009 — `crates/flow-cli/tests/cli_drift.rs`
//     `t010_drift_warning_fires_on_stale_cli_md`,
//     `t010_no_drift_warning_when_cli_md_is_fresh`, and
//     `t009_drift_check_silent_when_cli_md_missing` cover the warning,
//     fresh-state, and absent-file behavior of the doctor check.
//
// - T-010 — `crates/flow-cli/tests/cli_drift.rs`
//     The `t010_*` tests are the integration test required by T-010.
//
// - T-011 — `crates/flow-cli/tests/docs_layout.rs`
//     The `t011_*` tests are the regression suite required by T-011.
//
// - T-012 — `crates/flow-cli/tests/cli_drift.rs` and
//   `crates/flow-cli/tests/docs_layout.rs`
//     T-012 is the consolidated `cargo test --workspace` / `cargo fmt` /
//     `cargo clippy` green-bar; both files participate in that bar.
