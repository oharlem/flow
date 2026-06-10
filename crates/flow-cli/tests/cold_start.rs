//! Cold-start latency checks.
//!
//! Verifies the §1.3 SLA: `flow status` on a fully-populated change repo
//! completes under 50 ms median wall time (the plan target is 20 ms; this
//! test uses 50 ms to leave headroom for slower CI runners).
//!
//! Run with `cargo test --release --test cold_start` for meaningful numbers;
//! the debug profile adds a large constant factor.

use assert_cmd::prelude::*;
use std::path::Path;
use std::process::Command;
use std::time::{Duration, Instant};
use tempfile::TempDir;

fn init_git(path: &Path) {
    for args in [
        vec!["init", "-q", "-b", "main"],
        vec!["config", "user.email", "cs@example.com"],
        vec!["config", "user.name", "cs"],
        vec!["commit", "--allow-empty", "-q", "-m", "init"],
    ] {
        let ok = Command::new("git")
            .args(&args)
            .current_dir(path)
            .output()
            .unwrap();
        assert!(ok.status.success());
    }
}

fn seed_repo(repo: &Path) {
    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo)
        .args(["init"])
        .assert()
        .success();
    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(repo)
        .env("FLOW_FORCE_ON_PROTECTED", "1")
        .args(["start", "cold", "start", "test"])
        .assert()
        .success();
    // Seed plan + tasks so `flow status` has real work to do.
    let feat = std::fs::read_dir(repo.join("flow").join("runs"))
        .unwrap()
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path().join("changes").join("cold-start-test"))
        .find(|path| path.join("status.md").is_file())
        .expect("start should create cold-start-test child change");
    std::fs::write(
        feat.join("plan.md"),
        "## Summary\n\nSpeed test.\n\n## Technical Context\n\n**Language/Version**: Rust\n\n## Documentation Impact\n\nImpact: none\n\nDocs already current because this fixture only measures status startup.\n",
    )
    .unwrap();
    std::fs::write(
        feat.join("tasks.md"),
        "## Tasks\n\n- [ ] **T-001**: First.\n",
    )
    .unwrap();
}

fn median(mut durs: Vec<Duration>) -> Duration {
    durs.sort();
    durs[durs.len() / 2]
}

/// Verifies `flow status` cold start on a real repo. Runs 7 invocations and
/// uses the median to absorb OS jitter.
#[test]
fn cold_start_flow_status_under_budget() {
    let td = TempDir::new().unwrap();
    init_git(td.path());
    seed_repo(td.path());

    // Warm caches with one throw-away run.
    Command::cargo_bin("flow")
        .unwrap()
        .current_dir(td.path())
        .args(["status"])
        .output()
        .unwrap();

    let mut durs = Vec::new();
    for _ in 0..7 {
        let start = Instant::now();
        let out = Command::cargo_bin("flow")
            .unwrap()
            .current_dir(td.path())
            .args(["status"])
            .output()
            .unwrap();
        durs.push(start.elapsed());
        assert!(out.status.success(), "flow status failed: {out:?}");
    }
    let med = median(durs.clone());

    // Debug builds easily blow past 20 ms; the plan's SLA targets the release
    // binary. Apply a generous CI-friendly budget here (250 ms in debug,
    // 50 ms in release). When you run under --release the tighter budget
    // catches regressions; in default `cargo test` we only guard against
    // catastrophic regressions (10×).
    let budget = if cfg!(debug_assertions) {
        Duration::from_millis(500)
    } else {
        Duration::from_millis(50)
    };

    eprintln!(
        "flow status cold-start median over 7 runs = {med:?} (budget: {budget:?}); samples = {durs:?}"
    );
    assert!(
        med <= budget,
        "flow status median {med:?} exceeds budget {budget:?} (samples: {durs:?})"
    );
}
