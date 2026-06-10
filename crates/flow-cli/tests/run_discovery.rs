//! Roadmap-scoped run discovery and schema regression tests.

use assert_cmd::Command;
use flow_cli::cmd::run;
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

fn write_run_log(repo: &Path, name: &str, status: &str, roadmap_text: &str) -> PathBuf {
    let dir = repo.join("flow").join("runs").join(name);
    std::fs::create_dir_all(&dir).unwrap();
    let fingerprint = flow_core::roadmap::fingerprint(roadmap_text);
    std::fs::write(
        dir.join("run.md"),
        format!(
            "# Run: Test\n\n**Run name**: {name}\n**Run type**: roadmap\n**Run scope**: (none)\n**Status**: {status}\n**Run branch**: (none)\n**Roadmap fingerprint**: {fingerprint}\n**Checkpoint commits**: disabled\n**Current milestone**: (none)\n**Current change**: (none)\n**Current phase**: roadmap-ready\n**Last saved Flow action**: roadmap-finalized\n**Next command**: $flow-run\n**Last checkpoint**: (none)\n\n## Changes\n\n(none)\n\n## Milestones\n\n- [ ] M-1 — Test\n"
        ),
    )
    .unwrap();
    std::fs::write(dir.join("roadmap.md"), roadmap_text).unwrap();
    std::fs::write(
        dir.join("log.md"),
        format!("# Run Log: Test\n\n**Run**: {name}\n**Target**: test\n**Started**: 2026-01-01T00:00:00Z\n**Status**: running\n\n## Event Log\n\n- 2026-01-01T00:00:00Z — run-started — Created run workspace.\n"),
    )
    .unwrap();
    std::fs::write(
        dir.join("manual.md"),
        "# Owner's Manual\n\n**Status**: draft\n\n## Quickstart\n\nTo be completed before the roadmap delivery run is finalized.\n",
    )
    .unwrap();
    std::fs::write(
        dir.join("release-notes.md"),
        "# Release Notes\n\n**Status**: draft\n\n## Delivered Changes\n\nTo be completed with milestone subsections before finalization.\n",
    )
    .unwrap();
    dir
}

#[test]
fn t001_t002_t006_run_all_renders_roadmap_scoped_schema_without_mode() {
    let repo = make_flow_repo();
    let run_dir = write_run_log(
        repo.path(),
        "20260101-schema-work",
        "planned",
        "# Roadmap: Schema Work\n\n## Milestones\n\n### [ ] M-1: Preserve identity\n",
    );
    commit_all(repo.path(), "roadmap");

    let out = Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .env("FLOW_RUN_DIR", &run_dir)
        .args(["run", "all"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Roadmap fingerprint"))
        .stdout(predicate::str::contains("## Milestones"))
        .stdout(predicate::str::contains("**Mode**:").not())
        .stdout(predicate::str::contains("Auto-finalize").not())
        .get_output()
        .stdout
        .clone();
    let text = String::from_utf8(out).unwrap();
    assert!(text.contains("**Run branch**: flow/run-"), "{text}");

    let state = std::fs::read_to_string(run_dir.join("run.md")).unwrap();
    assert!(state.contains("**Run name**:"));
    assert!(state.contains("# Run: Test"));
    assert!(state.contains("**Roadmap fingerprint**: sha256:"));
    assert!(state.contains("## Milestones"));
    assert!(!state.contains("**Mode**:"));
    assert!(!state.contains("Auto-finalize"));

    let manual = std::fs::read_to_string(run_dir.join("manual.md")).unwrap();
    assert!(manual.contains("roadmap delivery"));
    let release_notes = std::fs::read_to_string(run_dir.join("release-notes.md")).unwrap();
    assert!(release_notes.contains("milestone subsections"));
}

#[test]
fn t003_roadmap_fingerprint_reads_run_local_roadmap_file() {
    let repo = make_flow_repo();
    let text = "# Roadmap\n\n## Milestones\n\n### [ ] M-1: One\n";
    let run_dir = write_run_log(repo.path(), "20260101-current", "planned", text);

    let actual = run::roadmap_fingerprint_at_path(&run_dir.join("roadmap.md")).unwrap();

    assert_eq!(actual, flow_core::roadmap::fingerprint(text));
    assert!(actual.starts_with("sha256:"));
    assert_eq!(actual.len(), "sha256:".len() + 12);
}

#[test]
fn t004_t005_find_open_roadmap_run_returns_single_planned_run() {
    let repo = make_flow_repo();
    let roadmap_text = "# Roadmap\n\n## Milestones\n\n### [ ] M-1: One\n";
    let matching = write_run_log(repo.path(), "20260101-current", "planned", roadmap_text);

    let found = run::find_open_roadmap_run(repo.path()).unwrap().unwrap();

    assert_eq!(found.run_dir, matching);
    assert_eq!(
        found.state.get("Roadmap fingerprint").map(String::as_str),
        Some(flow_core::roadmap::fingerprint(roadmap_text).as_str())
    );
}

#[test]
fn t004_t005_find_open_roadmap_run_errors_on_multiple_active_runs() {
    let repo = make_flow_repo();
    write_run_log(
        repo.path(),
        "20260101-one",
        "planned",
        "# Roadmap\n\n## Milestones\n\n### [ ] M-1: One\n",
    );
    write_run_log(
        repo.path(),
        "20260101-two",
        "running",
        "# Roadmap\n\n## Milestones\n\n### [ ] M-2: Two\n",
    );

    let err = run::find_open_roadmap_run(repo.path()).unwrap_err();

    assert!(
        err.to_string()
            .contains("Multiple planned or running roadmap runs"),
        "{err}"
    );
    assert!(err.to_string().contains("FLOW_RUN_DIR"), "{err}");
}

#[test]
fn t004_t005_incomplete_run_state_is_a_hard_error() {
    let repo = make_flow_repo();
    let dir = repo
        .path()
        .join("flow")
        .join("runs")
        .join("20260101-incomplete");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join("run.md"),
        "# Run: Incomplete\n\n**Run branch**: flow/run-current\n",
    )
    .unwrap();

    let err = run::find_open_roadmap_run(repo.path()).unwrap_err();

    assert!(err.to_string().contains("Run name"), "{err}");
}
