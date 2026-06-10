//! Integration tests for `/flow-start` positional M-NNNN parsing. T-006, T-007.

use assert_cmd::Command;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

fn make_flow_repo() -> TempDir {
    let td = TempDir::new().unwrap();
    let path = td.path();
    std::process::Command::new("git")
        .args(["init", "-q"])
        .current_dir(path)
        .output()
        .unwrap();
    std::process::Command::new("git")
        .args(["config", "user.email", "test@example.com"])
        .current_dir(path)
        .output()
        .unwrap();
    std::process::Command::new("git")
        .args(["config", "user.name", "test"])
        .current_dir(path)
        .output()
        .unwrap();
    std::process::Command::new("git")
        .args(["commit", "--allow-empty", "-q", "-m", "init"])
        .current_dir(path)
        .output()
        .unwrap();
    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(path)
        .args(["init"])
        .assert()
        .success();
    td
}

fn seed_roadmap_with(repo: &Path, ids: &[&str]) -> PathBuf {
    let milestones: Vec<(&str, String)> = ids
        .iter()
        .map(|id| (*id, format!("Title for {id}")))
        .collect();
    seed_roadmap_with_titles(repo, &milestones)
}

fn seed_roadmap_with_titles(repo: &Path, milestones: &[(&str, String)]) -> PathBuf {
    let run_dir = repo.join("flow").join("runs").join(format!(
        "{}-roadmap-start",
        chrono::Utc::now().format("%Y%m%d")
    ));
    std::fs::create_dir_all(run_dir.join("changes")).unwrap();
    let mut body = String::from("# Roadmap\n\n## Milestones\n\n");
    for (id, title) in milestones {
        body.push_str(&format!("### [ ] {id}: {title}\n\n"));
    }
    std::fs::write(run_dir.join("roadmap.md"), &body).unwrap();
    let fingerprint = flow_core::roadmap::fingerprint(&body);
    let snapshot = flow_core::parse::roadmap::parse_str(&body)
        .into_iter()
        .map(|m| format!("- [ ] {} — {}", m.id, m.title))
        .collect::<Vec<_>>()
        .join("\n");
    std::fs::write(
        run_dir.join("run.md"),
        format!(
            "# Run: Start\n\n**Run name**: {}\n**Run type**: roadmap\n**Run scope**: (none)\n**Status**: running\n**Run branch**: (none)\n**Roadmap fingerprint**: {fingerprint}\n**Checkpoint commits**: disabled\n**Current milestone**: (none)\n**Current change**: (none)\n**Current phase**: roadmap-ready\n**Last saved Flow action**: roadmap-finalized\n**Next command**: $flow-run\n**Last checkpoint**: (none)\n\n## Changes\n\n(none)\n\n## Milestones\n\n{snapshot}\n",
            run_dir.file_name().unwrap().to_string_lossy(),
        ),
    )
    .unwrap();
    std::fs::write(run_dir.join("log.md"), "# Run Log\n\n## Event Log\n\n").unwrap();
    run_dir
}

fn feature_dirs(repo: &Path) -> Vec<std::path::PathBuf> {
    let mut entries = Vec::new();
    let runs = repo.join("flow").join("runs");
    for run in std::fs::read_dir(runs).unwrap().flatten() {
        let changes = run.path().join("changes");
        if let Ok(read) = std::fs::read_dir(changes) {
            entries.extend(
                read.flatten()
                    .filter(|e| e.path().join("status.md").is_file())
                    .map(|e| e.path()),
            );
        }
    }
    entries.sort();
    entries
}

#[test]
fn t007_start_with_milestone_only_argument() {
    let repo = make_flow_repo();
    let run_dir = seed_roadmap_with_titles(repo.path(), &[("M-3", "Release Flow UX".to_string())]);
    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .env("FLOW_RUN_DIR", &run_dir)
        .env("FLOW_FORCE_ON_PROTECTED", "1")
        .args(["start", "M-3"])
        .assert()
        .success();
    // Status.md should have **Milestone**: M-3.
    let entries = feature_dirs(repo.path());
    assert!(!entries.is_empty(), "no change dir created");
    let dir = entries.first().expect("no change directory");
    assert_eq!(
        dir.file_name().unwrap().to_string_lossy(),
        "M-3-release-flow-ux"
    );
    let status = std::fs::read_to_string(dir.join("status.md")).unwrap();
    assert!(
        status.contains("**Milestone**: M-3"),
        "status.md should reference M-3:\n{status}"
    );
}

#[test]
fn t002_t006_start_accepts_non_padded_milestone_id() {
    let repo = make_flow_repo();
    let run_dir = seed_roadmap_with_titles(repo.path(), &[("M-1", "Release Flow UX".to_string())]);
    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .env("FLOW_RUN_DIR", &run_dir)
        .env("FLOW_FORCE_ON_PROTECTED", "1")
        .args(["start", "M-1"])
        .assert()
        .success();

    let entries = feature_dirs(repo.path());
    let dir = entries.first().expect("no change directory");
    assert_eq!(
        dir.file_name().unwrap().to_string_lossy(),
        "M-1-release-flow-ux"
    );
    let status = std::fs::read_to_string(dir.join("status.md")).unwrap();
    assert!(status.contains("**Milestone**: M-1"), "{status}");
}

#[test]
fn t007_start_milestone_then_title() {
    let repo = make_flow_repo();
    let run_dir = seed_roadmap_with_titles(repo.path(), &[("M-3", "Release Flow UX".to_string())]);
    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .env("FLOW_RUN_DIR", &run_dir)
        .env("FLOW_FORCE_ON_PROTECTED", "1")
        .args(["start", "M-3", "implement", "auth"])
        .assert()
        .success();
    let entries = feature_dirs(repo.path());
    let dir = entries.first().expect("no change directory");
    assert_eq!(
        dir.file_name().unwrap().to_string_lossy(),
        "M-3-release-flow-ux",
        "linked milestone should control feature naming even when extra description is supplied"
    );
}

#[test]
fn t007_start_title_then_milestone() {
    let repo = make_flow_repo();
    let run_dir = seed_roadmap_with(repo.path(), &["M-3"]);
    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .env("FLOW_RUN_DIR", &run_dir)
        .env("FLOW_FORCE_ON_PROTECTED", "1")
        .args(["start", "implement", "auth", "M-3"])
        .assert()
        .success();
}

#[test]
fn t007_start_rejects_milestone_flag() {
    // The --milestone flag must not exist; clap should fail to parse.
    let repo = make_flow_repo();
    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .env("FLOW_FORCE_ON_PROTECTED", "1")
        .args(["start", "--milestone", "M-3", "implement"])
        .assert()
        .failure();
}

#[test]
fn t007_start_rejects_missing_milestone() {
    let repo = make_flow_repo();
    let run_dir = seed_roadmap_with(repo.path(), &["M-1"]);
    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .env("FLOW_RUN_DIR", &run_dir)
        .env("FLOW_FORCE_ON_PROTECTED", "1")
        .args(["start", "M-999", "missing"])
        .assert()
        .failure();
}

#[test]
fn t007_start_rejects_two_m_nnn_tokens() {
    let repo = make_flow_repo();
    let run_dir = seed_roadmap_with(repo.path(), &["M-1", "M-2"]);
    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .env("FLOW_RUN_DIR", &run_dir)
        .env("FLOW_FORCE_ON_PROTECTED", "1")
        .args(["start", "M-1", "M-2", "ambiguous"])
        .assert()
        .failure();
}

#[test]
fn t007_start_quoted_m3_in_text_does_not_extract() {
    let repo = make_flow_repo();
    // Roadmap has no milestones — should not error on missing milestone.
    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .env("FLOW_FORCE_ON_PROTECTED", "1")
        // "M-3 looks weird" is one quoted argument; M-3 is not a stand-alone token here.
        // Note: clap parses argv tokens — assert_cmd passes them as separate args, so a single string arg
        // with internal whitespace is still received as one element.
        .args(["start", "M-3 looks weird"])
        .assert()
        .success();
}

#[test]
fn t006_start_creates_no_milestone_without_link() {
    let repo = make_flow_repo();
    let original_roadmap = "# Roadmap\n\n## Milestones\n\n### [x] M-1: Old\n\n";
    let flow = repo.path().join("flow");
    std::fs::create_dir_all(&flow).unwrap();
    std::fs::write(flow.join("roadmap.md"), original_roadmap).unwrap();
    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .env("FLOW_FORCE_ON_PROTECTED", "1")
        .args(["start", "fix typo"])
        .assert()
        .success();
    let roadmap = std::fs::read_to_string(flow.join("roadmap.md")).unwrap();
    assert_eq!(roadmap, original_roadmap, "roadmap should be untouched");
    let entries = feature_dirs(repo.path());
    let dir = entries.first().expect("no change dir created");
    let status = std::fs::read_to_string(dir.join("status.md")).unwrap();
    assert!(
        !status.contains("**Milestone**:"),
        "status.md should not have a Milestone line:\n{status}"
    );
}
