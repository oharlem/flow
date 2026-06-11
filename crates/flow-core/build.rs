//! Build-time source stamp for Cargo git installs.

use std::path::{Path, PathBuf};
use std::process::Command;

fn main() {
    let sha = git_output(["rev-parse", "--short=12", "HEAD"]).unwrap_or_else(|| "unknown".into());
    println!("cargo:rustc-env=FLOW_CORE_GIT_SHA={sha}");

    println!("cargo:rerun-if-changed=build.rs");
    watch_git_head();
}

fn git_output<const N: usize>(args: [&str; N]) -> Option<String> {
    let out = Command::new("git").args(args).output().ok()?;
    out.status
        .success()
        .then(|| String::from_utf8_lossy(&out.stdout).trim().to_string())
        .filter(|s| !s.is_empty())
}

fn watch_git_head() {
    let Some(git_dir) = git_output(["rev-parse", "--git-dir"]) else {
        return;
    };
    let git_dir = absolutize_git_path(&git_dir);
    let head = git_dir.join("HEAD");
    if !head.exists() {
        return;
    }

    println!("cargo:rerun-if-changed={}", head.display());

    let Ok(head_text) = std::fs::read_to_string(&head) else {
        return;
    };
    let Some(reference) = head_text.trim().strip_prefix("ref: ") else {
        return;
    };

    let ref_path = git_dir.join(reference);
    if ref_path.exists() {
        println!("cargo:rerun-if-changed={}", ref_path.display());
    }

    let packed_refs = git_dir.join("packed-refs");
    if packed_refs.exists() {
        println!("cargo:rerun-if-changed={}", packed_refs.display());
    }
}

fn absolutize_git_path(path: &str) -> PathBuf {
    let path = Path::new(path);
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        Path::new(env!("CARGO_MANIFEST_DIR")).join(path)
    }
}
