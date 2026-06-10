//! Snapshot tests for byte-stable rendered outputs.
//!
//! These lock `insta` snapshots of pure string outputs produced by
//! `flow-core`: drift render, template rendering, status stamping, and
//! envelope composition. Any incidental wording change fails CI until
//! `cargo insta review` intentionally accepts the new snapshot.

use flow_core::{
    drift::{self, render::Mode},
    parse,
};

#[test]
fn snapshot_drift_clean_status_mode() {
    let report = drift::Report::default();
    let rendered = drift::render::render(&report, Mode::Status, "/flow-status", false);
    insta::assert_snapshot!("drift_clean_status_mode", rendered);
}

#[test]
fn snapshot_drift_missing_tasks() {
    let report = drift::Report::default();
    let rendered = drift::render::render(&report, Mode::Plan, "/flow-plan", true);
    insta::assert_snapshot!("drift_missing_tasks", rendered);
}

#[test]
fn snapshot_status_stamp_result() {
    use flow_core::parse::status::{parse_str, stamp, State};
    let td = tempfile::TempDir::new().unwrap();
    let feat = td.path().join("f");
    std::fs::create_dir_all(&feat).unwrap();
    std::fs::write(
        feat.join("status.md"),
        "# Status: foo\n\
         \n**Change**: foo\n\
         **Started**: 2026-05-06\n\
         **Updated**: 2026-05-06T00:00:00Z\n\
         **State**: drafting\n\
         **Branch**: flow/foo\n\
         \n## History\n\n\
         - 2026-05-06T00:00:00Z — started — seeded\n",
    )
    .unwrap();

    stamp(
        &feat,
        Some(State::Building),
        "plan-complete",
        "plan finalized",
    )
    .unwrap();
    let s = parse_str(&std::fs::read_to_string(feat.join("status.md")).unwrap());
    // Snapshot structural shape (not timestamps): change/state/branch/history action sequence.
    let shape = format!(
        "change={}\nstate={:?}\nbranch={}\nhistory_actions=[{}]\n",
        s.feature,
        s.state,
        s.branch,
        s.history
            .iter()
            .map(|h| h.action.as_str())
            .collect::<Vec<_>>()
            .join(","),
    );
    insta::assert_snapshot!("status_stamp_shape", shape);
}

#[test]
fn snapshot_tasks_parser_output() {
    let text = "## Tasks\n\n\
                - [ ] **T-001**: first task\n  - Covers: FR-001\n  - Verifies: SC-001\n\
                - [x] **T-002**: second\n  - Covers: FR-002\n\
                - [~] **T-003**: third\n  - Covers: FR-003\n  - Depends-On: T-001, T-002\n";
    let parsed = parse::tasks::parse_str(text);
    let mut out = String::new();
    for t in &parsed {
        out.push_str(&format!(
            "{} state={:?} done={} covers={:?} verifies={:?} deps={:?}\n",
            t.id, t.state, t.done, t.covers, t.verifies, t.depends_on
        ));
    }
    insta::assert_snapshot!("tasks_parser_shape", out);
}

#[test]
fn snapshot_roadmap_parser_output() {
    let text = "## Milestones\n\n\
                ### [ ] M-1: First\n\nDesc 1.\n\n\
                ### [x] M-2: Second\n\nDesc 2.\n\n\
                ### [ ] M-3: Third\n";
    let ms = parse::roadmap::parse_str(text);
    let mut out = String::new();
    for m in &ms {
        out.push_str(&format!(
            "{} done={} title={:?} desc={:?}\n",
            m.id, m.done, m.title, m.description
        ));
    }
    insta::assert_snapshot!("roadmap_parser_shape", out);
}
