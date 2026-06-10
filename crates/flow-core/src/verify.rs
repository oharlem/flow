//! Test command detection + invocation.

use crate::config::Config;
use std::path::Path;
use std::process::{Command, ExitStatus};

/// Configured test runner for the repository.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TestRunner {
    /// The command to execute (as shell text).
    pub command: String,
    /// Optional pre-command.
    pub pre_command: Option<String>,
    /// Timeout in seconds.
    pub timeout_seconds: u64,
}

/// Return the configured or auto-detected test runner for this repository.
pub fn detect(repo: &Path, cfg: &Config) -> Option<TestRunner> {
    if let Some(cmd) = cfg
        .test
        .command
        .clone()
        .filter(|cmd| !cmd.trim().is_empty())
    {
        return Some(TestRunner {
            command: cmd,
            pre_command: cfg.test.pre_command.clone(),
            timeout_seconds: cfg.test.timeout_seconds,
        });
    }

    if detects_cargo_workspace(repo) {
        return Some(TestRunner {
            command: "cargo test --workspace".to_string(),
            pre_command: cfg.test.pre_command.clone(),
            timeout_seconds: cfg.test.timeout_seconds,
        });
    }

    None
}

fn detects_cargo_workspace(repo: &Path) -> bool {
    let manifest = repo.join("Cargo.toml");
    let Ok(text) = std::fs::read_to_string(manifest) else {
        return false;
    };

    has_manifest_table(&text, "package") || workspace_members_non_empty(&text)
}

fn has_manifest_table(text: &str, table: &str) -> bool {
    let needle = format!("[{table}]");
    text.lines().any(|line| line.trim() == needle)
}

fn workspace_members_non_empty(text: &str) -> bool {
    let mut in_workspace = false;
    let mut lines = text.lines().peekable();

    while let Some(line) = lines.next() {
        let trimmed = strip_comment(line).trim();
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            in_workspace = trimmed == "[workspace]";
            continue;
        }
        if !in_workspace || !trimmed.starts_with("members") {
            continue;
        }

        let Some((_, value)) = trimmed.split_once('=') else {
            continue;
        };
        let value = value.trim();
        if value.starts_with('[') && value.ends_with(']') {
            return !value
                .trim_start_matches('[')
                .trim_end_matches(']')
                .trim()
                .is_empty();
        }
        if value.starts_with('[') {
            for member_line in lines.by_ref() {
                let member = strip_comment(member_line).trim();
                if member.starts_with(']') {
                    return false;
                }
                if !member.is_empty() {
                    return true;
                }
            }
        }
    }

    false
}

fn strip_comment(line: &str) -> &str {
    line.split_once('#').map_or(line, |(before, _)| before)
}

/// Run the detected test runner in `repo`. Returns the process exit status.
pub fn run(repo: &Path, runner: &TestRunner) -> std::io::Result<ExitStatus> {
    run_with_env_removed(repo, runner, &[])
}

/// Run the detected test runner after removing selected environment variables
/// from the child processes.
pub fn run_with_env_removed(
    repo: &Path,
    runner: &TestRunner,
    env_remove: &[&str],
) -> std::io::Result<ExitStatus> {
    if let Some(pre) = &runner.pre_command {
        let status = shell_command(repo, pre, env_remove).status()?;
        if !status.success() {
            return Ok(status);
        }
    }
    shell_command(repo, &runner.command, env_remove).status()
}

fn shell_command(repo: &Path, command: &str, env_remove: &[&str]) -> Command {
    let mut cmd = Command::new("sh");
    cmd.arg("-c").arg(command).current_dir(repo);
    for name in env_remove {
        cmd.env_remove(name);
    }
    cmd
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn configured_command_takes_precedence() {
        let repo = tempfile::tempdir().unwrap();
        std::fs::write(
            repo.path().join("Cargo.toml"),
            "[package]\nname = \"fixture\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
        )
        .unwrap();
        let mut cfg = Config::default();
        cfg.test.command = Some("cargo test -p fixture".to_string());

        let runner = detect(repo.path(), &cfg).unwrap();
        assert_eq!(runner.command, "cargo test -p fixture");
    }

    #[test]
    fn detects_root_cargo_package() {
        let repo = tempfile::tempdir().unwrap();
        std::fs::write(
            repo.path().join("Cargo.toml"),
            "[package]\nname = \"fixture\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
        )
        .unwrap();

        let runner = detect(repo.path(), &Config::default()).unwrap();
        assert_eq!(runner.command, "cargo test --workspace");
    }

    #[test]
    fn detects_non_empty_cargo_workspace() {
        let repo = tempfile::tempdir().unwrap();
        std::fs::write(
            repo.path().join("Cargo.toml"),
            "[workspace]\nmembers = [\n  \"crates/fixture\",\n]\n",
        )
        .unwrap();

        let runner = detect(repo.path(), &Config::default()).unwrap();
        assert_eq!(runner.command, "cargo test --workspace");
    }

    #[test]
    fn ignores_empty_virtual_cargo_workspace() {
        let repo = tempfile::tempdir().unwrap();
        std::fs::write(
            repo.path().join("Cargo.toml"),
            "[workspace]\nmembers = []\n\n[workspace.package]\nversion = \"1.0.0\"\n",
        )
        .unwrap();

        assert!(detect(repo.path(), &Config::default()).is_none());
    }

    #[test]
    fn run_with_env_removed_removes_selected_environment_from_child_process() {
        let repo = tempfile::tempdir().unwrap();
        const ENV_NAME: &str = "FLOW_VERIFY_TEST_HOST_MARKER";
        std::env::set_var(ENV_NAME, "present");
        let runner = TestRunner {
            command: format!("test -z \"${ENV_NAME}\""),
            pre_command: Some(format!("test -z \"${ENV_NAME}\"")),
            timeout_seconds: 600,
        };

        let status = run_with_env_removed(repo.path(), &runner, &[ENV_NAME]).unwrap();
        std::env::remove_var(ENV_NAME);

        assert!(status.success());
    }

    #[test]
    fn run_with_env_removed_stops_when_pre_command_fails() {
        let repo = tempfile::tempdir().unwrap();
        let marker = repo.path().join("main-command-ran");
        let runner = TestRunner {
            command: format!("touch {}", marker.display()),
            pre_command: Some("exit 1".to_string()),
            timeout_seconds: 600,
        };

        let status = run_with_env_removed(repo.path(), &runner, &[]).unwrap();

        assert!(!status.success());
        assert!(!marker.exists());
    }
}
