//! Run branch and checkpoint behavior for shared lifecycle runs.

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
        .find_map(|line| line.trim().strip_prefix("# Roadmap:").map(str::trim))
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
        "# Run Log\n\n## Event Log\n\n## Operations\n\n",
    )
    .unwrap();
    std::fs::write(
        run_dir.join("manual.md"),
        "# Owner's Manual\n\n**Status**: draft\n\n",
    )
    .unwrap();
    std::fs::write(
        run_dir.join("release-notes.md"),
        "# Release Notes\n\n**Status**: draft\n\n",
    )
    .unwrap();
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

fn head_count(repo: &Path) -> usize {
    let out = std::process::Command::new("git")
        .args(["rev-list", "--count", "HEAD"])
        .current_dir(repo)
        .output()
        .unwrap();
    String::from_utf8_lossy(&out.stdout).trim().parse().unwrap()
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

fn today_branch_prefix(slug: &str) -> String {
    format!("flow/run-{}-{slug}", chrono::Utc::now().format("%Y%m%d"))
}

#[test]
fn t001_t004_default_run_milestone_creates_run_branch() {
    let repo = make_flow_repo();
    write_roadmap(
        repo.path(),
        "# Roadmap: Branch Model\n\n## Milestones\n\n### [ ] M-1: First\n",
    );
    commit_all(repo.path(), "roadmap");

    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .args(["run", "M-1"])
        .assert()
        .success();

    let branch = current_branch(repo.path());
    assert_eq!(branch, today_branch_prefix("roadmap-branch-model"));
    let state = std::fs::read_to_string(run_dirs(repo.path()).remove(0).join("run.md")).unwrap();
    assert!(state.contains(&format!("**Run branch**: {branch}")));
    assert!(state.contains("**Checkpoint commits**: enabled"));
}

#[test]
fn t002_t004_run_branch_false_creates_branchless_run_and_disables_checkpoint() {
    let repo = make_flow_repo();
    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .args(["set", "run_branch=false"])
        .assert()
        .success();
    write_roadmap(
        repo.path(),
        "# Roadmap: Branch Model\n\n## Milestones\n\n### [ ] M-1: First\n",
    );
    commit_all(repo.path(), "roadmap");
    let initial_branch = current_branch(repo.path());

    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .args(["run", "M-1"])
        .assert()
        .success();

    assert_eq!(current_branch(repo.path()), initial_branch);
    let state = std::fs::read_to_string(run_dirs(repo.path()).remove(0).join("run.md")).unwrap();
    assert!(state.contains("**Run branch**: (none)"));
    assert!(state.contains("**Checkpoint commits**: disabled"));
}

#[test]
fn t002_t004_attach_keeps_existing_run_branch() {
    let repo = make_flow_repo();
    write_roadmap(
        repo.path(),
        "# Roadmap: Branch Model\n\n## Milestones\n\n### [ ] M-1: First\n",
    );
    commit_all(repo.path(), "roadmap");
    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .args(["run", "M-1"])
        .assert()
        .success();
    let branch = current_branch(repo.path());

    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .args(["run", "M-1"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Attached to run"));

    assert_eq!(current_branch(repo.path()), branch);
}

#[test]
fn t001_t005_checkpoint_config_uses_new_setting_only() {
    let repo = make_flow_repo();
    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .args(["set", "run_checkpoint_commits=false"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Set run_checkpoint_commits=false"));
    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .args(["settings"])
        .assert()
        .success()
        .stdout(predicate::str::contains("run_checkpoint_commits=false"));

    let config = std::fs::read_to_string(repo.path().join(".flow/config.yaml")).unwrap();
    assert!(config.contains("run_checkpoint_commits: false"));

    std::fs::write(
        repo.path().join(".flow/config.yaml"),
        "git:\n  run_all_checkpoint_commits: false\n",
    )
    .unwrap();
    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .args(["settings"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("run_all_checkpoint_commits"));
}

#[test]
fn t005_incremental_checkpoint_creates_local_commit_when_enabled() {
    let repo = make_flow_repo();
    write_roadmap(
        repo.path(),
        "# Roadmap: Branch Model\n\n## Milestones\n\n### [ ] M-1: First\n",
    );
    commit_all(repo.path(), "roadmap");
    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .args(["run", "M-1"])
        .assert()
        .success();
    let run_dir = run_dirs(repo.path()).remove(0);
    write_roadmap(
        repo.path(),
        "# Roadmap: Branch Model\n\n## Milestones\n\n### [x] M-1: First\n",
    );
    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .args(["run", "--rescan", run_dir.to_str().unwrap()])
        .assert()
        .success();
    std::fs::write(repo.path().join("checkpoint.txt"), "saved").unwrap();
    let before = head_count(repo.path());

    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .args([
            "run",
            "--checkpoint",
            run_dir.to_str().unwrap(),
            "--milestone",
            "M-1",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Checkpoint committed:"));

    assert_eq!(head_count(repo.path()), before + 1);
}
