//! Project-local Flow settings.

use crate::{Error, Result};
use serde::{Deserialize, Serialize, Serializer};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// Persisted project settings.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Settings {
    /// Whether CLI confirmation prompts are enabled.
    pub confirmation: ConfirmationSetting,
    /// Next preferred milestone number for generated roadmap milestones.
    pub counter: u32,
    /// Whether printed `Next command: flow X --finalize` footers are
    /// suppressed (M-24). Default: green-path collapse, footer suppressed.
    pub review: ReviewSetting,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            confirmation: ConfirmationSetting::No,
            counter: 1,
            review: ReviewSetting::default(),
        }
    }
}

impl Settings {
    /// Return `true` when the printed `Next command: flow <cmd> --finalize`
    /// footer should be suppressed for `cmd`. M-24: per-command overrides
    /// (`review.per_command.<cmd>: true|false`) take precedence over the
    /// global `review.before_finalize` default. The default behavior is to
    /// collapse the green-path footer (return `true`).
    ///
    /// Resolution semantics:
    /// - `before_finalize: false` (default) means "do not require the
    ///   user to review before finalize" — therefore the footer is
    ///   suppressed (return `true`).
    /// - `before_finalize: true` means "review before finalize" — the
    ///   footer is emitted (return `false`).
    /// - The `per_command` map overrides the global default for one
    ///   subcommand at a time using the same boolean semantics.
    #[must_use]
    pub fn review_skip_finalize_footer(&self, cmd: &str) -> bool {
        let before_finalize = self
            .review
            .per_command
            .get(cmd)
            .copied()
            .unwrap_or(self.review.before_finalize);
        !before_finalize
    }
}

/// Review-before-finalize setting (M-24).
///
/// `before_finalize: false` (default) means the green-path agent collapses
/// prepare-and-finalize without requiring a user review of the printed
/// envelope between the two halves. `true` keeps today's two-stage
/// protocol, where the printed `Next command: flow X --finalize` footer is
/// the user's review checkpoint.
///
/// `per_command` is a map keyed by Flow subcommand name (`start`, `amend`,
/// `plan`, `build`, `build-task`, `test`, `close`, `roadmap`, `run`)
/// granting per-command overrides; e.g., `review.per_command.plan: true`
/// keeps the two-stage protocol for `flow plan` only while other
/// subcommands collapse.
#[derive(Clone, Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
#[serde(default)]
pub struct ReviewSetting {
    /// Default review-before-finalize behavior. `false` suppresses the
    /// printed finalize footer; the agent still runs `flow <cmd> --finalize`
    /// in the same session when artifacts are ready.
    pub before_finalize: bool,
    /// Per-Flow-subcommand override of `before_finalize`.
    pub per_command: BTreeMap<String, bool>,
}

/// Confirmation prompt setting.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum ConfirmationSetting {
    /// Ask for confirmation where Flow normally asks.
    Yes,
    /// Skip confirmation prompts.
    #[default]
    No,
}

impl ConfirmationSetting {
    /// Parse a user-facing setting value.
    pub fn parse(input: &str) -> std::result::Result<Self, String> {
        match input.trim().to_ascii_lowercase().as_str() {
            "yes" => Ok(Self::Yes),
            "no" => Ok(Self::No),
            other => Err(format!(
                "unsupported confirmation value {other:?}; expected yes or no"
            )),
        }
    }

    /// Return the stable persisted/user-facing value.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Yes => "yes",
            Self::No => "no",
        }
    }

    /// Return true when confirmations should be skipped.
    #[must_use]
    pub fn is_disabled(self) -> bool {
        matches!(self, Self::No)
    }
}

impl Serialize for ConfirmationSetting {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for ConfirmationSetting {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        ConfirmationSetting::parse(&value).map_err(serde::de::Error::custom)
    }
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct RawState {
    counter: Option<u32>,
}

impl Settings {
    /// Load effective settings from `.flow/config.yaml` and `.flow/state.yaml`,
    /// returning defaults when absent.
    pub fn load_for_repo(repo: &Path) -> Result<Self> {
        let confirmation_from_config = load_config_confirmation(repo)?;
        let review_from_config = load_config_review(repo)?;
        let counter_from_state = load_state_counter(repo)?;
        let counter = counter_from_state.unwrap_or(1);
        validate_counter(counter, &state_path(repo))?;
        Ok(Self {
            confirmation: confirmation_from_config.unwrap_or_default(),
            counter,
            review: review_from_config.unwrap_or_default(),
        })
    }

    /// Persist mutable Flow state to `.flow/state.yaml`.
    pub fn save_for_repo(&self, repo: &Path) -> Result<()> {
        let flow_dir = repo.join(".flow");
        std::fs::create_dir_all(&flow_dir)?;
        let path = state_path(repo);
        let tmp = temp_path(&path);
        std::fs::write(&tmp, self.render_state())?;
        std::fs::rename(tmp, path)?;
        Ok(())
    }

    fn render_state(&self) -> String {
        format!(
            "# .flow/state.yaml\n# Flow-owned mutable project state.\nschema_version: 1.0\ncounter: {}\n",
            self.counter
        )
    }

    /// Parse a user-facing milestone counter value.
    pub fn parse_counter(input: &str) -> std::result::Result<u32, String> {
        let trimmed = input.trim();
        let parsed = trimmed.parse::<u32>().map_err(|_| {
            format!("unsupported counter value {trimmed:?}; expected a positive integer")
        })?;
        if parsed == 0 {
            return Err("counter must be a positive integer".to_string());
        }
        Ok(parsed)
    }
}

fn load_config_confirmation(repo: &Path) -> Result<Option<ConfirmationSetting>> {
    let path = repo.join(".flow").join("config.yaml");
    if !path.exists() {
        return Ok(None);
    }
    let text = std::fs::read_to_string(&path)?;
    if text.trim().is_empty() {
        return Ok(None);
    }
    let value: serde_yaml::Value = serde_yaml::from_str(&text)
        .map_err(|e| Error::Config(format!("{}: {e}", path.display())))?;
    let serde_yaml::Value::Mapping(map) = value else {
        return Ok(None);
    };
    let key = serde_yaml::Value::String("confirmation".to_string());
    let Some(raw) = map.get(&key) else {
        return Ok(None);
    };
    let confirmation = serde_yaml::from_value(raw.clone())
        .map_err(|e| Error::Config(format!("{}: {e}", path.display())))?;
    Ok(Some(confirmation))
}

fn load_config_review(repo: &Path) -> Result<Option<ReviewSetting>> {
    let path = repo.join(".flow").join("config.yaml");
    if !path.exists() {
        return Ok(None);
    }
    let text = std::fs::read_to_string(&path)?;
    if text.trim().is_empty() {
        return Ok(None);
    }
    let value: serde_yaml::Value = serde_yaml::from_str(&text)
        .map_err(|e| Error::Config(format!("{}: {e}", path.display())))?;
    let serde_yaml::Value::Mapping(map) = value else {
        return Ok(None);
    };
    let key = serde_yaml::Value::String("review".to_string());
    let Some(raw) = map.get(&key) else {
        return Ok(None);
    };
    let review = serde_yaml::from_value(raw.clone())
        .map_err(|e| Error::Config(format!("{}: {e}", path.display())))?;
    Ok(Some(review))
}

fn load_state_counter(repo: &Path) -> Result<Option<u32>> {
    let path = state_path(repo);
    if !path.exists() {
        return Ok(None);
    }
    let text = std::fs::read_to_string(&path)?;
    if text.trim().is_empty() {
        return Ok(None);
    }
    let raw: RawState = serde_yaml::from_str(&text)
        .map_err(|e| Error::Config(format!("{}: {e}", path.display())))?;
    if let Some(counter) = raw.counter {
        validate_counter(counter, &path)?;
    }
    Ok(raw.counter)
}

fn validate_counter(counter: u32, path: &Path) -> Result<()> {
    if counter == 0 {
        return Err(Error::Config(format!(
            "{}: counter must be a positive integer",
            path.display()
        )));
    }
    Ok(())
}

/// Return the path to the Flow-owned mutable project state file.
#[must_use]
pub fn state_path(repo: &Path) -> PathBuf {
    repo.join(".flow").join("state.yaml")
}

fn temp_path(path: &Path) -> PathBuf {
    path.with_file_name(format!(
        "{}.tmp.{}",
        path.file_name().unwrap_or_default().to_string_lossy(),
        std::process::id()
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn t006_missing_settings_default_confirmation_no() {
        let td = tempfile::TempDir::new().unwrap();
        let settings = Settings::load_for_repo(td.path()).unwrap();
        assert_eq!(settings.confirmation, ConfirmationSetting::No);
        assert_eq!(settings.counter, 1);
    }

    #[test]
    fn t001_t004_state_round_trip_persists_counter_only() {
        let td = tempfile::TempDir::new().unwrap();
        let settings = Settings {
            confirmation: ConfirmationSetting::Yes,
            counter: 2,
            review: ReviewSetting::default(),
        };
        settings.save_for_repo(td.path()).unwrap();

        let loaded = Settings::load_for_repo(td.path()).unwrap();
        assert_eq!(loaded.confirmation, ConfirmationSetting::No);
        assert_eq!(loaded.counter, 2);
        let rendered = std::fs::read_to_string(state_path(td.path())).unwrap();
        assert!(rendered.contains("# .flow/state.yaml"));
        assert!(rendered.contains("counter: 2"));
        assert!(!rendered.contains("confirmation"));
    }

    #[test]
    fn boolean_confirmation_is_rejected() {
        let td = tempfile::TempDir::new().unwrap();
        std::fs::create_dir_all(td.path().join(".flow")).unwrap();
        std::fs::write(
            td.path().join(".flow").join("config.yaml"),
            "confirmation: true\n",
        )
        .unwrap();

        assert!(Settings::load_for_repo(td.path()).is_err());
    }

    #[test]
    fn t001_counter_rejects_non_positive_values() {
        assert_eq!(Settings::parse_counter("1").unwrap(), 1);
        assert_eq!(Settings::parse_counter("0002").unwrap(), 2);
        assert!(Settings::parse_counter("0").is_err());
        assert!(Settings::parse_counter("-1").is_err());
        assert!(Settings::parse_counter("two").is_err());
    }

    /// M-24: empty config defaults to `review.before_finalize: false`,
    /// which means the green-path footer is suppressed.
    #[test]
    fn t006_review_defaults_collapse_finalize_footer() {
        let td = tempfile::TempDir::new().unwrap();
        let settings = Settings::load_for_repo(td.path()).unwrap();
        assert!(!settings.review.before_finalize);
        assert!(settings.review.per_command.is_empty());
        assert!(settings.review_skip_finalize_footer("plan"));
        assert!(settings.review_skip_finalize_footer("build"));
    }

    /// M-24: setting `review.before_finalize: true` keeps today's
    /// two-stage protocol (the printed footer is emitted).
    #[test]
    fn t006_review_before_finalize_true_keeps_two_stage() {
        let td = tempfile::TempDir::new().unwrap();
        std::fs::create_dir_all(td.path().join(".flow")).unwrap();
        std::fs::write(
            td.path().join(".flow").join("config.yaml"),
            "review:\n  before_finalize: true\n",
        )
        .unwrap();
        let settings = Settings::load_for_repo(td.path()).unwrap();
        assert!(settings.review.before_finalize);
        assert!(!settings.review_skip_finalize_footer("plan"));
        assert!(!settings.review_skip_finalize_footer("build"));
    }

    /// M-24: per-command override resolves correctly.
    #[test]
    fn t006_review_per_command_override_resolves() {
        let td = tempfile::TempDir::new().unwrap();
        std::fs::create_dir_all(td.path().join(".flow")).unwrap();
        std::fs::write(
            td.path().join(".flow").join("config.yaml"),
            "review:\n  before_finalize: false\n  per_command:\n    plan: true\n",
        )
        .unwrap();
        let settings = Settings::load_for_repo(td.path()).unwrap();
        // plan opted into review-before-finalize: footer printed.
        assert!(!settings.review_skip_finalize_footer("plan"));
        // other commands collapse on the green path.
        assert!(settings.review_skip_finalize_footer("build"));
        assert!(settings.review_skip_finalize_footer("amend"));
    }

    /// M-24: `review` and `confirmation` are independent settings; setting
    /// one does not change the other.
    #[test]
    fn t006_review_is_orthogonal_to_confirmation() {
        let td = tempfile::TempDir::new().unwrap();
        std::fs::create_dir_all(td.path().join(".flow")).unwrap();
        std::fs::write(
            td.path().join(".flow").join("config.yaml"),
            "confirmation: yes\nreview:\n  before_finalize: true\n",
        )
        .unwrap();
        let settings = Settings::load_for_repo(td.path()).unwrap();
        assert_eq!(settings.confirmation, ConfirmationSetting::Yes);
        assert!(settings.review.before_finalize);

        let td2 = tempfile::TempDir::new().unwrap();
        std::fs::create_dir_all(td2.path().join(".flow")).unwrap();
        std::fs::write(
            td2.path().join(".flow").join("config.yaml"),
            "confirmation: no\nreview:\n  before_finalize: false\n",
        )
        .unwrap();
        let settings2 = Settings::load_for_repo(td2.path()).unwrap();
        assert_eq!(settings2.confirmation, ConfirmationSetting::No);
        assert!(!settings2.review.before_finalize);
    }
}
