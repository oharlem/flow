//! Shared lifecycle tests for roadmap-scoped runs.

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
    let run_name = today_run_dir_name(&format!("roadmap-{}", slug(descriptor)));
    let run_dir = runs.join(run_name);
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

fn today_run_dir_name(slug: &str) -> String {
    format!("{}-{slug}", chrono::Utc::now().format("%Y%m%d"))
}

#[test]
fn t003_second_run_milestone_reuses_matching_open_run() {
    let repo = make_flow_repo();
    write_roadmap(
        repo.path(),
        "# Roadmap: Shared Lifecycle\n\n## Milestones\n\n### [ ] M-1: First\n",
    );
    commit_all(repo.path(), "roadmap");

    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .args(["run", "M-1"])
        .assert()
        .success();
    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .args(["run", "M-1"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Attached to run"));

    let dirs = run_dirs(repo.path());
    assert_eq!(dirs.len(), 1);
    assert_eq!(
        dirs[0].file_name().unwrap().to_string_lossy(),
        today_run_dir_name("roadmap-shared-lifecycle")
    );
}

#[test]
fn t002_t003_run_all_attaches_to_run_created_by_milestone() {
    let repo = make_flow_repo();
    write_roadmap(
        repo.path(),
        "# Roadmap: Shared Lifecycle\n\n## Milestones\n\n### [ ] M-1: First\n\n### [ ] M-2: Second\n",
    );
    commit_all(repo.path(), "roadmap");

    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .args(["run", "M-1"])
        .assert()
        .success();
    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .args(["run", "all"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Attached to run"));

    assert_eq!(run_dirs(repo.path()).len(), 1);
}

#[test]
fn t006_run_refuses_different_mid_phase_milestone() {
    let repo = make_flow_repo();
    write_roadmap(
        repo.path(),
        "# Roadmap: Shared Lifecycle\n\n## Milestones\n\n### [ ] M-1: First\n\n### [ ] M-2: Second\n",
    );
    commit_all(repo.path(), "roadmap");

    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .args(["run", "M-1"])
        .assert()
        .success();

    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .args(["run", "M-2"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("finish the in-progress milestone"));
}

#[test]
fn t006_fingerprint_mismatch_points_to_rescan_or_finalize() {
    let repo = make_flow_repo();
    write_roadmap(
        repo.path(),
        "# Roadmap: Shared Lifecycle\n\n## Milestones\n\n### [ ] M-1: First\n",
    );
    commit_all(repo.path(), "roadmap");
    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .args(["run", "M-1"])
        .assert()
        .success();

    write_roadmap(
        repo.path(),
        "# Roadmap: Shared Lifecycle\n\n## Milestones\n\n### [ ] M-1: First edited\n",
    );

    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .args(["run", "M-1"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("flow run --rescan"))
        .stderr(predicate::str::contains("flow run --finalize"));
}

#[test]
fn t002_t009_bare_run_continues_open_run_or_errors() {
    let repo = make_flow_repo();

    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .arg("run")
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "No planned or running roadmap run",
        ));

    write_roadmap(
        repo.path(),
        "# Roadmap: Shared Lifecycle\n\n## Milestones\n\n### [ ] M-1: First\n",
    );
    commit_all(repo.path(), "roadmap");
    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .args(["run", "M-1"])
        .assert()
        .success();

    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .arg("run")
        .assert()
        .success()
        .stdout(predicate::str::contains("## Flow Run Summary"))
        .stdout(predicate::str::contains("Continuing run"))
        .stdout(predicate::str::contains("Current next command:"))
        .stdout(predicate::str::contains("# Flow Run Resume").not());
}
