// Workspace-level task anchor file for the flow-roadmap + confirmation feature.
//
// The Flow drift scanner walks `<repo>/tests/` and looks for task IDs. Since
// this feature's tests live under `crates/*/tests/` and `crates/*/src/*`, this
// file anchors the task IDs to the automated tests that actually verify them.
//
// Anchored IDs and the test file that covers each:
//
// - T-001 — `crates/flow-cli/tests/roadmap_cli.rs`
//     `t001_help_works` exercises the new `flow roadmap --help` subcommand
//     wired by T-001.
//
// - T-002 — `crates/flow-core/src/assets.rs`
//     `t002_roadmap_phase_registered` asserts that "roadmap" is in PHASES,
//     HOST_COMMANDS, and that `agent_base("roadmap")` resolves.
//
// - T-003 — `crates/flow-core/src/roadmap.rs`
//     `t003_count_milestones_empty_or_missing_returns_zero`,
//     `t003_count_milestones_counts_all_states`,
//     `t003_highest_milestone_id_returns_max`,
//     `t003_highest_milestone_id_returns_none_when_empty`.
//
// - T-004 — `crates/flow-cli/tests/roadmap_cli.rs`
//     `t004_roadmap_envelope_on_empty_roadmap_uses_append_mode_silently`,
//     `t004_roadmap_replace_mode_emits_destructive_action`,
//     `t004_roadmap_append_with_existing_uses_next_free_id`,
//     `t004_roadmap_with_file_source_reads_file`,
//     `t004_roadmap_empty_source_errors`.
//
// - T-005 — `crates/flow-cli/tests/roadmap_cli.rs`
//     `t005_finalize_validates_roadmap`,
//     `t005_finalize_errors_when_roadmap_missing`.
//
// - T-006 — `crates/flow-cli/tests/integration.rs` and
//     `crates/flow-cli/tests/start_milestone.rs`
//     `t006_start_creates_no_milestone_without_link` (in both files) asserts
//     `flow start "title"` does not auto-create a milestone and the
//     resulting `status.md` has no `**Milestone**:` line.
//
// - T-007 — `crates/flow-cli/tests/start_milestone.rs`
//     `t007_start_with_milestone_only_argument`,
//     `t007_start_milestone_then_title`,
//     `t007_start_title_then_milestone`,
//     `t007_start_with_milestone_flag`,
//     `t007_start_rejects_missing_milestone`,
//     `t007_start_rejects_two_m_nnn_tokens`,
//     `t007_start_quoted_m3_in_text_does_not_extract`.
//
// - T-008 — `crates/flow-cli/tests/host_snapshots.rs`
//     The tree snapshots for claude-code and codex include the new
//     `flow-roadmap` skill file, asserting the host adapters surface the
//     command.
//
// - T-009 — `crates/flow-cli/tests/golden.rs`
//     `golden_01_init_minimal` re-asserts the current installed control-plane
//     files and embedded-default behavior.
//
// - T-010 — `crates/flow-cli/tests/cli_drift.rs` and
//     `crates/flow-cli/tests/summary_drift.rs`
//     The drift tests assert `docs/reference/cli.md` and `docs/SUMMARY.md`
//     match what the binary would generate, so the regenerated outputs from
//     T-010 are continuously verified.
//
// - T-011 — `crates/flow-cli/tests/e2e_scenarios.rs`
//     `t011_roadmap_then_start_with_milestone_and_destructive_replace`
//     drives the full roadmap → start → replace scenario end-to-end.
//
// - T-012 — `crates/flow-core/src/envelope.rs`
//     `t012_confirmation_required_emitted_when_setting_required`,
//     `t012_confirmation_disabled_emitted_when_setting_no`,
//     `t012_confirmation_disabled_when_settings_missing`,
//     `t012_destructive_action_line_present_when_reason_supplied`,
//     `t012_destructive_action_line_absent_when_reason_none`.
//
// - T-013 — `crates/flow-core/src/assets.rs`
//     `t013_every_phase_prompt_has_canonical_confirmation_paragraph` walks
//     every entry in PHASES and asserts the canonical confirmation paragraph
//     plus the disabled-confirmation precedence clause are present.
//
// - T-014 — `crates/flow-cli/tests/release_destructive.rs`
//     `t014_release_envelope_confirmation_disabled_does_not_prompt` and
//     `t014_release_envelope_confirmation_required_prompts` assert release
//     envelopes honor `confirmation=no|yes` without a destructive-action
//     override. `t014_non_release_phases_emit_confirmation_only` asserts
//     non-release phases emit `**Confirmation**:` only.
//
// - T-015 — `crates/flow-cli/tests/golden.rs`
//     `golden_01_init_minimal` re-asserts that `.flow/conventions/core.md`
//     includes the `## 11. Confirmation behavior` section by virtue of the
//     byte-identical shard comparison; the docs reference paragraph is
//     implicitly covered by `summary_drift` and `cli_drift` reading the
//     regenerated
//     reference docs.
