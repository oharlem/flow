//! Install/release-surface guard tests.
//!
//! Flow v0.1.0 installs from GitHub via `cargo install --git`
//! (see ADR-0017). This file asserts the Cargo-only posture so a future
//! contributor cannot add a release pipeline, Homebrew tap, crates.io-first
//! publish path, or shell/PowerShell installer without also updating
//! documentation and ADR-0017.

use std::path::PathBuf;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
}

fn ci_workflow() -> String {
    std::fs::read_to_string(repo_root().join(".github").join("workflows").join("ci.yml"))
        .expect("CI workflow")
}

fn doc(path: &str) -> String {
    std::fs::read_to_string(repo_root().join(path)).unwrap_or_else(|err| {
        panic!("could not read {path}: {err}");
    })
}

#[test]
fn cargo_only_artifacts_are_gone() {
    let root = repo_root();
    let removed = [
        ".github/workflows/release.yml",
        "installers/install.sh",
        "installers/install.ps1",
        "installers/homebrew/flow.rb.tmpl",
        "installers",
        "dist-workspace.toml",
    ];
    for path in removed {
        assert!(
            !root.join(path).exists(),
            "ADR-0017 (Cargo Git install) forbids `{path}`; remove it or update the ADR"
        );
    }
}

#[test]
fn ci_runs_automatically_and_has_no_release_workflow() {
    let ci = ci_workflow();

    assert!(
        ci.contains("pull_request:"),
        "CI should run automatically on pull requests"
    );
    assert!(
        ci.contains("branches: [main]"),
        "CI should run automatically on pushes to main"
    );
    assert!(
        ci.contains("workflow_dispatch:"),
        "CI should also be manually dispatchable"
    );
    assert!(
        !ci.contains("installer-paths"),
        "ADR-0017 removed the installer-paths job; do not reintroduce without updating the ADR"
    );
    assert!(
        !ci.contains("PSScriptAnalyzer"),
        "ADR-0017 removed the PowerShell installer lint; do not reintroduce without updating the ADR"
    );
}

#[test]
fn docs_advertise_cargo_git_install() {
    let readme = doc("README.md");
    let security = doc("docs/security.md");
    let commands = doc("docs/reference/commands.md");

    for (path, text) in [
        ("README.md", &readme),
        ("docs/security.md", &security),
        ("docs/reference/commands.md", &commands),
    ] {
        assert!(
            !text.contains("brew install"),
            "{path}: should not advertise Homebrew install (ADR-0017)"
        );
        assert!(
            !text.contains("install.sh"),
            "{path}: should not advertise the shell installer (ADR-0017)"
        );
        assert!(
            !text.contains("install.ps1"),
            "{path}: should not advertise the PowerShell installer (ADR-0017)"
        );
    }

    assert!(
        readme.contains("cargo install --git https://github.com/oharlem/flow --locked flow-cli"),
        "README must show the supported install command"
    );
    assert!(
        readme.contains("early prototype"),
        "README must describe the v0.1.0 state honestly"
    );
    assert!(
        readme.contains("crates.io publish is not required"),
        "README must make clear crates.io is not required for v0.1.0"
    );
    assert!(
        readme.contains("rustup"),
        "README must mention that a Rust toolchain is required"
    );
    assert!(
        readme.contains("0017-cargo-only-install"),
        "README must link to ADR-0017"
    );
    assert!(
        !repo_root().join("docs").join("release.md").exists(),
        "docs/release.md is no longer part of the published docs set"
    );
}
