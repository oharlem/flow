//! T-014: close envelopes honor the project confirmation setting.

use assert_cmd::Command;
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
    td
}

fn seed_close_ready_feature(repo: &Path, slug: &str) {
    let feature = slug.to_string();
    let run_dir = repo
        .join("flow")
        .join("runs")
        .join(format!("20260101-{slug}"));
    let dir = run_dir.join("changes").join(&feature);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        run_dir.join("run.md"),
        format!(
            "# Run: {feature}\n\n**Run name**: 20260101-{feature}\n**Run type**: one-off\n**Status**: running\n**Run branch**: flow/{feature}\n**Roadmap fingerprint**: (none)\n**Checkpoint commits**: false\n**Current milestone**: (none)\n**Current change**: flow/runs/20260101-{feature}/changes/{feature}\n**Current phase**: build\n**Last saved Flow action**: build-complete\n**Next command**: flow close\n**Last checkpoint**: (none)\n\n## Changes\n\n- [ ] flow/runs/20260101-{feature}/changes/{feature} — {feature}\n\n## Milestones\n\n- (none)\n",
        ),
    )
    .unwrap();
    std::fs::write(
        run_dir.join("log.md"),
        "# Run Log\n\n- 2026-01-01T00:00:00Z — run-created — fixture\n",
    )
    .unwrap();
    std::fs::write(
        dir.join("spec.md"),
        format!("# Spec: {feature}\n\n## What & Why\n\nOK.\n"),
    )
    .unwrap();
    std::fs::write(
        dir.join("plan.md"),
        "# Plan\n\n## Summary\n\nOK.\n\n## Technical Context\n\n**Language/Version**: Rust 1.81\n**Primary Dependencies**: none\n**Storage**: filesystem\n**Testing**: cargo test\n**Target Platform**: any\n**Project Type**: CLI\n**Performance Goals**: ms\n**Constraints**: none\n**Scale/Scope**: small\n\n## Documentation Impact\n\nImpact: none\n\nDocs already current because this fixture only verifies close safety.\n",
    )
    .unwrap();
    std::fs::write(dir.join("tasks.md"), "## Tasks\n\n- [x] **T-001**: Done.\n").unwrap();
    let status = format!(
        "# Status: {feature}\n\n**Change**: {feature}\n**Started**: 2026-01-01\n**Updated**: 2026-01-01T00:00:00Z\n**State**: building\n**Branch**: flow/{feature}\n\n## History\n\n- 2026-01-01T00:00:00Z — start — start\n- 2026-01-01T00:00:00Z — build-complete — done\n"
    );
    std::fs::write(dir.join("status.md"), status).unwrap();
    // Also create the branch so flow can find it.
    std::process::Command::new("git")
        .args(["checkout", "-q", "-b", &format!("flow/{feature}")])
        .current_dir(repo)
        .output()
        .unwrap();
}

#[test]
fn t014_close_envelope_confirmation_disabled_does_not_prompt() {
    let repo = make_flow_repo();
    seed_close_ready_feature(repo.path(), "close-test");
    let out = Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .args(["close"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let text = String::from_utf8(out).unwrap();
    let runtime = text
        .split("\n# Runtime Context\n")
        .nth(1)
        .expect("envelope must have Runtime Context");
    let runtime_only: &str = runtime.split("\n---\n").next().unwrap();
    assert!(
        runtime_only.contains("**Confirmation**: disabled"),
        "close envelope must honor default disabled confirmation:\n{runtime_only}"
    );
    assert!(
        !runtime_only.contains("**Destructive action**:"),
        "close envelope must not override disabled confirmation with a destructive marker:\n{runtime_only}"
    );
    assert!(
        runtime_only.contains("**Review**: collapsed"),
        "close envelope must declare collapsed review on the default green path:\n{runtime_only}"
    );
    assert!(
        runtime_only.contains("flow close --finalize"),
        "close envelope must name the collapsed finalize command:\n{runtime_only}"
    );
    assert!(
        !text.contains("Ask the user to reply `yes` or `y`"),
        "close output must not ask for confirmation when confirmation=no:\n{text}"
    );
}

#[test]
fn t014_close_envelope_confirmation_required_prompts() {
    let repo = make_flow_repo();
    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .args(["set", "confirmation=yes"])
        .assert()
        .success();
    let config_path = repo.path().join(".flow/config.yaml");
    let mut config = std::fs::read_to_string(&config_path).unwrap();
    config = config.replace("before_finalize: false", "before_finalize: true");
    std::fs::write(&config_path, config).unwrap();
    seed_close_ready_feature(repo.path(), "close-test");
    let out = Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .args(["close"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let text = String::from_utf8(out).unwrap();
    let runtime = text
        .split("\n# Runtime Context\n")
        .nth(1)
        .expect("envelope must have Runtime Context");
    let runtime_only: &str = runtime.split("\n---\n").next().unwrap();
    assert!(
        runtime_only.contains("**Confirmation**: required"),
        "close envelope must honor required confirmation setting:\n{runtime_only}"
    );
    assert!(
        !runtime_only.contains("**Destructive action**:"),
        "close envelope must not use a destructive-action override:\n{runtime_only}"
    );
    assert!(
        text.contains("Ask the user to reply `yes` or `y` to save Flow state. Then run:"),
        "close finalization instructions must ask when confirmation=yes:\n{text}"
    );
}

#[test]
fn t014_other_phases_emit_confirmation_only() {
    let repo = make_flow_repo();
    let out = Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .env("FLOW_HOST", "codex")
        .args(["roadmap", "Build something"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let text = String::from_utf8(out).unwrap();
    let runtime = text
        .split("\n# Runtime Context\n")
        .nth(1)
        .expect("envelope must have Runtime Context");
    let runtime_only: &str = runtime.split("\n---\n").next().unwrap();
    assert!(
        runtime_only.contains("**Confirmation**:"),
        "non-destructive phase must have confirmation marker:\n{runtime_only}"
    );
    assert!(
        !runtime_only.contains("**Destructive action**:"),
        "non-destructive phase must not have destructive marker:\n{runtime_only}"
    );
}
