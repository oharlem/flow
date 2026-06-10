use assert_cmd::Command;
use predicates::prelude::*;
use std::path::Path;
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
    commit_all(path, "flow init");
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

    let descriptor = roadmap_descriptor(body);
    let run_name = today_run_dir_name(&format!("roadmap-{}", paths_slug(&descriptor)));
    let run_dir = runs.join(run_name);
    std::fs::create_dir_all(run_dir.join("changes")).unwrap();
    std::fs::write(run_dir.join("roadmap.md"), body).unwrap();
    let fingerprint = flow_core::roadmap::fingerprint(body);
    let milestones = milestone_snapshot(body);
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
    std::fs::write(
        run_dir.join("manual.md"),
        format!(
            "# Owner's Manual: {descriptor}\n\n**Status**: draft\n\n## Quickstart\n\nTo be completed before the roadmap delivery run is finalized.\n\n## Resulting State\n\nTo be completed before the roadmap delivery run is finalized.\n"
        ),
    )
    .unwrap();
    std::fs::write(
        run_dir.join("release-notes.md"),
        "# Release Notes\n\n**Status**: draft\n\n## Delivered Changes\n\nTo be completed before the roadmap delivery run is finalized.\n\n## User Impact\n\nTo be completed before the roadmap delivery run is finalized.\n",
    )
    .unwrap();
}

fn roadmap_descriptor(body: &str) -> String {
    body.lines()
        .find_map(|line| line.trim().strip_prefix("# ").map(str::trim))
        .map(|title| {
            title
                .strip_prefix("Roadmap:")
                .or_else(|| title.strip_prefix("Roadmap -"))
                .map(str::trim)
                .unwrap_or(title)
                .to_string()
        })
        .filter(|title| {
            let slug = paths_slug(title);
            !matches!(slug.as_str(), "roadmap")
        })
        .or_else(|| {
            let title = flow_core::parse::roadmap::parse_str(body)
                .into_iter()
                .filter(|m| !m.done)
                .take(3)
                .map(|m| m.title)
                .collect::<Vec<_>>()
                .join(" ");
            (!title.trim().is_empty()).then_some(title)
        })
        .unwrap_or_else(|| "Roadmap".to_string())
}

fn paths_slug(value: &str) -> String {
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

fn milestone_snapshot(body: &str) -> String {
    let milestones = flow_core::parse::roadmap::parse_str(body);
    if milestones.is_empty() {
        return "(none)".to_string();
    }
    milestones
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
        .join("\n")
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

fn branch_exists(repo: &Path, branch: &str) -> bool {
    std::process::Command::new("git")
        .args([
            "show-ref",
            "--verify",
            "--quiet",
            &format!("refs/heads/{branch}"),
        ])
        .current_dir(repo)
        .status()
        .unwrap()
        .success()
}

fn head_count(repo: &Path) -> usize {
    let out = std::process::Command::new("git")
        .args(["rev-list", "--count", "HEAD"])
        .current_dir(repo)
        .output()
        .unwrap();
    String::from_utf8_lossy(&out.stdout).trim().parse().unwrap()
}

fn checkpoint_sha_from_stdout(text: &str) -> String {
    text.lines()
        .find_map(|line| line.strip_prefix("Checkpoint committed: "))
        .map(str::trim)
        .filter(|sha| is_full_sha(sha))
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| panic!("T-001 expected full checkpoint SHA in output: {text}"))
}

fn last_checkpoint_from_state(state: &str) -> String {
    state
        .lines()
        .find_map(|line| line.strip_prefix("**Last checkpoint**: "))
        .map(str::trim)
        .filter(|sha| is_full_sha(sha))
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| panic!("T-001 expected full Last checkpoint SHA in run.md:\n{state}"))
}

fn is_full_sha(value: &str) -> bool {
    value.len() == 40 && value.chars().all(|ch| ch.is_ascii_hexdigit())
}

fn today_branch_prefix(slug: &str) -> String {
    format!("flow/run-{}-{slug}", chrono::Utc::now().format("%Y%m%d"))
}

fn today_run_dir_name(slug: &str) -> String {
    format!("{}-{slug}", chrono::Utc::now().format("%Y%m%d"))
}

fn run_dirs(repo: &Path) -> Vec<std::path::PathBuf> {
    let root = repo.join("flow").join("runs");
    let mut dirs = std::fs::read_dir(root)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.is_dir())
        .collect::<Vec<_>>();
    dirs.sort();
    dirs
}

fn child_change_dir(run_dir: &Path, name: &str) -> std::path::PathBuf {
    run_dir.join("changes").join(name)
}

fn rel(repo: &Path, path: &Path) -> String {
    path.strip_prefix(repo)
        .unwrap()
        .display()
        .to_string()
        .replace('\\', "/")
}

fn completed_manual() -> &'static str {
    "# Owner's Manual\n\n**Status**: draft\n\n## Quickstart\n\nRun it.\n\n## Resulting State\n\nThe run result is ready to operate.\n"
}

fn completed_release_notes() -> &'static str {
    "# Release Notes\n\n**Status**: draft\n\n## Delivered Changes\n\nImplemented the requested run.\n\n## User Impact\n\nUsers can use the completed behavior.\n\n## Upgrade Notes\n\nNo upgrade action required.\n\n## Verification Summary\n\nVerification passed.\n\n## Source Milestones\n\nM-1.\n"
}

#[test]
fn t014_run_milestone_creates_run_workspace_and_envelope() {
    let repo = make_flow_repo();
    write_roadmap(
        repo.path(),
        "# Roadmap\n\n## Milestones\n\n### [ ] M-1: Build automation\n\nDo it.\n",
    );
    commit_all(repo.path(), "roadmap");

    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .args(["run", "M-1"])
        .assert()
        .success()
        .stdout(predicate::str::contains("# Phase Agent: run"))
        .stdout(predicate::str::contains("**Invocation**: milestone"))
        .stdout(predicate::str::contains("**Milestone**: M-1"))
        .stdout(predicate::str::contains("**Release notes**: flow/runs/"))
        .stdout(predicate::str::contains("Release notes: flow/runs/"))
        .stdout(predicate::str::contains(
            "**Run finalize requires**: `release-notes.md`",
        ))
        .stdout(predicate::str::contains(
            "Current next command: `FLOW_RUN_DIR=\"flow/runs/20260610-roadmap-build-automation\" flow start M-1`",
        ))
        .stdout(predicate::str::contains("flow run --finalize"))
        .stdout(predicate::str::contains("Next command: `flow run`"));

    let dirs = run_dirs(repo.path());
    assert_eq!(dirs.len(), 1);
    let run_name = dirs[0].file_name().unwrap().to_string_lossy();
    assert_eq!(run_name, today_run_dir_name("roadmap-build-automation"));
    assert!(dirs[0].join("run.md").is_file());
    assert!(dirs[0].join("log.md").is_file());
    assert!(dirs[0].join("manual.md").is_file());
    assert!(dirs[0].join("release-notes.md").is_file());
    let log = std::fs::read_to_string(dirs[0].join("log.md")).unwrap();
    assert!(log.contains("## Event Log"));
    let state = std::fs::read_to_string(dirs[0].join("run.md")).unwrap();
    assert!(state.contains("# Run: Build automation"));
    assert!(state.contains("**Run type**: roadmap"));
    assert!(state.contains("**Roadmap fingerprint**: sha256:"));
    assert!(state.contains("Build automation"));
    let manual = std::fs::read_to_string(dirs[0].join("manual.md")).unwrap();
    assert!(manual.contains("## Resulting State"));
    assert!(!manual.contains("## What Was Built"));
    let release_notes = std::fs::read_to_string(dirs[0].join("release-notes.md")).unwrap();
    assert!(release_notes.contains("## Delivered Changes"));
    assert!(release_notes.contains("## User Impact"));
}

#[test]
fn t014_run_milestone_run_dir_conflicts_get_same_day_suffix() {
    let repo = make_flow_repo();
    write_roadmap(
        repo.path(),
        "# Roadmap\n\n## Milestones\n\n### [ ] M-1: Build automation\n\nDo it.\n",
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

    let dir_names = run_dirs(repo.path())
        .into_iter()
        .map(|path| path.file_name().unwrap().to_string_lossy().to_string())
        .collect::<Vec<_>>();
    assert_eq!(
        dir_names,
        vec![today_run_dir_name("roadmap-build-automation")]
    );
}

#[test]
fn t014_run_all_lists_open_milestones_only() {
    let repo = make_flow_repo();
    write_roadmap(
        repo.path(),
        "# Roadmap\n\n## Milestones\n\n### [ ] M-1: First\n\n### [x] M-2: Done\n\n### [~] M-3: Active\n",
    );
    commit_all(repo.path(), "roadmap");

    let out = Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .args(["run", "all"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let text = String::from_utf8(out).unwrap();
    assert!(
        text.contains("**Invocation**: all open milestones"),
        "{text}"
    );
    assert!(text.contains("- [ ] M-1 — First"), "{text}");
    assert!(text.contains("- [~] M-3 — Active"), "{text}");
    assert!(!text.contains("M-2 — Done"), "{text}");
    assert!(text.contains("**Run branch**: flow/run-"), "{text}");
    assert!(text.contains("**Release notes**: flow/runs/"), "{text}");
    assert!(text.contains("Next command: `flow run`"), "{text}");
    let dirs = run_dirs(repo.path());
    assert_eq!(dirs.len(), 1);
    assert!(dirs[0].join("release-notes.md").is_file());
}

#[test]
fn t014_run_all_prints_env_prefixed_first_child_start_command() {
    // T-001 / T-003: the green path must hand the first child start command a
    // FLOW_RUN_DIR directly, so no child-start-retry is needed.
    let repo = make_flow_repo();
    write_roadmap(
        repo.path(),
        "# Roadmap\n\n## Milestones\n\n### [ ] M-1: First\n\n### [ ] M-2: Second\n",
    );
    commit_all(repo.path(), "roadmap");

    let out = Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .args(["run", "all"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let text = String::from_utf8(out).unwrap();
    let run_dir = run_dirs(repo.path()).remove(0);
    let run_rel = run_dir
        .strip_prefix(repo.path())
        .unwrap()
        .display()
        .to_string()
        .replace('\\', "/");
    let expected = format!("First child command: `FLOW_RUN_DIR=\"{run_rel}\" flow start M-1`");
    assert!(text.contains(&expected), "{text}");

    let log = std::fs::read_to_string(run_dir.join("log.md")).unwrap();
    assert!(!log.contains("child-start-retry"), "{log}");
}

#[test]
fn t014_run_all_uses_descriptor_for_branch_dir_and_titles() {
    let repo = make_flow_repo();
    write_roadmap(
        repo.path(),
        "# Roadmap: Warranty Metafields\n\n## Milestones\n\n### [ ] M-1: Warranty Metafield Exists\n",
    );
    commit_all(repo.path(), "roadmap");

    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .args(["run", "all"])
        .assert()
        .success();

    let branch = current_branch(repo.path());
    assert_eq!(branch, today_branch_prefix("roadmap-warranty-metafields"));
    let run_dir = run_dirs(repo.path()).remove(0);
    let run_name = run_dir.file_name().unwrap().to_string_lossy();
    assert_eq!(run_name, today_run_dir_name("roadmap-warranty-metafields"));
    let state = std::fs::read_to_string(run_dir.join("run.md")).unwrap();
    assert!(state.contains("# Run: Warranty Metafields"));
    assert!(state.contains("**Roadmap fingerprint**: sha256:"));
    assert!(state.contains("## Milestones"));
    assert!(state.contains(&format!("**Run branch**: {branch}")));
    let log = std::fs::read_to_string(run_dir.join("log.md")).unwrap();
    assert!(log.contains("# Run Log: Warranty Metafields"));
    let manual = std::fs::read_to_string(run_dir.join("manual.md")).unwrap();
    assert!(manual.contains("# Owner's Manual: Warranty Metafields"));
}

#[test]
fn t014_run_all_generic_roadmap_title_falls_back_to_milestones() {
    let repo = make_flow_repo();
    write_roadmap(
        repo.path(),
        "# Roadmap\n\n## Milestones\n\n### [ ] M-1: Import Customers\n\n### [ ] M-2: Reconcile Orders\n",
    );
    commit_all(repo.path(), "roadmap");

    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .args(["run", "all"])
        .assert()
        .success();

    assert_eq!(
        current_branch(repo.path()),
        today_branch_prefix("roadmap-import-customers-reconcile-orders")
    );
    let run_dir = run_dirs(repo.path()).remove(0);
    let run_name = run_dir.file_name().unwrap().to_string_lossy();
    assert_eq!(
        run_name,
        today_run_dir_name("roadmap-import-customers-reconcile-orders")
    );
}

#[test]
fn t014_run_all_branch_conflicts_get_suffix() {
    let repo = make_flow_repo();
    write_roadmap(
        repo.path(),
        "# Customer Import Roadmap\n\n## Milestones\n\n### [ ] M-1: Parse CSV\n",
    );
    commit_all(repo.path(), "roadmap");
    let base = today_branch_prefix("roadmap-customer-import-roadmap");
    std::process::Command::new("git")
        .args(["branch", &base])
        .current_dir(repo.path())
        .output()
        .unwrap();

    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .args(["run", "all"])
        .assert()
        .success();

    assert_eq!(current_branch(repo.path()), format!("{base}-2"));
}

#[test]
fn t014_run_all_run_dir_conflicts_get_same_day_suffix() {
    let repo = make_flow_repo();
    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .args(["set", "run_checkpoint_commits=false"])
        .assert()
        .success();
    write_roadmap(
        repo.path(),
        "# Roadmap: Warranty Metafields\n\n## Milestones\n\n### [ ] M-1: Warranty Metafield Exists\n",
    );

    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .args(["run", "all"])
        .assert()
        .success();
    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .args(["run", "all"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Attached to run"));

    assert_eq!(
        current_branch(repo.path()),
        today_branch_prefix("roadmap-warranty-metafields")
    );
    let dir_names = run_dirs(repo.path())
        .into_iter()
        .map(|path| path.file_name().unwrap().to_string_lossy().to_string())
        .collect::<Vec<_>>();
    assert_eq!(
        dir_names,
        vec![today_run_dir_name("roadmap-warranty-metafields")]
    );
}

#[test]
fn t014_run_all_checkpoint_default_requires_clean_worktree() {
    let repo = make_flow_repo();
    write_roadmap(
        repo.path(),
        "# Customer Import Roadmap\n\n## Milestones\n\n### [ ] M-1: Parse CSV\n",
    );
    std::fs::write(repo.path().join("unrelated.txt"), "local work").unwrap();

    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .args(["run", "all"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("requires a clean worktree"));
}

#[test]
fn t014_run_all_checkpoint_default_allows_planned_run_bootstrap_changes() {
    let repo = make_flow_repo();
    write_roadmap(
        repo.path(),
        "# Customer Import Roadmap\n\n## Milestones\n\n### [ ] M-1: Parse CSV\n",
    );
    let before = head_count(repo.path());

    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .args(["run", "all"])
        .assert()
        .success();

    assert_eq!(head_count(repo.path()), before);
    assert_eq!(
        current_branch(repo.path()),
        today_branch_prefix("roadmap-customer-import-roadmap")
    );
}

#[test]
fn t014_run_all_checkpoint_disabled_allows_dirty_worktree_without_commit() {
    let repo = make_flow_repo();
    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .args(["set", "run_checkpoint_commits=false"])
        .assert()
        .success();
    write_roadmap(
        repo.path(),
        "# Customer Import Roadmap\n\n## Milestones\n\n### [ ] M-1: Parse CSV\n",
    );
    let before = head_count(repo.path());

    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .args(["run", "all"])
        .assert()
        .success();

    assert_eq!(head_count(repo.path()), before);
}

#[test]
fn t014_run_all_start_reuses_run_branch_with_context() {
    let repo = make_flow_repo();
    write_roadmap(
        repo.path(),
        "# Customer Import Roadmap\n\n## Milestones\n\n### [ ] M-1: Parse CSV\n",
    );
    commit_all(repo.path(), "roadmap");
    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .args(["run", "all"])
        .assert()
        .success();
    let run_dir = run_dirs(repo.path()).remove(0);
    let run_branch = current_branch(repo.path());

    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .env("FLOW_RUN_DIR", &run_dir)
        .args(["start", "M-1"])
        .assert()
        .success();

    assert_eq!(current_branch(repo.path()), run_branch);
    assert!(!branch_exists(repo.path(), "flow/M-1-parse-csv"));
    let feature_dir = child_change_dir(&run_dir, "M-1-parse-csv");
    let status = std::fs::read_to_string(feature_dir.join("status.md")).unwrap();
    assert!(status.contains(&format!("**Branch**: {run_branch}")));
    let state = std::fs::read_to_string(run_dir.join("run.md")).unwrap();
    assert!(state.contains(&format!(
        "**Current change**: {}",
        rel(repo.path(), &feature_dir)
    )));
    assert!(state.contains("**Next command**: $flow-plan"));
}

#[test]
fn t014_run_all_child_finalizers_update_run_state_for_resume() {
    let repo = make_flow_repo();
    write_roadmap(
        repo.path(),
        "# Customer Import Roadmap\n\n## Milestones\n\n### [ ] M-1: Parse CSV\n",
    );
    commit_all(repo.path(), "roadmap");
    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .args(["run", "all"])
        .assert()
        .success();
    let run_dir = run_dirs(repo.path()).remove(0);
    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .env("FLOW_RUN_DIR", &run_dir)
        .args(["start", "M-1"])
        .assert()
        .success();
    let feature_dir = child_change_dir(&run_dir, "M-1-parse-csv");
    std::fs::write(
        feature_dir.join("spec.md"),
        "# Spec: M-1-parse-csv\n\n## What & Why\n\nParse CSV imports for customer data.\n\n## Requirements\n\n### Functional Requirements\n\n- **FR-001**: Parse customer CSV rows.\n\n## Success Criteria\n\n### Measurable Outcomes\n\n- **SC-001**: CSV rows are parsed into customer records.\n",
    )
    .unwrap();

    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .env("FLOW_RUN_DIR", &run_dir)
        .env("FLOW_CHANGE_DIR", feature_dir.to_str().unwrap())
        .args(["start", "--finalize"])
        .assert()
        .success();
    let state = std::fs::read_to_string(run_dir.join("run.md")).unwrap();
    assert!(state.contains("**Current phase**: spec-complete"));
    assert!(state.contains("**Next command**: $flow-plan"));

    std::fs::write(
        feature_dir.join("plan.md"),
        "# Plan: M-1-parse-csv\n\n## Summary\n\nImplement CSV parsing.\n\n## Technical Context\n\nUse the existing code layout.\n\n## Documentation Impact\n\nImpact: none\n\nDocs already current: parser behavior is covered by the feature artifacts.\n",
    )
    .unwrap();
    std::fs::write(
        feature_dir.join("tasks.md"),
        "# Tasks: M-1-parse-csv\n\n## Tasks\n\n- [ ] **T-001**: Add CSV parser.\n  - Covers: FR-001\n  - Verifies: SC-001\n",
    )
    .unwrap();

    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .env("FLOW_RUN_DIR", &run_dir)
        .env("FLOW_CHANGE_DIR", feature_dir.to_str().unwrap())
        .args(["plan", "--finalize"])
        .assert()
        .success();
    let state = std::fs::read_to_string(run_dir.join("run.md")).unwrap();
    assert!(state.contains("**Current phase**: plan-complete"));
    assert!(state.contains("**Last saved Flow action**: plan-complete"));
    assert!(state.contains("**Next command**: $flow-build"));
}

#[test]
fn t014_bare_run_continues_running_all_scope_from_next_command() {
    let repo = make_flow_repo();
    write_roadmap(
        repo.path(),
        "# Customer Import Roadmap\n\n## Milestones\n\n### [ ] M-1: Parse CSV\n",
    );
    commit_all(repo.path(), "roadmap");
    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .args(["run", "all"])
        .assert()
        .success();
    let run_dir = run_dirs(repo.path()).remove(0);
    let run_rel = rel(repo.path(), &run_dir);
    let run_path = run_dir.join("run.md");
    let state = std::fs::read_to_string(&run_path).unwrap();
    let state = state
        .replace(
            "**Current phase**: run-attached",
            "**Current phase**: plan-complete",
        )
        .replace(
            "**Last saved Flow action**: run-attached",
            "**Last saved Flow action**: plan-complete",
        )
        .replace(
            "**Next command**: $flow-start M-1",
            "**Next command**: $flow-build",
        );
    std::fs::write(&run_path, state).unwrap();

    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .arg("run")
        .assert()
        .success()
        .stdout(predicate::str::contains("# Flow Run Resume").not())
        .stdout(predicate::str::contains("# Phase Agent: run"))
        .stdout(predicate::str::contains(format!(
            "Current next command: `FLOW_RUN_DIR=\"{run_rel}\" flow build`"
        )));
}

#[test]
fn t014_run_context_build_preflight_block_resumes_with_flow_run() {
    let repo = make_flow_repo();
    write_roadmap(
        repo.path(),
        "# Customer Import Roadmap\n\n## Milestones\n\n### [ ] M-1: Parse CSV\n",
    );
    commit_all(repo.path(), "roadmap");
    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .args(["run", "all"])
        .assert()
        .success();
    let run_dir = run_dirs(repo.path()).remove(0);

    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .env("FLOW_RUN_DIR", &run_dir)
        .args(["start", "M-1"])
        .assert()
        .success();
    let feature_dir = child_change_dir(&run_dir, "M-1-parse-csv");
    std::fs::write(
        feature_dir.join("spec.md"),
        "# Spec: M-1-parse-csv\n\n## What & Why\n\nParse CSV imports for customer data.\n\n## Requirements\n\n### Functional Requirements\n\n- **FR-001**: Parse customer CSV rows.\n\n## Success Criteria\n\n### Measurable Outcomes\n\n- **SC-001**: CSV rows are parsed into customer records.\n",
    )
    .unwrap();
    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .env("FLOW_RUN_DIR", &run_dir)
        .env("FLOW_CHANGE_DIR", feature_dir.to_str().unwrap())
        .args(["start", "--finalize"])
        .assert()
        .success();
    std::fs::write(
        feature_dir.join("plan.md"),
        "# Plan: M-1-parse-csv\n\n## Summary\n\nImplement CSV parsing.\n\n## Technical Context\n\nUse the existing code layout.\n\n## Documentation Impact\n\nImpact: none\n\nDocs already current: parser behavior is covered by the feature artifacts.\n",
    )
    .unwrap();
    std::fs::write(
        feature_dir.join("tasks.md"),
        "# Tasks: M-1-parse-csv\n\n## Tasks\n\n- [ ] **T-001**: Add CSV parser.\n  - Covers: FR-001\n  - Verifies: SC-001\n  - Depends-On: (none)\n  - Requires: localdb\n",
    )
    .unwrap();
    std::fs::write(
        repo.path().join(".flow").join("config.yaml"),
        "preflight:\n  requirements:\n    localdb:\n      description: Local database is ready\n      command: \"exit 1\"\n      remediation: Start the local database.\n",
    )
    .unwrap();
    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .env("FLOW_RUN_DIR", &run_dir)
        .env("FLOW_CHANGE_DIR", feature_dir.to_str().unwrap())
        .args(["plan", "--finalize"])
        .assert()
        .success();

    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .env("FLOW_HOST", "codex")
        .env("FLOW_RUN_DIR", &run_dir)
        .env("FLOW_CHANGE_DIR", feature_dir.to_str().unwrap())
        .args(["build"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Blocked by environment"))
        .stdout(predicate::str::contains("Next command: `$flow-build`").not())
        .stdout(predicate::str::contains(
            "Next command: `$flow-run` - continue this roadmap automation run after required resources are available.",
        ));
}

#[test]
fn t014_run_checkpoint_records_full_sha_for_resume() {
    let repo = make_flow_repo();
    write_roadmap(
        repo.path(),
        "# Customer Import Roadmap\n\n## Milestones\n\n### [ ] M-1: Parse CSV\n",
    );
    commit_all(repo.path(), "roadmap");
    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .args(["run", "all"])
        .assert()
        .success();
    let run_dir = run_dirs(repo.path()).remove(0);
    std::fs::write(repo.path().join("checkpoint.txt"), "saved").unwrap();
    let before = head_count(repo.path());

    let out = Command::cargo_bin("flow")
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
        .get_output()
        .stdout
        .clone();

    assert_eq!(head_count(repo.path()), before + 1);
    let text = String::from_utf8(out).unwrap();
    let sha = checkpoint_sha_from_stdout(&text);
    let state = std::fs::read_to_string(run_dir.join("run.md")).unwrap();
    assert!(state.contains("**Current phase**: checkpoint-complete"));
    assert!(state.contains("**Last saved Flow action**: close-finalized"));
    assert!(state.contains("**Next command**: $flow-run"));
    assert_eq!(last_checkpoint_from_state(&state), sha);
    assert!(!state.contains("**Last checkpoint**: HEAD"));
    assert!(!state.contains("**Last checkpoint**: pending"));
    let log = std::fs::read_to_string(run_dir.join("log.md")).unwrap();
    assert!(log.contains("Preparing local checkpoint commit"));

    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .args(["run", "--resume", run_dir.to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains(format!(
            "Last checkpoint: `{sha}`"
        )));
}

#[test]
fn t014_run_checkpoint_records_finalize_next_command_when_roadmap_is_complete() {
    let repo = make_flow_repo();
    let completed_roadmap =
        "# Customer Import Roadmap\n\n## Milestones\n\n### [x] M-1: Parse CSV\n";
    write_roadmap(
        repo.path(),
        "# Customer Import Roadmap\n\n## Milestones\n\n### [ ] M-1: Parse CSV\n",
    );
    commit_all(repo.path(), "roadmap");
    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .args(["run", "all"])
        .assert()
        .success();
    let run_dir = run_dirs(repo.path()).remove(0);
    write_roadmap(repo.path(), completed_roadmap);
    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .args(["run", "--rescan", run_dir.to_str().unwrap()])
        .assert()
        .success();
    std::fs::write(repo.path().join("checkpoint.txt"), "saved").unwrap();

    let out = Command::cargo_bin("flow")
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
        .get_output()
        .stdout
        .clone();

    let text = String::from_utf8(out).unwrap();
    let sha = checkpoint_sha_from_stdout(&text);
    let state = std::fs::read_to_string(run_dir.join("run.md")).unwrap();
    assert!(state.contains("**Status**: running"));
    assert!(state.contains("**Current phase**: checkpoint-complete"));
    assert!(state.contains("**Last saved Flow action**: close-finalized"));
    assert!(state.contains("**Next command**: flow run --finalize"));
    assert!(!state.contains("**Next command**: flow run --finalize \""));
    assert_eq!(last_checkpoint_from_state(&state), sha);
    assert!(!state.contains("**Last checkpoint**: HEAD"));
    let archived_roadmap = std::fs::read_to_string(run_dir.join("roadmap.md")).unwrap();
    assert_eq!(archived_roadmap, completed_roadmap);
    assert!(
        !repo.path().join("flow/roadmap.md").exists(),
        "run-first flow must not create or reset a root roadmap"
    );
}

#[test]
fn t014_run_resume_reports_run_state_next_command() {
    let repo = make_flow_repo();
    write_roadmap(
        repo.path(),
        "# Customer Import Roadmap\n\n## Milestones\n\n### [ ] M-1: Parse CSV\n",
    );
    commit_all(repo.path(), "roadmap");
    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .args(["run", "all"])
        .assert()
        .success();
    let run_dir = run_dirs(repo.path()).remove(0);

    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .args(["run", "--resume", run_dir.to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("# Flow Run Resume"))
        .stdout(predicate::str::contains("Current milestone: `M-1`"))
        .stdout(predicate::str::contains("$flow-start M-1"));
}

#[test]
fn t014_run_resume_reports_finalize_after_terminal_checkpoint() {
    let repo = make_flow_repo();
    write_roadmap(
        repo.path(),
        "# Customer Import Roadmap\n\n## Milestones\n\n### [ ] M-1: Parse CSV\n",
    );
    commit_all(repo.path(), "roadmap");
    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .args(["run", "all"])
        .assert()
        .success();
    let run_dir = run_dirs(repo.path()).remove(0);
    write_roadmap(
        repo.path(),
        "# Customer Import Roadmap\n\n## Milestones\n\n### [x] M-1: Parse CSV\n",
    );
    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .args(["run", "--rescan", run_dir.to_str().unwrap()])
        .assert()
        .success();
    std::fs::write(repo.path().join("checkpoint.txt"), "saved").unwrap();
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
        .success();

    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .args(["run", "--resume", run_dir.to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("flow run --finalize"))
        .stdout(predicate::str::contains("FLOW_RUN_DIR"))
        .stdout(predicate::str::contains("No next command").not());
}

#[test]
fn t014_status_resolves_current_change_from_run_state_on_run_branch() {
    let repo = make_flow_repo();
    write_roadmap(
        repo.path(),
        "# Customer Import Roadmap\n\n## Milestones\n\n### [ ] M-1: Parse CSV\n",
    );
    commit_all(repo.path(), "roadmap");
    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .args(["run", "all"])
        .assert()
        .success();
    let run_dir = run_dirs(repo.path()).remove(0);

    let current_feature = "2026-01-02-current";
    let current_dir = child_change_dir(&run_dir, current_feature);
    std::fs::create_dir_all(&current_dir).unwrap();
    std::fs::write(
        current_dir.join("status.md"),
        "# Status: current\n\n**Change**: current\n**Started**: 2026-01-01\n**Updated**: 2026-01-02T00:00:00Z\n**State**: closed\n**Branch**: flow/other-branch\n\n## History\n\n- 2026-01-02T00:00:00Z — closed — change closed\n",
    )
    .unwrap();
    let run_path = run_dir.join("run.md");
    let run_state = std::fs::read_to_string(&run_path).unwrap();
    std::fs::write(
        &run_path,
        run_state.replace(
            "**Current change**: (none)",
            &format!("**Current change**: {}", rel(repo.path(), &current_dir)),
        ),
    )
    .unwrap();

    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .args(["status"])
        .assert()
        .success()
        .stdout(predicate::str::contains("**Change**: current"))
        .stdout(predicate::str::contains("**State**: closed"));
}

#[test]
fn t014_run_all_errors_when_no_open_milestones() {
    let repo = make_flow_repo();
    write_roadmap(
        repo.path(),
        "# Roadmap\n\n## Milestones\n\n### [x] M-1: Done\n",
    );

    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .args(["run", "all"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("No open milestones found"));
}

#[test]
fn t014_run_finalize_requires_completed_manual() {
    let repo = make_flow_repo();
    write_roadmap(
        repo.path(),
        "# Roadmap\n\n## Milestones\n\n### [ ] M-1: Build automation\n",
    );
    commit_all(repo.path(), "roadmap");
    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .args(["run", "M-1"])
        .assert()
        .success();
    let run_dir = run_dirs(repo.path()).remove(0);

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

    std::fs::write(run_dir.join("release-notes.md"), completed_release_notes()).unwrap();
    write_roadmap(
        repo.path(),
        "# Roadmap\n\n## Milestones\n\n### [x] M-1: Build automation\n",
    );
    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .args(["run", "--rescan", run_dir.to_str().unwrap()])
        .assert()
        .success();
    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .env("FLOW_RUN_DIR", run_dir.to_str().unwrap())
        .args(["run", "--finalize"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Run finalized."))
        .stdout(predicate::str::contains("Release notes:"));

    let state = std::fs::read_to_string(run_dir.join("run.md")).unwrap();
    let release_notes = std::fs::read_to_string(run_dir.join("release-notes.md")).unwrap();
    assert!(state.contains("**Status**: complete"));
    assert!(release_notes.contains("**Status**: complete"));
}

#[test]
fn t014_multi_milestone_run_requires_full_handoff() {
    let repo = make_flow_repo();
    write_roadmap(
        repo.path(),
        "# Roadmap\n\n## Milestones\n\n### [ ] M-1: First\n\nFirst milestone.\n\n### [ ] M-2: Second\n\nSecond milestone.\n",
    );
    commit_all(repo.path(), "roadmap");
    // `flow run all` records `Run scope: all`, so the full log+manual handoff
    // is required at finalize.
    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .args(["run", "all"])
        .assert()
        .success();
    let run_dir = run_dirs(repo.path()).remove(0);

    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .env("FLOW_RUN_DIR", run_dir.to_str().unwrap())
        .args(["run", "--finalize"])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "run manual still contains template placeholders",
        ));

    std::fs::write(run_dir.join("manual.md"), completed_manual()).unwrap();
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

    std::fs::write(run_dir.join("release-notes.md"), completed_release_notes()).unwrap();
    std::fs::write(
        run_dir.join("log.md"),
        format!(
            "# Run Log\n\n**Run**: {}\n**Target**: all open milestones\n**Started**: 2026-01-01T00:00:00Z\n**Status**: running\n\n## Event Log\n\n- 2026-01-01T00:00:00Z — run-started — Created run workspace.\n\n## Operations\n\n- 2026-01-01T00:00:00Z — milestone-started — Began M-1.\n",
            run_dir.file_name().unwrap().to_string_lossy()
        ),
    )
    .unwrap();
    std::fs::write(
        run_dir.join("roadmap.md"),
        "# Roadmap\n\n## Milestones\n\n### [x] M-1: First\n\nFirst milestone.\n\n### [x] M-2: Second\n\nSecond milestone.\n",
    )
    .unwrap();
    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .args(["run", "--rescan", run_dir.to_str().unwrap()])
        .assert()
        .success();

    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .env("FLOW_RUN_DIR", run_dir.to_str().unwrap())
        .args(["run", "--finalize"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Run finalized."));
}

#[test]
fn t014_single_milestone_run_skips_log_and_manual_validation() {
    // Confirms the tier semantics: `flow run M-N` records `Run scope: single`
    // and skips the log/manual placeholder validation at finalize, even when
    // the roadmap snapshot has more than one milestone.
    let repo = make_flow_repo();
    write_roadmap(
        repo.path(),
        "# Roadmap\n\n## Milestones\n\n### [ ] M-1: First\n\nFirst milestone.\n\n### [ ] M-2: Second\n\nSecond milestone.\n",
    );
    commit_all(repo.path(), "roadmap");
    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .args(["run", "M-1"])
        .assert()
        .success();
    let run_dir = run_dirs(repo.path()).remove(0);

    // Tick all milestones and refresh fingerprint so finalize is not blocked
    // by the roadmap-completion gate (orthogonal to the handoff tier we test
    // here).
    let completed = "# Roadmap\n\n## Milestones\n\n### [x] M-1: First\n\nFirst milestone.\n\n### [x] M-2: Second\n\nSecond milestone.\n";
    write_roadmap(repo.path(), completed);
    std::fs::write(run_dir.join("roadmap.md"), completed).unwrap();
    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .args(["run", "--rescan", run_dir.to_str().unwrap()])
        .assert()
        .success();

    // Provide only release-notes; manual.md and log.md still hold template
    // placeholders. Single-milestone scope must accept this.
    std::fs::write(run_dir.join("release-notes.md"), completed_release_notes()).unwrap();
    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .env("FLOW_RUN_DIR", run_dir.to_str().unwrap())
        .args(["run", "--finalize"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Run finalized."));
}

#[test]
fn t014_run_finalize_requires_release_notes_file() {
    let repo = make_flow_repo();
    write_roadmap(
        repo.path(),
        "# Roadmap\n\n## Milestones\n\n### [ ] M-1: Build automation\n",
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
    std::fs::remove_file(run_dir.join("release-notes.md")).unwrap();

    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .env("FLOW_RUN_DIR", run_dir.to_str().unwrap())
        .args(["run", "--finalize"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("run release notes"));
}

#[test]
fn t014_run_finalize_archives_completed_full_roadmap_when_checkpoints_are_disabled() {
    let repo = make_flow_repo();
    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .args(["set", "run_checkpoint_commits=false"])
        .assert()
        .success();
    let completed_roadmap =
        "# Customer Import Roadmap\n\n## Milestones\n\n### [x] M-1: Parse CSV\n";
    write_roadmap(
        repo.path(),
        "# Customer Import Roadmap\n\n## Milestones\n\n### [ ] M-1: Parse CSV\n",
    );
    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .args(["run", "all"])
        .assert()
        .success();
    let run_dir = run_dirs(repo.path()).remove(0);
    write_roadmap(repo.path(), completed_roadmap);
    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .args(["run", "--rescan", run_dir.to_str().unwrap()])
        .assert()
        .success();
    std::fs::write(run_dir.join("manual.md"), completed_manual()).unwrap();
    std::fs::write(run_dir.join("release-notes.md"), completed_release_notes()).unwrap();

    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .env("FLOW_RUN_DIR", run_dir.to_str().unwrap())
        .args(["run", "--finalize"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Roadmap:"));

    let archived_roadmap = std::fs::read_to_string(run_dir.join("roadmap.md")).unwrap();
    assert_eq!(archived_roadmap, completed_roadmap);
    let release_notes = std::fs::read_to_string(run_dir.join("release-notes.md")).unwrap();
    assert!(release_notes.contains("**Status**: complete"));
    assert!(
        !repo.path().join("flow/roadmap.md").exists(),
        "run-first flow must leave root roadmap absent"
    );
    let state = std::fs::read_to_string(run_dir.join("run.md")).unwrap();
    assert!(state.contains("**Current phase**: completed"));
    assert!(state.contains("**Last saved Flow action**: run-complete"));
    assert!(state.contains("**Next command**: none"));
}
