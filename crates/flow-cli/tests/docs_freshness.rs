//! Integration tests for Phase D docs freshness checks.
//!
//! Tasks covered:
//! - T-001: git.default_branch config field (via CLI config loading)
//! - T-002: DocsConfig / TouchMapEntry parsing (via CLI config loading)
//! - T-003: git::merge_base and git::diff_files (exercised by touch-map tests)
//! - T-006: review-marker warnings in `flow doctor`
//! - T-010: touch-map warnings in `flow doctor`

use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use tempfile::TempDir;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Create a minimal Flow-installed git repo on a "main" branch.
///
/// Passes the structural checks that `flow doctor` performs so only the new
/// freshness checks are exercised.
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
    // Rename to "main" so resolve_default_branch always finds it.
    std::process::Command::new("git")
        .args(["branch", "-M", "main"])
        .current_dir(path)
        .output()
        .unwrap();

    // Minimal Flow installation so structural doctor checks pass.
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

    td
}

/// Extend `make_flow_repo` by creating a Flow branch and committing files.
///
/// `branch_files` is a list of `(relative_path, content)` pairs. They are
/// written and committed on the Flow branch so `git diff <merge-base>`
/// picks them up as changed.
fn make_flow_repo_with_branch(branch_files: &[(&str, &str)]) -> TempDir {
    let td = make_flow_repo();
    let path = td.path();

    std::process::Command::new("git")
        .args(["switch", "-c", "feature"])
        .current_dir(path)
        .output()
        .unwrap();

    for (rel, content) in branch_files {
        let full = path.join(rel);
        if let Some(parent) = full.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(&full, content).unwrap();
    }

    if !branch_files.is_empty() {
        std::process::Command::new("git")
            .args(["add", "."])
            .current_dir(path)
            .output()
            .unwrap();
        std::process::Command::new("git")
            .args(["commit", "-q", "-m", "feature changes"])
            .current_dir(path)
            .output()
            .unwrap();
    }

    td
}

fn flow() -> Command {
    Command::cargo_bin("flow").unwrap()
}

// ---------------------------------------------------------------------------
// T-006: review-marker tests
// ---------------------------------------------------------------------------

/// T-006 / SC-002: `flow doctor` warns when a page under a monitored dir lacks
/// the `**Reviewed**: YYYY-MM-DD` marker.
#[test]
fn t006_missing_review_marker_warns() {
    let repo = make_flow_repo();
    fs::create_dir_all(repo.path().join("docs").join("how-to")).unwrap();
    fs::write(
        repo.path().join("docs").join("how-to").join("guide.md"),
        "# Guide\n\nContent without a review marker.\n",
    )
    .unwrap();

    flow()
        .arg("doctor")
        .current_dir(repo.path())
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "lacks **Reviewed**: YYYY-MM-DD marker",
        ));
}

/// T-006 / SC-003: `flow doctor` warns when the review marker date is older
/// than `docs.review_max_age_days`.
#[test]
fn t006_stale_review_marker_warns() {
    let repo = make_flow_repo();
    fs::create_dir_all(repo.path().join("docs").join("explanation")).unwrap();
    fs::write(
        repo.path().join("docs").join("explanation").join("arch.md"),
        "# Architecture\n\n**Reviewed**: 2000-01-01\n\nContent.\n",
    )
    .unwrap();
    fs::create_dir_all(repo.path().join(".flow")).unwrap();
    fs::write(
        repo.path().join(".flow").join("config.yaml"),
        "layout:\n  version: 2\ndocs:\n  review_max_age_days: 30\n",
    )
    .unwrap();

    flow()
        .arg("doctor")
        .current_dir(repo.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("older than 30 days"));
}

/// T-006 / SC-002: `flow doctor` does NOT warn when the review marker is
/// present and within the age threshold.
#[test]
fn t006_fresh_review_marker_no_warn() {
    let repo = make_flow_repo();
    fs::create_dir_all(repo.path().join("docs").join("start-here")).unwrap();
    fs::write(
        repo.path()
            .join("docs")
            .join("start-here")
            .join("install.md"),
        "# Install\n\n**Reviewed**: 2099-01-01\n\nContent.\n",
    )
    .unwrap();
    fs::write(
        repo.path().join(".flow").join("config.yaml"),
        "layout:\n  version: 2\ndocs:\n  review_max_age_days: 180\n",
    )
    .unwrap();

    flow()
        .arg("doctor")
        .current_dir(repo.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("docs freshness").not());
}

/// T-006: Files under `docs/reference/` are not scanned for review markers.
#[test]
fn t006_reference_dir_is_exempt() {
    let repo = make_flow_repo();
    fs::create_dir_all(repo.path().join("docs").join("reference")).unwrap();
    fs::write(
        repo.path().join("docs").join("reference").join("cli.md"),
        "# CLI\n\nNo marker here.\n",
    )
    .unwrap();

    flow()
        .arg("doctor")
        .current_dir(repo.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("docs freshness").not());
}

/// T-006: Files under `docs/decisions/` are not scanned for review markers.
#[test]
fn t006_decisions_dir_is_exempt() {
    let repo = make_flow_repo();
    fs::create_dir_all(repo.path().join("docs").join("decisions")).unwrap();
    fs::write(
        repo.path()
            .join("docs")
            .join("decisions")
            .join("0009-test.md"),
        "# ADR\n\nNo marker.\n",
    )
    .unwrap();

    flow()
        .arg("doctor")
        .current_dir(repo.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("docs freshness").not());
}

/// T-006 / SC-001: `flow doctor` emits no new output when `docs:` config key
/// is absent (zero regression).
#[test]
fn t006_no_config_no_freshness_output() {
    // No docs freshness config and no docs/ dirs.
    let repo = make_flow_repo();

    flow()
        .arg("doctor")
        .current_dir(repo.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("docs freshness").not())
        .stdout(predicate::str::contains("docs touch-map").not());
}

// ---------------------------------------------------------------------------
// T-010: touch-map tests (T-003 git utilities exercised here)
// ---------------------------------------------------------------------------

/// T-010 / SC-004: Warning fires when a source glob matches a changed file and
/// the mapped doc path was not touched. (T-003: exercises merge_base + diff_files)
#[test]
fn t010_touch_map_fires_when_source_changed_doc_not_touched() {
    // Flow branch: crates/flow-core/src/new.rs committed; doc not touched.
    let repo = make_flow_repo_with_branch(&[("crates/flow-core/src/new.rs", "// new\n")]);

    fs::write(
        repo.path().join(".flow").join("config.yaml"),
        concat!(
            "layout:\n",
            "  version: 2\n",
            "docs:\n",
            "  touch_map:\n",
            "    - paths:\n",
            "        - 'crates/flow-core/**'\n",
            "      docs:\n",
            "        - 'docs/reference/commands.md'\n",
        ),
    )
    .unwrap();
    // Create the mapped doc so the "missing on disk" sub-warning doesn't fire.
    fs::create_dir_all(repo.path().join("docs").join("reference")).unwrap();
    fs::write(
        repo.path()
            .join("docs")
            .join("reference")
            .join("commands.md"),
        "# Commands\n\n**Reviewed**: 2099-01-01\n",
    )
    .unwrap();

    flow()
        .arg("doctor")
        .current_dir(repo.path())
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "docs touch-map: crates/flow-core/src/new.rs changed",
        ))
        .stdout(predicate::str::contains(
            "consider updating docs/reference/commands.md",
        ));
}

/// T-010 / SC-005: No warning when the mapped doc also appears in the diff.
#[test]
fn t010_no_warning_when_doc_also_changed() {
    // Flow branch: both source and doc are changed.
    let repo = make_flow_repo_with_branch(&[
        ("crates/flow-core/src/new.rs", "// new\n"),
        ("docs/reference/commands.md", "# Commands updated\n"),
    ]);

    fs::write(
        repo.path().join(".flow").join("config.yaml"),
        concat!(
            "layout:\n",
            "  version: 2\n",
            "docs:\n",
            "  touch_map:\n",
            "    - paths:\n",
            "        - 'crates/flow-core/**'\n",
            "      docs:\n",
            "        - 'docs/reference/commands.md'\n",
        ),
    )
    .unwrap();

    flow()
        .arg("doctor")
        .current_dir(repo.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("docs touch-map").not());
}

/// T-010: No warning when the source glob matches nothing in the diff.
#[test]
fn t010_no_warning_when_source_glob_misses() {
    // Flow branch: only a README changed — glob targets crates/**
    let repo = make_flow_repo_with_branch(&[("README.md", "# Updated readme\n")]);

    fs::write(
        repo.path().join(".flow").join("config.yaml"),
        concat!(
            "layout:\n",
            "  version: 2\n",
            "docs:\n",
            "  touch_map:\n",
            "    - paths:\n",
            "        - 'crates/**'\n",
            "      docs:\n",
            "        - 'docs/reference/commands.md'\n",
        ),
    )
    .unwrap();

    flow()
        .arg("doctor")
        .current_dir(repo.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("docs touch-map").not());
}

/// T-010: `suppress: true` on an entry silences its warning.
#[test]
fn t010_suppress_true_silences_entry() {
    let repo = make_flow_repo_with_branch(&[("crates/flow-core/src/new.rs", "// new\n")]);

    fs::write(
        repo.path().join(".flow").join("config.yaml"),
        concat!(
            "layout:\n",
            "  version: 2\n",
            "docs:\n",
            "  touch_map:\n",
            "    - paths:\n",
            "        - 'crates/flow-core/**'\n",
            "      docs:\n",
            "        - 'docs/reference/commands.md'\n",
            "      suppress: true\n",
        ),
    )
    .unwrap();

    flow()
        .arg("doctor")
        .current_dir(repo.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("docs touch-map").not());
}

/// T-010: `touch_map_warnings: false` globally silences all touch-map warnings.
#[test]
fn t010_touch_map_warnings_false_silences_all() {
    let repo = make_flow_repo_with_branch(&[("crates/flow-core/src/new.rs", "// new\n")]);

    fs::write(
        repo.path().join(".flow").join("config.yaml"),
        concat!(
            "layout:\n",
            "  version: 2\n",
            "docs:\n",
            "  touch_map_warnings: false\n",
            "  touch_map:\n",
            "    - paths:\n",
            "        - 'crates/flow-core/**'\n",
            "      docs:\n",
            "        - 'docs/reference/commands.md'\n",
        ),
    )
    .unwrap();

    flow()
        .arg("doctor")
        .current_dir(repo.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("docs touch-map").not());
}

/// T-010 / SC-001: No output when `docs.touch_map` is absent (zero regression).
#[test]
fn t010_no_output_when_touch_map_absent() {
    // Repo with Flow branch changes but no touch_map configured.
    let repo = make_flow_repo_with_branch(&[("crates/flow-core/src/new.rs", "// new\n")]);

    flow()
        .arg("doctor")
        .current_dir(repo.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("docs touch-map").not());
}
