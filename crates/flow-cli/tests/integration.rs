//! End-to-end CLI integration tests.
//!
//! These tests exercise the compiled `flow` binary against a scratch git repo.

use assert_cmd::Command;
use flow_cli::{cli_help, ownership, summary};
use flow_core::{assets, config::Config};
use predicates::prelude::*;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

fn make_repo() -> TempDir {
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
    td
}

fn fake_flow_on_path() -> TempDir {
    let td = TempDir::new().unwrap();
    let exe = td
        .path()
        .join(if cfg!(windows) { "flow.cmd" } else { "flow" });
    std::fs::write(
        &exe,
        if cfg!(windows) {
            "@echo off\r\n"
        } else {
            "#!/bin/sh\n"
        },
    )
    .unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut permissions = std::fs::metadata(&exe).unwrap().permissions();
        permissions.set_mode(0o755);
        std::fs::set_permissions(&exe, permissions).unwrap();
    }
    td
}

fn git_only_path() -> TempDir {
    let td = TempDir::new().unwrap();
    let git = executable_on_path("git").expect("git must be available for integration tests");
    let wrapper = td
        .path()
        .join(if cfg!(windows) { "git.cmd" } else { "git" });
    #[cfg(unix)]
    std::os::unix::fs::symlink(&git, &wrapper).unwrap();
    #[cfg(windows)]
    std::fs::write(
        &wrapper,
        format!("@echo off\r\n\"{}\" %*\r\n", git.display()),
    )
    .unwrap();
    td
}

fn executable_on_path(name: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&path) {
        let candidate = dir.join(name);
        if candidate.is_file() {
            return Some(candidate);
        }
        #[cfg(windows)]
        {
            for ext in ["exe", "cmd", "bat"] {
                let candidate = dir.join(format!("{name}.{ext}"));
                if candidate.is_file() {
                    return Some(candidate);
                }
            }
        }
    }
    None
}

fn path_with_dirs(dirs: &[&Path]) -> OsString {
    std::env::join_paths(dirs).unwrap()
}

fn commit_all(repo: &Path, message: &str) {
    std::process::Command::new("git")
        .args(["add", "."])
        .current_dir(repo)
        .output()
        .unwrap();
    std::process::Command::new("git")
        .args(["commit", "-q", "-m", message])
        .current_dir(repo)
        .output()
        .unwrap();
}

fn canonical_change_dir(repo: &Path, slug: &str) -> PathBuf {
    let run_name = format!("20260101-{slug}");
    let run_rel = format!("flow/runs/{run_name}");
    let change_rel = format!("{run_rel}/changes/{slug}");
    let run_dir = repo.join(&run_rel);
    let change_dir = repo.join(&change_rel);
    std::fs::create_dir_all(&change_dir).unwrap();
    std::fs::write(
        run_dir.join("run.md"),
        format!(
            "# Run: {slug}\n\n**Run name**: {run_name}\n**Run type**: one-off\n**Status**: running\n**Run branch**: (none)\n**Roadmap fingerprint**: (none)\n**Checkpoint commits**: disabled\n**Current milestone**: (none)\n**Current change**: {change_rel}\n**Current phase**: seeded\n**Last saved Flow action**: seeded\n**Next command**: flow build\n**Last checkpoint**: (none)\n\n## Changes\n\n- [ ] {change_rel} — milestone: (none)\n\n## Milestones\n\n(none)\n"
        ),
    )
    .unwrap();
    std::fs::write(
        run_dir.join("log.md"),
        format!(
            "# Run Log: {slug}\n\n**Run**: {run_name}\n**Target**: {slug}\n**Started**: 2026-01-01T00:00:00Z\n**Status**: running\n\n## Event Log\n\n- 2026-01-01T00:00:00Z — seeded — Test fixture.\n\n## Decisions\n\n(none)\n\n## Operations\n\n(none)\n"
        ),
    )
    .unwrap();
    change_dir
}

fn seed_roadmap_run(repo: &Path, name: &str, roadmap_text: &str) -> PathBuf {
    let run_dir = repo.join("flow").join("runs").join(name);
    std::fs::create_dir_all(run_dir.join("changes")).unwrap();
    std::fs::write(run_dir.join("roadmap.md"), roadmap_text).unwrap();
    let fingerprint = flow_core::roadmap::fingerprint(roadmap_text);
    let milestones = flow_core::parse::roadmap::parse_str(roadmap_text)
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
            "# Run: Test\n\n**Run name**: {name}\n**Run type**: roadmap\n**Run scope**: (none)\n**Status**: running\n**Run branch**: (none)\n**Roadmap fingerprint**: {fingerprint}\n**Checkpoint commits**: disabled\n**Current milestone**: (none)\n**Current change**: (none)\n**Current phase**: roadmap-ready\n**Last saved Flow action**: roadmap-finalized\n**Next command**: $flow-run\n**Last checkpoint**: (none)\n\n## Changes\n\n(none)\n\n## Milestones\n\n{milestones}\n"
        ),
    )
    .unwrap();
    std::fs::write(run_dir.join("log.md"), "# Run Log\n\n## Event Log\n\n").unwrap();
    run_dir
}

fn find_change_dir(repo: &Path, slug: &str) -> PathBuf {
    let runs = repo.join("flow").join("runs");
    let mut matches = Vec::new();
    if let Ok(run_dirs) = std::fs::read_dir(&runs) {
        for run in run_dirs.flatten() {
            let candidate = run.path().join("changes").join(slug);
            if candidate.join("status.md").is_file() {
                matches.push(candidate);
            }
        }
    }
    matches.sort();
    matches.pop().unwrap_or_else(|| {
        panic!(
            "expected change directory for {slug} under {}",
            runs.display()
        )
    })
}

fn seed_building_feature(repo: &Path, slug: &str, tasks: &str) -> PathBuf {
    let feature = slug.to_string();
    let feature_dir = canonical_change_dir(repo, slug);
    std::fs::write(
        feature_dir.join("spec.md"),
        format!(
            "# Spec: {feature}\n\n## What & Why\n\nBuild {slug}.\n\n## Requirements\n\n### Functional Requirements\n\n- **FR-001**: Provide {slug}.\n\n## Success Criteria\n\n### Measurable Outcomes\n\n- **SC-001**: {slug} is available.\n"
        ),
    )
    .unwrap();
    std::fs::write(
        feature_dir.join("plan.md"),
        "# Implementation Plan\n\n## Summary\n\nBuild it.\n\n## Technical Context\n\n**Language/Version**: Rust\n\n## Documentation Impact\n\nImpact: none\n\nDocs already current because this fixture does not change current Flow documentation.\n",
    )
    .unwrap();
    std::fs::write(feature_dir.join("tasks.md"), tasks).unwrap();
    std::fs::write(
        feature_dir.join("status.md"),
        format!(
            "# Status: {feature}\n\n**Change**: {feature}\n**Started**: 2026-01-01\n**Updated**: 2026-01-01T00:00:00Z\n**State**: building\n**Branch**: flow/{feature}\n\n## History\n\n- 2026-01-01T00:00:00Z — plan-complete — plan and tasks finalized\n"
        ),
    )
    .unwrap();
    feature_dir
}

fn seed_drafting_feature(repo: &Path, slug: &str) -> PathBuf {
    let feature = slug.to_string();
    let feature_dir = canonical_change_dir(repo, slug);
    std::fs::write(
        feature_dir.join("spec.md"),
        format!(
            "# Spec: {feature}\n\n**Change**: {feature}\n**Created**: 2026-01-01\n\n## What & Why\n\nDraft {slug}.\n"
        ),
    )
    .unwrap();
    std::fs::write(
        feature_dir.join("status.md"),
        format!(
            "# Status: {feature}\n\n**Change**: {feature}\n**Started**: 2026-01-01\n**Updated**: 2026-01-01T00:00:00Z\n**State**: drafting\n**Branch**: flow/{feature}\n\n## History\n\n- 2026-01-01T00:00:00Z — spec-complete — spec drafted\n"
        ),
    )
    .unwrap();
    feature_dir
}

fn seed_unfinalized_plan_feature(repo: &Path, slug: &str) -> PathBuf {
    let feature = slug.to_string();
    let feature_dir = canonical_change_dir(repo, slug);
    std::fs::write(
        feature_dir.join("plan.md"),
        "## Summary\n\nx\n\n## Technical Context\n\n**Language/Version**: Rust\n\n## Documentation Impact\n\nImpact: none\n\nDocs already current because this fixture only checks the plan-complete gate.\n",
    )
    .unwrap();
    std::fs::write(
        feature_dir.join("tasks.md"),
        "## Tasks\n\n- [ ] **T-001**: a\n    - Covers: FR-001\n    - Verifies: SC-001\n    - Depends-On: (none)\n",
    )
    .unwrap();
    std::fs::write(
        feature_dir.join("status.md"),
        format!(
            "# Status: {feature}\n\n**Change**: {feature}\n**Started**: 2026-01-01\n**Updated**: 2026-01-01T00:00:00Z\n**State**: drafting\n**Branch**: flow/{feature}\n\n## History\n\n- 2026-01-01T00:00:00Z — started — seeded\n"
        ),
    )
    .unwrap();
    feature_dir
}

#[test]
fn version_command() {
    Command::cargo_bin("flow")
        .unwrap()
        .arg("--version")
        .assert()
        .success()
        // Version is "flow <package-version> (<sha> built <YYYY-MM-DD>)" at build time.
        .stdout(predicate::str::contains(format!(
            "flow {} (",
            env!("CARGO_PKG_VERSION")
        )))
        .stdout(predicate::str::contains(" built "));
}

#[test]
fn help_lists_all_commands() {
    Command::cargo_bin("flow")
        .unwrap()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("init"))
        .stdout(predicate::str::contains("start"))
        .stdout(predicate::str::contains("plan"))
        .stdout(predicate::str::contains("build-task"))
        .stdout(predicate::str::contains("set"))
        .stdout(predicate::str::contains("settings"))
        .stdout(predicate::str::contains("close"))
        .stdout(predicate::str::contains("release-patch").not())
        .stdout(predicate::str::contains("release-minor").not())
        .stdout(predicate::str::contains("release-major").not())
        .stdout(predicate::str::contains("ship").not());
}

#[test]
fn init_creates_flow_tree() {
    let repo = make_repo();
    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .args(["init"])
        .assert()
        .success()
        .stderr(predicate::str::contains(
            "run `flow export-assets --dir <DIR>` to inspect embedded defaults",
        ));
    assert!(
        !repo
            .path()
            .join(".flow")
            .join("conventions")
            .join("core.md")
            .is_file(),
        "flow init should rely on embedded conventions by default"
    );
    assert!(!repo.path().join(".flow").join("conventions.md").is_file());
    assert!(!repo.path().join("flow").join("conventions.md").is_file());
    assert_eq!(
        std::fs::read_to_string(repo.path().join(".flow").join("version"))
            .unwrap()
            .trim(),
        env!("CARGO_PKG_VERSION")
    );
    assert!(repo.path().join(".flow").join("config.yaml").is_file());
    assert!(
        !repo
            .path()
            .join(".flow")
            .join("agents")
            .join("start.base.md")
            .is_file(),
        "flow init should rely on embedded base prompts by default"
    );
    assert!(
        !repo.path().join(".flow").join("bin").join("flow").exists(),
        "flow init must not create a project-local launcher"
    );
}

#[test]
fn doctor_green_after_init() {
    // T-001 / T-003: doctor stays green after init.
    let repo = make_repo();
    let fake_flow = fake_flow_on_path();
    let fake_git = git_only_path();
    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .args(["init"])
        .assert()
        .success();
    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .env("PATH", path_with_dirs(&[fake_flow.path(), fake_git.path()]))
        .args(["doctor"])
        .assert()
        .success()
        .stdout(predicate::str::contains(".flow/version"))
        .stdout(predicate::str::contains(format!(
            "Installed Flow version: {}",
            env!("CARGO_PKG_VERSION")
        )))
        .stdout(predicate::str::contains("Embedded asset version: 1.1"))
        .stdout(predicate::str::contains(
            "run `flow export-assets --dir <DIR>` to inspect embedded defaults",
        ))
        .stdout(predicate::str::contains("Flow is installed"));
}

#[test]
fn doctor_fails_when_version_marker_missing() {
    let repo = make_repo();
    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .args(["init"])
        .assert()
        .success();
    std::fs::remove_file(repo.path().join(".flow").join("version")).unwrap();

    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .args(["doctor"])
        .assert()
        .failure()
        .stdout(predicate::str::contains("[MISSING] .flow/version"));
}

#[test]
fn doctor_warns_when_version_marker_is_stale() {
    let repo = make_repo();
    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .args(["init"])
        .assert()
        .success();
    std::fs::write(repo.path().join(".flow").join("version"), "0.0.9\n").unwrap();

    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .args(["doctor"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Installed Flow version: 0.0.9"))
        .stdout(predicate::str::contains(format!(
            "running Flow binary is {} (newer than recorded 0.0.9)",
            env!("CARGO_PKG_VERSION")
        )))
        .stdout(predicate::str::contains("run `flow update`"));
}

#[test]
fn doctor_advises_reinstall_when_marker_is_newer_than_binary() {
    // When `.flow/version` records a newer Flow than the running binary
    // (for example, after installing an older binary), doctor
    // must steer users to reinstall rather than to `flow update`, which
    // would otherwise refuse with a downgrade error.
    let repo = make_repo();
    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .args(["init"])
        .assert()
        .success();
    std::fs::write(repo.path().join(".flow").join("version"), "99.0.0\n").unwrap();

    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .args(["doctor"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Installed Flow version: 99.0.0"))
        .stdout(predicate::str::contains(format!(
            "running Flow binary is {} (older than recorded 99.0.0)",
            env!("CARGO_PKG_VERSION")
        )))
        .stdout(predicate::str::contains(
            "cargo install --git https://github.com/oharlem/flow --locked --force flow-cli",
        ))
        .stdout(predicate::str::contains("flow update --force"));
}

#[test]
fn doctor_fails_before_init() {
    let repo = make_repo();
    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .args(["doctor"])
        .assert()
        .failure();
}

#[test]
fn start_creates_feature_and_envelope() {
    let repo = make_repo();
    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .args(["init"])
        .assert()
        .success();
    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .args(["start", "add", "login", "form"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Flow Artifact Conventions"))
        .stdout(predicate::str::contains("**Review**: collapsed"))
        .stdout(predicate::str::contains("Finalization Instructions").not());
    let feature_dir = find_change_dir(repo.path(), "add-login-form");
    assert!(feature_dir.join("spec.md").is_file());
    assert!(feature_dir.join("status.md").is_file());
}

#[test]
fn status_on_fresh_feature_reports_missing_tasks() {
    let repo = make_repo();
    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .args(["init"])
        .assert()
        .success();
    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .args(["start", "hello"])
        .assert()
        .success();
    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .args(["status"])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Flow could not run the consistency check yet.",
        ));
}

#[test]
fn close_ticks_linked_milestone() {
    let repo = make_repo();
    let root = repo.path();
    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(root)
        .args(["init"])
        .assert()
        .success();
    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(root)
        .env("FLOW_FORCE_ON_PROTECTED", "1")
        .args(["start", "linked roadmap"])
        .assert()
        .success();

    let feature_dir = seed_ready_to_close_feature(root, "linked-roadmap");
    let status_path = feature_dir.join("status.md");
    let status = std::fs::read_to_string(&status_path).unwrap();
    std::fs::write(
        &status_path,
        status.replace(
            "**Branch**: flow/linked-roadmap\n",
            "**Branch**: flow/linked-roadmap\n**Milestone**: M-1\n",
        ),
    )
    .unwrap();
    for phase in ["plan", "build-task", "test"] {
        Command::cargo_bin("flow")
            .unwrap()
            .current_dir(root)
            .env("FLOW_CHANGE_DIR", feature_dir.to_str().unwrap())
            .args([phase, "--finalize"])
            .assert()
            .success();
    }
    let run_dir = feature_dir
        .parent()
        .and_then(|p| p.parent())
        .expect("feature should live under a run");
    let roadmap_text =
        "# Roadmap\n\n## Milestones\n\n### [~] M-1: Legacy roadmap\n\nKeep this milestone body.\n";
    std::fs::write(run_dir.join("roadmap.md"), roadmap_text).unwrap();
    let run_path = run_dir.join("run.md");
    let run_state = std::fs::read_to_string(&run_path).unwrap();
    std::fs::write(
        &run_path,
        run_state
            .replace("**Run type**: one-off", "**Run type**: roadmap")
            .replace(
                "**Roadmap fingerprint**: (none)",
                &format!(
                    "**Roadmap fingerprint**: {}",
                    flow_core::roadmap::fingerprint(roadmap_text)
                ),
            ),
    )
    .unwrap();
    std::fs::write(
        run_dir.join("roadmap.md"),
        "# Roadmap\n\n## Milestones\n\n### [~] M-1: Legacy roadmap\n\nKeep this milestone body.\n",
    )
    .unwrap();

    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(root)
        .env("FLOW_CHANGE_DIR", feature_dir.to_str().unwrap())
        .args(["close", "--finalize"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Change 'linked-roadmap' closed."));

    let roadmap = std::fs::read_to_string(run_dir.join("roadmap.md")).unwrap();

    assert!(
        roadmap.contains("### [x] M-1: Legacy roadmap\n\nKeep this milestone body."),
        "{roadmap}"
    );
    assert!(
        feature_dir.join("status.md").is_file(),
        "closed change should remain in place"
    );
}

#[test]
fn build_rejects_missing_plan_complete_history() {
    let repo = make_repo();
    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .args(["init"])
        .assert()
        .success();
    let feature_dir = seed_unfinalized_plan_feature(repo.path(), "nofinal");

    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .env("FLOW_CHANGE_DIR", &feature_dir)
        .args(["build"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("plan-complete"));
}

#[test]
fn build_finalizers_reject_missing_plan_complete_history() {
    let repo = make_repo();
    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .args(["init"])
        .assert()
        .success();
    let feature_dir = seed_unfinalized_plan_feature(repo.path(), "nofinal-finalize");
    let feature_arg = feature_dir.to_string_lossy().to_string();

    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .env("FLOW_CHANGE_DIR", &feature_arg)
        .args(["build-task", "T-001", "--finalize"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("plan-complete"));

    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .env("FLOW_CHANGE_DIR", &feature_arg)
        .args(["build", "--completed", "T-001", "--finalize"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("plan-complete"));
}

#[test]
fn status_does_not_write_flow_test_cache() {
    let repo = make_repo();
    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .args(["init"])
        .assert()
        .success();
    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .env("FLOW_FORCE_ON_PROTECTED", "1")
        .args(["start", "statcache"])
        .assert()
        .success();
    let feature_dir = find_change_dir(repo.path(), "statcache");
    std::fs::write(
        feature_dir.join("plan.md"),
        "## Summary\n\nx\n\n## Technical Context\n\n**Language/Version**: Rust\n\n## Documentation Impact\n\nImpact: none\n\nDocs already current because this fixture only checks status side effects.\n",
    )
    .unwrap();
    std::fs::write(
        feature_dir.join("tasks.md"),
        "## Tasks\n\n- [ ] **T-001**: a\n    - Covers: FR-001\n    - Verifies: SC-001\n    - Depends-On: (none)\n",
    )
    .unwrap();

    let cache = feature_dir.join(".flow-test.last.md");
    let _ = std::fs::remove_file(&cache);
    assert!(!cache.exists(), "precondition: no drift cache file");

    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .env("FLOW_CHANGE_DIR", &feature_dir)
        .args(["status"])
        .assert()
        .success();

    assert!(
        !cache.exists(),
        "flow status must not create .flow-test.last.md"
    );
}

#[test]
fn start_refuses_amendment_phrasing() {
    let repo = make_repo();
    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .args(["init"])
        .assert()
        .success();
    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .args(["start", "adjust", "the", "spec"])
        .assert()
        .failure();
}

#[test]
fn init_with_claude_code_host_writes_skills() {
    let repo = make_repo();
    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .args(["init", "--host", "claude-code"])
        .assert()
        .success();
    let skills = repo.path().join(".claude").join("skills");
    assert!(skills.is_dir());
    for command in flow_core::assets::HOST_COMMANDS {
        let command = command.name;
        let skill = skills.join(format!("flow-{command}")).join("SKILL.md");
        assert!(skill.is_file(), "missing skill for {command}");
        let body = std::fs::read_to_string(&skill).unwrap();
        assert!(body.contains("never run `git push`") || body.contains("Never run `git push`"));
        assert!(body.contains("worktree, dirty-file, or modified `status.md` issues"));
    }
}

#[test]
fn init_with_codex_host_writes_skills() {
    let repo = make_repo();
    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .args(["init", "--host", "codex"])
        .assert()
        .success();
    let skills = repo.path().join(".agents").join("skills");
    assert!(skills.is_dir());
    let skill = skills.join("flow-start").join("SKILL.md");
    let body = std::fs::read_to_string(&skill).unwrap();
    assert!(body.contains("$flow-start"));
}

#[test]
fn init_with_host_list_writes_multiple_adapters() {
    let repo = make_repo();
    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .args(["init", "--host", "codex,cursor"])
        .assert()
        .success();
    assert!(repo
        .path()
        .join(".agents")
        .join("skills")
        .join("flow-start")
        .join("SKILL.md")
        .is_file());
    assert!(repo
        .path()
        .join(".cursor")
        .join("rules")
        .join("flow.mdc")
        .is_file());
    let config = std::fs::read_to_string(repo.path().join(".flow").join("config.yaml")).unwrap();
    assert!(config.contains("hosts:\n  - codex\n  - cursor"));
}

#[test]
fn init_rejects_positional_agents() {
    let repo = make_repo();
    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .args(["init", "codex", "cursor"])
        .assert()
        .failure();
}

#[test]
fn init_rejects_agent_flag() {
    let repo = make_repo();
    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .args(["init", "--agent", "codex", "--agent", "cursor"])
        .assert()
        .failure();
}

#[test]
fn t006_start_creates_no_milestone_without_link() {
    let repo = make_repo();
    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .args(["init"])
        .assert()
        .success();
    let original_roadmap = "# Roadmap\n\n## Milestones\n\n### [x] M-1: Old\n\nOld body.\n";
    std::fs::write(
        repo.path().join("flow").join("roadmap.md"),
        original_roadmap,
    )
    .unwrap();

    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .env("FLOW_FORCE_ON_PROTECTED", "1")
        .args(["start", "Close command"])
        .assert()
        .success();

    // Roadmap is byte-identical to the original — start does not auto-create milestones.
    let roadmap = std::fs::read_to_string(repo.path().join("flow").join("roadmap.md")).unwrap();
    assert_eq!(
        roadmap, original_roadmap,
        "flow/roadmap.md must be byte-identical when /flow-start has no milestone link"
    );
    let status =
        std::fs::read_to_string(find_change_dir(repo.path(), "close-command").join("status.md"))
            .unwrap();
    assert!(
        !status.contains("**Milestone**:"),
        "status.md must not have a Milestone line when no milestone was supplied:\n{status}"
    );
    assert!(status.contains("**Branch**: flow/close-command"));
}

#[test]
fn t005_start_uses_linked_milestone_label_for_branch_slug() {
    let repo = make_repo();
    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .args(["init"])
        .assert()
        .success();
    let run_dir = seed_roadmap_run(
        repo.path(),
        "20260101-release-commands",
        "# Roadmap\n\n## Milestones\n\n### [ ] M-1: Release commands\n\nUse semver release commands.\n",
    );

    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .env("FLOW_RUN_DIR", &run_dir)
        .env("FLOW_FORCE_ON_PROTECTED", "1")
        .args([
            "start",
            "M-1",
            "implementation notes should not name the branch",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Created change `M-1-release-commands`",
        ))
        .stdout(predicate::str::contains(
            "Linked existing roadmap milestone `M-1`: Release commands.",
        ));

    let feature_dir = find_change_dir(repo.path(), "M-1-release-commands");
    let status = std::fs::read_to_string(feature_dir.join("status.md")).unwrap();
    assert!(status.contains("**Milestone**: M-1"));
    assert!(!repo.path().join("flow/changes").exists());
    assert!(!feature_dir
        .parent()
        .unwrap()
        .join("release-commands")
        .exists());
    assert!(!feature_dir
        .parent()
        .unwrap()
        .join("implementation-notes-should-not-name-the-branch")
        .exists());
}

#[test]
fn t007_t008_start_fails_when_feature_branch_ref_namespace_is_blocked() {
    // Covers: T-007, T-008.
    let repo = make_repo();
    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .args(["init"])
        .assert()
        .success();
    let branch = std::process::Command::new("git")
        .args(["branch", "flow"])
        .current_dir(repo.path())
        .output()
        .unwrap();
    assert!(
        branch.status.success(),
        "git branch flow failed: {branch:?}"
    );

    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .env("FLOW_FORCE_ON_PROTECTED", "1")
        .args(["start", "Namespace Blocked"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("flow/namespace-blocked"))
        .stderr(predicate::str::contains("ref namespace"));

    assert!(!repo.path().join("flow/namespace-blocked").exists());
}

#[test]
fn update_refreshes_detected_host_assets_without_installing_new_hosts() {
    let repo = make_repo();
    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .args(["init"])
        .assert()
        .success();

    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .args(["update"])
        .assert()
        .success()
        .stderr(predicate::str::contains(format!(
            "Refreshed Flow at version {}",
            env!("CARGO_PKG_VERSION")
        )));
    assert!(
        !repo.path().join(".claude").join("skills").exists(),
        "plain update should not install a host that has no existing Flow app assets"
    );

    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .args(["setup", "--host", "codex"])
        .assert()
        .success();
    let skill = repo
        .path()
        .join(".agents")
        .join("skills")
        .join("flow-start")
        .join("SKILL.md");
    std::fs::write(&skill, "stale codex skill").unwrap();
    let agents_md_before = std::fs::read_to_string(repo.path().join("AGENTS.md")).unwrap();

    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .args(["update"])
        .assert()
        .success();

    let refreshed = std::fs::read_to_string(&skill).unwrap();
    assert!(refreshed.contains("$flow-start"));
    let agents_md_after = std::fs::read_to_string(repo.path().join("AGENTS.md")).unwrap();
    assert_eq!(
        agents_md_after, agents_md_before,
        "flow update should refresh app assets without rewriting root AGENTS.md"
    );
    assert!(repo
        .path()
        .join(".agents")
        .join("skills")
        .join("flow-doctor")
        .join("SKILL.md")
        .is_file());
    assert_eq!(
        std::fs::read_to_string(repo.path().join(".flow").join("version"))
            .unwrap()
            .trim(),
        env!("CARGO_PKG_VERSION")
    );
}

#[test]
fn update_refreshes_marked_claude_agents_notes_as_flow_owned() {
    let repo = make_repo();
    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .args(["init"])
        .assert()
        .success();
    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .args(["setup", "--host", "claude-code"])
        .assert()
        .success();

    let agents = repo.path().join("AGENTS.md");
    let before = "# AGENTS.md\n\nUser-owned intro.\n\n<!-- FLOW:CLAUDE-CODE-NOTES:START -->\n## Claude Code Notes (Flow-owned)\n\nStale generated body.\n<!-- FLOW:CLAUDE-CODE-NOTES:END -->\n\n## User Notes\n\nKeep this section.\n";
    std::fs::write(&agents, before).unwrap();

    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .args(["update"])
        .assert()
        .success();

    let refreshed = std::fs::read_to_string(&agents).unwrap();
    assert!(refreshed.contains("<!-- FLOW:CLAUDE-CODE-NOTES:START -->"));
    assert!(refreshed.contains("## Claude Code Notes (Flow-owned)"));
    assert!(refreshed.contains("/flow-roadmap"));
    assert!(refreshed.contains("/flow-run [M-N]"));
    assert!(!refreshed.contains("Stale generated body."));
    assert!(refreshed.contains("## User Notes\n\nKeep this section."));
}

#[test]
fn update_reports_and_refreshes_installed_version_marker() {
    let repo = make_repo();
    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .args(["init"])
        .assert()
        .success();
    std::fs::write(repo.path().join(".flow").join("version"), "0.0.9\n").unwrap();

    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .args(["update"])
        .assert()
        .success()
        .stderr(predicate::str::contains(format!(
            "Upgraded Flow version: 0.0.9 -> {}",
            env!("CARGO_PKG_VERSION")
        )));

    assert_eq!(
        std::fs::read_to_string(repo.path().join(".flow").join("version"))
            .unwrap()
            .trim(),
        env!("CARGO_PKG_VERSION")
    );
    assert!(
        !repo.path().join(".flow").join("bin").join("flow").exists(),
        "flow update must not create a project-local launcher"
    );
}

#[test]
fn update_refuses_to_downgrade_marker_without_force() {
    let repo = make_repo();
    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .args(["init"])
        .assert()
        .success();

    // Pretend a newer Flow binary previously stamped the marker. The
    // currently-running test binary is `CARGO_PKG_VERSION` (e.g. 0.11.2),
    // so 99.0.0 is unambiguously newer.
    let marker_path = repo.path().join(".flow").join("version");
    std::fs::write(&marker_path, "99.0.0\n").unwrap();

    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .args(["update"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("refusing to downgrade Flow"))
        .stderr(predicate::str::contains(".flow/version is 99.0.0"))
        .stderr(predicate::str::contains("flow update --force"));

    assert_eq!(
        std::fs::read_to_string(&marker_path).unwrap().trim(),
        "99.0.0",
        "the marker must be preserved when the downgrade is refused"
    );
}

#[test]
fn update_force_allows_downgrade_and_rewrites_marker() {
    let repo = make_repo();
    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .args(["init"])
        .assert()
        .success();

    let marker_path = repo.path().join(".flow").join("version");
    std::fs::write(&marker_path, "99.0.0\n").unwrap();

    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .args(["update", "--force"])
        .assert()
        .success()
        // A forced downgrade must be loud: an explicit WARN line so the user
        // does not mistake the success exit for a normal upgrade…
        .stderr(predicate::str::contains("WARN"))
        .stderr(predicate::str::contains("forcing downgrade of Flow"))
        .stderr(predicate::str::contains("99.0.0"))
        // …and the closing summary must say "Downgraded", not "Installed".
        .stderr(predicate::str::contains(format!(
            "Downgraded Flow version: 99.0.0 -> {} (forced)",
            env!("CARGO_PKG_VERSION")
        )));

    assert_eq!(
        std::fs::read_to_string(&marker_path).unwrap().trim(),
        env!("CARGO_PKG_VERSION"),
        "--force must accept the downgrade and rewrite the marker"
    );
}

#[test]
fn update_proceeds_when_marker_is_unparseable() {
    // A garbage marker must not strand the user — the comparator returns
    // false on parse failure, so `flow update` refreshes the repo normally.
    let repo = make_repo();
    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .args(["init"])
        .assert()
        .success();

    let marker_path = repo.path().join(".flow").join("version");
    std::fs::write(&marker_path, "not-a-version\n").unwrap();

    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .args(["update"])
        .assert()
        .success();

    assert_eq!(
        std::fs::read_to_string(&marker_path).unwrap().trim(),
        env!("CARGO_PKG_VERSION")
    );
}

#[test]
fn update_refreshes_marked_generated_docs() {
    let repo = make_repo();
    let root = repo.path();
    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(root)
        .args(["init"])
        .assert()
        .success();

    std::fs::create_dir_all(root.join("docs").join("reference")).unwrap();
    std::fs::write(root.join("docs").join("README.md"), "# Introduction\n").unwrap();
    std::fs::write(
        root.join("docs").join("SUMMARY.md"),
        format!(
            "# Summary\n\n{}\n\nthis generated summary is stale\n",
            ownership::SUMMARY_MARKER
        ),
    )
    .unwrap();
    std::fs::write(
        root.join("docs").join("reference").join("cli.md"),
        format!(
            "# CLI reference\n\n{}\n\nthis generated CLI reference is stale\n",
            ownership::CLI_REFERENCE_MARKER
        ),
    )
    .unwrap();

    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(root)
        .args(["update"])
        .assert()
        .success()
        .stderr(predicate::str::contains(
            "Refreshed generated docs: docs/SUMMARY.md, docs/reference/cli.md",
        ));

    assert_eq!(
        std::fs::read_to_string(root.join("docs").join("SUMMARY.md")).unwrap(),
        summary::render_full(root)
    );
    assert_eq!(
        std::fs::read_to_string(root.join("docs").join("reference").join("cli.md")).unwrap(),
        cli_help::render_full()
    );

    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(root)
        .args(["doctor"])
        .assert()
        .success()
        .stdout(predicate::str::contains("generated docs are stale").not());
}

#[test]
fn update_preserves_unmarked_and_missing_generated_docs() {
    let repo = make_repo();
    let root = repo.path();
    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(root)
        .args(["init"])
        .assert()
        .success();

    std::fs::create_dir_all(root.join("docs")).unwrap();
    let unmarked_summary = "# Summary\n\nthis app owns its own summary\n";
    std::fs::write(root.join("docs").join("SUMMARY.md"), unmarked_summary).unwrap();
    let missing_cli = root.join("docs").join("reference").join("cli.md");

    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(root)
        .args(["update"])
        .assert()
        .success()
        .stderr(predicate::str::contains("Refreshed generated docs").not());

    assert_eq!(
        std::fs::read_to_string(root.join("docs").join("SUMMARY.md")).unwrap(),
        unmarked_summary
    );
    assert!(
        !missing_cli.exists(),
        "flow update must not create absent generated docs"
    );
}

#[test]
fn t002_update_preserves_state_and_local_overrides() {
    let repo = make_repo();
    let root = repo.path();
    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(root)
        .args(["init"])
        .assert()
        .success();

    std::fs::write(
        root.join(".flow").join("config.yaml"),
        "schema_version: 1.0\nprefix: product\nhosts: []\nlayout:\n  version: 2\ntest:\n  command: \"cargo test --workspace\"\n",
    )
    .unwrap();
    std::fs::create_dir_all(root.join("product")).unwrap();
    std::fs::write(root.join("product").join("roadmap.md"), "# Roadmap\n").unwrap();
    std::fs::write(root.join(".flow").join("state.yaml"), "counter: 42\n").unwrap();
    std::fs::write(root.join(".flow").join("version"), "0.0.9\n").unwrap();
    std::fs::create_dir_all(root.join(".flow").join("conventions")).unwrap();
    std::fs::write(
        root.join(".flow").join("conventions").join("core.md"),
        assets::conventions_shard("core").unwrap(),
    )
    .unwrap();
    std::fs::write(
        root.join(".flow").join("agents").join("plan.base.md"),
        assets::agent_base("plan").unwrap(),
    )
    .unwrap();
    std::fs::write(
        root.join(".flow").join("agents").join("plan.local.md"),
        "T-002 local override must survive update\n",
    )
    .unwrap();

    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(root)
        .args(["update"])
        .assert()
        .success()
        .stderr(predicate::str::contains(format!(
            "Upgraded Flow version: 0.0.9 -> {}",
            env!("CARGO_PKG_VERSION")
        )));

    let parsed = Config::load(&root.join(".flow").join("config.yaml")).unwrap();
    assert_eq!(parsed.prefix, "product");
    assert_eq!(
        parsed.test.command.as_deref(),
        Some("cargo test --workspace")
    );
    assert!(
        std::fs::read_to_string(root.join(".flow").join("state.yaml"))
            .unwrap()
            .contains("counter: 42"),
        "T-002: flow update must preserve repo-local state"
    );
    assert!(
        root.join(".flow")
            .join("agents")
            .join("plan.local.md")
            .is_file(),
        "T-002: flow update must preserve local prompt overrides"
    );
    assert!(
        !root
            .join(".flow")
            .join("agents")
            .join("plan.base.md")
            .exists(),
        "T-002: flow update must remove generated base prompt copies"
    );
    assert!(
        !root
            .join(".flow")
            .join("conventions")
            .join("core.md")
            .exists(),
        "T-002: flow update must remove generated convention copies"
    );
    assert!(!root.join(".flow").join("bin").join("flow").exists());

    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(root)
        .args(["doctor"])
        .assert()
        .success()
        .stdout(predicate::str::contains("embedded conventions"))
        .stdout(predicate::str::contains("embedded base prompts"))
        .stdout(predicate::str::contains(".flow/conventions/core.md").not())
        .stdout(predicate::str::contains("Flow is installed"));
}

#[test]
fn update_rejects_old_checkpoint_config_key() {
    let repo = make_repo();
    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .args(["init"])
        .assert()
        .success();

    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .args(["setup", "--host", "codex"])
        .assert()
        .success();

    std::fs::write(
        repo.path().join(".flow").join("config.yaml"),
        "schema_version: 1.0\nprefix: product\nhosts:\n  - claude-code\ngit:\n  run_all_checkpoint_commits: false\ntest:\n  command: \"cargo test --workspace\"\n",
    )
    .unwrap();

    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .args(["update"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("run_all_checkpoint_commits"));
}

#[test]
fn update_refreshes_config_defaults_and_detected_hosts() {
    let repo = make_repo();
    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .args(["init"])
        .assert()
        .success();

    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .args(["setup", "--host", "codex"])
        .assert()
        .success();

    std::fs::write(
        repo.path().join(".flow").join("config.yaml"),
        "schema_version: 1.0\nprefix: product\nhosts:\n  - claude-code\nlayout:\n  version: 2\ngit:\n  run_checkpoint_commits: false\ntest:\n  command: \"cargo test --workspace\"\n",
    )
    .unwrap();

    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .args(["update"])
        .assert()
        .success();

    let config_path = repo.path().join(".flow").join("config.yaml");
    let parsed = Config::load(&config_path).unwrap();
    assert_eq!(parsed.prefix, "product");
    assert_eq!(parsed.hosts, vec!["codex".to_string()]);
    assert_eq!(parsed.layout.version, 2);
    assert!(!parsed.git.run_checkpoint_commits);
    assert_eq!(
        parsed.test.command.as_deref(),
        Some("cargo test --workspace")
    );

    let config = std::fs::read_to_string(config_path).unwrap();
    assert!(config.contains("timeout_seconds: 600"));
    assert!(config.contains("layout:"));
    assert!(config.contains("version: 2"));
    assert!(config.contains("run_checkpoint_commits: false"));
    assert!(!config.contains("run_all_checkpoint_commits"));
    assert!(config.contains("phases:"));
    assert!(config.contains("ui:"));
    assert!(
        !config.contains("- claude-code"),
        "expected stale default host to be removed when detected host assets disagree, got:\n{config}"
    );
}

#[test]
fn update_refreshes_detected_cursor_rule() {
    let repo = make_repo();
    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .args(["init", "--host", "cursor"])
        .assert()
        .success();
    let rule = repo.path().join(".cursor").join("rules").join("flow.mdc");
    std::fs::write(&rule, "stale cursor rule").unwrap();

    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .args(["update"])
        .assert()
        .success();

    let refreshed = std::fs::read_to_string(rule).unwrap();
    assert!(refreshed.contains("Supported commands"));
    assert!(refreshed.contains("`doctor`"));
}

#[test]
fn init_with_cursor_host_writes_rules() {
    let repo = make_repo();
    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .args(["init", "--host", "cursor"])
        .assert()
        .success();
    assert!(repo
        .path()
        .join(".cursor")
        .join("rules")
        .join("flow.mdc")
        .is_file());
}

#[test]
fn status_json_emits_well_formed_payload() {
    let repo = make_repo();
    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .args(["init"])
        .assert()
        .success();
    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .args(["start", "hello"])
        .assert()
        .success();

    let out = Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .args(["--json", "status"])
        .output()
        .unwrap();
    assert!(out.status.success());
    let body = String::from_utf8_lossy(&out.stdout).to_string();
    let json: serde_json::Value = serde_json::from_str(&body).unwrap_or_else(|e| {
        panic!("status --json was not valid JSON: {e}\n---\n{body}\n---");
    });
    assert_eq!(json["change"], "hello");
    assert_eq!(json["state"], "drafting");
    assert_eq!(json["next_command"], "flow-plan");
}

#[test]
fn amend_ask_answer_appends_clarification() {
    let repo = make_repo();
    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .args(["init"])
        .assert()
        .success();
    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .args(["start", "demo"])
        .assert()
        .success();
    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .args(["amend", "--ask", "Who?", "--answer", "Us."])
        .assert()
        .success();
    let spec =
        std::fs::read_to_string(find_change_dir(repo.path(), "demo").join("spec.md")).unwrap();
    assert!(spec.contains("## Clarifications"));
    assert!(spec.contains("- Q: Who? → A: Us."));
}

#[test]
fn amend_roadmap_shaped_request_suggests_roadmap_workflow() {
    let repo = make_repo();
    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .args(["init"])
        .assert()
        .success();
    std::process::Command::new("git")
        .args(["checkout", "-b", "syndy-integration"])
        .current_dir(repo.path())
        .output()
        .unwrap();
    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .args([
            "amend",
            "the current roadmap with tasks to implement with Option B or Option C",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("use "))
        .stderr(predicate::str::contains(" instead"))
        .stderr(predicate::str::contains("flow roadmap"))
        .stderr(predicate::str::contains("flow start"))
        .stderr(predicate::str::contains("flow plan"))
        .stderr(predicate::str::contains("spec.md"));
}

#[test]
fn amend_without_active_feature_explains_resolution() {
    let repo = make_repo();
    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .args(["init"])
        .assert()
        .success();
    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .args(["amend", "tweak the login copy in the spec"])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "Cannot resolve active change directory",
        ))
        .stderr(predicate::str::contains("spec.md"))
        .stderr(predicate::str::contains("FLOW_CHANGE_DIR"))
        .stderr(predicate::str::contains("flow roadmap"))
        .stderr(predicate::str::contains("flow start"));
}

#[test]
fn principles_loaded_live_into_envelope() {
    let repo = make_repo();
    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .args(["init"])
        .assert()
        .success();
    // Seed a principles file.
    std::fs::create_dir_all(repo.path().join("docs")).unwrap();
    std::fs::write(
        repo.path().join("docs").join("principles.md"),
        "# Engineering Principles\n\n## Engineering Principles\n\n- **P-001**: Small commits. Rationale: easier review.\n",
    )
    .unwrap();
    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .args(["start", "hello"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Small commits"))
        .stdout(predicate::str::contains("P-001"));
}

#[test]
fn protected_branch_warns_and_honors_force_env() {
    // Default: on `main`, non-TTY stdin → warn+proceed (tests run non-TTY).
    let repo = make_repo();
    let out = Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .args(["init"])
        .output()
        .unwrap();
    assert!(out.status.success());
    let start_out = Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .args(["start", "demo"])
        .output()
        .unwrap();
    assert!(start_out.status.success());
    let stderr = String::from_utf8_lossy(&start_out.stderr);
    assert!(stderr.contains("protected branch 'main'"));

    // With FLOW_FORCE_ON_PROTECTED=1: no warning prompt printed.
    let repo2 = make_repo();
    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo2.path())
        .args(["init"])
        .assert()
        .success();
    let force_out = Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo2.path())
        .env("FLOW_FORCE_ON_PROTECTED", "1")
        .args(["start", "demo"])
        .output()
        .unwrap();
    assert!(force_out.status.success());
    let stderr2 = String::from_utf8_lossy(&force_out.stderr);
    assert!(
        !stderr2.contains("stdin is not a TTY"),
        "FLOW_FORCE_ON_PROTECTED=1 should skip the prompt entirely, got:\n{stderr2}"
    );
}

#[test]
fn t001_t002_flow_set_writes_project_confirmation_setting() {
    // Covers: T-001, T-002.
    let repo = make_repo();
    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .args(["init"])
        .assert()
        .success();

    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .args(["set", "confirmation=no"])
        .assert()
        .success()
        .stdout(predicate::str::contains("confirmation=no"));

    let config_path = repo.path().join(".flow").join("config.yaml");
    let config = std::fs::read_to_string(&config_path).unwrap();
    assert!(config.contains("confirmation: \"no\""));
    assert!(config.contains("# Flow project configuration"));

    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .args(["set", "confirmation=yes"])
        .assert()
        .success()
        .stdout(predicate::str::contains("confirmation=yes"));

    let config = std::fs::read_to_string(config_path).unwrap();
    assert!(config.contains("confirmation: \"yes\""));
    assert!(config.contains("# Flow project configuration"));
}

#[test]
fn t001_flow_set_writes_project_counter_setting() {
    let repo = make_repo();
    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .args(["init"])
        .assert()
        .success();

    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .args(["set", "counter=2"])
        .assert()
        .success()
        .stdout(predicate::str::contains("counter=2"));

    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .args(["settings"])
        .assert()
        .success()
        .stdout(predicate::str::contains("counter=2"));

    let state = std::fs::read_to_string(repo.path().join(".flow/state.yaml")).unwrap();
    assert!(state.contains("counter: 2"), "{state}");
    let config = std::fs::read_to_string(repo.path().join(".flow/config.yaml")).unwrap();
    assert!(!config.contains("counter: 2"), "{config}");
}

#[test]
fn t001_flow_set_counter_rejects_invalid_or_colliding_values() {
    let repo = make_repo();
    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .args(["init"])
        .assert()
        .success();
    seed_roadmap_run(
        repo.path(),
        "20260101-counter",
        "# Roadmap\n\n## Milestones\n\n### [ ] M-1: One\n\n",
    );
    let state_path = repo.path().join(".flow").join("state.yaml");
    let before = std::fs::read_to_string(&state_path).unwrap_or_default();

    for args in [
        ["set", "counter=0"],
        ["set", "counter=-1"],
        ["set", "counter=1"],
    ] {
        Command::cargo_bin("flow")
            .unwrap()
            .current_dir(repo.path())
            .args(args)
            .assert()
            .failure();
        let after = std::fs::read_to_string(&state_path).unwrap_or_default();
        assert_eq!(after, before);
    }
}

#[test]
fn t002_t005_flow_set_rejects_invalid_settings_without_changes() {
    // Covers: T-002, T-005.
    let repo = make_repo();
    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .args(["init"])
        .assert()
        .success();
    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .args(["set", "confirmation=no"])
        .assert()
        .success();

    let config_path = repo.path().join(".flow").join("config.yaml");
    let state_path = repo.path().join(".flow").join("state.yaml");
    let before_config = std::fs::read_to_string(&config_path).unwrap();
    let before_state = std::fs::read_to_string(&state_path).unwrap();

    for args in [
        ["set", "confirmation=maybe"],
        ["set", "unknown=yes"],
        ["set", "confirmation"],
    ] {
        Command::cargo_bin("flow")
            .unwrap()
            .current_dir(repo.path())
            .args(args)
            .assert()
            .failure();
        assert_eq!(
            std::fs::read_to_string(&config_path).unwrap(),
            before_config
        );
        assert_eq!(std::fs::read_to_string(&state_path).unwrap(), before_state);
    }
}

#[test]
fn t003_confirmation_no_suppresses_protected_branch_prompt() {
    // Covers: T-003.
    let repo = make_repo();
    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .args(["init"])
        .assert()
        .success();
    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .args(["set", "confirmation=no"])
        .assert()
        .success();

    let out = Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .args(["start", "demo"])
        .output()
        .unwrap();

    assert!(out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!stderr.contains("Continue? Type `y` or `yes`"));
    assert!(!stderr.contains("stdin is not a TTY"));
}

#[test]
fn t003_t005_confirmation_no_suppresses_finalize_confirmation_text() {
    // Covers: T-003, T-005.
    let repo = make_repo();
    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .args(["init"])
        .assert()
        .success();
    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .args(["set", "confirmation=no"])
        .assert()
        .success();
    let feature_dir = seed_building_feature(
        repo.path(),
        "demo",
        "## Tasks\n\n- [ ] **T-001**: Do it.\n  - Covers: FR-001\n  - Verifies: SC-001\n  - Depends-On: (none)\n",
    );

    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .env("FLOW_CHANGE_DIR", &feature_dir)
        .args(["build"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Ask the user to reply `yes` or `y`").not())
        .stdout(predicate::str::contains("**Confirmation**: disabled"));
}

#[test]
fn t006_default_confirmation_no_suppresses_protected_branch_prompt() {
    // Covers: T-006.
    let repo = make_repo();
    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .args(["init"])
        .assert()
        .success();

    let out = Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .args(["start", "demo"])
        .output()
        .unwrap();

    assert!(out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("protected branch 'main'"));
    assert!(stderr.contains("confirmation=no"));
    assert!(!stderr.contains("stdin is not a TTY"));
}

#[test]
fn t007_t008_flow_settings_lists_defaults_and_saved_values() {
    // Covers: T-007, T-008.
    let repo = make_repo();
    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .args(["init"])
        .assert()
        .success();

    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .args(["settings"])
        .assert()
        .success()
        .stdout(predicate::str::contains("confirmation=no"));

    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .args(["set", "confirmation=yes"])
        .assert()
        .success();

    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .args(["settings"])
        .assert()
        .success()
        .stdout(predicate::str::contains("confirmation=yes"));
}

// ---------------------------------------------------------------------------
// Regressions discovered during the plan audit.
// ---------------------------------------------------------------------------

fn seed_ready_to_close_feature(repo: &std::path::Path, slug: &str) -> std::path::PathBuf {
    let feature_dir = canonical_change_dir(repo, slug);
    std::fs::write(
        feature_dir.join("spec.md"),
        "## What & Why\n\nReady to close.\n\n### Functional Requirements\n- **FR-001**: One.\n\n### Measurable Outcomes\n- **SC-001**: One.\n",
    )
    .unwrap();
    std::fs::write(
        feature_dir.join("plan.md"),
        "## Summary\n\nReady.\n\n## Technical Context\n\n**Language/Version**: Rust\n\n## Documentation Impact\n\nImpact: none\n\nDocs already current because this fixture does not change current Flow documentation.\n",
    )
    .unwrap();
    std::fs::write(
        feature_dir.join("tasks.md"),
        "## Tasks\n\n- [x] **T-001**: Done.\n    - Covers: FR-001\n    - Verifies: SC-001\n",
    )
    .unwrap();
    if !feature_dir.join("status.md").exists() {
        std::fs::write(
            feature_dir.join("status.md"),
            format!(
                "# Status: {slug}\n\n**Change**: {slug}\n**Started**: 2026-05-09\n**Updated**: 2026-05-09T00:00:00Z\n**State**: building\n**Branch**: flow/{slug}\n\n## History\n\n- 2026-05-09T00:00:00Z — plan-complete — plan finalized\n"
            ),
        )
        .unwrap();
    }
    feature_dir
}

fn seed_root_version_files(repo: &std::path::Path, version: &str) {
    std::fs::write(
        repo.join("Cargo.toml"),
        format!(
            "[workspace]\nmembers = []\n\n[workspace.package]\nversion = \"{version}\"\nedition = \"2021\"\n"
        ),
    )
    .unwrap();
}

/// G2 — `flow close` refuses until `build-complete` is stamped.
#[test]
fn close_requires_build_complete_in_history() {
    let repo = make_repo();
    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .args(["init"])
        .assert()
        .success();
    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .env("FLOW_FORCE_ON_PROTECTED", "1")
        .args(["start", "close-guard"])
        .assert()
        .success();
    let feature_dir = seed_ready_to_close_feature(repo.path(), "close-guard");
    // status.md was seeded by /flow-start with only a `started` entry; no build-complete yet.
    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .args(["close"])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "Build verification is not complete",
        ));
    // --finalize should also refuse.
    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .env("FLOW_CHANGE_DIR", feature_dir.to_str().unwrap())
        .args(["close", "--finalize"])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "Build verification is not complete",
        ));
}

#[test]
fn close_in_place_without_version_commit_tag_or_staging() {
    let repo = make_repo();
    let root = repo.path();
    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(root)
        .args(["init"])
        .assert()
        .success();
    seed_root_version_files(root, "1.2.3");
    std::fs::create_dir_all(root.join("backend")).unwrap();
    std::fs::write(
        root.join("backend/Cargo.toml"),
        "[package]\nname = \"backend\"\nversion = \"9.9.9\"\nedition = \"2021\"\n",
    )
    .unwrap();
    std::fs::write(root.join("backend/Cargo.lock"), "# backend lock\n").unwrap();
    let root_cargo_before = std::fs::read_to_string(root.join("Cargo.toml")).unwrap();
    let backend_cargo_before = std::fs::read_to_string(root.join("backend/Cargo.toml")).unwrap();
    let backend_lock_before = std::fs::read_to_string(root.join("backend/Cargo.lock")).unwrap();
    let feature_dir = seed_ready_to_close_feature(root, "close-command");
    let run_dir = feature_dir
        .parent()
        .and_then(|p| p.parent())
        .expect("feature should live under a run");
    let roadmap_text = "# Roadmap\n\n## Milestones\n\n### [~] M-1: Close command\n\nBody.\n";
    std::fs::write(run_dir.join("roadmap.md"), roadmap_text).unwrap();
    let run_path = run_dir.join("run.md");
    let run_state = std::fs::read_to_string(&run_path).unwrap();
    std::fs::write(
        &run_path,
        run_state
            .replace("**Run type**: one-off", "**Run type**: roadmap")
            .replace(
                "**Roadmap fingerprint**: (none)",
                &format!(
                    "**Roadmap fingerprint**: {}",
                    flow_core::roadmap::fingerprint(roadmap_text)
                ),
            ),
    )
    .unwrap();
    std::fs::write(
        feature_dir.join("status.md"),
        "# Status: close-command\n\n**Change**: close-command\n**Started**: 2026-05-09\n**Updated**: 2026-05-09T00:00:00Z\n**State**: building\n**Branch**: flow/close-command\n**Milestone**: M-1\n\n## History\n\n- 2026-05-09T00:00:00Z — build-complete — ok\n",
    )
    .unwrap();
    std::fs::write(root.join("UNRELATED.txt"), "do not commit me\n").unwrap();
    let head_before = std::process::Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(root)
        .output()
        .unwrap()
        .stdout;

    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(root)
        .env("FLOW_CHANGE_DIR", feature_dir.to_str().unwrap())
        .args(["close", "--finalize"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Change 'close-command' closed."))
        .stdout(predicate::str::contains("Local tag").not());

    assert_eq!(
        std::fs::read_to_string(root.join("Cargo.toml")).unwrap(),
        root_cargo_before
    );
    assert_eq!(
        std::fs::read_to_string(root.join("backend/Cargo.toml")).unwrap(),
        backend_cargo_before
    );
    assert_eq!(
        std::fs::read_to_string(root.join("backend/Cargo.lock")).unwrap(),
        backend_lock_before
    );
    let roadmap = std::fs::read_to_string(run_dir.join("roadmap.md")).unwrap();
    assert!(roadmap.contains("### [x] M-1: Close command"), "{roadmap}");
    assert!(roadmap.contains("Body."), "{roadmap}");

    let head_after = std::process::Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(root)
        .output()
        .unwrap();
    assert_eq!(
        head_after.stdout, head_before,
        "close must not create commits"
    );
    let tags = std::process::Command::new("git")
        .args(["tag", "-l"])
        .current_dir(root)
        .output()
        .unwrap();
    assert!(
        String::from_utf8_lossy(&tags.stdout).trim().is_empty(),
        "close must not create tags"
    );
    let staged = std::process::Command::new("git")
        .args(["diff", "--cached", "--name-only"])
        .current_dir(root)
        .output()
        .unwrap();
    assert!(
        String::from_utf8_lossy(&staged.stdout).trim().is_empty(),
        "close must not stage files"
    );

    assert!(!root.join("flow/archive").exists());
    let spec_after = std::fs::read_to_string(feature_dir.join("spec.md")).unwrap();
    assert!(spec_after.contains("**Closed**:"));
    let status_after = std::fs::read_to_string(feature_dir.join("status.md")).unwrap();
    assert!(status_after.contains("**State**: closed"));
    assert!(status_after.contains("change closed"));
}

#[test]
fn close_succeeds_without_root_cargo_toml() {
    let repo = make_repo();
    let root = repo.path();
    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(root)
        .args(["init"])
        .assert()
        .success();
    let feature_dir = seed_ready_to_close_feature(root, "no-cargo");
    std::fs::write(
        feature_dir.join("status.md"),
        "# Status: no-cargo\n\n**Change**: no-cargo\n**Started**: 2026-05-09\n**Updated**: 2026-05-09T00:00:00Z\n**State**: building\n**Branch**: flow/no-cargo\n\n## History\n\n- 2026-05-09T00:00:00Z — build-complete — ok\n",
    )
    .unwrap();

    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(root)
        .env("FLOW_CHANGE_DIR", feature_dir.to_str().unwrap())
        .args(["close", "--finalize"])
        .assert()
        .success()
        .stdout(predicate::str::contains("closed"));

    assert!(!root.join("Cargo.toml").exists());
}

#[test]
fn t001_doctor_rejects_non_current_layout_version() {
    let repo = make_repo();
    let root = repo.path();
    std::fs::create_dir_all(root.join(".flow/agents")).unwrap();
    std::fs::write(root.join(".flow/version"), env!("CARGO_PKG_VERSION")).unwrap();
    std::fs::write(
        root.join(".flow/config.yaml"),
        "schema_version: 1.0\nprefix: flow\nhosts: []\nlayout:\n  version: 1\n",
    )
    .unwrap();
    std::fs::write(root.join("AGENTS.md"), "# Agents\n").unwrap();

    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(root)
        .args(["doctor"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("layout.version must be 2"));
}

#[test]
fn t026_update_removes_generated_convention_copies() {
    let repo = make_repo();
    let root = repo.path();
    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(root)
        .args(["init"])
        .assert()
        .success();
    let config_path = root.join(".flow/config.yaml");
    let config = std::fs::read_to_string(&config_path).unwrap();
    std::fs::write(
        &config_path,
        config.replace("prefix: flow", "prefix: product"),
    )
    .unwrap();

    std::fs::create_dir_all(root.join(".flow/conventions")).unwrap();
    std::fs::write(
        root.join(".flow/conventions/core.md"),
        assets::conventions_shard("core").unwrap(),
    )
    .unwrap();

    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(root)
        .args(["update"])
        .assert()
        .success();

    assert!(!root.join(".flow/conventions/core.md").is_file());
}

#[test]
fn removed_release_commands_are_unknown() {
    for command in ["ship", "release-patch", "release-minor", "release-major"] {
        Command::cargo_bin("flow")
            .unwrap()
            .args([command])
            .assert()
            .failure()
            .stderr(predicate::str::contains("unrecognized subcommand"));
    }
}

/// G3 — when all tasks are already complete, `flow build-task --finalize`
/// chains to verification and stamps `build-complete`.
#[test]
fn build_task_finalize_chains_to_test_when_all_tasks_done() {
    let repo = make_repo();
    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .args(["init"])
        .assert()
        .success();
    let feature_dir = seed_ready_to_close_feature(repo.path(), "bt-stamp");
    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .env("FLOW_CHANGE_DIR", feature_dir.to_str().unwrap())
        .args(["build-task", "--finalize"])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Verification complete. Build phase closed.",
        ))
        .stdout(predicate::str::contains("Next command: `flow close`"));
    let status = std::fs::read_to_string(feature_dir.join("status.md")).unwrap();
    assert!(status.contains("task-complete"));
    assert!(status.contains("build-complete — verification passed"));
}

/// G3b — partial `flow build-task --finalize` stamps `task-complete` only.
#[test]
fn build_task_finalize_does_not_chain_when_tasks_remain() {
    let repo = make_repo();
    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .args(["init"])
        .assert()
        .success();
    let feature_dir = seed_building_feature(
        repo.path(),
        "bt-partial",
        "# Tasks\n\n## Tasks\n\n- [ ] **T-001**: First.\n    - Covers: FR-001\n    - Verifies: SC-001\n    - Depends-On: (none)\n\n- [ ] **T-002**: Second.\n    - Covers: FR-001\n    - Verifies: SC-001\n    - Depends-On: T-001\n",
    );
    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .env("FLOW_CHANGE_DIR", feature_dir.to_str().unwrap())
        .args(["build-task", "T-001", "--finalize"])
        .assert()
        .success()
        .stdout(predicate::str::contains("flow build-task"));
    let status = std::fs::read_to_string(feature_dir.join("status.md")).unwrap();
    assert!(status.contains("task-complete — T-001 task state saved"));
    assert!(!status.contains("build-complete"));
}

#[test]
fn t006_flow_test_reports_no_configured_test_command_but_can_finalize() {
    let repo = make_repo();
    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .args(["init"])
        .assert()
        .success();
    let feature_dir = seed_building_feature(
        repo.path(),
        "no-test-command",
        "# Tasks\n\n## Tasks\n\n- [x] **T-001**: Implement checked task.\n    - Covers: FR-001\n    - Verifies: SC-001\n    - Depends-On: (none)\n",
    );

    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .env("FLOW_CHANGE_DIR", &feature_dir)
        .args(["test"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Detected test runner"))
        .stdout(predicate::str::contains(
            "No configured or auto-detected test runner",
        ))
        .stdout(predicate::str::contains("Tests: NOT RUN"));

    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .env("FLOW_CHANGE_DIR", feature_dir.to_str().unwrap())
        .args(["test", "--finalize"])
        .assert()
        .success()
        .stdout(predicate::str::contains("flow close"));
    let status = std::fs::read_to_string(feature_dir.join("status.md")).unwrap();
    assert!(status.contains("build-complete"));
}

#[test]
fn t006_flow_test_auto_detects_cargo_test_runner() {
    let repo = make_repo();
    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .args(["init"])
        .assert()
        .success();
    std::fs::create_dir_all(repo.path().join("src")).unwrap();
    std::fs::create_dir_all(repo.path().join("tests")).unwrap();
    std::fs::write(
        repo.path().join("Cargo.toml"),
        "[package]\nname = \"flow-test-fixture\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    )
    .unwrap();
    std::fs::write(
        repo.path().join("src/lib.rs"),
        "pub fn fixture() -> bool { true }\n",
    )
    .unwrap();
    std::fs::write(
        repo.path().join("tests/t001.rs"),
        "#[test]\nfn t001_auto_detected_runner() {\n    // T-001\n    assert!(flow_test_fixture::fixture());\n}\n",
    )
    .unwrap();
    let feature_dir = seed_building_feature(
        repo.path(),
        "cargo-test-command",
        "# Tasks\n\n## Tasks\n\n- [x] **T-001**: Implement checked task.\n    - Covers: FR-001\n    - Verifies: SC-001\n    - Depends-On: (none)\n",
    );

    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .env("FLOW_CHANGE_DIR", &feature_dir)
        .args(["test"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Detected test runner"))
        .stdout(predicate::str::contains("`cargo test --workspace`"))
        .stdout(predicate::str::contains("Tests: PASS"));
}

#[test]
fn flow_test_refuses_finalize_hint_when_runner_fails() {
    let repo = make_repo();
    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .args(["init"])
        .assert()
        .success();
    std::fs::write(
        repo.path().join(".flow").join("config.yaml"),
        "test:\n  command: \"exit 1\"\n",
    )
    .unwrap();
    let feature_dir = seed_building_feature(
        repo.path(),
        "failing-test-command",
        "# Tasks\n\n## Tasks\n\n- [x] **T-001**: Implement checked task.\n    - Covers: FR-001\n    - Verifies: SC-001\n    - Depends-On: (none)\n",
    );

    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .env("FLOW_CHANGE_DIR", &feature_dir)
        .args(["test"])
        .assert()
        .failure()
        .stdout(predicate::str::contains("Tests: FAIL"))
        .stdout(predicate::str::contains("Finalization Instructions").not())
        .stdout(predicate::str::contains("Next command: `flow test`"))
        .stderr(predicate::str::contains("Verification failed"));

    let status = std::fs::read_to_string(feature_dir.join("status.md")).unwrap();
    assert!(!status.contains("build-complete"));
}

#[test]
fn build_task_finalize_marks_selected_task_and_records_task_id() {
    let repo = make_repo();
    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .args(["init"])
        .assert()
        .success();
    let feature_dir = seed_building_feature(
        repo.path(),
        "accepted-task",
        "# Tasks\n\n## Tasks\n\n- [ ] **T-001**: Implement first task.\n    - Covers: FR-001\n    - Verifies: SC-001\n    - Depends-On: (none)\n\n- [ ] **T-002**: Implement second task.\n    - Covers: FR-001\n    - Verifies: SC-001\n    - Depends-On: T-001\n",
    );

    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .env("FLOW_CHANGE_DIR", feature_dir.to_str().unwrap())
        .args(["build-task", "T-001", "--finalize"])
        .assert()
        .success()
        .stdout(predicate::str::contains("flow build-task"));

    let tasks = std::fs::read_to_string(feature_dir.join("tasks.md")).unwrap();
    assert!(tasks.contains("- [x] **T-001**"));
    assert!(tasks.contains("- [ ] **T-002**"));
    let status = std::fs::read_to_string(feature_dir.join("status.md")).unwrap();
    assert!(status.contains("task-complete — T-001 task state saved"));
}

#[test]
fn build_finalize_marks_completed_ids_and_records_them() {
    let repo = make_repo();
    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .args(["init"])
        .assert()
        .success();
    let feature_dir = seed_building_feature(
        repo.path(),
        "accepted-build",
        "# Tasks\n\n## Tasks\n\n- [ ] **T-001**: Implement first task.\n    - Covers: FR-001\n    - Verifies: SC-001\n    - Depends-On: (none)\n\n- [ ] **T-002**: Implement second task.\n    - Covers: FR-001\n    - Verifies: SC-001\n    - Depends-On: T-001\n",
    );

    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .env("FLOW_CHANGE_DIR", feature_dir.to_str().unwrap())
        .args(["build", "--completed", "T-001", "--finalize"])
        .assert()
        .success()
        .stdout(predicate::str::contains("flow build-task"));

    let tasks = std::fs::read_to_string(feature_dir.join("tasks.md")).unwrap();
    assert!(tasks.contains("- [x] **T-001**"));
    assert!(tasks.contains("- [ ] **T-002**"));
    let status = std::fs::read_to_string(feature_dir.join("status.md")).unwrap();
    assert!(status.contains("build-progress — T-001 build progress saved"));
}

#[test]
fn build_finalize_runs_verification_and_stamps_complete_when_final_tests_pass() {
    let repo = make_repo();
    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .args(["init"])
        .assert()
        .success();
    std::fs::write(
        repo.path().join(".flow").join("config.yaml"),
        "test:\n  command: \"test -f pass-marker\"\n",
    )
    .unwrap();
    std::fs::write(repo.path().join("pass-marker"), "").unwrap();
    let feature_dir = seed_building_feature(
        repo.path(),
        "accepted-build-final",
        "# Tasks\n\n## Tasks\n\n- [ ] **T-001**: Implement first task.\n    - Covers: FR-001\n    - Verifies: SC-001\n    - Depends-On: (none)\n",
    );

    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .env("FLOW_CHANGE_DIR", feature_dir.to_str().unwrap())
        .args(["build", "--completed", "T-001", "--finalize"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Tests: PASS"))
        .stdout(predicate::str::contains(
            "Verification complete. Build phase closed.",
        ))
        .stdout(predicate::str::contains("Next command: `flow close`"));

    let tasks = std::fs::read_to_string(feature_dir.join("tasks.md")).unwrap();
    assert!(tasks.contains("- [x] **T-001**"));
    let status = std::fs::read_to_string(feature_dir.join("status.md")).unwrap();
    assert!(status.contains("build-progress — T-001 build progress saved; all tasks implemented"));
    assert!(status.contains("build-complete — verification passed"));
}

#[test]
fn build_finalize_keeps_build_open_when_final_tests_fail() {
    let repo = make_repo();
    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .args(["init"])
        .assert()
        .success();
    std::fs::write(
        repo.path().join(".flow").join("config.yaml"),
        "test:\n  command: \"exit 1\"\n",
    )
    .unwrap();
    let feature_dir = seed_building_feature(
        repo.path(),
        "accepted-build-failing-tests",
        "# Tasks\n\n## Tasks\n\n- [ ] **T-001**: Implement first task.\n    - Covers: FR-001\n    - Verifies: SC-001\n    - Depends-On: (none)\n",
    );

    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .env("FLOW_CHANGE_DIR", feature_dir.to_str().unwrap())
        .args(["build", "--completed", "T-001", "--finalize"])
        .assert()
        .failure()
        .stdout(predicate::str::contains("Tests: FAIL"))
        .stdout(predicate::str::contains("Next command: `flow test`"))
        .stderr(predicate::str::contains("Verification failed"));

    let tasks = std::fs::read_to_string(feature_dir.join("tasks.md")).unwrap();
    assert!(tasks.contains("- [x] **T-001**"));
    let status = std::fs::read_to_string(feature_dir.join("status.md")).unwrap();
    assert!(status.contains("build-progress — T-001 build progress saved; all tasks implemented"));
    assert!(!status.contains("build-complete"));
}

#[test]
fn build_finalize_promotes_awaiting_acceptance_tasks() {
    let repo = make_repo();
    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .args(["init"])
        .assert()
        .success();
    let feature_dir = seed_building_feature(
        repo.path(),
        "awaiting-build",
        "# Tasks\n\n## Tasks\n\n- [~] **T-001**: Implement first task.\n    - Covers: FR-001\n    - Verifies: SC-001\n    - Depends-On: (none)\n\n- [ ] **T-002**: Implement second task.\n    - Covers: FR-001\n    - Verifies: SC-001\n    - Depends-On: T-001\n",
    );

    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .env("FLOW_CHANGE_DIR", feature_dir.to_str().unwrap())
        .args(["build", "--completed", "T-001", "--finalize"])
        .assert()
        .success()
        .stdout(predicate::str::contains("flow build-task"));

    let tasks = std::fs::read_to_string(feature_dir.join("tasks.md")).unwrap();
    assert!(tasks.contains("- [x] **T-001**"));
    assert!(tasks.contains("- [ ] **T-002**"));
}

#[test]
fn build_pauses_when_tasks_await_acceptance() {
    let repo = make_repo();
    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .args(["init"])
        .assert()
        .success();
    let feature_dir = seed_building_feature(
        repo.path(),
        "awaiting-queue",
        "# Tasks\n\n## Tasks\n\n- [~] **T-001**: Implement first task.\n    - Covers: FR-001\n    - Verifies: SC-001\n    - Depends-On: (none)\n\n- [ ] **T-002**: Implement second task.\n    - Covers: FR-001\n    - Verifies: SC-001\n    - Depends-On: T-001\n",
    );

    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .env("FLOW_CHANGE_DIR", &feature_dir)
        .args(["build"])
        .assert()
        .success()
        .stdout(predicate::str::contains("awaiting user acceptance"))
        .stdout(predicate::str::contains("flow build --finalize"))
        .stdout(predicate::str::contains("--completed T-001").not())
        .stdout(predicate::str::contains("--completed T-002").not());
}

#[test]
fn build_task_reports_awaiting_acceptance_without_requeueing() {
    let repo = make_repo();
    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .args(["init"])
        .assert()
        .success();
    let feature_dir = seed_building_feature(
        repo.path(),
        "awaiting-task",
        "# Tasks\n\n## Tasks\n\n- [~] **T-001**: Implement first task.\n    - Covers: FR-001\n    - Verifies: SC-001\n    - Depends-On: (none)\n\n- [ ] **T-002**: Implement second task.\n    - Covers: FR-001\n    - Verifies: SC-001\n    - Depends-On: T-001\n",
    );

    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .env("FLOW_CHANGE_DIR", &feature_dir)
        .args(["build-task"])
        .assert()
        .success()
        .stdout(predicate::str::contains("## Active Task"))
        .stdout(predicate::str::contains("flow build-task T-001 --finalize"));
}

#[test]
fn test_refuses_tasks_awaiting_acceptance() {
    let repo = make_repo();
    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .args(["init"])
        .assert()
        .success();
    let feature_dir = seed_building_feature(
        repo.path(),
        "awaiting-test",
        "# Tasks\n\n## Tasks\n\n- [~] **T-001**: Implement first task.\n    - Covers: FR-001\n    - Verifies: SC-001\n    - Depends-On: (none)\n",
    );

    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .env("FLOW_CHANGE_DIR", feature_dir.to_str().unwrap())
        .args(["test", "--finalize"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("awaiting user acceptance"));
}

#[test]
fn build_envelope_finalize_command_names_queued_tasks() {
    let repo = make_repo();
    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .args(["init"])
        .assert()
        .success();
    let feature_dir = seed_building_feature(
        repo.path(),
        "queued-build",
        "# Tasks\n\n## Tasks\n\n- [ ] **T-001**: Implement first task.\n    - Covers: FR-001\n    - Verifies: SC-001\n    - Depends-On: (none)\n\n- [ ] **T-002**: Implement second task.\n    - Covers: FR-001\n    - Verifies: SC-001\n    - Depends-On: T-001\n",
    );

    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .env("FLOW_CHANGE_DIR", &feature_dir)
        .args(["build"])
        .assert()
        .success()
        .stdout(predicate::str::contains("flow build --finalize"))
        .stdout(predicate::str::contains("--completed").not());
}

/// M-22 (T-006): the printed `flow build` finalize footer is the same
/// stable string regardless of which task IDs are queued; no `--completed`
/// chain leaks into the printed footer.
#[test]
fn t007_build_finalize_footer_is_stable_across_rounds() {
    let repo = make_repo();
    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .args(["init"])
        .assert()
        .success();

    // Round 1: queue T-001 only (T-002 is dependency-blocked).
    let feature_dir = seed_building_feature(
        repo.path(),
        "stable-footer-r1",
        "# Tasks\n\n## Tasks\n\n- [ ] **T-001**: Implement first task.\n    - Covers: FR-001\n    - Verifies: SC-001\n    - Depends-On: (none)\n\n- [ ] **T-002**: Implement second task.\n    - Covers: FR-001\n    - Verifies: SC-001\n    - Depends-On: T-001\n",
    );
    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .env("FLOW_CHANGE_DIR", &feature_dir)
        .args(["build"])
        .assert()
        .success()
        .stdout(predicate::str::contains("flow build --finalize"))
        .stdout(predicate::str::contains("--completed").not());

    // Round 2: a fresh feature with two top-level tasks queued together.
    let feature_dir2 = seed_building_feature(
        repo.path(),
        "stable-footer-r2",
        "# Tasks\n\n## Tasks\n\n- [ ] **T-001**: A.\n    - Covers: FR-001\n    - Verifies: SC-001\n    - Depends-On: (none)\n\n- [ ] **T-002**: B.\n    - Covers: FR-001\n    - Verifies: SC-001\n    - Depends-On: (none)\n",
    );
    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .env("FLOW_CHANGE_DIR", &feature_dir2)
        .args(["build"])
        .assert()
        .success()
        .stdout(predicate::str::contains("flow build --finalize"))
        .stdout(predicate::str::contains("--completed").not());
}

/// M-22 (T-006): `flow build` prepare writes the pending-completion queue
/// to per-change state at `<change_dir>/.flow/build-pending.yaml`.
#[test]
fn t007_build_prepare_writes_pending_state_file() {
    let repo = make_repo();
    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .args(["init"])
        .assert()
        .success();
    let feature_dir = seed_building_feature(
        repo.path(),
        "build-pending-write",
        "# Tasks\n\n## Tasks\n\n- [ ] **T-001**: A.\n    - Covers: FR-001\n    - Verifies: SC-001\n    - Depends-On: (none)\n",
    );
    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .env("FLOW_CHANGE_DIR", &feature_dir)
        .args(["build"])
        .assert()
        .success();
    let pending_path = feature_dir.join(".flow").join("build-pending.yaml");
    assert!(
        pending_path.is_file(),
        "expected build-pending.yaml at {}",
        pending_path.display()
    );
    let body = std::fs::read_to_string(&pending_path).unwrap();
    assert!(
        body.contains("schema_version") && body.contains("pending") && body.contains("T-001"),
        "build-pending.yaml does not contain expected schema/pending/T-001: {body:?}"
    );
}

/// M-22 (T-006): a stale `<change_dir>/.flow/build-pending.yaml` left
/// behind by an interrupted prior run is overwritten with the new queue at
/// the next `flow build` prepare; a clear warning is emitted.
#[test]
fn t007_build_pending_stale_state_is_cleared_with_warning() {
    let repo = make_repo();
    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .args(["init"])
        .assert()
        .success();
    let feature_dir = seed_building_feature(
        repo.path(),
        "build-pending-stale",
        "# Tasks\n\n## Tasks\n\n- [ ] **T-001**: A.\n    - Covers: FR-001\n    - Verifies: SC-001\n    - Depends-On: (none)\n",
    );
    let pending_path = feature_dir.join(".flow").join("build-pending.yaml");
    std::fs::create_dir_all(pending_path.parent().unwrap()).unwrap();
    std::fs::write(
        &pending_path,
        "schema_version: 1\npending:\n- T-099\n- T-098\n",
    )
    .unwrap();
    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .env("FLOW_CHANGE_DIR", &feature_dir)
        .args(["build"])
        .assert()
        .success()
        .stderr(
            predicate::str::contains("Stale build-pending state")
                .and(predicate::str::contains("T-099"))
                .and(predicate::str::contains("T-098")),
        );
    let body = std::fs::read_to_string(&pending_path).unwrap();
    assert!(
        body.contains("T-001") && !body.contains("T-099") && !body.contains("T-098"),
        "stale state was not overwritten: {body:?}"
    );
}

/// M-22 (T-006): the `--completed T-NNN` flag remains a working
/// scripted-override path. After `flow build --completed T-001 --finalize`,
/// T-001 is marked done and the pending state file is cleared.
#[test]
fn t007_build_completed_override_still_works_with_finalize() {
    let repo = make_repo();
    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .args(["init"])
        .assert()
        .success();
    let feature_dir = seed_building_feature(
        repo.path(),
        "build-completed-override",
        "# Tasks\n\n## Tasks\n\n- [ ] **T-001**: A.\n    - Covers: FR-001\n    - Verifies: SC-001\n    - Depends-On: (none)\n",
    );
    // Pre-create stale state to verify the override still clears it.
    let pending_path = feature_dir.join(".flow").join("build-pending.yaml");
    std::fs::create_dir_all(pending_path.parent().unwrap()).unwrap();
    std::fs::write(&pending_path, "schema_version: 1\npending:\n- T-099\n").unwrap();
    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .env("FLOW_CHANGE_DIR", &feature_dir)
        .args(["build", "--completed", "T-001", "--finalize"])
        .assert()
        .success();
    assert!(
        !pending_path.exists(),
        "build-pending.yaml should be cleared after finalize"
    );
    let tasks_body = std::fs::read_to_string(feature_dir.join("tasks.md")).unwrap();
    assert!(
        tasks_body.contains("[~] **T-001**") || tasks_body.contains("[x] **T-001**"),
        "T-001 should be marked done or awaiting acceptance after override finalize: {tasks_body}"
    );
}

#[test]
fn build_refuses_when_open_tasks_are_dependency_blocked() {
    let repo = make_repo();
    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .args(["init"])
        .assert()
        .success();
    let feature_dir = seed_building_feature(
        repo.path(),
        "blocked-by-dependency-build",
        "# Tasks\n\n## Tasks\n\n- [ ] **T-002**: Implement second task.\n    - Covers: FR-001\n    - Verifies: SC-001\n    - Depends-On: T-001\n",
    );

    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .env("FLOW_CHANGE_DIR", &feature_dir)
        .args(["build"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("No runnable open tasks"))
        .stderr(predicate::str::contains("Depends-On"))
        .stdout(predicate::str::contains("All tasks are checked").not())
        .stdout(predicate::str::contains("Finalization Instructions").not());
}

#[test]
fn build_task_refuses_when_open_tasks_are_dependency_blocked() {
    let repo = make_repo();
    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .args(["init"])
        .assert()
        .success();
    let feature_dir = seed_building_feature(
        repo.path(),
        "blocked-by-dependency-task",
        "# Tasks\n\n## Tasks\n\n- [ ] **T-002**: Implement second task.\n    - Covers: FR-001\n    - Verifies: SC-001\n    - Depends-On: T-001\n",
    );

    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .env("FLOW_CHANGE_DIR", &feature_dir)
        .args(["build-task"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("No runnable open tasks"))
        .stderr(predicate::str::contains("Depends-On"))
        .stdout(predicate::str::contains("## Active Task").not())
        .stdout(predicate::str::contains("Finalization Instructions").not());
}

#[test]
fn build_task_blocks_when_selected_task_preflight_fails() {
    let repo = make_repo();
    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .args(["init"])
        .assert()
        .success();
    std::fs::write(
        repo.path().join(".flow").join("config.yaml"),
        "preflight:\n  requirements:\n    localdb:\n      description: Local database is ready\n      command: \"exit 1\"\n      remediation: Start the local database.\n",
    )
    .unwrap();
    let feature_dir = seed_building_feature(
        repo.path(),
        "blocked-task",
        "# Tasks\n\n## Tasks\n\n- [ ] **T-001**: Implement first task.\n    - Covers: FR-001\n    - Verifies: SC-001\n    - Depends-On: (none)\n    - Requires: localdb\n",
    );

    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .env("FLOW_CHANGE_DIR", &feature_dir)
        .args(["build-task"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Blocked by environment"))
        .stdout(predicate::str::contains("localdb"))
        .stdout(predicate::str::contains("T-001"))
        .stdout(predicate::str::contains("Start the local database."))
        .stdout(predicate::str::contains("Finalization Instructions").not())
        .stdout(predicate::str::contains("Next command: `flow build-task`"));
}

#[test]
fn build_blocks_entire_queue_when_later_task_preflight_fails() {
    let repo = make_repo();
    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .args(["init"])
        .assert()
        .success();
    std::fs::write(
        repo.path().join(".flow").join("config.yaml"),
        "preflight:\n  requirements:\n    localdb:\n      description: Local database is ready\n      command: \"exit 1\"\n      remediation: Start the local database.\n",
    )
    .unwrap();
    let feature_dir = seed_building_feature(
        repo.path(),
        "blocked-queue",
        "# Tasks\n\n## Tasks\n\n- [ ] **T-001**: Implement first task.\n    - Covers: FR-001\n    - Verifies: SC-001\n    - Depends-On: (none)\n\n- [ ] **T-002**: Implement second task.\n    - Covers: FR-001\n    - Verifies: SC-001\n    - Depends-On: T-001\n    - Requires: localdb\n",
    );

    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .env("FLOW_CHANGE_DIR", &feature_dir)
        .args(["build"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Blocked by environment"))
        .stdout(predicate::str::contains("T-002"))
        .stdout(predicate::str::contains("Finalization Instructions").not())
        .stdout(predicate::str::contains("Next command: `flow build`"));
}

#[test]
fn build_task_without_requires_ignores_failing_preflight_config() {
    let repo = make_repo();
    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .args(["init"])
        .assert()
        .success();
    std::fs::write(
        repo.path().join(".flow").join("config.yaml"),
        "preflight:\n  requirements:\n    localdb:\n      description: Local database is ready\n      command: \"exit 1\"\n      remediation: Start the local database.\n",
    )
    .unwrap();
    let feature_dir = seed_building_feature(
        repo.path(),
        "unblocked-task",
        "# Tasks\n\n## Tasks\n\n- [ ] **T-001**: Implement first task.\n    - Covers: FR-001\n    - Verifies: SC-001\n    - Depends-On: (none)\n",
    );

    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .env("FLOW_CHANGE_DIR", &feature_dir)
        .args(["build-task"])
        .assert()
        .success()
        .stdout(predicate::str::contains("failed for T-001").not())
        .stdout(predicate::str::contains("**Review**: collapsed"))
        .stdout(predicate::str::contains(
            "**Save state with**: `flow build-task T-001 --finalize`",
        ));
}

#[test]
fn plan_finalize_rejects_unknown_preflight_requirement() {
    let repo = make_repo();
    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .args(["init"])
        .assert()
        .success();
    let feature_dir = seed_building_feature(
        repo.path(),
        "unknown-requirement",
        "# Tasks\n\n## Tasks\n\n- [ ] **T-001**: Implement first task.\n    - Covers: FR-001\n    - Verifies: SC-001\n    - Depends-On: (none)\n    - Requires: mystery\n",
    );

    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .env("FLOW_CHANGE_DIR", feature_dir.to_str().unwrap())
        .args(["plan", "--finalize"])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "unknown preflight requirement 'mystery'",
        ));
}

#[test]
fn build_task_finalize_does_not_record_task_totals() {
    let repo = make_repo();
    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .args(["init"])
        .assert()
        .success();
    let feature_dir = seed_building_feature(
        repo.path(),
        "countless-task",
        "# Tasks\n\n## Tasks\n\n- [x] **T-001**: Implement first task.\n    - Covers: FR-001\n    - Verifies: SC-001\n    - Depends-On: (none)\n\n- [ ] **T-002**: Implement second task.\n    - Covers: FR-001\n    - Verifies: SC-001\n    - Depends-On: T-001\n",
    );
    let forbidden = ["task(s)", "remaining"].join(" ");

    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .env("FLOW_CHANGE_DIR", feature_dir.to_str().unwrap())
        .args(["build-task", "T-001", "--finalize"])
        .assert()
        .success()
        .stdout(predicate::str::contains("flow build-task"))
        .stdout(predicate::str::contains(forbidden.clone()).not())
        .stderr(predicate::str::contains(forbidden.clone()).not());

    let status = std::fs::read_to_string(feature_dir.join("status.md")).unwrap();
    assert!(status.contains("task-complete"));
    assert!(status.contains("T-001 task state saved"));
    assert!(
        !status.contains(&forbidden),
        "status.md must not document task totals:\n{status}"
    );
}

#[test]
fn build_finalize_does_not_record_task_totals() {
    let repo = make_repo();
    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .args(["init"])
        .assert()
        .success();
    let feature_dir = seed_building_feature(
        repo.path(),
        "countless-build",
        "# Tasks\n\n## Tasks\n\n- [x] **T-001**: Implement first task.\n    - Covers: FR-001\n    - Verifies: SC-001\n    - Depends-On: (none)\n\n- [ ] **T-002**: Implement second task.\n    - Covers: FR-001\n    - Verifies: SC-001\n    - Depends-On: T-001\n",
    );
    let forbidden = ["task(s)", "remaining"].join(" ");

    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .env("FLOW_CHANGE_DIR", feature_dir.to_str().unwrap())
        .args(["build", "--finalize"])
        .assert()
        .success()
        .stdout(predicate::str::contains("flow build-task"))
        .stdout(predicate::str::contains(forbidden.clone()).not())
        .stderr(predicate::str::contains(forbidden.clone()).not());

    let status = std::fs::read_to_string(feature_dir.join("status.md")).unwrap();
    assert!(status.contains("build-progress"));
    assert!(
        !status.contains(&forbidden),
        "status.md must not document task totals:\n{status}"
    );
}

// T-005: **Capability** field does not trigger drift warnings.
#[test]
fn t005_capability_field_no_drift_warning() {
    let repo = make_repo();
    let feature_dir = repo.path().join("flow").join("cap-test");
    std::fs::create_dir_all(&feature_dir).unwrap();
    // Spec with explicit **Capability** in preamble.
    std::fs::write(
        feature_dir.join("spec.md"),
        "# Spec: cap-test\n\n**Change**: cap-test\n**Capability**: cap-test\n\n## What & Why\n\nBecause.\n\n## Requirements\n\n### Functional Requirements\n\n- **FR-001**: Provide cap-test.\n\n## Success Criteria\n\n### Measurable Outcomes\n\n- **SC-001**: cap-test is available.\n",
    )
    .unwrap();
    std::fs::write(
        feature_dir.join("plan.md"),
        "## Summary\n\nBuild it.\n\n## Technical Context\n\n**Language/Version**: Rust\n\n## Documentation Impact\n\nImpact: none\n\nDocs already current because this fixture verifies metadata parsing only.\n",
    )
    .unwrap();
    std::fs::write(
        feature_dir.join("tasks.md"),
        "## Tasks\n\n- [ ] **T-001**: Do it.\n  - Covers: FR-001\n  - Verifies: SC-001\n",
    )
    .unwrap();
    std::fs::write(
        feature_dir.join("status.md"),
        "# Status: cap-test\n\n**Change**: cap-test\n**Started**: 2026-05-09\n**Updated**: 2026-05-09T00:00:00Z\n**State**: building\n**Branch**: flow/cap-test\n\n## History\n\n- 2026-05-09T00:00:00Z — plan-complete — plan finalized\n",
    )
    .unwrap();
    // `flow status` exercises the drift check path.
    let out = Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .args(["status"])
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    // No D-warning about **Capability** itself.
    assert!(
        !stdout.contains("D-warning: Capability") && !stderr.contains("D-warning: Capability"),
        "Unexpected Capability drift warning.\nstdout: {stdout}\nstderr: {stderr}"
    );
}

// T-006: central documentation evidence close scenarios.

/// Helper: create a closable feature for documentation evidence tests.
fn seed_documented_feature(
    repo: &std::path::Path,
    slug: &str,
    docs_impact: &str,
) -> std::path::PathBuf {
    let feature = slug.to_string();
    let feature_dir = canonical_change_dir(repo, slug);
    std::fs::write(
        feature_dir.join("spec.md"),
        format!(
            "# Spec: {feature}\n\n**Change**: {feature}\n\n## What & Why\n\nReady to close.\n\n## Requirements\n\n### Functional Requirements\n\n- **FR-001**: One.\n\n## Success Criteria\n\n### Measurable Outcomes\n\n- **SC-001**: One.\n"
        ),
    )
    .unwrap();
    std::fs::write(
        feature_dir.join("plan.md"),
        format!(
            "## Summary\n\nReady.\n\n## Technical Context\n\n**Language/Version**: Rust\n\n## Documentation Impact\n\n{docs_impact}\n"
        ),
    )
    .unwrap();
    std::fs::write(
        feature_dir.join("tasks.md"),
        "## Tasks\n\n- [x] **T-001**: Done.\n  - Covers: FR-001\n  - Verifies: SC-001\n",
    )
    .unwrap();
    std::fs::write(
        feature_dir.join("status.md"),
        format!(
            "# Status: {feature}\n\n**Change**: {feature}\n**Started**: 2026-05-09\n**Updated**: 2026-05-09T00:00:00Z\n**State**: building\n**Branch**: flow/{feature}\n\n## History\n\n- 2026-05-09T00:00:00Z — build-complete — ok\n"
        ),
    )
    .unwrap();
    feature_dir
}

/// T-006a / T-002: `Impact: none` plus a docs-current rationale satisfies close evidence.
#[test]
fn t006a_docs_already_current_rationale_allows_close() {
    let repo = make_repo();
    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .args(["init"])
        .assert()
        .success();
    let feature_dir = seed_documented_feature(
        repo.path(),
        "docs-current",
        "Impact: none\n\nDocs already current because this change only adjusts internal path resolution.",
    );
    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .env("FLOW_CHANGE_DIR", feature_dir.to_str().unwrap())
        .args(["close", "--finalize"])
        .assert()
        .success();
    assert!(feature_dir.join("status.md").is_file());
    assert!(!repo.path().join("flow/archive").exists());
}

/// Close finalize removes per-change scratch state: the `.flow-test.last.md`
/// consistency cache and the `<change_dir>/.flow/` build-pending directory
/// only carry state between phases of an open change, so a closed change
/// directory keeps just the durable artifacts.
#[test]
fn close_finalize_removes_per_change_scratch_state() {
    let repo = make_repo();
    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .args(["init"])
        .assert()
        .success();
    let feature_dir = seed_documented_feature(
        repo.path(),
        "scratch-cleanup",
        "Impact: none\n\nDocs already current because this change only adjusts internal path resolution.",
    );
    std::fs::write(
        feature_dir.join(".flow-test.last.md"),
        "## Consistency Check\n\nNo findings.\n",
    )
    .unwrap();
    let pending_path = feature_dir.join(".flow").join("build-pending.yaml");
    std::fs::create_dir_all(pending_path.parent().unwrap()).unwrap();
    std::fs::write(&pending_path, "schema_version: 1\npending:\n- T-099\n").unwrap();
    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .env("FLOW_CHANGE_DIR", feature_dir.to_str().unwrap())
        .args(["close", "--finalize"])
        .assert()
        .success();
    assert!(
        !feature_dir.join(".flow-test.last.md").exists(),
        "consistency cache should be removed at close finalize"
    );
    assert!(
        !feature_dir.join(".flow").exists(),
        "per-change .flow/ scratch directory should be removed at close finalize"
    );
    assert!(feature_dir.join("status.md").is_file());
}

/// T-006b: close refuses when neither docs changed nor plan records rationale.
#[test]
fn t006b_missing_documentation_evidence_blocks_close() {
    let repo = make_repo();
    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .args(["init"])
        .assert()
        .success();
    commit_all(repo.path(), "init flow");
    let feature_dir = seed_documented_feature(
        repo.path(),
        "docs-missing",
        "- `flow/docs/guide.md` must be updated.",
    );
    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .env("FLOW_CHANGE_DIR", feature_dir.to_str().unwrap())
        .args(["close", "--finalize"])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "Central documentation evidence is missing",
        ));
    assert!(feature_dir.is_dir());
}

/// T-006c: changed files under `flow/docs/**` satisfy close evidence.
#[test]
fn t006c_changed_flow_docs_allow_close() {
    let repo = make_repo();
    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .args(["init"])
        .assert()
        .success();
    commit_all(repo.path(), "init flow");
    let feature_dir = seed_documented_feature(
        repo.path(),
        "docs-changed",
        "- `flow/docs/guide.md` must be updated.",
    );
    std::fs::create_dir_all(repo.path().join("flow/docs")).unwrap();
    std::fs::write(
        repo.path().join("flow/docs/guide.md"),
        "# Guide\n\nCurrent behavior.\n",
    )
    .unwrap();
    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .env("FLOW_CHANGE_DIR", feature_dir.to_str().unwrap())
        .args(["close", "--finalize"])
        .assert()
        .success();
    assert!(repo.path().join("flow/docs/guide.md").is_file());
}

/// M-20 closeout invariant: `flow/backlog/may-13-refactoring.md` is gone and
/// `flow/backlog/README.md` still records where the May-13 refactoring backlog
/// landed. Covers T-1, T-2, T-3, T-4.
#[test]
fn m20_may_13_backlog_is_retired_with_closeout_note() {
    // T-1 — this test guards the invariant.
    // T-2 — flow/backlog/may-13-refactoring.md is gone.
    // T-3 — flow/backlog/README.md records the closeout cross-reference.
    // T-4 — workspace green-bar runs this test alongside the rest.
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let repo_root = manifest_dir
        .ancestors()
        .nth(2)
        .expect("crate manifest dir has a workspace root two levels up");
    let stale = repo_root.join("flow/backlog/may-13-refactoring.md");
    assert!(
        !stale.exists(),
        "flow/backlog/may-13-refactoring.md must be retired; the May-13 refactoring backlog landed via roadmap milestones M-1..M-18 plus 2026-05-14 ad-hoc fixes"
    );
    let readme = std::fs::read_to_string(repo_root.join("flow/backlog/README.md"))
        .expect("flow/backlog/README.md must be readable");
    assert!(
        readme.contains("may-13"),
        "flow/backlog/README.md must keep a closeout cross-reference for the May-13 refactoring backlog (look for the substring 'may-13')"
    );
}

/// M-24 (T-007): `flow init` seeds the `review:` block into a fresh
/// `.flow/config.yaml`. The seeded body MUST contain
/// `before_finalize: false` and an explanatory comment about per-command
/// overrides. The agent and downstream tooling rely on the seeded shape.
#[test]
fn t007_review_block_seeded_into_fresh_config_yaml() {
    let repo = make_repo();
    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .args(["init"])
        .assert()
        .success();
    let config = std::fs::read_to_string(repo.path().join(".flow/config.yaml")).unwrap();
    assert!(
        config.contains("review:"),
        "config.yaml should contain a `review:` block; got:\n{config}"
    );
    assert!(
        config.contains("before_finalize: false"),
        "config.yaml `review` block should default to before_finalize: false; got:\n{config}"
    );
    assert!(
        config.contains("per_command"),
        "config.yaml `review` block should document per_command overrides; got:\n{config}"
    );
}

/// M-24 (T-007): `Settings::review_skip_finalize_footer` returns the
/// expected boolean for default and per-command-overridden configs when
/// loaded end-to-end through `flow init` + a manual `review:` edit.
#[test]
fn t007_review_skip_finalize_footer_resolves_through_full_init() {
    let repo = make_repo();
    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .args(["init"])
        .assert()
        .success();
    // Default config: footer is suppressed (collapse on green path).
    let settings = flow_core::settings::Settings::load_for_repo(repo.path()).unwrap();
    assert!(settings.review_skip_finalize_footer("plan"));

    // Override `before_finalize: true` in the seeded config.
    let config_path = repo.path().join(".flow/config.yaml");
    let mut config = std::fs::read_to_string(&config_path).unwrap();
    config = config.replace("before_finalize: false", "before_finalize: true");
    std::fs::write(&config_path, config).unwrap();
    let settings = flow_core::settings::Settings::load_for_repo(repo.path()).unwrap();
    assert!(!settings.review_skip_finalize_footer("plan"));
    assert!(!settings.review_skip_finalize_footer("build"));
}

/// Default `review.before_finalize: false` suppresses the printed finalize footer.
#[test]
fn plan_prepare_suppresses_finalize_footer_by_default() {
    let repo = make_repo();
    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .args(["init"])
        .assert()
        .success();
    let feature_dir = seed_drafting_feature(repo.path(), "review-footer");
    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .env("FLOW_CHANGE_DIR", &feature_dir)
        .args(["plan"])
        .assert()
        .success()
        .stdout(predicate::str::contains("**Review**: collapsed"))
        .stdout(predicate::str::contains("Finalization Instructions").not());
    assert!(feature_dir.join("plan.md").exists());
    assert!(feature_dir.join("tasks.md").exists());
}

/// `review.before_finalize: true` restores the printed finalize footer.
#[test]
fn plan_prepare_emits_finalize_footer_when_review_requires_two_stage() {
    let repo = make_repo();
    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .args(["init"])
        .assert()
        .success();
    let config_path = repo.path().join(".flow/config.yaml");
    let mut config = std::fs::read_to_string(&config_path).unwrap();
    config = config.replace("before_finalize: false", "before_finalize: true");
    std::fs::write(&config_path, config).unwrap();
    let feature_dir = seed_drafting_feature(repo.path(), "review-two-stage");
    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .env("FLOW_CHANGE_DIR", &feature_dir)
        .args(["plan"])
        .assert()
        .success()
        .stdout(predicate::str::contains("**Review**: two-stage"))
        .stdout(predicate::str::contains("Finalization Instructions"));
}

#[test]
fn flow_settings_surfaces_review_and_git_run_options() {
    let repo = make_repo();
    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .args(["init"])
        .assert()
        .success();
    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .args(["settings"])
        .assert()
        .success()
        .stdout(predicate::str::contains("review.before_finalize=false"))
        .stdout(predicate::str::contains("run_branch="))
        .stdout(predicate::str::contains("run_checkpoint_commits="));
}

/// M-25 (T-004): on `host=claude-code` with no `.claude/settings*.json`
/// present, `flow doctor` prints the recommended-rules advisory block.
#[test]
fn t008_doctor_prints_claude_code_advisory_when_minimal() {
    let repo = make_repo();
    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .args(["init"])
        .assert()
        .success();
    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .env("FLOW_HOST", "claude-code")
        .args(["doctor"])
        .assert()
        .success()
        .stdout(
            predicate::str::contains(
                "Note (claude-code): Flow detected that your `.claude/settings.local.json`",
            )
            .and(predicate::str::contains("\"Bash(flow *)\""))
            .and(predicate::str::contains("\"Bash(FLOW_HOST=* flow *)\""))
            .and(predicate::str::contains("\"Edit(flow/**)\""))
            .and(predicate::str::contains("\"Write(.flow/**)\""))
            .and(predicate::str::contains(
                "Skip this if you already manage Claude Code permissions another way.",
            )),
        );
}

/// M-25 (T-004): on `host=claude-code` with a majority of recommended
/// rules already present, `flow doctor` does NOT print the advisory.
#[test]
fn t008_doctor_suppresses_claude_code_advisory_when_majority_present() {
    let repo = make_repo();
    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .args(["init"])
        .assert()
        .success();
    std::fs::create_dir_all(repo.path().join(".claude")).unwrap();
    std::fs::write(
        repo.path().join(".claude/settings.local.json"),
        r#"{
  "permissions": {
    "allow": [
      "Bash(flow *)",
      "Bash(FLOW_HOST=* flow *)",
      "Edit(flow/**)",
      "Write(flow/**)",
      "Edit(.flow/**)"
    ]
  }
}"#,
    )
    .unwrap();
    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .env("FLOW_HOST", "claude-code")
        .args(["doctor"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Note (claude-code):").not());
}

/// M-25 (T-004): on `host=claude-code` with `defaultMode: bypassPermissions`
/// in either settings file, the advisory is suppressed.
#[test]
fn t008_doctor_suppresses_claude_code_advisory_when_bypass_mode_set() {
    let repo = make_repo();
    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .args(["init"])
        .assert()
        .success();
    std::fs::create_dir_all(repo.path().join(".claude")).unwrap();
    std::fs::write(
        repo.path().join(".claude/settings.local.json"),
        r#"{"defaultMode":"bypassPermissions"}"#,
    )
    .unwrap();
    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .env("FLOW_HOST", "claude-code")
        .args(["doctor"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Note (claude-code):").not());
}

/// M-25 (T-004): on non-Claude-Code hosts, the advisory is never emitted.
#[test]
fn t008_doctor_does_not_emit_claude_code_advisory_for_other_hosts() {
    for host in ["codex", "cursor"] {
        let repo = make_repo();
        Command::cargo_bin("flow")
            .unwrap()
            .current_dir(repo.path())
            .args(["init"])
            .assert()
            .success();
        Command::cargo_bin("flow")
            .unwrap()
            .current_dir(repo.path())
            .env("FLOW_HOST", host)
            .args(["doctor"])
            .assert()
            .success()
            .stdout(predicate::str::contains("Note (claude-code):").not());
    }
}

/// M-25 (T-003): `flow doctor` MUST NOT write to `.claude/settings.json` or
/// `.claude/settings.local.json` even when the advisory is shown.
#[test]
fn t008_doctor_does_not_write_claude_code_settings_files() {
    let repo = make_repo();
    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .args(["init"])
        .assert()
        .success();
    std::fs::create_dir_all(repo.path().join(".claude")).unwrap();
    let main_path = repo.path().join(".claude/settings.json");
    let local_path = repo.path().join(".claude/settings.local.json");
    let main_body = r#"{"permissions":{"allow":["Bash(flow *)"]}}"#;
    let local_body = r#"{"permissions":{"allow":["Edit(flow/**)"]}}"#;
    std::fs::write(&main_path, main_body).unwrap();
    std::fs::write(&local_path, local_body).unwrap();
    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .env("FLOW_HOST", "claude-code")
        .args(["doctor"])
        .assert()
        .success();
    let main_after = std::fs::read_to_string(&main_path).unwrap();
    let local_after = std::fs::read_to_string(&local_path).unwrap();
    assert_eq!(main_after, main_body);
    assert_eq!(local_after, local_body);
}

/// M-25 (T-007): the binding scope rule for the entire effort. The
/// recommended-rules constant must remain literally byte-for-byte aligned
/// with the spec — defenders against accidental edits.
#[test]
fn t008_recommended_rules_match_m25_spec_literals() {
    use flow_host_claude_code::RECOMMENDED_RULES;
    assert_eq!(
        RECOMMENDED_RULES,
        &[
            "Bash(flow *)",
            "Bash(FLOW_HOST=* flow *)",
            "Edit(flow/**)",
            "Write(flow/**)",
            "Edit(.flow/**)",
            "Write(.flow/**)",
        ]
    );
}

/// M-21 (T-006): `flow start --finalize` (no path) outside any feature
/// branch and with no FLOW_CHANGE_DIR / no run-state must fail with an
/// actionable error pointing at FLOW_CHANGE_DIR / branch checkout, and
/// must exit non-zero.
#[test]
fn t006_finalize_without_path_errors_when_no_feature_can_be_inferred() {
    let repo = make_repo();
    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .args(["init"])
        .assert()
        .success();
    // The repo's current branch is the default (`main` or similar) and has no
    // matching child change directory; FLOW_CHANGE_DIR is unset
    // and no run-state has been written. `flow start --finalize` (no path)
    // must therefore fail to infer a change directory.
    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .env_remove("FLOW_CHANGE_DIR")
        .env_remove("FLOW_RUN_DIR")
        .args(["start", "--finalize"])
        .assert()
        .failure()
        .stderr(
            predicate::str::contains("Cannot resolve active change directory")
                .and(predicate::str::contains("FLOW_CHANGE_DIR")),
        );
}

/// M-21 (T-006): `flow build --finalize` (no path) under the same conditions
/// must fail in the same actionable way as `flow start --finalize`. This
/// guards FR-005 across the build path which also goes through
/// `resolve_feature_dir`.
#[test]
fn t006_finalize_without_path_errors_for_build_when_no_feature_can_be_inferred() {
    let repo = make_repo();
    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .args(["init"])
        .assert()
        .success();
    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .env_remove("FLOW_CHANGE_DIR")
        .env_remove("FLOW_RUN_DIR")
        .args(["build", "--finalize"])
        .assert()
        .failure()
        .stderr(
            predicate::str::contains("Cannot resolve active change directory")
                .and(predicate::str::contains("FLOW_CHANGE_DIR")),
        );
}
