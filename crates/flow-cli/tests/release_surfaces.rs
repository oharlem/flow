//! Release-surface regression tests.
//!
//! Task anchors:
//! - T-001: release version metadata stays aligned.
//! - T-003: changelog names the first public prototype release surface.
//! - T-004: security docs describe the released install model.
//! - T-005: these checks participate in the workspace verification gate.
//!
//! T-002 (Homebrew formula template version sync) was removed when the
//! Homebrew tap was dropped — see ADR-0017.

use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest_dir
        .parent()
        .and_then(Path::parent)
        .map(Path::to_path_buf)
        .expect("expected workspace root above crates/flow-cli")
}

fn read(rel: &str) -> String {
    let path = repo_root().join(rel);
    std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("expected to read {}: {e}", path.display()))
}

#[test]
fn t001_release_version_surfaces_match_package_version() {
    let version = env!("CARGO_PKG_VERSION");

    assert_eq!(read(".flow/version").trim(), version);
    assert!(read("Cargo.lock").contains(&format!("name = \"flow-cli\"\nversion = \"{version}\"")));
}

#[test]
fn t003_changelog_names_install_boundary_release_work() {
    let version = env!("CARGO_PKG_VERSION");
    let changelog = read("CHANGELOG.md");

    assert!(changelog.contains(&format!("## [{version}]")));
    assert!(changelog.contains("First public early prototype release line"));
    assert!(changelog.contains("Cargo-only"));
    assert!(changelog.contains("GitHub-first"));
    assert!(changelog.contains("Claude Code, Codex, and Cursor"));
}

#[test]
fn t001_t002_changelog_starts_at_public_prototype() {
    let changelog = read("CHANGELOG.md");

    assert!(changelog.contains("## [0.1.0]"));
    assert!(changelog.contains("First public early prototype release line"));
    assert!(!changelog.contains("## [0.10.0]"));
    assert!(!changelog.contains(concat!("flow ", "learn")));
    assert!(!changelog.contains(concat!("flow ", "next")));
}

#[test]
fn t002_release_docs_keep_flow_git_safety_rule() {
    let readme = read("README.md");
    let security = read("docs/security.md");

    assert!(security.contains("git push"));
    assert!(security.contains("git pull"));
    assert!(security.contains("git fetch"));
    assert!(security.contains("gh"));
    assert!(security.contains("glab"));
    assert!(security.contains("tag"));

    assert!(readme.contains("pushes"));
    assert!(readme.contains("pulls"));
    assert!(readme.contains("fetch"));
    assert!(readme.contains("tags"));
    assert!(readme.contains("gh"));
    assert!(readme.contains("glab"));
    assert!(security.contains("force operations"));
    assert!(readme.contains("checkpoint commits"));
    assert!(readme.contains("roadmap runs"));
}

#[test]
fn t004_release_docs_describe_current_install_model() {
    let hosts = read("docs/hosts.md");
    let security = read("docs/security.md");
    let commands = read("docs/reference/commands.md");

    assert!(hosts.contains("Generated host assets invoke the installed `flow` binary"));
    assert!(hosts.contains("FLOW_HOST=<host>"));
    assert!(!hosts.contains(".flow/bin/flow"));
    assert!(security.contains("flow export-assets --dir <DIR>"));
    assert!(commands.contains("Host adapters"));
}
