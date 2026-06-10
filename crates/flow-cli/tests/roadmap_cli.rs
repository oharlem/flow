//! Integration tests for `flow roadmap`. T-001, T-004, T-005.

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
    commit_all(path, "flow init");
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

fn write_run_roadmap(repo: &Path, name: &str, body: &str) -> std::path::PathBuf {
    let run_dir = repo.join("flow").join("runs").join(name);
    std::fs::create_dir_all(&run_dir).unwrap();
    std::fs::write(run_dir.join("roadmap.md"), body).unwrap();
    run_dir
}

fn write_planned_run(repo: &Path, name: &str, roadmap: &str) -> std::path::PathBuf {
    let run_dir = write_run_roadmap(repo, name, roadmap);
    let fingerprint = flow_core::roadmap::fingerprint(roadmap);
    std::fs::write(
        run_dir.join("run.md"),
        format!(
            "# Run: Test\n\n**Run name**: {name}\n**Run type**: roadmap\n**Run scope**: (none)\n**Status**: planned\n**Run branch**: (none)\n**Roadmap fingerprint**: {fingerprint}\n**Checkpoint commits**: disabled\n**Current milestone**: (none)\n**Current change**: (none)\n**Current phase**: roadmap-ready\n**Last saved Flow action**: roadmap-finalized\n**Next command**: $flow-run\n**Last checkpoint**: (none)\n\n## Changes\n\n(none)\n\n## Milestones\n\n(none)\n"
        ),
    )
    .unwrap();
    run_dir
}

fn flow_roadmap_cmd() -> Command {
    let mut cmd = Command::cargo_bin("flow").unwrap();
    cmd.env("FLOW_HOST", "codex");
    cmd
}

#[test]
fn t001_help_works() {
    let out = Command::cargo_bin("flow")
        .unwrap()
        .args(["roadmap", "--help"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let text = String::from_utf8(out).unwrap();
    assert!(text.contains("--append"), "missing --append:\n{text}");
    assert!(text.contains("--replace"), "missing --replace:\n{text}");
    assert!(text.contains("--finalize"), "missing --finalize:\n{text}");
    assert!(
        !text.contains("--convert-ids"),
        "convert-ids should not be public help:\n{text}"
    );
}

#[test]
fn t004_roadmap_direct_cli_requires_agent_host() {
    let repo = make_flow_repo();
    let prd = repo.path().join("prd.md");
    std::fs::write(&prd, "# PRD\n\nWe need a thing.\n").unwrap();

    let assert = Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .args(["roadmap", "prd.md"])
        .assert()
        .failure();
    let output = assert.get_output();
    let stdout = String::from_utf8(output.stdout.clone()).unwrap();
    let stderr = String::from_utf8(output.stderr.clone()).unwrap();

    assert!(
        stderr.contains("host-assisted command"),
        "expected host-assisted error:\n{stderr}"
    );
    assert!(
        !stdout.contains("## Source Content"),
        "direct CLI must not dump the roadmap prompt:\n{stdout}"
    );
}

#[test]
fn t004_roadmap_envelope_creates_run_local_roadmap() {
    let repo = make_flow_repo();
    let out = flow_roadmap_cmd()
        .current_dir(repo.path())
        .args(["roadmap", "Build a thing. Then build another thing."])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let text = String::from_utf8(out).unwrap();
    assert!(
        text.contains("**Operation**: replace"),
        "expected replace mode:\n{text}"
    );
    assert!(text.contains("**Run directory**: flow/runs/"), "{text}");
    assert!(text.contains("**Roadmap file**: flow/runs/"), "{text}");
    assert!(
        text.contains("**Next free milestone**: M-1"),
        "expected M-1 next-free:\n{text}"
    );
    assert!(
        text.contains("**Source**: inline text"),
        "expected inline text source:\n{text}"
    );
    let runtime_context = text
        .split("\n# Runtime Context\n")
        .nth(1)
        .expect("envelope must have Runtime Context");
    let runtime_only: &str = runtime_context.split("\n---\n").next().unwrap();
    assert!(
        !runtime_only.contains("**Destructive action**:"),
        "new run roadmap mode must not be destructive in runtime context:\n{runtime_only}"
    );
    assert!(!repo.path().join("flow/roadmap.md").exists());
    let run_count = std::fs::read_dir(repo.path().join("flow/runs"))
        .unwrap()
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.path().join("roadmap.md").is_file())
        .count();
    assert_eq!(run_count, 1);
}

#[test]
fn t004_roadmap_cleans_abandoned_skeleton_run_dirs() {
    let repo = make_flow_repo();
    let run_name = format!(
        "{}-roadmap-spec-08-quadrant-syndication-playbook-ui-only",
        chrono::Utc::now().format("%Y%m%d")
    );
    let runs = repo.path().join("flow").join("runs");
    std::fs::create_dir_all(runs.join(&run_name).join("changes")).unwrap();
    std::fs::create_dir_all(runs.join(format!("{run_name}-2")).join("changes")).unwrap();

    let prd = repo.path().join("spec-08.md");
    std::fs::write(
        &prd,
        "# SPEC-08: Quadrant Syndication Playbook (UI only)\n\nPlan the UI-only work.\n",
    )
    .unwrap();
    let out = flow_roadmap_cmd()
        .current_dir(repo.path())
        .args(["roadmap", "spec-08.md"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let text = String::from_utf8(out).unwrap();

    assert!(
        text.contains(&format!("**Run directory**: flow/runs/{run_name}\n")),
        "expected abandoned skeletons to be reclaimed instead of suffixing:\n{text}"
    );
    let mut dir_names = std::fs::read_dir(&runs)
        .unwrap()
        .filter_map(|entry| {
            entry
                .ok()
                .map(|entry| entry.file_name().to_string_lossy().to_string())
        })
        .collect::<Vec<_>>();
    dir_names.sort();
    assert_eq!(dir_names, vec![run_name.clone()]);
    let run_dir = runs.join(run_name);
    assert!(run_dir.join("run.md").is_file());
    assert!(run_dir.join("roadmap.md").is_file());
    assert!(run_dir.join("log.md").is_file());
    assert!(run_dir.join("manual.md").is_file());
    assert!(run_dir.join("release-notes.md").is_file());
}

#[test]
fn t004_roadmap_generic_source_heading_does_not_duplicate_run_slug() {
    let repo = make_flow_repo();
    flow_roadmap_cmd()
        .current_dir(repo.path())
        .args(["roadmap", "# Roadmap"])
        .assert()
        .success();

    let run_dirs = std::fs::read_dir(repo.path().join("flow/runs"))
        .unwrap()
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| path.join("roadmap.md").is_file())
        .collect::<Vec<_>>();
    assert_eq!(run_dirs.len(), 1);
    let run_name = run_dirs[0].file_name().unwrap().to_string_lossy();
    assert!(
        !run_name.contains("roadmap-roadmap"),
        "generic source title should not duplicate roadmap in run name: {run_name}"
    );
    assert!(
        run_name.ends_with("-roadmap-planned-work"),
        "generic source title should use neutral default descriptor: {run_name}"
    );
    let state = std::fs::read_to_string(run_dirs[0].join("run.md")).unwrap();
    assert!(state.contains("# Run: Planned Work"), "{state}");
}

#[test]
fn t004_roadmap_replace_mode_is_not_destructive_for_new_run() {
    let repo = make_flow_repo();
    let out = flow_roadmap_cmd()
        .current_dir(repo.path())
        .args(["roadmap", "--replace", "New plan"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let text = String::from_utf8(out).unwrap();
    assert!(
        text.contains("**Operation**: replace"),
        "expected replace mode:\n{text}"
    );
    let runtime_context = text
        .split("\n# Runtime Context\n")
        .nth(1)
        .expect("envelope must have Runtime Context");
    let runtime_only: &str = runtime_context.split("\n---\n").next().unwrap();
    assert!(
        !runtime_only.contains("**Destructive action**:"),
        "new run roadmap creation must not be destructive:\n{runtime_only}"
    );
}

#[test]
fn t004_roadmap_with_existing_run_roadmap_uses_next_free_id() {
    let repo = make_flow_repo();
    write_run_roadmap(
        repo.path(),
        "20260101-existing",
        "# Roadmap\n\n## Milestones\n\n### [ ] M-1: First\n\n### [x] M-5: Fifth\n",
    );
    commit_all(repo.path(), "existing run roadmap");
    let out = flow_roadmap_cmd()
        .current_dir(repo.path())
        .args(["roadmap", "Add more"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let text = String::from_utf8(out).unwrap();
    assert!(
        text.contains("**Next free milestone**: M-2"),
        "expected M-2 next-free:\n{text}"
    );
}

#[test]
fn t001_t003_roadmap_uses_counter_setting_for_next_free_id() {
    let repo = make_flow_repo();
    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .args(["set", "counter=2"])
        .assert()
        .success();
    commit_all(repo.path(), "counter");

    let out = flow_roadmap_cmd()
        .current_dir(repo.path())
        .args(["roadmap", "Add one milestone"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let text = String::from_utf8(out).unwrap();
    assert!(
        text.contains("**Next free milestone**: M-2"),
        "expected M-2 next-free from counter setting:\n{text}"
    );
}

#[test]
fn t003_roadmap_finalize_advances_counter_after_written_milestones() {
    let repo = make_flow_repo();
    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .args(["set", "counter=2"])
        .assert()
        .success();
    let run_dir = write_planned_run(
        repo.path(),
        "20260101-new",
        "# Roadmap\n\n## Milestones\n\n### [ ] M-2: Two\n\n### [ ] M-3: Three\n",
    );

    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .env("FLOW_RUN_DIR", run_dir.to_str().unwrap())
        .args(["roadmap", "--finalize"])
        .assert()
        .success();

    let state = std::fs::read_to_string(repo.path().join(".flow/state.yaml")).unwrap();
    assert!(state.contains("counter: 4"), "{state}");
}

#[test]
fn t004_roadmap_convert_ids_is_not_a_public_option() {
    let repo = make_flow_repo();

    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .args(["roadmap", "--convert-ids"])
        .assert()
        .failure()
        .stderr(predicates::str::contains("unexpected argument"));
}

#[test]
fn t004_roadmap_with_file_source_reads_file() {
    let repo = make_flow_repo();
    let prd = repo.path().join("prd.md");
    std::fs::write(&prd, "# PRD\n\nWe need a thing.\n").unwrap();
    commit_all(repo.path(), "prd");
    let out = flow_roadmap_cmd()
        .current_dir(repo.path())
        .args(["roadmap", "prd.md"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let text = String::from_utf8(out).unwrap();
    assert!(
        text.contains("**Source**: file:"),
        "expected file source:\n{text}"
    );
    assert!(
        text.contains("We need a thing."),
        "expected file content embedded:\n{text}"
    );
}

#[test]
fn t001_t002_roadmap_missing_path_like_source_errors_without_envelope() {
    let repo = make_flow_repo();
    let assert = flow_roadmap_cmd()
        .current_dir(repo.path())
        .args(["roadmap", "docs/missing.md"])
        .assert()
        .failure();
    let output = assert.get_output();
    let stdout = String::from_utf8(output.stdout.clone()).unwrap();
    let stderr = String::from_utf8(output.stderr.clone()).unwrap();

    assert!(
        stderr.contains("Source file not found"),
        "expected source file error:\n{stderr}"
    );
    assert!(
        !stdout.contains("## Source Content"),
        "missing file must not dump the roadmap prompt:\n{stdout}"
    );
}

#[test]
fn t004_roadmap_empty_source_errors() {
    let repo = make_flow_repo();
    flow_roadmap_cmd()
        .current_dir(repo.path())
        .args(["roadmap"])
        .write_stdin("")
        .assert()
        .failure();
}

#[test]
fn t005_finalize_validates_roadmap() {
    let repo = make_flow_repo();
    let run_dir = write_planned_run(
        repo.path(),
        "20260101-valid",
        "# Roadmap\n\n## Milestones\n\n### [ ] M-1: Valid\n",
    );
    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .env("FLOW_RUN_DIR", run_dir.to_str().unwrap())
        .args(["roadmap", "--finalize"])
        .assert()
        .success();
}

#[test]
fn t005_printed_save_state_command_executes_verbatim() {
    let repo = make_flow_repo();
    let out = flow_roadmap_cmd()
        .current_dir(repo.path())
        .args(["roadmap", "Build a thing."])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let text = String::from_utf8(out).unwrap();
    let save_line = text
        .lines()
        .find(|line| line.starts_with("**Save state with**: `"))
        .expect("envelope must print a Save state with command");
    let command = save_line
        .trim_start_matches("**Save state with**: `")
        .trim_end_matches('`');
    let (env_prefix, rest) = command
        .split_once(' ')
        .expect("save command must carry FLOW_RUN_DIR");
    let run_dir = env_prefix
        .strip_prefix("FLOW_RUN_DIR=\"")
        .and_then(|prefix| prefix.strip_suffix('"'))
        .unwrap_or_else(|| panic!("save command must set FLOW_RUN_DIR: {command}"));
    assert_eq!(
        rest, "flow roadmap --finalize",
        "unexpected save command shape: {command}"
    );

    std::fs::write(
        repo.path().join(run_dir).join("roadmap.md"),
        "# Roadmap: Demo\n\n## Milestones\n\n### [ ] M-1: First\n\nOutcome: demo.\n",
    )
    .unwrap();

    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .env("FLOW_RUN_DIR", run_dir)
        .args(["roadmap", "--finalize"])
        .assert()
        .success();
}

#[test]
fn t005_finalize_errors_when_roadmap_missing() {
    let repo = make_flow_repo();
    let run_dir = repo.path().join("flow/runs/20260101-missing");
    std::fs::create_dir_all(&run_dir).unwrap();
    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo.path())
        .env("FLOW_RUN_DIR", run_dir.to_str().unwrap())
        .args(["roadmap", "--finalize"])
        .assert()
        .failure();
}
