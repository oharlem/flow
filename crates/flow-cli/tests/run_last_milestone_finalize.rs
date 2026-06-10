//! Last-milestone finalization behavior for roadmap-scoped runs.

use assert_cmd::Command;
use predicates::prelude::*;
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

fn write_roadmap(repo: &Path, body: &str) {
    let runs = repo.join("flow").join("runs");
    if runs.exists() {
        let mut dirs = std::fs::read_dir(&runs)
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .filter(|path| path.is_dir())
            .collect::<Vec<_>>();
        dirs.sort();
        if let Some(run_dir) = dirs.first() {
            std::fs::write(run_dir.join("roadmap.md"), body).unwrap();
            return;
        }
    }
    let descriptor = body
        .lines()
        .find_map(|line| line.trim().strip_prefix("# ").map(str::trim))
        .and_then(|title| title.strip_prefix("Roadmap:").map(str::trim))
        .unwrap_or("Roadmap");
    let run_dir = runs.join(format!(
        "{}-roadmap-{}",
        chrono::Utc::now().format("%Y%m%d"),
        slug(descriptor)
    ));
    std::fs::create_dir_all(run_dir.join("changes")).unwrap();
    std::fs::write(run_dir.join("roadmap.md"), body).unwrap();
    let fingerprint = flow_core::roadmap::fingerprint(body);
    let milestones = flow_core::parse::roadmap::parse_str(body)
        .into_iter()
        .map(|m| {
            let state = if m.done {
                "[x]"
            } else if m.in_progress {
                "[~]"
            } else {
                "[ ]"
            };
            format!("- {state} {} — {}", m.id, m.title)
        })
        .collect::<Vec<_>>()
        .join("\n");
    std::fs::write(
        run_dir.join("run.md"),
        format!(
            "# Run: {descriptor}\n\n**Run name**: {}\n**Run type**: roadmap\n**Run scope**: (none)\n**Status**: planned\n**Run branch**: (none)\n**Roadmap fingerprint**: {fingerprint}\n**Checkpoint commits**: enabled\n**Current milestone**: (none)\n**Current change**: (none)\n**Current phase**: roadmap-ready\n**Last saved Flow action**: roadmap-finalized\n**Next command**: $flow-run\n**Last checkpoint**: (none)\n\n## Changes\n\n(none)\n\n## Milestones\n\n{milestones}\n",
            run_dir.file_name().unwrap().to_string_lossy(),
        ),
    )
    .unwrap();
    std::fs::write(
        run_dir.join("log.md"),
        format!("# Run Log: {descriptor}\n\n**Run**: {}\n**Target**: planned roadmap\n**Started**: 2026-01-01T00:00:00Z\n**Status**: planned\n\n## Event Log\n\n- 2026-01-01T00:00:00Z — run-started — Created run workspace.\n\n## Operations\n\n", run_dir.file_name().unwrap().to_string_lossy()),
    )
    .unwrap();
    std::fs::write(run_dir.join("manual.md"), "# Owner's Manual\n\n**Status**: draft\n\n## Quickstart\n\nTo be completed before the roadmap delivery run is finalized.\n").unwrap();
    std::fs::write(run_dir.join("release-notes.md"), "# Release Notes\n\n**Status**: draft\n\n## Delivered Changes\n\nTo be completed before the roadmap delivery run is finalized.\n").unwrap();
}

fn slug(value: &str) -> String {
    value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() {
                ch.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>()
        .split('-')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

fn commit_all(repo: &Path, message: &str) {
    std::process::Command::new("git")
        .args(["add", "-A"])
        .current_dir(repo)
        .output()
        .unwrap();
    std::process::Command::new("git")
        .args(["commit", "-q", "-m", message])
        .current_dir(repo)
        .output()
        .unwrap();
}

fn current_branch(repo: &Path) -> String {
    let out = std::process::Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .current_dir(repo)
        .output()
        .unwrap();
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

fn run_dirs(repo: &Path) -> Vec<PathBuf> {
    let root = repo.join("flow").join("runs");
    let mut dirs = std::fs::read_dir(root)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.is_dir())
        .collect::<Vec<_>>();
    dirs.sort();
    dirs
}

fn completed_manual() -> &'static str {
    "# Owner's Manual\n\n**Status**: draft\n\n## Quickstart\n\nRun it.\n\n## Resulting State\n\nThe run result is ready to operate.\n"
}

fn completed_release_notes() -> &'static str {
    "# Release Notes\n\n**Status**: draft\n\n## Delivered Changes\n\nImplemented the requested run.\n\n## User Impact\n\nUsers can use the completed behavior.\n\n## Upgrade Notes\n\nNo upgrade action required.\n\n## Verification Summary\n\nVerification passed.\n\n## Source Milestones\n\nM-1.\n"
}

fn create_close_ready_feature(repo: &Path, run_dir: &Path) -> PathBuf {
    let feature_dir = run_dir.join("changes").join("M-1-final-milestone");
    std::fs::create_dir_all(&feature_dir).unwrap();
    std::fs::write(
        feature_dir.join("spec.md"),
        "# Spec: M-1-final-milestone\n\n## What & Why\n\nFinish the final milestone.\n\n## Requirements\n\n### Functional Requirements\n\n- **FR-001**: Finish the final milestone.\n\n## Success Criteria\n\n### Measurable Outcomes\n\n- **SC-001**: The final milestone is closed.\n",
    )
    .unwrap();
    std::fs::write(
        feature_dir.join("plan.md"),
        "# Plan: M-1-final-milestone\n\n## Summary\n\nFinish it.\n\n## Technical Context\n\nUse Flow.\n\n## Documentation Impact\n\nImpact: none\n\nDocs already current because this test creates a synthetic closeout.\n",
    )
    .unwrap();
    std::fs::write(
        feature_dir.join("tasks.md"),
        "# Tasks: M-1-final-milestone\n\n## Tasks\n\n- [x] **T-001**: Finish final milestone.\n  - Covers: FR-001\n  - Verifies: SC-001\n",
    )
    .unwrap();
    std::fs::write(
        feature_dir.join("status.md"),
        format!(
            "# Status: M-1-final-milestone\n\n**Change**: M-1-final-milestone\n**Started**: 2026-01-01\n**Updated**: 2026-01-01T00:00:00Z\n**State**: building\n**Branch**: {}\n**Milestone**: M-1\n\n## History\n\n- 2026-01-01T00:00:00Z — build-complete — verification passed\n",
            current_branch(repo)
        ),
    )
    .unwrap();
    feature_dir
}

fn setup_last_milestone_run(review_config: Option<&str>) -> (TempDir, PathBuf) {
    let repo = make_flow_repo();
    if let Some(config) = review_config {
        std::fs::write(repo.path().join(".flow/config.yaml"), config).unwrap();
    }
    write_roadmap(
        repo.path(),
        "# Roadmap: Finalize\n\n## Milestones\n\n### [ ] M-1: Final\n",
    );
    commit_all(repo.path(), "roadmap");
    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .args(["run", "M-1"])
        .assert()
        .success();
    let run_dir = run_dirs(repo.path()).remove(0);
    let feature_dir = create_close_ready_feature(repo.path(), &run_dir);
    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .env("FLOW_RUN_DIR", &run_dir)
        .env("FLOW_CHANGE_DIR", feature_dir.to_str().unwrap())
        .args(["close", "--finalize"])
        .assert()
        .success();
    (repo, run_dir)
}

#[test]
fn t003_t004_t005_last_close_exposes_checkpoint_before_run_finalize_command() {
    let (_repo, run_dir) = setup_last_milestone_run(None);

    let log = std::fs::read_to_string(run_dir.join("run.md")).unwrap();
    assert!(
        log.contains("**Next command**: flow run --checkpoint "),
        "{log}"
    );
    assert!(log.contains("--milestone M-1"), "{log}");
    assert!(log.contains("**Roadmap fingerprint**: sha256:"), "{log}");
    let roadmap = std::fs::read_to_string(run_dir.join("roadmap.md")).unwrap();
    assert!(roadmap.contains("### [x] M-1: Final"), "{roadmap}");
}

#[test]
fn t001_t004_review_required_still_exposes_footer_style_finalize_command() {
    let (_repo, run_dir) = setup_last_milestone_run(Some(
        "review:\n  before_finalize: true\n  per_command:\n    run: true\n",
    ));

    let log = std::fs::read_to_string(run_dir.join("run.md")).unwrap();
    assert!(
        log.contains("**Next command**: flow run --checkpoint "),
        "{log}"
    );
    assert!(log.contains("--milestone M-1"), "{log}");
}

#[test]
fn t003_t005_finalize_archives_completed_milestone_run_after_artifacts_are_ready() {
    let (repo, run_dir) = setup_last_milestone_run(None);
    std::fs::write(run_dir.join("manual.md"), completed_manual()).unwrap();
    std::fs::write(run_dir.join("release-notes.md"), completed_release_notes()).unwrap();

    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .env("FLOW_RUN_DIR", &run_dir)
        .args(["run", "--finalize"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Run finalized."))
        .stdout(predicate::str::contains("Roadmap:"));

    assert!(run_dir.join("roadmap.md").is_file());
    let run_roadmap = std::fs::read_to_string(run_dir.join("roadmap.md")).unwrap();
    assert!(run_roadmap.contains("### [x] M-1: Final"), "{run_roadmap}");
    assert!(!repo.path().join("flow/roadmap.md").exists());
}

#[test]
fn t003_finalize_refuses_while_milestones_remain_open() {
    let repo = make_flow_repo();
    write_roadmap(
        repo.path(),
        "# Roadmap: Finalize\n\n## Milestones\n\n### [ ] M-1: First\n\n### [ ] M-2: Second\n",
    );
    commit_all(repo.path(), "roadmap");
    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .args(["run", "M-1"])
        .assert()
        .success();
    let run_dir = run_dirs(repo.path()).remove(0);
    std::fs::write(run_dir.join("manual.md"), completed_manual()).unwrap();
    std::fs::write(run_dir.join("release-notes.md"), completed_release_notes()).unwrap();

    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .env("FLOW_RUN_DIR", run_dir.to_str().unwrap())
        .args(["run", "--finalize"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("still has open milestones"));
}

fn git_stdout(repo: &Path, args: &[&str]) -> String {
    let out = std::process::Command::new("git")
        .args(args)
        .current_dir(repo)
        .output()
        .unwrap();
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

#[test]
fn finalize_creates_closing_commit_and_leaves_run_dir_clean() {
    let (repo, run_dir) = setup_last_milestone_run(None);
    std::fs::write(run_dir.join("manual.md"), completed_manual()).unwrap();
    std::fs::write(run_dir.join("release-notes.md"), completed_release_notes()).unwrap();

    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .env("FLOW_RUN_DIR", &run_dir)
        .args(["run", "--finalize"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Closing commit: "))
        .stdout(predicate::str::contains("Verify this run:"));

    let run_name = run_dir.file_name().unwrap().to_string_lossy().to_string();
    assert_eq!(
        git_stdout(repo.path(), &["log", "-1", "--format=%s"]),
        format!("flow run finalize: {run_name}")
    );
    let dirty = git_stdout(repo.path(), &["status", "--porcelain"]);
    assert!(
        !dirty.contains("flow/runs/"),
        "finalize must commit all run artifacts:\n{dirty}"
    );
}

#[test]
fn finalize_keeps_run_dir_uncommitted_when_checkpoint_commits_disabled() {
    let (repo, run_dir) = setup_last_milestone_run(None);
    let state = std::fs::read_to_string(run_dir.join("run.md")).unwrap();
    std::fs::write(
        run_dir.join("run.md"),
        state.replace(
            "**Checkpoint commits**: enabled",
            "**Checkpoint commits**: disabled",
        ),
    )
    .unwrap();
    std::fs::write(run_dir.join("manual.md"), completed_manual()).unwrap();
    std::fs::write(run_dir.join("release-notes.md"), completed_release_notes()).unwrap();
    let head_before = git_stdout(repo.path(), &["rev-parse", "HEAD"]);

    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .env("FLOW_RUN_DIR", &run_dir)
        .args(["run", "--finalize"])
        .assert()
        .success()
        .stdout(predicate::str::contains("intentionally left uncommitted"));

    assert_eq!(
        git_stdout(repo.path(), &["rev-parse", "HEAD"]),
        head_before,
        "finalize must not commit when checkpoint commits are disabled"
    );
}

#[test]
fn t003_t005_finalize_blocks_template_placeholders_before_archive() {
    let (repo, run_dir) = setup_last_milestone_run(None);

    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .env("FLOW_RUN_DIR", run_dir.to_str().unwrap())
        .args(["run", "--finalize"])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "run release notes still contain template placeholders",
        ));
}
