//! End-to-end scenario tests that mirror `~/flow/tests/integration/scenarios/`.
//!
//! Each test spawns the compiled `flow` binary in a fresh scratch repo and
//! drives the full Flow workflow end-to-end.

use assert_cmd::Command;
use predicates::prelude::*;
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
    std::fs::write(
        path.join("Cargo.toml"),
        "[workspace]\nmembers = []\n\n[workspace.package]\nversion = \"1.0.0\"\n",
    )
    .unwrap();
    std::process::Command::new("git")
        .args(["add", "Cargo.toml"])
        .current_dir(path)
        .output()
        .unwrap();
    std::process::Command::new("git")
        .args(["commit", "-q", "-m", "init"])
        .current_dir(path)
        .output()
        .unwrap();
    td
}

fn flow() -> Command {
    Command::cargo_bin("flow").unwrap()
}

fn change_dir(root: &std::path::Path, slug: &str) -> std::path::PathBuf {
    let runs = root.join("flow").join("runs");
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

fn seed_roadmap_run(root: &std::path::Path, name: &str, roadmap_text: &str) -> std::path::PathBuf {
    let run_dir = root.join("flow").join("runs").join(name);
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

fn run_dirs(root: &std::path::Path) -> Vec<std::path::PathBuf> {
    let mut dirs = std::fs::read_dir(root.join("flow").join("runs"))
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.is_dir())
        .collect::<Vec<_>>();
    dirs.sort();
    dirs
}

/// Mirrors `~/flow/tests/integration/scenarios/05-end-to-end-walkthrough.sh`:
/// start → (spec body) → plan → (plan + tasks) → build-task x N → test → release.
#[test]
fn scenario_full_walkthrough_closes_change_in_place() {
    let repo = make_repo();
    let root = repo.path();

    // /flow-init
    flow().current_dir(root).args(["init"]).assert().success();

    // /flow-start
    flow()
        .current_dir(root)
        .args(["start", "track", "plants"])
        .assert()
        .success()
        .stdout(predicate::str::contains("**Review**: collapsed"))
        .stdout(predicate::str::contains("Finalization Instructions").not());

    let feature_dir = change_dir(root, "track-plants");
    assert!(feature_dir.is_dir());

    // Write a complete spec.md with one FR + one SC.
    std::fs::write(
        feature_dir.join("spec.md"),
        "# Spec: track-plants\n\n## What & Why\n\nTrack plants.\n\n## Requirements\n\n### Functional Requirements\n\n- **FR-001**: Track plants with watering history.\n\n## Success Criteria\n\n### Measurable Outcomes\n\n- **SC-001**: Users see when each plant was last watered.\n",
    )
    .unwrap();

    // /flow-start --finalize
    flow()
        .current_dir(root)
        .env("FLOW_CHANGE_DIR", feature_dir.to_str().unwrap())
        .args(["start", "--finalize"])
        .assert()
        .success()
        .stdout(predicate::str::contains("flow plan"));

    // State should be drafting + action spec-complete
    let status = std::fs::read_to_string(feature_dir.join("status.md")).unwrap();
    assert!(status.contains("**State**: drafting"));
    assert!(status.contains("spec-complete"));

    // /flow-plan (emit envelope)
    flow().current_dir(root).args(["plan"]).assert().success();

    // Write plan.md + tasks.md with a single automated task that a test will reference.
    std::fs::write(
        feature_dir.join("plan.md"),
        "# Implementation Plan\n\n## Summary\n\nTrack plants.\n\n## Technical Context\n\n**Language/Version**: Rust\n\n## Documentation Impact\n\nImpact: none\n\nDocs already current because this fixture does not change current Flow documentation.\n",
    )
    .unwrap();
    std::fs::write(
        feature_dir.join("tasks.md"),
        "# Tasks\n\n## Tasks\n\n- [ ] **T-001**: Implement plant tracking.\n    - Covers: FR-001\n    - Verifies: SC-001\n    - Depends-On: (none)\n",
    )
    .unwrap();

    // /flow-plan --finalize
    flow()
        .current_dir(root)
        .env("FLOW_CHANGE_DIR", feature_dir.to_str().unwrap())
        .args(["plan", "--finalize"])
        .assert()
        .success();

    let status = std::fs::read_to_string(feature_dir.join("status.md")).unwrap();
    assert!(status.contains("**State**: building"));
    assert!(status.contains("plan-complete"));

    // Write an automated test that references T-001.
    std::fs::create_dir_all(root.join("tests")).unwrap();
    std::fs::write(
        root.join("tests").join("test_plants.py"),
        "def test_t001_tracks_plants():\n    assert True\n",
    )
    .unwrap();

    // /flow-build-task --finalize (final accepted task → checks task, stamps task-complete, routes to /flow-test)
    flow()
        .current_dir(root)
        .env("FLOW_CHANGE_DIR", feature_dir.to_str().unwrap())
        .args(["build-task", "T-001", "--finalize"])
        .assert()
        .success();

    let tasks = std::fs::read_to_string(feature_dir.join("tasks.md")).unwrap();
    assert!(tasks.contains("- [x] **T-001**"));
    let status = std::fs::read_to_string(feature_dir.join("status.md")).unwrap();
    assert!(status.contains("task-complete"));
    assert!(status.contains("build-complete — verification passed"));

    // /flow-test --finalize is not required; build-task chained verification.

    // /flow-close --finalize (closes in place)
    flow()
        .current_dir(root)
        .env("FLOW_CHANGE_DIR", feature_dir.to_str().unwrap())
        .args(["close", "--finalize"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Change 'track-plants' closed."));

    // Close invariants: change dir remains stable and Closed: header is stamped.
    assert!(
        feature_dir.exists(),
        "close must keep the change directory in place"
    );
    assert!(!root.join("flow/archive").exists());
    let spec_after = std::fs::read_to_string(feature_dir.join("spec.md")).unwrap();
    assert!(spec_after.contains("**Closed**:"));
    let status_after = std::fs::read_to_string(feature_dir.join("status.md")).unwrap();
    assert!(status_after.contains("**State**: closed"));
    assert!(status_after.contains("change closed"));
}

/// `flow status` on a closed feature should print `**State**: closed`.
#[test]
fn status_after_close_shows_closed() {
    let repo = make_repo();
    let root = repo.path();
    flow().current_dir(root).args(["init"]).assert().success();
    flow()
        .current_dir(root)
        .args(["start", "closeit"])
        .assert()
        .success();
    let feature_dir = change_dir(root, "closeit");
    std::fs::write(feature_dir.join("spec.md"), "## What & Why\n\nClose it.\n").unwrap();
    std::fs::write(
        feature_dir.join("plan.md"),
        "## Summary\n\nClose.\n\n## Technical Context\n\n**Language/Version**: Rust\n\n## Documentation Impact\n\nImpact: none\n\nDocs already current because this fixture exercises closed status only.\n",
    )
    .unwrap();
    std::fs::write(
        feature_dir.join("tasks.md"),
        "## Tasks\n\n- [x] **T-001**: Demo.\n",
    )
    .unwrap();
    flow()
        .current_dir(root)
        .env("FLOW_CHANGE_DIR", feature_dir.to_str().unwrap())
        .args(["plan", "--finalize"])
        .assert()
        .success();
    flow()
        .current_dir(root)
        .env("FLOW_CHANGE_DIR", feature_dir.to_str().unwrap())
        .args(["build-task", "--finalize"])
        .assert()
        .success();
    flow()
        .current_dir(root)
        .env("FLOW_CHANGE_DIR", feature_dir.to_str().unwrap())
        .args(["test", "--finalize"])
        .assert()
        .success();
    flow()
        .current_dir(root)
        .env("FLOW_CHANGE_DIR", feature_dir.to_str().unwrap())
        .args(["close", "--finalize"])
        .assert()
        .success();

    flow()
        .current_dir(root)
        .args(["status"])
        .assert()
        .success()
        .stdout(predicate::str::contains("**State**: closed"));
}

/// Milestone flow: roadmap seeded, `flow start M-1`, close flips the checkbox.
#[test]
fn milestone_tick_on_close() {
    let repo = make_repo();
    let root = repo.path();
    flow().current_dir(root).args(["init"]).assert().success();

    let run_dir = seed_roadmap_run(
        root,
        "20260101-milestone",
        "# Roadmap\n\n## Milestones\n\n### [ ] M-1: First\n\nKick things off.\n",
    );

    flow()
        .current_dir(root)
        .env("FLOW_RUN_DIR", &run_dir)
        .args(["start", "M-1"])
        .assert()
        .success();

    let feature_dir = change_dir(root, "M-1-first");
    assert!(feature_dir.is_dir());

    let status = std::fs::read_to_string(feature_dir.join("status.md")).unwrap();
    assert!(status.contains("**Milestone**: M-1"));

    // Fill out the artifacts so close can proceed.
    std::fs::write(feature_dir.join("spec.md"), "## What & Why\n\nFirst.\n").unwrap();
    std::fs::write(
        feature_dir.join("plan.md"),
        "## Summary\n\nFirst.\n\n## Technical Context\n\n**Language/Version**: Rust\n\n## Documentation Impact\n\nImpact: none\n\nDocs already current because this fixture exercises milestone ticking only.\n",
    )
    .unwrap();
    std::fs::write(
        feature_dir.join("tasks.md"),
        "## Tasks\n\n- [x] **T-001**: Done.\n",
    )
    .unwrap();
    for phase in ["plan", "build-task", "test", "close"] {
        flow()
            .current_dir(root)
            .env("FLOW_RUN_DIR", &run_dir)
            .env("FLOW_CHANGE_DIR", feature_dir.to_str().unwrap())
            .args([phase, "--finalize"])
            .assert()
            .success();
    }

    let roadmap = std::fs::read_to_string(run_dir.join("roadmap.md")).unwrap();
    assert!(
        roadmap.contains("### [x] M-1: First"),
        "roadmap should have a ticked M-1, got:\n{roadmap}"
    );
    assert!(roadmap.contains("Kick things off."));
}

/// T-011: End-to-end scenario covering /flow-roadmap → /flow-start with M-N
/// in a run-local roadmap.
#[test]
fn t011_roadmap_then_start_with_milestone_run_local() {
    let td = make_repo();
    let root = td.path();
    flow().current_dir(root).args(["init"]).assert().success();
    flow()
        .current_dir(root)
        .args(["set", "confirmation=no"])
        .assert()
        .success();

    // (2) flow roadmap with PRD via stdin → empty roadmap → silent append.
    let prd = "We need a login system, then a dashboard, then notifications.";
    let out = flow()
        .current_dir(root)
        .env("FLOW_HOST", "codex")
        .args(["roadmap"])
        .write_stdin(prd)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let text = String::from_utf8(out).unwrap();
    assert!(
        text.contains("**Operation**: replace"),
        "expected fresh run-local roadmap mode"
    );
    let run_dir = run_dirs(root)
        .into_iter()
        .find(|path| path.join("roadmap.md").is_file())
        .expect("roadmap run should be created");
    let runtime = text
        .split("\n# Runtime Context\n")
        .nth(1)
        .expect("runtime context");
    let runtime_only: &str = runtime.split("\n---\n").next().unwrap();
    assert!(
        runtime_only.contains("**Confirmation**: disabled"),
        "expected disabled confirmation:\n{runtime_only}"
    );
    assert!(
        !runtime_only.contains("**Destructive action**:"),
        "append must not be destructive"
    );

    // (3) Seed roadmap as if the agent wrote it, with M-1 and M-2.
    let roadmap = run_dir.join("roadmap.md");
    std::fs::write(
        &roadmap,
        "# Roadmap\n\n## Milestones\n\n### [ ] M-1: Login system\n\nLogin description.\n\n### [ ] M-2: Dashboard\n\nDashboard description.\n",
    )
    .unwrap();

    // (4) flow start M-1 → status.md has Milestone: M-1.
    flow()
        .current_dir(root)
        .env("FLOW_RUN_DIR", &run_dir)
        .env("FLOW_FORCE_ON_PROTECTED", "1")
        .args(["start", "M-1", "implement", "login"])
        .assert()
        .success();
    let dir = change_dir(root, "M-1-login-system");
    let status = std::fs::read_to_string(dir.join("status.md")).unwrap();
    assert!(status.contains("**Milestone**: M-1"));

    // (6) A separate flow roadmap --replace creates a new planned run; it does
    // not destructively replace the existing run-local roadmap.
    let out2 = flow()
        .current_dir(root)
        .env("FLOW_HOST", "codex")
        .args(["roadmap", "--replace", "Brand new plan"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let text2 = String::from_utf8(out2).unwrap();
    let runtime2 = text2
        .split("\n# Runtime Context\n")
        .nth(1)
        .expect("runtime context");
    let runtime_only2: &str = runtime2.split("\n---\n").next().unwrap();
    assert!(
        !runtime_only2.contains("**Destructive action**:"),
        "new run-local roadmap should not be destructive:\n{runtime_only2}"
    );
    assert!(runtime_only2.contains("**Confirmation**: disabled"));

    // (7) flow start M-999 (missing) → fails.
    let original_roadmap_bytes = std::fs::read(&roadmap).unwrap();
    flow()
        .current_dir(root)
        .env("FLOW_RUN_DIR", &run_dir)
        .env("FLOW_FORCE_ON_PROTECTED", "1")
        .args(["start", "M-999", "missing"])
        .assert()
        .failure();
    // Roadmap is unchanged after the failed start.
    let after = std::fs::read(&roadmap).unwrap();
    assert_eq!(original_roadmap_bytes, after, "roadmap should be untouched");

    // (8) flow start (no milestone) → status.md has no Milestone line, roadmap unchanged.
    flow()
        .current_dir(root)
        .env("FLOW_FORCE_ON_PROTECTED", "1")
        .args(["start", "no", "milestone"])
        .assert()
        .success();
    let after2 = std::fs::read(&roadmap).unwrap();
    assert_eq!(original_roadmap_bytes, after2, "roadmap byte-identical");
    let latest = change_dir(root, "no-milestone");
    let status = std::fs::read_to_string(latest.join("status.md")).unwrap();
    assert!(
        !status.contains("**Milestone**:"),
        "no-milestone feature must not have Milestone line:\n{status}"
    );
}

fn git_stdout(root: &std::path::Path, args: &[&str]) -> String {
    let out = std::process::Command::new("git")
        .args(args)
        .current_dir(root)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "git {args:?} failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout).unwrap().trim().to_string()
}

fn commit_all(root: &std::path::Path, message: &str) {
    git_stdout(root, &["add", "-A"]);
    git_stdout(root, &["commit", "-q", "-m", message]);
}

/// Drive one milestone's child change from `flow start M-N` through
/// `flow close --finalize`, writing minimal close-ready artifacts in between.
fn close_milestone_child(
    root: &std::path::Path,
    run_dir: &std::path::Path,
    milestone: &str,
    title_slug: &str,
) {
    flow()
        .current_dir(root)
        .env("FLOW_RUN_DIR", run_dir)
        .args(["start", milestone])
        .assert()
        .success();
    let feature_dir = change_dir(root, &format!("{milestone}-{title_slug}"));
    std::fs::write(
        feature_dir.join("spec.md"),
        format!("## What & Why\n\nDeliver {milestone}.\n"),
    )
    .unwrap();
    std::fs::write(
        feature_dir.join("plan.md"),
        "## Summary\n\nDo it.\n\n## Technical Context\n\n**Language/Version**: Rust\n\n## Documentation Impact\n\nImpact: none\n\nDocs already current because this scenario exercises run mechanics.\n",
    )
    .unwrap();
    std::fs::write(
        feature_dir.join("tasks.md"),
        "## Tasks\n\n- [x] **T-001**: Done.\n",
    )
    .unwrap();
    for phase in ["plan", "build-task", "test", "close"] {
        flow()
            .current_dir(root)
            .env("FLOW_RUN_DIR", run_dir)
            .env("FLOW_CHANGE_DIR", feature_dir.to_str().unwrap())
            .args([phase, "--finalize"])
            .assert()
            .success();
    }
}

/// Full green path chaining the real commands: on `main`, `/flow-roadmap` a
/// simple scope, save it with the printed finalize command verbatim, then
/// `/flow-run` the whole roadmap. The run must do its work on its own
/// `flow/run-*` branch with one checkpoint commit per closed milestone, and
/// `main` must never move.
#[test]
fn scenario_roadmap_then_run_works_on_own_branch_with_checkpoint_commits() {
    let repo = make_repo();
    let root = repo.path();
    git_stdout(root, &["branch", "-M", "main"]);
    flow().current_dir(root).args(["init"]).assert().success();
    commit_all(root, "flow init");

    // /flow-roadmap with a simple scope, invoked from `main`.
    let out = flow()
        .current_dir(root)
        .env("FLOW_HOST", "codex")
        .args(["roadmap"])
        .write_stdin("Add a hello command, then add a goodbye command.")
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let text = String::from_utf8(out).unwrap();
    let save_line = text
        .lines()
        .find(|line| line.starts_with("**Save state with**: `"))
        .expect("roadmap envelope must print a Save state with command");
    let command = save_line
        .trim_start_matches("**Save state with**: `")
        .trim_end_matches('`');
    let run_dir_rel = command
        .strip_prefix("FLOW_RUN_DIR=\"")
        .and_then(|cmd| cmd.strip_suffix("\" flow roadmap --finalize"))
        .unwrap_or_else(|| panic!("unexpected save command shape: {command}"));
    let run_dir = root.join(run_dir_rel);

    // The agent writes the decomposed roadmap, then runs the printed command.
    std::fs::write(
        run_dir.join("roadmap.md"),
        "# Roadmap: Greetings\n\n## Milestones\n\n### [ ] M-1: Hello command\n\nOutcome: hello prints.\n\n### [ ] M-2: Goodbye command\n\nOutcome: goodbye prints.\n",
    )
    .unwrap();
    flow()
        .current_dir(root)
        .env("FLOW_RUN_DIR", run_dir_rel)
        .args(["roadmap", "--finalize"])
        .assert()
        .success();

    // Roadmap drafting never branches; the planned run is still Flow-owned
    // local state when `/flow-run` starts.
    assert_eq!(git_stdout(root, &["branch", "--show-current"]), "main");
    let main_before_run = git_stdout(root, &["rev-parse", "main"]);

    // /flow-run warns about `main`, then moves to its own run branch.
    let assert = flow().current_dir(root).args(["run"]).assert().success();
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).unwrap();
    assert!(
        stderr.contains("protected branch 'main'"),
        "expected protected-branch warning when starting from main:\n{stderr}"
    );
    let run_name = run_dir.file_name().unwrap().to_str().unwrap().to_string();
    let run_branch = format!("flow/run-{run_name}");
    assert_eq!(git_stdout(root, &["branch", "--show-current"]), run_branch);
    let state = std::fs::read_to_string(run_dir.join("run.md")).unwrap();
    assert!(state.contains("**Status**: running"), "{state}");
    assert!(
        state.contains(&format!("**Run branch**: {run_branch}")),
        "{state}"
    );
    assert!(state.contains("**Checkpoint commits**: enabled"), "{state}");

    // M-1: child change through close, then the printed checkpoint command.
    close_milestone_child(root, &run_dir, "M-1", "hello-command");
    let state = std::fs::read_to_string(run_dir.join("run.md")).unwrap();
    let checkpoint_m1 = format!("flow run --checkpoint \"{run_dir_rel}\" --milestone M-1");
    assert!(
        state.contains(&format!("**Next command**: {checkpoint_m1}")),
        "close must point at the checkpoint command while milestones remain open:\n{state}"
    );
    flow()
        .current_dir(root)
        .args(["run", "--checkpoint", run_dir_rel, "--milestone", "M-1"])
        .assert()
        .success();
    // The checkpoint commits the milestone work first, then records its SHA
    // into run.md/log.md; only that bookkeeping may remain uncommitted.
    let dirty = git_stdout(root, &["status", "--porcelain"]);
    assert!(
        dirty
            .lines()
            .all(|line| line.ends_with("/run.md") || line.ends_with("/log.md")),
        "checkpoint must commit all milestone work:\n{dirty}"
    );
    let state = std::fs::read_to_string(run_dir.join("run.md")).unwrap();
    assert!(
        !state.contains("**Last checkpoint**: (none)"),
        "checkpoint must record its SHA:\n{state}"
    );

    // A bare `flow run` against the in-flight all-scope run re-enters the
    // run agent and points at the next open milestone.
    flow()
        .current_dir(root)
        .args(["run"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Continuing run"))
        .stdout(predicate::str::contains(format!(
            "First child command: `FLOW_RUN_DIR=\"{run_dir_rel}\" flow start M-2`"
        )))
        .stdout(predicate::str::contains("# Flow Run Resume").not());

    // M-2 (last milestone): close, complete handoff docs, checkpoint, finalize.
    close_milestone_child(root, &run_dir, "M-2", "goodbye-command");
    let state = std::fs::read_to_string(run_dir.join("run.md")).unwrap();
    assert!(state.contains("**Current milestone**: M-2"), "{state}");
    let state = std::fs::read_to_string(run_dir.join("run.md")).unwrap();
    assert!(
        state.contains(&format!(
            "**Next command**: flow run --checkpoint \"{run_dir_rel}\" --milestone M-2"
        )),
        "last close must point at the final milestone checkpoint:\n{state}"
    );
    std::fs::write(
        run_dir.join("manual.md"),
        "# Owner's Manual\n\n**Status**: draft\n\n## Quickstart\n\nRun it.\n\n## Resulting State\n\nThe greetings commands are ready to operate.\n",
    )
    .unwrap();
    std::fs::write(
        run_dir.join("release-notes.md"),
        "# Release Notes\n\n**Status**: draft\n\n## Delivered Changes\n\nHello and goodbye commands.\n\n## User Impact\n\nUsers can greet.\n\n## Upgrade Notes\n\nNone.\n\n## Verification Summary\n\nVerification passed.\n\n## Source Milestones\n\nM-1, M-2.\n",
    )
    .unwrap();
    flow()
        .current_dir(root)
        .args(["run", "--checkpoint", run_dir_rel, "--milestone", "M-2"])
        .assert()
        .success();
    flow()
        .current_dir(root)
        .env("FLOW_RUN_DIR", run_dir_rel)
        .args(["run", "--finalize"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Run finalized."))
        .stdout(predicate::str::contains("Closing commit: "))
        .stdout(predicate::str::contains("Verify this run:"));

    // The closing commit sweeps up the post-checkpoint run bookkeeping; a
    // finalized run leaves a clean worktree.
    let dirty = git_stdout(root, &["status", "--porcelain"]);
    assert!(
        dirty.is_empty(),
        "finalize must leave a clean worktree:\n{dirty}"
    );

    // The run worked in its own branch with one checkpoint commit per
    // milestone plus the closing commit; `main` never moved.
    assert_eq!(git_stdout(root, &["rev-parse", "main"]), main_before_run);
    let subjects = git_stdout(root, &["log", "--format=%s", "main..HEAD"]);
    assert_eq!(
        subjects.lines().collect::<Vec<_>>(),
        [
            format!("flow run finalize: {run_name}").as_str(),
            "flow run checkpoint: M-2 Goodbye command",
            "flow run checkpoint: M-1 Hello command",
        ],
        "expected one checkpoint commit per closed milestone plus the closing commit"
    );
    let roadmap = std::fs::read_to_string(run_dir.join("roadmap.md")).unwrap();
    assert!(roadmap.contains("### [x] M-1: Hello command"), "{roadmap}");
    assert!(
        roadmap.contains("### [x] M-2: Goodbye command"),
        "{roadmap}"
    );
}
