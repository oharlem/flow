//! Embedded default asset behavior.

use assert_cmd::Command;
use flow_core::{assets, envelope::compose};
use predicates::prelude::*;
use std::path::Path;
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

#[test]
fn t001_init_and_update_do_not_materialize_generated_defaults() {
    let repo = make_repo();
    let root = repo.path();

    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(root)
        .args(["init"])
        .assert()
        .success();

    assert!(
        !root.join(".flow/conventions/core.md").exists(),
        "T-001: flow init should rely on embedded conventions by default"
    );
    assert!(
        !root.join(".flow/agents/start.base.md").exists(),
        "T-001: flow init should rely on embedded base prompts by default"
    );
    assert!(
        root.join(".flow/agents").is_dir(),
        "T-001: local override directory should still exist"
    );

    std::fs::create_dir_all(root.join(".flow/conventions")).unwrap();
    std::fs::write(
        root.join(".flow/conventions/core.md"),
        assets::conventions_shard("core").unwrap(),
    )
    .unwrap();
    std::fs::write(
        root.join(".flow/agents/start.base.md"),
        assets::agent_base("start").unwrap(),
    )
    .unwrap();
    std::fs::write(
        root.join(".flow/agents/start.local.md"),
        "T-001 local override must survive update\n",
    )
    .unwrap();

    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(root)
        .args(["update"])
        .assert()
        .success();

    assert!(
        !root.join(".flow/conventions/core.md").exists(),
        "T-001: flow update should remove old generated convention copies"
    );
    assert!(
        !root.join(".flow/agents/start.base.md").exists(),
        "T-001: flow update should remove old generated base prompt copies"
    );
    assert!(
        root.join(".flow/agents/start.local.md").is_file(),
        "T-001: flow update must preserve local prompt overrides"
    );
}

#[test]
fn t005_update_preserves_divergent_generated_defaults_with_warning() {
    let repo = make_repo();
    let root = repo.path();

    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(root)
        .args(["init"])
        .assert()
        .success();

    std::fs::create_dir_all(root.join(".flow/conventions")).unwrap();
    let divergent_core = format!(
        "{}\nT-005 local convention edit must survive update\n",
        assets::conventions_shard("core").unwrap()
    );
    let divergent_start = format!(
        "{}\nT-005 local base prompt edit must survive update\n",
        assets::agent_base("start").unwrap()
    );
    std::fs::write(root.join(".flow/conventions/core.md"), &divergent_core).unwrap();
    std::fs::write(root.join(".flow/agents/start.base.md"), &divergent_start).unwrap();
    std::fs::write(
        root.join(".flow/agents/start.local.md"),
        "T-005 local override must also survive update\n",
    )
    .unwrap();

    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(root)
        .args(["update"])
        .assert()
        .success()
        .stderr(predicate::str::contains(
            "preserved 2 modified default-asset copies",
        ))
        .stderr(predicate::str::contains(".flow/conventions/core.md"))
        .stderr(predicate::str::contains(".flow/agents/start.base.md"))
        .stderr(predicate::str::contains("flow update --force"))
        .stderr(predicate::str::contains(".flow/agents/*.local.md"))
        .stderr(predicate::str::contains("flow export-assets --dir <DIR>"));

    assert_eq!(
        std::fs::read_to_string(root.join(".flow/conventions/core.md")).unwrap(),
        divergent_core,
        "T-005: divergent convention copies must survive flow update"
    );
    assert_eq!(
        std::fs::read_to_string(root.join(".flow/agents/start.base.md")).unwrap(),
        divergent_start,
        "T-005: divergent base prompt copies must survive flow update"
    );
    assert!(
        root.join(".flow/agents/start.local.md").is_file(),
        "T-005: local prompt overrides must survive update"
    );
}

#[test]
fn t006_update_help_describes_force_as_reset_path() {
    Command::cargo_bin("flow")
        .unwrap()
        .args(["update", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--force"))
        .stdout(predicate::str::contains("divergent"))
        .stdout(predicate::str::contains(".flow/agents/*.local.md"));
}

#[test]
fn t007_update_force_resets_divergent_generated_defaults() {
    let repo = make_repo();
    let root = repo.path();

    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(root)
        .args(["init"])
        .assert()
        .success();

    std::fs::create_dir_all(root.join(".flow/conventions")).unwrap();
    let divergent_core = format!(
        "{}\nT-007 stale snapshot to be reset\n",
        assets::conventions_shard("core").unwrap()
    );
    let divergent_start = format!(
        "{}\nT-007 stale snapshot to be reset\n",
        assets::agent_base("start").unwrap()
    );
    std::fs::write(root.join(".flow/conventions/core.md"), &divergent_core).unwrap();
    std::fs::write(root.join(".flow/agents/start.base.md"), &divergent_start).unwrap();
    std::fs::write(
        root.join(".flow/agents/start.local.md"),
        "T-007 local override must survive --force\n",
    )
    .unwrap();

    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(root)
        .args(["update", "--force"])
        .assert()
        .success()
        .stderr(predicate::str::contains(
            "removed 2 divergent default-asset copies",
        ))
        .stderr(predicate::str::contains(".flow/conventions/core.md"))
        .stderr(predicate::str::contains(".flow/agents/start.base.md"));

    assert!(
        !root.join(".flow/conventions/core.md").exists(),
        "T-007: --force should remove divergent convention copies"
    );
    assert!(
        !root.join(".flow/agents/start.base.md").exists(),
        "T-007: --force should remove divergent base prompt copies"
    );
    assert!(
        root.join(".flow/agents/start.local.md").is_file(),
        "T-007: --force must preserve local prompt overrides"
    );
}

#[test]
fn t002_doctor_accepts_missing_generated_defaults() {
    let repo = make_repo();
    let root = repo.path();

    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(root)
        .args(["init"])
        .assert()
        .success();

    assert!(!root.join(".flow/conventions/core.md").exists());
    assert!(!root.join(".flow/agents/plan.base.md").exists());

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
fn t003_export_assets_writes_embedded_defaults() {
    let td = TempDir::new().unwrap();
    let out_dir = td.path().join("review-assets");

    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(td.path())
        .args(["export-assets", "--dir"])
        .arg(&out_dir)
        .assert()
        .success()
        .stdout(predicate::str::contains("T-003").not())
        .stderr(predicate::str::contains("Exported embedded Flow assets"));

    let core = std::fs::read_to_string(out_dir.join("conventions/core.md")).unwrap();
    let start = std::fs::read_to_string(out_dir.join("agents/start.base.md")).unwrap();

    assert!(
        core.contains("Conventions-Version"),
        "T-003: exported conventions should come from embedded defaults"
    );
    assert!(
        start.contains("Phase Agent: start"),
        "T-003: exported base prompts should come from embedded defaults"
    );
}

#[test]
fn t004_envelope_uses_embedded_defaults_with_local_override() {
    let repo = TempDir::new().unwrap();
    let feature_dir = repo.path().join("flow").join("changes").join("example");
    std::fs::create_dir_all(&feature_dir).unwrap();
    std::fs::create_dir_all(repo.path().join(".flow/agents")).unwrap();
    std::fs::write(
        repo.path().join(".flow/agents/plan.local.md"),
        "T-004 repo-local override sentinel\n",
    )
    .unwrap();

    let envelope = compose(repo.path(), "plan", Path::new(&feature_dir), None).unwrap();

    assert!(
        envelope.contains("Flow Artifact Conventions — Core"),
        "T-004: embedded conventions should be used when disk copies are absent"
    );
    assert!(
        envelope.contains("Phase Agent: plan"),
        "T-004: embedded base prompt should be used when disk copies are absent"
    );
    assert!(
        envelope.contains("T-004 repo-local override sentinel"),
        "T-004: repo-local overrides should still be appended"
    );
}

#[test]
fn t001_t003_run_prompt_describes_one_roadmap_scoped_workflow() {
    let body = assets::agent_base("run").expect("embedded run prompt exists");

    assert!(
        body.contains("## Roadmap-Scoped Run Workflow"),
        "T-001/T-003: run prompt must name the unified roadmap-scoped workflow"
    );
    assert!(
        body.contains("Attach to or create the roadmap-scoped run"),
        "T-001/T-003: run prompt must start from attach-or-create behavior"
    );
    assert!(
        body.contains("Loop over the milestone or milestones requested by `Invocation`"),
        "T-001/T-003: run prompt must describe invocation-scoped milestone looping"
    );
    assert!(
        body.contains("start -> plan -> build -> test -> close"),
        "T-001/T-003: run prompt must preserve the child phase order"
    );
    assert!(
        body.contains("refresh `log.md`, `manual.md`, and `release-notes.md` after each close"),
        "T-001/T-003: run prompt must keep run artifacts current after each milestone"
    );
    assert!(
        body.contains("follow the `run.md` `Next command` exactly"),
        "T-001/T-003: run prompt must keep the driver-owned next command authoritative"
    );
    assert!(
        !body.contains("## One-Milestone Workflow") && !body.contains("## Full-Roadmap Workflow"),
        "T-001/T-003: run prompt must not split the workflow into obsolete sections"
    );
}
