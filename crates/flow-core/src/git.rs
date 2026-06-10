//! Thin git-process wrapper. Flow shells out to `git(1)`; no remote commands.

use crate::error::{Error, Result};
use std::path::Path;
use std::process::Command;

/// Return the current short branch name, or `"HEAD"` when detached.
pub fn current_branch(repo: &Path) -> Result<String> {
    let out = Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .current_dir(repo)
        .output()?;
    if !out.status.success() {
        return Err(Error::User("git rev-parse failed".into()));
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

/// Return `true` when the working tree or index has any uncommitted changes.
pub fn is_dirty(repo: &Path) -> Result<bool> {
    let out = Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(repo)
        .output()?;
    Ok(!out.stdout.is_empty())
}

/// Return repo-relative paths with uncommitted worktree or index changes.
pub fn dirty_paths(repo: &Path) -> Result<Vec<String>> {
    let out = Command::new("git")
        .args(["status", "--porcelain", "--untracked-files=all"])
        .current_dir(repo)
        .output()?;
    if !out.status.success() {
        return Err(Error::User("git status failed".into()));
    }
    Ok(String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(parse_status_path)
        .collect())
}

fn parse_status_path(line: &str) -> Option<String> {
    let path = line.get(3..)?.trim();
    if path.is_empty() {
        return None;
    }
    let path = path
        .rsplit_once(" -> ")
        .map_or(path, |(_, target)| target)
        .trim_matches('"')
        .trim_end_matches('/');
    (!path.is_empty()).then(|| path.to_string())
}

/// Stage all local changes in the repository.
pub fn stage_all(repo: &Path) -> Result<()> {
    let output = Command::new("git")
        .args(["add", "-A"])
        .current_dir(repo)
        .output()?;
    if !output.status.success() {
        return Err(Error::User(git_failure_message(
            "git add -A failed",
            &command_detail(&output),
        )));
    }
    Ok(())
}

/// Stage all changes under the given paths only.
pub fn stage_paths(repo: &Path, paths: &[&Path]) -> Result<()> {
    let output = Command::new("git")
        .args(["add", "-A", "--"])
        .args(paths)
        .current_dir(repo)
        .output()?;
    if !output.status.success() {
        return Err(Error::User(git_failure_message(
            "git add failed",
            &command_detail(&output),
        )));
    }
    Ok(())
}

/// Create a local commit restricted to the given paths, leaving anything
/// staged outside them untouched, and return the new full HEAD SHA.
pub fn commit_paths(repo: &Path, message: &str, paths: &[&Path]) -> Result<String> {
    let output = Command::new("git")
        .args(["commit", "-m", message, "--"])
        .args(paths)
        .current_dir(repo)
        .output()?;
    if !output.status.success() {
        return Err(Error::User(git_failure_message(
            "git commit failed",
            &command_detail(&output),
        )));
    }
    head_sha(repo)
}

/// Create a local commit and return the new full HEAD SHA.
pub fn commit(repo: &Path, message: &str) -> Result<String> {
    let output = Command::new("git")
        .args(["commit", "-m", message])
        .current_dir(repo)
        .output()?;
    if !output.status.success() {
        return Err(Error::User(git_failure_message(
            "git commit failed",
            &command_detail(&output),
        )));
    }
    head_sha(repo)
}

/// Return the full SHA for HEAD.
pub fn head_sha(repo: &Path) -> Result<String> {
    let out = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(repo)
        .output()?;
    if !out.status.success() {
        return Err(Error::User("git rev-parse HEAD failed".into()));
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

/// Return the short SHA for HEAD.
pub fn short_head(repo: &Path) -> Result<String> {
    let out = Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .current_dir(repo)
        .output()?;
    if !out.status.success() {
        return Err(Error::User("git rev-parse --short HEAD failed".into()));
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

fn command_detail(output: &std::process::Output) -> String {
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if stderr.is_empty() {
        stdout
    } else {
        stderr
    }
}

/// Create and switch to a new branch.
pub fn create_branch(repo: &Path, name: &str) -> Result<()> {
    let output = Command::new("git")
        .args(["switch", "-c", name])
        .current_dir(repo)
        .output()?;
    if !output.status.success() {
        return Err(branch_command_error("create", name, &output));
    }
    Ok(())
}

/// Switch to an existing local branch.
pub fn switch_branch(repo: &Path, name: &str) -> Result<()> {
    let output = Command::new("git")
        .args(["switch", name])
        .current_dir(repo)
        .output()?;
    if !output.status.success() {
        return Err(branch_command_error("switch to", name, &output));
    }
    Ok(())
}

fn branch_command_error(action: &str, name: &str, output: &std::process::Output) -> Error {
    Error::User(branch_error_message(action, name, &command_detail(output)))
}

fn branch_error_message(action: &str, name: &str, detail: &str) -> String {
    let permission_hint = permission_denied_hint(detail);
    let ref_namespace_hint = if permission_hint.is_empty()
        && (detail.contains("cannot lock ref")
            || detail.contains("exists; cannot create")
            || detail.contains("not a directory"))
    {
        " This looks like a git ref namespace conflict: another branch or ref already owns part of that branch path."
    } else {
        ""
    };
    format!(
        "could not {action} Flow branch '{name}'.{permission_hint}{ref_namespace_hint} Git reported: {detail}"
    )
}

/// Hint appended when git output indicates a filesystem permission failure,
/// e.g. an agent sandbox that allows reading `.git` but blocks ref writes.
/// Checked before the ref-namespace hint: git reports sandbox-blocked ref
/// writes as `cannot lock ref ... Permission denied`, which would otherwise
/// be misclassified as a branch-name conflict.
fn permission_denied_hint(detail: &str) -> &'static str {
    let lower = detail.to_lowercase();
    if lower.contains("permission denied")
        || lower.contains("operation not permitted")
        || lower.contains("read-only file system")
    {
        " Flow could not write to `.git`. If this command is running inside an agent sandbox, re-run it with escalated permissions or allow git writes for this workspace."
    } else {
        ""
    }
}

fn git_failure_message(summary: &str, detail: &str) -> String {
    format!(
        "{summary}.{hint} Git reported: {detail}",
        hint = permission_denied_hint(detail)
    )
}

/// Return `true` if a branch with the given name already exists locally.
pub fn branch_exists(repo: &Path, name: &str) -> Result<bool> {
    let out = Command::new("git")
        .args([
            "show-ref",
            "--verify",
            "--quiet",
            &format!("refs/heads/{name}"),
        ])
        .current_dir(repo)
        .status()?;
    Ok(out.success())
}

/// Return `true` when the given path exists inside a git repository.
#[must_use]
pub fn is_git_repo(path: &Path) -> bool {
    Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .current_dir(path)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Return `true` when the named branch is a protected branch per `patterns`.
///
/// Supports bare names (`main`) and glob patterns (`release/*`). The match
/// is case-sensitive.
#[must_use]
pub fn branch_is_protected(branch: &str, patterns: &[String]) -> bool {
    for pat in patterns {
        if pat == branch {
            return true;
        }
        if let Ok(compiled) = glob::Pattern::new(pat) {
            if compiled.matches(branch) {
                return true;
            }
        }
    }
    false
}

/// Return the merge-base commit SHA between branches `a` and `b`. (T-003)
///
/// Fails with `Error::User` when the operation fails, e.g. when either
/// branch does not exist locally.
pub fn merge_base(repo: &Path, a: &str, b: &str) -> Result<String> {
    let out = Command::new("git")
        .args(["merge-base", a, b])
        .current_dir(repo)
        .output()?;
    if !out.status.success() {
        return Err(Error::User(format!(
            "git merge-base {a} {b} failed — '{b}' may not exist locally"
        )));
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

/// Return repo-relative paths of files changed between `base` commit and the
/// working tree. (T-003)
///
/// Runs `git diff --name-only <base>`, which covers committed changes on the
/// current branch plus any unstaged modifications to tracked files.
pub fn diff_files(repo: &Path, base: &str) -> Result<Vec<std::path::PathBuf>> {
    let out = Command::new("git")
        .args(["diff", "--name-only", base])
        .current_dir(repo)
        .output()?;
    if !out.status.success() {
        return Err(Error::User(format!("git diff --name-only {base} failed")));
    }
    Ok(String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter(|l| !l.is_empty())
        .map(std::path::PathBuf::from)
        .collect())
}

/// Create a git worktree for `branch` at `<repo>/../<repo>-<worktree_name>`.
pub fn create_worktree(
    repo: &Path,
    branch: &str,
    worktree_name: &str,
) -> Result<std::path::PathBuf> {
    let parent = repo
        .parent()
        .ok_or_else(|| Error::User("repo has no parent".into()))?;
    let base = repo.file_name().and_then(|s| s.to_str()).unwrap_or("repo");
    let target = parent.join(format!("{base}-{worktree_name}"));
    let output = Command::new("git")
        .args([
            "worktree",
            "add",
            target
                .to_str()
                .ok_or_else(|| Error::User("bad path".into()))?,
            "-b",
            branch,
        ])
        .current_dir(repo)
        .output()?;
    if !output.status.success() {
        let detail = command_detail(&output);
        return Err(Error::User(git_failure_message(
            &format!(
                "could not create worktree for Flow branch '{name}' at {}",
                target.display(),
                name = branch,
            ),
            &detail,
        )));
    }
    Ok(target)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_protected_via_glob() {
        assert!(branch_is_protected("main", &["main".to_string()]));
        assert!(branch_is_protected(
            "release/1.0",
            &["release/*".to_string()]
        ));
        assert!(!branch_is_protected("feature/a", &["main".to_string()]));
    }

    const SANDBOX_HINT: &str = "Flow could not write to `.git`";
    const NAMESPACE_HINT: &str = "git ref namespace conflict";

    #[test]
    fn branch_error_permission_denied_wins_over_namespace_hint() {
        let detail = "fatal: cannot lock ref 'refs/heads/flow/run-20260610-roadmap-x': \
             Unable to create '/repo/.git/refs/heads/flow/run-20260610-roadmap-x.lock': \
             Permission denied";
        let msg = branch_error_message("create", "flow/run-20260610-roadmap-x", detail);
        assert!(msg.contains(SANDBOX_HINT), "missing sandbox hint: {msg}");
        assert!(
            !msg.contains(NAMESPACE_HINT),
            "permission failure misclassified as namespace conflict: {msg}"
        );
    }

    #[test]
    fn branch_error_real_namespace_conflict_keeps_namespace_hint() {
        let detail = "fatal: cannot lock ref 'refs/heads/flow/run-x': \
             'refs/heads/flow' exists; cannot create 'refs/heads/flow/run-x'";
        let msg = branch_error_message("create", "flow/run-x", detail);
        assert!(
            msg.contains(NAMESPACE_HINT),
            "missing namespace hint: {msg}"
        );
        assert!(!msg.contains(SANDBOX_HINT));
    }

    #[test]
    fn branch_error_other_failures_get_no_hint() {
        let msg = branch_error_message("switch to", "flow/run-x", "fatal: invalid reference");
        assert!(!msg.contains(SANDBOX_HINT));
        assert!(!msg.contains(NAMESPACE_HINT));
        assert!(msg.contains("Git reported: fatal: invalid reference"));
    }

    #[test]
    fn permission_hint_matches_sandbox_error_variants() {
        for detail in [
            "error: Permission denied",
            "fatal: Operation not permitted",
            "fatal: Unable to write new index file: Read-only file system",
        ] {
            assert!(
                !permission_denied_hint(detail).is_empty(),
                "expected sandbox hint for: {detail}"
            );
        }
        assert!(permission_denied_hint("fatal: bad object HEAD").is_empty());
    }

    #[test]
    fn commit_failure_message_carries_permission_hint() {
        let msg = git_failure_message(
            "git commit failed",
            "error: unable to write commit object: Permission denied",
        );
        assert!(msg.contains(SANDBOX_HINT));
        let clean = git_failure_message("git commit failed", "nothing to commit");
        assert_eq!(clean, "git commit failed. Git reported: nothing to commit");
    }
}
