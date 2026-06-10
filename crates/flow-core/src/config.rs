//! `.flow/config.yaml` loader.

use crate::error::{Error, Result};
use crate::settings::{ConfirmationSetting, ReviewSetting};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;

/// Root of the Flow project configuration.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    /// Schema version of this config file.
    pub schema_version: f32,
    /// Whether Flow phase confirmations are enabled.
    pub confirmation: ConfirmationSetting,
    /// Visible Flow artifact root and git branch namespace.
    pub prefix: String,
    /// Hosts this project is wired for.
    pub hosts: Vec<String>,
    /// Git settings.
    pub git: GitConfig,
    /// Test settings.
    pub test: TestConfig,
    /// Task-scoped preflight requirement checks.
    pub preflight: PreflightConfig,
    /// Per-phase effort/model overrides.
    pub phases: Phases,
    /// UI settings.
    pub ui: UiConfig,
    /// Documentation freshness settings. (T-002)
    pub docs: DocsConfig,
    /// Installed artifact layout settings. (T-001)
    pub layout: LayoutConfig,
    /// Review-before-finalize settings (M-24).
    pub review: ReviewSetting,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            schema_version: 1.0,
            confirmation: ConfirmationSetting::No,
            prefix: "flow".to_string(),
            hosts: vec!["claude-code".to_string()],
            git: GitConfig::default(),
            test: TestConfig::default(),
            preflight: PreflightConfig::default(),
            phases: Phases::default(),
            ui: UiConfig::default(),
            docs: DocsConfig::default(),
            layout: LayoutConfig::default(),
            review: ReviewSetting::default(),
        }
    }
}

/// Installed Flow workspace layout settings. (T-001)
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct LayoutConfig {
    /// Installed artifact layout version.
    pub version: u8,
}

impl Default for LayoutConfig {
    fn default() -> Self {
        Self { version: 2 }
    }
}

/// Git-related config.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct GitConfig {
    /// Branches where Flow warns before running.
    pub protected_branches: Vec<String>,
    /// When true, each feature is created in a sibling git worktree.
    pub worktrees: bool,
    /// When true, `flow run` creates a local run branch at run creation.
    pub run_branch: bool,
    /// When true, roadmap-scoped runs create local checkpoint commits after
    /// each milestone closes successfully.
    pub run_checkpoint_commits: bool,
    /// Local default branch used as the base for `docs.touch_map` diff checks.
    /// When absent, Flow tries `main` then `master`. (T-001)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_branch: Option<String>,
}

impl Default for GitConfig {
    fn default() -> Self {
        Self {
            protected_branches: vec![
                "main".to_string(),
                "master".to_string(),
                "trunk".to_string(),
                "develop".to_string(),
                "release/*".to_string(),
            ],
            worktrees: false,
            run_branch: true,
            run_checkpoint_commits: true,
            default_branch: None,
        }
    }
}

/// Documentation freshness settings. (T-002)
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct DocsConfig {
    /// Days before a review marker is considered stale. `None` skips the age
    /// check; marker-presence checks still run when docs dirs exist.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub review_max_age_days: Option<u64>,
    /// Source-to-doc mappings evaluated by `flow doctor` and `flow close`.
    pub touch_map: Vec<TouchMapEntry>,
    /// When `false`, all touch-map warnings are globally suppressed.
    #[serde(default = "default_touch_map_warnings")]
    pub touch_map_warnings: bool,
    /// Optional repo-relative destination for Flow-maintained documentation. (T-006)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub documentation_path: Option<String>,
}

impl Default for DocsConfig {
    fn default() -> Self {
        Self {
            review_max_age_days: None,
            touch_map: Vec::new(),
            touch_map_warnings: true,
            documentation_path: None,
        }
    }
}

fn default_touch_map_warnings() -> bool {
    true
}

/// One entry in `docs.touch_map`. (T-002)
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct TouchMapEntry {
    /// Source-path glob patterns. When any matched file appears in the branch
    /// diff, the mapped `docs` paths are expected to be touched.
    pub paths: Vec<String>,
    /// Doc file paths (exact, repo-relative) that should appear in the diff.
    pub docs: Vec<String>,
    /// When `true`, warnings for this entry are suppressed.
    pub suppress: bool,
}

/// Test runner config.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct TestConfig {
    /// Optional test command override. When absent, Flow may auto-detect one.
    pub command: Option<String>,
    /// Optional command to run before the test command.
    pub pre_command: Option<String>,
    /// Timeout in seconds for the test command.
    #[serde(default = "default_timeout")]
    pub timeout_seconds: u64,
}

impl Default for TestConfig {
    fn default() -> Self {
        Self {
            command: None,
            pre_command: None,
            timeout_seconds: default_timeout(),
        }
    }
}

fn default_timeout() -> u64 {
    600
}

/// Task-scoped preflight checks.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct PreflightConfig {
    /// Project-defined or built-in requirement overrides keyed by requirement ID.
    pub requirements: BTreeMap<String, PreflightRequirementConfig>,
}

/// A single preflight requirement check.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct PreflightRequirementConfig {
    /// Human-readable requirement description.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Shell command that must succeed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
    /// User-facing remediation when the command fails.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remediation: Option<String>,
    /// Timeout in seconds for this check.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout_seconds: Option<u64>,
}

/// Per-phase overrides.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Phases {
    /// `setup` phase overrides.
    pub setup: PhaseConfig,
    /// `start` phase overrides.
    pub start: PhaseConfig,
    /// `amend` phase overrides.
    pub amend: PhaseConfig,
    /// `plan` phase overrides.
    pub plan: PhaseConfig,
    /// `build` phase overrides.
    pub build: PhaseConfig,
    /// `check` (a.k.a. `test`) phase overrides.
    pub check: PhaseConfig,
    /// `close` phase overrides.
    pub close: PhaseConfig,
}

/// A single phase's effort + model override.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct PhaseConfig {
    /// Reasoning effort hint: `"standard"` or `"high"`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub effort: Option<String>,
    /// Specific model ID or `null`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
}

/// UI-facing flags.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct UiConfig {
    /// Voice hint: `"friendly"`, `"terse"`, `"formal"`.
    pub voice: String,
    /// Ask agents to restrict output to ASCII.
    pub ascii_only: bool,
}

impl Default for UiConfig {
    fn default() -> Self {
        Self {
            voice: "friendly".to_string(),
            ascii_only: false,
        }
    }
}

impl Config {
    /// Parse a `.flow/config.yaml` at `path`. Missing file → [`Config::default`].
    pub fn load(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let text = std::fs::read_to_string(path)?;
        if text.trim().is_empty() {
            return Ok(Self::default());
        }
        let config: Self =
            serde_yaml::from_str(&text).map_err(|e| Error::Config(format!("{path:?}: {e}")))?;
        config.validate(path)?;
        Ok(config)
    }

    /// Load a config from a repository root.
    pub fn load_for_repo(repo: &Path) -> Result<Self> {
        Self::load(&repo.join(".flow").join("config.yaml"))
    }

    /// Persist config to `.flow/config.yaml`.
    pub fn save_for_repo(&self, repo: &Path) -> Result<()> {
        let flow_dir = repo.join(".flow");
        std::fs::create_dir_all(&flow_dir)?;
        let path = flow_dir.join("config.yaml");
        let tmp = path.with_file_name(format!(
            "{}.tmp.{}",
            path.file_name().unwrap_or_default().to_string_lossy(),
            std::process::id()
        ));
        let text = serde_yaml::to_string(self)
            .map_err(|e| Error::Config(format!("could not render config.yaml: {e}")))?;
        std::fs::write(&tmp, text)?;
        std::fs::rename(tmp, path)?;
        Ok(())
    }

    fn validate(&self, path: &Path) -> Result<()> {
        if self.layout.version != 2 {
            return Err(Error::Config(format!(
                "{}: layout.version must be 2",
                path.display()
            )));
        }
        Ok(())
    }
}

/// Persist the root `confirmation` setting while preserving unrelated config
/// comments and formatting where possible.
pub fn save_confirmation_for_repo(repo: &Path, confirmation: ConfirmationSetting) -> Result<()> {
    let flow_dir = repo.join(".flow");
    std::fs::create_dir_all(&flow_dir)?;
    let path = flow_dir.join("config.yaml");
    if !path.exists() {
        let config = Config {
            confirmation,
            ..Config::default()
        };
        return config.save_for_repo(repo);
    }

    let text = std::fs::read_to_string(&path)?;
    if text.trim().is_empty() {
        let config = Config {
            confirmation,
            ..Config::default()
        };
        return config.save_for_repo(repo);
    }

    let updated = upsert_root_scalar(
        &text,
        "confirmation",
        &format!("\"{}\"", confirmation.as_str()),
    );
    serde_yaml::from_str::<Config>(&updated)
        .map_err(|e| Error::Config(format!("{}: {e}", path.display())))?;
    let tmp = path.with_file_name(format!(
        "{}.tmp.{}",
        path.file_name().unwrap_or_default().to_string_lossy(),
        std::process::id()
    ));
    std::fs::write(&tmp, updated)?;
    std::fs::rename(tmp, path)?;
    Ok(())
}

fn upsert_root_scalar(text: &str, key: &str, rendered_value: &str) -> String {
    let mut found = false;
    let mut lines = Vec::new();
    for line in text.lines() {
        let trimmed = line.trim_start();
        let is_root_key = line.len() == trimmed.len()
            && trimmed
                .split_once(':')
                .is_some_and(|(candidate, _)| candidate == key);
        if is_root_key {
            lines.push(format!("{key}: {rendered_value}"));
            found = true;
        } else {
            lines.push(line.to_string());
        }
    }
    if !found {
        if lines.last().is_some_and(|line| !line.trim().is_empty()) {
            lines.push(String::new());
        }
        lines.push(format!("{key}: {rendered_value}"));
    }
    let mut updated = lines.join("\n");
    updated.push('\n');
    updated
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_sane() {
        let c = Config::default();
        assert_eq!(c.schema_version, 1.0);
        assert_eq!(c.confirmation.as_str(), "no");
        assert_eq!(c.prefix, "flow");
        assert_eq!(c.ui.voice, "friendly");
        assert_eq!(c.test.timeout_seconds, 600);
        assert_eq!(c.layout.version, 2);
        assert!(c.git.run_branch);
        assert!(c.git.run_checkpoint_commits);
    }

    #[test]
    fn missing_file_returns_defaults() {
        let path = Path::new("/tmp/flow_does_not_exist.yaml");
        let cfg = Config::load(path).unwrap();
        assert_eq!(cfg.ui.voice, "friendly");
    }

    #[test]
    fn parses_minimal_yaml() {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(tmp.path(), "hosts:\n  - claude-code\n  - codex\n").unwrap();
        let cfg = Config::load(tmp.path()).unwrap();
        assert_eq!(cfg.hosts.len(), 2);
        assert_eq!(cfg.confirmation.as_str(), "no");
    }

    #[test]
    fn t001_t002_confirmation_parses_from_config() {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(tmp.path(), "confirmation: yes\n").unwrap();
        let cfg = Config::load(tmp.path()).unwrap();
        assert_eq!(cfg.confirmation.as_str(), "yes");
    }

    #[test]
    fn removed_branch_template_key_is_rejected() {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(
            tmp.path(),
            "git:\n  feature_branch_template: \"{sequence}-{title}\"\n",
        )
        .unwrap();
        let err = Config::load(tmp.path()).unwrap_err();
        assert!(format!("{err}").contains("feature_branch_template"));
    }

    // T-001: git.default_branch field parses from YAML.
    #[test]
    fn t001_git_default_branch_parses() {
        let yaml = "git:\n  default_branch: develop\n";
        let cfg: Config = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(cfg.git.default_branch, Some("develop".to_string()));
    }

    // T-001: git.default_branch defaults to None when absent.
    #[test]
    fn t001_git_default_branch_absent_is_none() {
        let cfg = Config::default();
        assert!(cfg.git.default_branch.is_none());
    }

    #[test]
    fn t001_run_checkpoint_commits_key_parses_and_old_key_is_rejected() {
        let yaml = "git:\n  run_checkpoint_commits: false\n";
        let cfg: Config = serde_yaml::from_str(yaml).unwrap();
        assert!(!cfg.git.run_checkpoint_commits);

        let yaml = "git:\n  run_all_checkpoint_commits: false\n";
        assert!(serde_yaml::from_str::<Config>(yaml).is_err());
    }

    #[test]
    fn t001_run_branch_defaults_true_and_parses_false() {
        assert!(Config::default().git.run_branch);
        let yaml = "git:\n  run_branch: false\n";
        let cfg: Config = serde_yaml::from_str(yaml).unwrap();
        assert!(!cfg.git.run_branch);
    }

    // T-002: DocsConfig defaults to touch_map_warnings=true and empty touch_map.
    #[test]
    fn t002_docs_config_defaults() {
        let cfg = Config::default();
        assert!(cfg.docs.touch_map_warnings);
        assert!(cfg.docs.touch_map.is_empty());
        assert!(cfg.docs.review_max_age_days.is_none());
    }

    // T-002: docs config parses a full touch_map entry from YAML.
    #[test]
    fn t002_docs_config_parses_touch_map() {
        let yaml = "docs:\n  review_max_age_days: 90\n  touch_map:\n    - paths:\n        - 'src/**'\n      docs:\n        - 'docs/reference/cli.md'\n      suppress: false\n  touch_map_warnings: true\n";
        let cfg: Config = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(cfg.docs.review_max_age_days, Some(90));
        assert_eq!(cfg.docs.touch_map.len(), 1);
        assert_eq!(cfg.docs.touch_map[0].paths, vec!["src/**"]);
        assert_eq!(cfg.docs.touch_map[0].docs, vec!["docs/reference/cli.md"]);
        assert!(!cfg.docs.touch_map[0].suppress);
        assert!(cfg.docs.touch_map_warnings);
    }

    // T-002: touch_map_warnings defaults to true even when docs key present but field absent.
    #[test]
    fn t002_touch_map_warnings_defaults_true() {
        let yaml = "docs:\n  review_max_age_days: 30\n";
        let cfg: Config = serde_yaml::from_str(yaml).unwrap();
        assert!(cfg.docs.touch_map_warnings);
    }

    // T-002: suppress field on TouchMapEntry defaults to false.
    #[test]
    fn t002_touch_map_entry_suppress_defaults_false() {
        let yaml = "docs:\n  touch_map:\n    - paths: ['src/']\n      docs: ['docs/foo.md']\n";
        let cfg: Config = serde_yaml::from_str(yaml).unwrap();
        assert!(!cfg.docs.touch_map[0].suppress);
    }

    // T-001: layout.version parses from YAML.
    #[test]
    fn t001_layout_version_parses() {
        let yaml = "layout:\n  version: 2\n";
        let cfg: Config = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(cfg.layout.version, 2);
    }

    // T-006: docs.documentation_path parses from YAML.
    #[test]
    fn t006_documentation_path_parses() {
        let yaml = "docs:\n  documentation_path: flow/docs\n";
        let cfg: Config = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(cfg.docs.documentation_path.as_deref(), Some("flow/docs"));
    }

    #[test]
    fn t006_old_capability_pages_path_key_is_rejected() {
        let yaml = "docs:\n  capability_pages_path: docs/features\n";
        assert!(serde_yaml::from_str::<Config>(yaml).is_err());
    }

    #[test]
    fn load_rejects_non_current_layout_version() {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(tmp.path(), "layout:\n  version: 1\n").unwrap();
        let err = Config::load(tmp.path()).unwrap_err();
        assert!(err.to_string().contains("layout.version must be 2"));
    }
}
