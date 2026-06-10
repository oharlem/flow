//! Integration tests for the `flow doctor` SUMMARY.md drift check.
//!
//! Tests covered: T-008. The example binary is covered by
//! `t002_generate_summary_example_registered_and_runs`.

use assert_cmd::Command;
use flow_cli::{ownership, summary};
use predicates::prelude::*;
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

const DRIFT_WARNING: &str = "warning: generated docs are stale: docs/SUMMARY.md";

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
    fs::create_dir_all(path.join(".flow").join("agents")).unwrap();
    fs::create_dir_all(path.join(".flow").join("bin")).unwrap();
    fs::create_dir_all(path.join(".flow").join("conventions")).unwrap();
    fs::write(
        path.join(".flow").join("conventions").join("core.md"),
        "x\n",
    )
    .unwrap();
    fs::write(
        path.join(".flow").join("bin").join("flow"),
        "#!/bin/sh\nexec flow \"$@\"\n",
    )
    .unwrap();
    fs::write(
        path.join(".flow").join("version"),
        env!("CARGO_PKG_VERSION"),
    )
    .unwrap();
    fs::write(
        path.join(".flow").join("config.yaml"),
        "schema_version: 1.0\nprefix: flow\nhosts: []\nlayout:\n  version: 2\n",
    )
    .unwrap();
    fs::write(path.join("AGENTS.md"), "# AGENTS.md\n").unwrap();
    fs::create_dir_all(path.join("flow").join("runs")).unwrap();
    fs::create_dir_all(path.join("flow").join("docs")).unwrap();
    fs::write(path.join("flow").join("roadmap.md"), "# Roadmap\n").unwrap();
    fs::create_dir_all(path.join("docs")).unwrap();
    fs::write(path.join("docs").join("README.md"), "# Introduction\n").unwrap();
    td
}

fn repo_root() -> PathBuf {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest_dir
        .parent()
        .and_then(Path::parent)
        .map(Path::to_path_buf)
        .expect("expected workspace root above crates/flow-cli")
}

/// T-002: the SUMMARY generator example is registered and prints the renderer
/// output for the current repository.
#[test]
fn t002_generate_summary_example_registered_and_runs() {
    Command::new("cargo")
        .args([
            "run",
            "-q",
            "-p",
            "flow-cli",
            "--example",
            "generate_summary",
        ])
        .current_dir(repo_root())
        .assert()
        .success()
        .stdout(predicate::str::contains(ownership::SUMMARY_MARKER));
}

/// T-008: a deliberately-stale `docs/SUMMARY.md` produces a warning, and
/// `flow doctor` still exits successfully.
#[test]
fn t008_drift_warning_fires_on_stale_summary() {
    let repo = make_flow_repo();
    fs::write(
        repo.path().join("docs").join("SUMMARY.md"),
        format!(
            "# Summary\n\n{}\n\nthis is intentionally stale\n",
            ownership::SUMMARY_MARKER
        ),
    )
    .unwrap();

    Command::cargo_bin("flow")
        .unwrap()
        .arg("doctor")
        .current_dir(repo.path())
        .assert()
        .success()
        .stdout(predicate::str::contains(DRIFT_WARNING))
        .stdout(predicate::str::contains(
            "Run `flow update` to refresh Flow-owned generated docs.",
        ))
        .stdout(predicate::str::contains("cargo run").not());
}

/// T-003: an app-owned unmarked `docs/SUMMARY.md` is not treated as generated
/// Flow documentation and does not produce the generator drift warning.
#[test]
fn t003_unmarked_summary_is_app_owned_and_silent() {
    let repo = make_flow_repo();
    fs::write(
        repo.path().join("docs").join("SUMMARY.md"),
        "# Summary\n\nthis app owns its own summary\n",
    )
    .unwrap();

    Command::cargo_bin("flow")
        .unwrap()
        .arg("doctor")
        .current_dir(repo.path())
        .assert()
        .success()
        .stdout(predicate::str::contains(DRIFT_WARNING).not());
}

/// T-001 / T-004: the generated docs summary includes Flow's ownership marker.
#[test]
fn t001_generated_summary_contains_marker() {
    let repo = make_flow_repo();
    let rendered = summary::render_full(repo.path());
    assert!(ownership::has_marker(&rendered, ownership::SUMMARY_MARKER));
}

/// T-008: a freshly-rendered `docs/SUMMARY.md` produces no drift warning.
#[test]
fn t008_no_drift_warning_when_summary_is_fresh() {
    let repo = make_flow_repo();
    fs::write(
        repo.path().join("docs").join("SUMMARY.md"),
        summary::render_full(repo.path()),
    )
    .unwrap();

    Command::cargo_bin("flow")
        .unwrap()
        .arg("doctor")
        .current_dir(repo.path())
        .assert()
        .success()
        .stdout(predicate::str::contains(DRIFT_WARNING).not());
}

/// T-008: when `docs/SUMMARY.md` is absent, the drift check is silent.
#[test]
fn t008_drift_check_silent_when_summary_missing() {
    let repo = make_flow_repo();

    Command::cargo_bin("flow")
        .unwrap()
        .arg("doctor")
        .current_dir(repo.path())
        .assert()
        .success()
        .stdout(predicate::str::contains(DRIFT_WARNING).not());
}

/// T-008: the committed `docs/SUMMARY.md` matches the generator.
#[test]
fn t008_committed_summary_matches_generator() {
    let repo = repo_root();
    let committed = fs::read_to_string(repo.join("docs").join("SUMMARY.md")).unwrap();
    let generated = summary::render_full(&repo);
    assert_eq!(
        committed, generated,
        "docs/SUMMARY.md is out of date; run `flow update` to refresh Flow-owned generated docs"
    );
}
