//! Repo-local state boundary checks.

use assert_cmd::Command;
use flow_core::envelope::compose;
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
fn t001_project_state_files_are_repo_local() {
    let repo = make_repo();
    let fake_home = TempDir::new().unwrap();
    std::fs::create_dir_all(fake_home.path().join(".flow")).unwrap();
    std::fs::write(
        fake_home.path().join(".flow").join("state.yaml"),
        "counter: 99\n",
    )
    .unwrap();

    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .env("HOME", fake_home.path())
        .args(["init"])
        .assert()
        .success();
    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .env("HOME", fake_home.path())
        .args(["set", "counter=6"])
        .assert()
        .success();

    assert!(repo.path().join(".flow").join("config.yaml").is_file());
    assert!(repo.path().join(".flow").join("state.yaml").is_file());
    assert!(repo.path().join(".flow").join("version").is_file());
    assert!(
        std::fs::read_to_string(repo.path().join(".flow").join("state.yaml"))
            .unwrap()
            .contains("counter: 6"),
        "T-001: counter state should be written to the repository .flow directory"
    );

    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .env("HOME", fake_home.path())
        .args(["settings"])
        .assert()
        .success()
        .stdout(predicate::str::contains("counter=6"))
        .stdout(predicate::str::contains("counter=99").not());
}

#[test]
fn t002_phase_local_override_is_repo_local() {
    let repo = TempDir::new().unwrap();
    let feature_dir = repo.path().join("flow").join("changes").join("example");
    std::fs::create_dir_all(&feature_dir).unwrap();
    std::fs::create_dir_all(repo.path().join(".flow").join("agents")).unwrap();
    std::fs::write(
        repo.path()
            .join(".flow")
            .join("agents")
            .join("plan.local.md"),
        "T-002 repo-local plan override sentinel\n",
    )
    .unwrap();

    let envelope = compose(repo.path(), "plan", Path::new(&feature_dir), None).unwrap();

    assert!(
        envelope.contains("T-002 repo-local plan override sentinel"),
        "repo-local .flow/agents/*.local.md override should be included in the envelope"
    );
}
