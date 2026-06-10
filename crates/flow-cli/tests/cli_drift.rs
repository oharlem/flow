//! Integration tests for the `flow doctor` cli.md drift check.
//!
//! Tests covered: T-001 (shared `cli_help::render_full()` helper),
//! T-009 (the doctor warning), T-010 (this test file).

use assert_cmd::Command;
use flow_cli::{cli_help, ownership};
use predicates::prelude::*;
use std::fs;
use tempfile::TempDir;

const DRIFT_WARNING: &str = "warning: generated docs are stale: docs/reference/cli.md";

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
    // Seed a minimal "Flow is installed" layout so doctor's structural
    // checks pass and only the cli.md drift check is exercised.
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
    fs::create_dir_all(path.join("docs").join("reference")).unwrap();
    td
}

/// T-009 / T-010: a deliberately-stale `docs/reference/cli.md` produces a
/// warning, and `flow doctor` still exits successfully.
#[test]
fn t010_drift_warning_fires_on_stale_cli_md() {
    // Covers: T-009 (warning emitted), T-010 (this assertion).
    let repo = make_flow_repo();
    fs::write(
        repo.path().join("docs").join("reference").join("cli.md"),
        format!(
            "# CLI reference\n\n{}\n\nthis is intentionally not what render_full produces\n",
            ownership::CLI_REFERENCE_MARKER
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

/// T-002: an app-owned unmarked `docs/reference/cli.md` is not treated as
/// Flow-generated documentation and does not produce the generator drift warning.
#[test]
fn t002_unmarked_cli_md_is_app_owned_and_silent() {
    let repo = make_flow_repo();
    fs::write(
        repo.path().join("docs").join("reference").join("cli.md"),
        "# CLI reference\n\nthis app owns its own CLI docs\n",
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

/// T-009 / T-010: a freshly-rendered `docs/reference/cli.md` produces no
/// drift warning. T-001's shared helper is exercised here.
#[test]
fn t010_no_drift_warning_when_cli_md_is_fresh() {
    // Covers: T-009 (no false positive), T-010 (this assertion).
    let repo = make_flow_repo();
    fs::write(
        repo.path().join("docs").join("reference").join("cli.md"),
        cli_help::render_full(),
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

/// T-010: a freshly-rendered `docs/reference/cli.md` stays fresh even when
/// the terminal width changes. This guards the generated reference against
/// clap's environment-sensitive help wrapping.
#[test]
fn t010_no_drift_warning_when_columns_changes() {
    let repo = make_flow_repo();
    fs::write(
        repo.path().join("docs").join("reference").join("cli.md"),
        cli_help::render_full(),
    )
    .unwrap();

    Command::cargo_bin("flow")
        .unwrap()
        .arg("doctor")
        .env("COLUMNS", "80")
        .current_dir(repo.path())
        .assert()
        .success()
        .stdout(predicate::str::contains(DRIFT_WARNING).not());
}

/// T-001 / T-004: the generated CLI reference includes Flow's ownership marker.
#[test]
fn t001_generated_cli_reference_contains_marker() {
    let rendered = cli_help::render_full();
    assert!(ownership::has_marker(
        &rendered,
        ownership::CLI_REFERENCE_MARKER
    ));
}

/// T-009: when `docs/reference/cli.md` is absent (a project that has not
/// adopted the file yet), the drift check is silent.
#[test]
fn t009_drift_check_silent_when_cli_md_missing() {
    // Covers: T-009.
    let repo = make_flow_repo();

    Command::cargo_bin("flow")
        .unwrap()
        .arg("doctor")
        .current_dir(repo.path())
        .assert()
        .success()
        .stdout(predicate::str::contains(DRIFT_WARNING).not());
}
