//! Task-scoped preflight requirement checks.

use crate::{
    config::{Config, PreflightRequirementConfig},
    parse::tasks::Task,
    Error, Result,
};
use std::{
    collections::BTreeMap,
    path::Path,
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};

const DEFAULT_TIMEOUT_SECONDS: u64 = 30;

/// A runnable preflight requirement.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Requirement {
    /// Stable requirement ID used in `tasks.md`.
    pub id: String,
    /// Human-readable description.
    pub description: String,
    /// Shell command that must succeed.
    pub command: String,
    /// User-facing remediation when the command fails.
    pub remediation: String,
    /// Maximum time to wait for the check command.
    pub timeout_seconds: u64,
}

/// Result of running the task-scoped preflight checks.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Report {
    /// Failed requirement checks.
    pub failures: Vec<Failure>,
}

impl Report {
    /// Return true when at least one required check failed.
    #[must_use]
    pub fn is_blocked(&self) -> bool {
        !self.failures.is_empty()
    }
}

/// A failed preflight check.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Failure {
    /// Failed requirement.
    pub requirement: Requirement,
    /// Task IDs that required this check.
    pub task_ids: Vec<String>,
    /// Failure reason.
    pub reason: FailureReason,
}

/// Why a preflight check failed.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FailureReason {
    /// The check command exited with a non-zero status.
    Exit(Option<i32>),
    /// The check command exceeded its timeout.
    Timeout,
    /// The check command could not be started or polled.
    Io(String),
}

impl FailureReason {
    fn render(&self) -> String {
        match self {
            Self::Exit(Some(code)) => format!("exit code {code}"),
            Self::Exit(None) => "terminated by signal".to_string(),
            Self::Timeout => "timed out".to_string(),
            Self::Io(message) => format!("runner error: {message}"),
        }
    }
}

/// Build the effective requirement catalog for a repository config.
///
/// Built-ins are inserted first, then `.flow/config.yaml` entries override or
/// extend them.
pub fn catalog(cfg: &Config) -> Result<BTreeMap<String, Requirement>> {
    let mut out = BTreeMap::new();
    out.insert(
        "docker".to_string(),
        Requirement {
            id: "docker".to_string(),
            description: "Docker daemon is running".to_string(),
            command: "docker info >/dev/null 2>&1".to_string(),
            remediation: "Start Docker Desktop or the Docker daemon, then rerun the Flow command."
                .to_string(),
            timeout_seconds: DEFAULT_TIMEOUT_SECONDS,
        },
    );

    for (raw_id, override_cfg) in &cfg.preflight.requirements {
        let id = normalize_requirement(raw_id);
        if id.is_empty() {
            return Err(Error::Config(
                "preflight requirement IDs must not be empty".to_string(),
            ));
        }
        let base = out.get(&id);
        let requirement = merge_requirement(&id, base, override_cfg)?;
        out.insert(id, requirement);
    }

    Ok(out)
}

/// Render the known preflight requirements for agent envelopes.
pub fn render_known_requirements(cfg: &Config) -> Result<String> {
    let catalog = catalog(cfg)?;
    let mut out = String::new();
    out.push_str("## Known Preflight Requirements\n\n");
    for requirement in catalog.values() {
        out.push_str(&format!(
            "- `{}` - {}\n",
            requirement.id, requirement.description
        ));
    }
    Ok(out)
}

/// Validate that every `Requires:` ID in `tasks` is known.
pub fn validate_task_requirements(tasks: &[Task], cfg: &Config) -> Result<()> {
    let catalog = catalog(cfg)?;
    for task in tasks {
        for requirement in &task.requires {
            let id = normalize_requirement(requirement);
            if !catalog.contains_key(&id) {
                return Err(unknown_requirement_error(&task.id, &id));
            }
        }
    }
    Ok(())
}

/// Run preflight checks required by `tasks`.
pub fn run_for_tasks(repo: &Path, cfg: &Config, tasks: &[&Task]) -> Result<Report> {
    let catalog = catalog(cfg)?;
    let required = requirements_for_tasks(tasks);
    let mut failures = Vec::new();

    for (id, task_ids) in required {
        let Some(requirement) = catalog.get(&id) else {
            let task_id = task_ids.first().map_or("(unknown)", String::as_str);
            return Err(unknown_requirement_error(task_id, &id));
        };
        if let Some(reason) = run_requirement(repo, requirement) {
            failures.push(Failure {
                requirement: requirement.clone(),
                task_ids,
                reason,
            });
        }
    }

    Ok(Report { failures })
}

/// Render a failed preflight report for CLI output.
#[must_use]
pub fn render_blocked(report: &Report) -> String {
    let mut out = String::new();
    out.push_str("Blocked by environment:\n\n");
    for failure in &report.failures {
        out.push_str(&format!(
            "- `{}` ({}) failed for {}.\n",
            failure.requirement.id,
            failure.requirement.description,
            failure.task_ids.join(", ")
        ));
        out.push_str(&format!("  Command: `{}`\n", failure.requirement.command));
        out.push_str(&format!("  Result: {}\n", failure.reason.render()));
        out.push_str(&format!(
            "  Remediation: {}\n",
            failure.requirement.remediation
        ));
    }
    out
}

fn merge_requirement(
    id: &str,
    base: Option<&Requirement>,
    cfg: &PreflightRequirementConfig,
) -> Result<Requirement> {
    let description = cfg
        .description
        .clone()
        .or_else(|| base.map(|r| r.description.clone()))
        .unwrap_or_else(|| id.to_string());
    let command = cfg
        .command
        .clone()
        .or_else(|| base.map(|r| r.command.clone()))
        .ok_or_else(|| {
            Error::Config(format!(
                "preflight requirement '{id}' must define a command"
            ))
        })?;
    let remediation = cfg
        .remediation
        .clone()
        .or_else(|| base.map(|r| r.remediation.clone()))
        .unwrap_or_else(|| {
            format!("Make requirement '{id}' available, then rerun the Flow command.")
        });
    let timeout_seconds = cfg
        .timeout_seconds
        .or_else(|| base.map(|r| r.timeout_seconds))
        .unwrap_or(DEFAULT_TIMEOUT_SECONDS)
        .max(1);

    Ok(Requirement {
        id: id.to_string(),
        description,
        command,
        remediation,
        timeout_seconds,
    })
}

fn requirements_for_tasks(tasks: &[&Task]) -> BTreeMap<String, Vec<String>> {
    let mut out: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for task in tasks {
        for requirement in &task.requires {
            out.entry(normalize_requirement(requirement))
                .or_default()
                .push(task.id.clone());
        }
    }
    out
}

fn run_requirement(repo: &Path, requirement: &Requirement) -> Option<FailureReason> {
    let mut child = match Command::new("sh")
        .arg("-c")
        .arg(&requirement.command)
        .current_dir(repo)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
    {
        Ok(child) => child,
        Err(e) => return Some(FailureReason::Io(e.to_string())),
    };

    let timeout = Duration::from_secs(requirement.timeout_seconds);
    let started = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(status)) if status.success() => return None,
            Ok(Some(status)) => return Some(FailureReason::Exit(status.code())),
            Ok(None) => {
                if started.elapsed() >= timeout {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Some(FailureReason::Timeout);
                }
                thread::sleep(Duration::from_millis(50));
            }
            Err(e) => return Some(FailureReason::Io(e.to_string())),
        }
    }
}

fn unknown_requirement_error(task_id: &str, requirement_id: &str) -> Error {
    Error::ArtifactError {
        file: "tasks.md".into(),
        message: format!(
            "unknown preflight requirement '{requirement_id}' in {task_id}; define it under preflight.requirements in .flow/config.yaml or remove the Requires entry"
        ),
    }
}

fn normalize_requirement(id: &str) -> String {
    id.trim().to_ascii_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::tasks;

    #[test]
    fn catalog_includes_builtin_docker() {
        let cfg = Config::default();
        let catalog = catalog(&cfg).unwrap();
        let docker = catalog.get("docker").unwrap();
        assert_eq!(docker.description, "Docker daemon is running");
        assert!(docker.command.contains("docker info"));
    }

    #[test]
    fn catalog_adds_custom_requirements() {
        let cfg: Config = serde_yaml::from_str(
            r#"
preflight:
  requirements:
    postgres:
      description: Local Postgres is ready
      command: pg_isready
      remediation: Start Postgres.
"#,
        )
        .unwrap();
        let catalog = catalog(&cfg).unwrap();
        assert!(catalog.contains_key("docker"));
        assert_eq!(catalog["postgres"].command, "pg_isready");
    }

    #[test]
    fn validates_unknown_task_requirements() {
        let cfg = Config::default();
        let tasks = tasks::parse_str("- [ ] **T-001**: x\n    - Requires: postgres\n");
        let err = validate_task_requirements(&tasks, &cfg).unwrap_err();
        assert!(err
            .to_string()
            .contains("unknown preflight requirement 'postgres'"));
    }

    #[test]
    fn runs_only_requirements_declared_by_tasks() {
        let cfg: Config = serde_yaml::from_str(
            r#"
preflight:
  requirements:
    never:
      command: exit 1
"#,
        )
        .unwrap();
        let tasks = tasks::parse_str("- [ ] **T-001**: x\n");
        let task_refs: Vec<&Task> = tasks.iter().collect();
        let report = run_for_tasks(Path::new("."), &cfg, &task_refs).unwrap();
        assert!(!report.is_blocked());
    }
}
