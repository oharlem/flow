//! Golden-fixture runner.
//!
//! A "golden fixture" is a directory under `tests/golden/<name>/` with:
//!
//! ```text
//! tests/golden/<name>/
//! ├── command          # one-line: the `flow` args to run (without `flow`)
//! ├── env              # optional KEY=value lines
//! ├── stdout.txt       # expected stdout (exact)          — optional
//! ├── stderr.sub.txt   # substring that must appear in stderr — optional
//! ├── exit_code        # expected exit code (default: 0)  — optional
//! ├── before/          # repo state before the command (optional)
//! │   └── …
//! └── after/           # files that must exist afterward (optional)
//!     └── …
//! ```
//!
//! The runner creates a scratch git repo, copies `before/` into it, runs the
//! `flow` binary, and asserts:
//!
//! 1. Exit code matches `exit_code` (or 0).
//! 2. Every relative path in `after/` exists in the scratch repo and has
//!    byte-identical content to the fixture file.
//! 3. When `stdout.txt` is present, the run's stdout matches exactly.
//! 4. When `stderr.sub.txt` is present, the run's stderr contains its text.
//!
//! Close-time `**Closed**: <date>` headers use `UTC today`, which would flap
//! between runs. Fixtures that depend on today's date must use the
//! `{{TODAY}}` placeholder in `stdout.txt` / `after/**/*` — the runner
//! substitutes it before comparing.

use assert_cmd::prelude::*;
use chrono::Utc;
use pretty_assertions::assert_eq;
use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::TempDir;

fn fixtures_root() -> PathBuf {
    // `CARGO_MANIFEST_DIR` = `crates/flow-cli`.
    let here = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    here.join("..").join("..").join("tests").join("golden")
}

fn init_repo(path: &Path) {
    for args in [
        vec!["init", "-q", "-b", "main"],
        vec!["config", "user.email", "golden@example.com"],
        vec!["config", "user.name", "golden"],
        vec!["commit", "--allow-empty", "-q", "-m", "init"],
    ] {
        let ok = Command::new("git")
            .args(&args)
            .current_dir(path)
            .output()
            .unwrap();
        assert!(ok.status.success(), "git {args:?} failed: {ok:?}");
    }
}

fn copy_tree(src: &Path, dst: &Path) {
    for entry in walkdir::WalkDir::new(src) {
        let entry = entry.unwrap();
        let rel = entry.path().strip_prefix(src).unwrap();
        if rel.as_os_str().is_empty() {
            continue;
        }
        let target = dst.join(rel);
        if entry.file_type().is_dir() {
            std::fs::create_dir_all(&target).unwrap();
        } else {
            if let Some(parent) = target.parent() {
                std::fs::create_dir_all(parent).unwrap();
            }
            std::fs::copy(entry.path(), &target).unwrap();
        }
    }
}

fn substitute_today(text: &str) -> String {
    let today = Utc::now().format("%Y-%m-%d").to_string();
    text.replace("{{TODAY}}", &today)
}

fn run_fixture(name: &str) {
    let fixture = fixtures_root().join(name);
    assert!(fixture.is_dir(), "missing fixture: {fixture:?}");

    let command = std::fs::read_to_string(fixture.join("command"))
        .unwrap_or_else(|e| panic!("missing command file for {name}: {e}"));
    let expected_exit: i32 = std::fs::read_to_string(fixture.join("exit_code"))
        .ok()
        .and_then(|s| s.trim().parse().ok())
        .unwrap_or(0);

    let scratch = TempDir::new().unwrap();
    init_repo(scratch.path());
    let before = fixture.join("before");
    if before.is_dir() {
        copy_tree(&before, scratch.path());
    }

    let args = shell_split(command.trim());
    let mut cmd = Command::cargo_bin("flow").unwrap();
    cmd.current_dir(scratch.path())
        .env("FLOW_FORCE_ON_PROTECTED", "1");
    let env_path = fixture.join("env");
    if env_path.is_file() {
        let body = std::fs::read_to_string(&env_path).unwrap();
        for line in body.lines().map(str::trim).filter(|line| !line.is_empty()) {
            let Some((key, value)) = line.split_once('=') else {
                panic!("fixture {name}: invalid env line {line:?}");
            };
            cmd.env(key, value);
        }
    }
    cmd.args(&args);
    let output = cmd.output().unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let actual_exit = output.status.code().unwrap_or(-1);

    assert_eq!(
        actual_exit, expected_exit,
        "fixture {name}: exit code mismatch\n--- stdout ---\n{stdout}\n--- stderr ---\n{stderr}"
    );

    let stdout_golden = fixture.join("stdout.txt");
    if stdout_golden.is_file() {
        let expected = substitute_today(&std::fs::read_to_string(&stdout_golden).unwrap());
        assert_eq!(stdout, expected, "fixture {name}: stdout byte diff");
    }

    let stdout_sub_path = fixture.join("stdout.sub.txt");
    if stdout_sub_path.is_file() {
        let body = substitute_today(&std::fs::read_to_string(&stdout_sub_path).unwrap());
        for line in body.lines().filter(|l| !l.trim().is_empty()) {
            assert!(
                stdout.contains(line),
                "fixture {name}: stdout missing substring {line:?}\n--- stdout ---\n{stdout}"
            );
        }
    }

    let stderr_golden = fixture.join("stderr.sub.txt");
    if stderr_golden.is_file() {
        let substring = substitute_today(&std::fs::read_to_string(&stderr_golden).unwrap());
        let substring = substring.trim();
        assert!(
            stderr.contains(substring),
            "fixture {name}: stderr missing substring {substring:?}\n--- stderr ---\n{stderr}"
        );
    }

    for marker in ["must-exist.txt", "must-not-exist.txt"] {
        let marker_path = fixture.join(marker);
        if !marker_path.is_file() {
            continue;
        }
        let body = substitute_today(&std::fs::read_to_string(&marker_path).unwrap());
        for rel in body.lines().filter(|l| !l.trim().is_empty()) {
            let actual = scratch.path().join(rel);
            if marker == "must-exist.txt" {
                assert!(
                    actual.exists(),
                    "fixture {name}: expected path {rel:?} to exist after command"
                );
            } else {
                assert!(
                    !actual.exists(),
                    "fixture {name}: expected path {rel:?} to be absent after command"
                );
            }
        }
    }

    let after = fixture.join("after");
    if after.is_dir() {
        for entry in walkdir::WalkDir::new(&after) {
            let entry = entry.unwrap();
            if !entry.file_type().is_file() {
                continue;
            }
            let rel = entry.path().strip_prefix(&after).unwrap();
            let actual_path = scratch.path().join(rel);
            assert!(
                actual_path.is_file(),
                "fixture {name}: expected file {rel:?} missing after command"
            );
            let expected = substitute_today(&std::fs::read_to_string(entry.path()).unwrap());
            let actual = std::fs::read_to_string(&actual_path).unwrap();
            assert_eq!(actual, expected, "fixture {name}: byte diff at {rel:?}");
        }
    }
}

/// Minimal POSIX-style splitter: handles double-quoted and single-quoted tokens.
fn shell_split(line: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut current = String::new();
    let mut chars = line.chars().peekable();
    let mut in_single = false;
    let mut in_double = false;
    while let Some(c) = chars.next() {
        match c {
            '\'' if !in_double => in_single = !in_single,
            '"' if !in_single => in_double = !in_double,
            ' ' | '\t' if !in_single && !in_double => {
                if !current.is_empty() {
                    out.push(std::mem::take(&mut current));
                }
            }
            '\\' if !in_single => {
                if let Some(next) = chars.next() {
                    current.push(next);
                }
            }
            other => current.push(other),
        }
    }
    if !current.is_empty() {
        out.push(current);
    }
    out
}

// ---------------------------------------------------------------------------
// Fixtures are discovered at test compile time via a manual dispatch list.
// Add a new test function per fixture directory.
// ---------------------------------------------------------------------------

#[test]
fn golden_01_init_minimal() {
    run_fixture("01-init-minimal");
}

#[test]
fn golden_02_start_seeds_spec_and_status() {
    run_fixture("02-start-seeds-spec");
}

#[test]
fn golden_03_status_reports_missing_tasks() {
    run_fixture("03-status-missing-tasks");
}

#[test]
fn golden_04_drift_d1() {
    run_fixture("04-drift-d1");
}

#[test]
fn golden_06_drift_d2() {
    run_fixture("06-drift-d2");
}

#[test]
fn golden_07_drift_d3() {
    run_fixture("07-drift-d3");
}

#[test]
fn golden_09_plan_finalize() {
    run_fixture("09-plan-finalize");
}

#[test]
fn golden_10_close_closes_in_place() {
    run_fixture("10-close-in-place");
}

#[test]
fn golden_12_milestone_tick() {
    run_fixture("12-milestone-tick");
}
