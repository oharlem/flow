//! Read-only Claude Code permissions advisory (M-25).
//!
//! Flow never modifies host configuration. When `flow doctor` runs on a
//! Claude Code repo and the user's allow list does not yet include rules
//! that would let Flow run with fewer permission prompts, doctor surfaces
//! a copy-paste advisory block. The user decides whether to add the rules
//! to their gitignored `.claude/settings.local.json`.
//!
//! This module is the single source of truth for:
//! - the [`RECOMMENDED_RULES`] set (literal copy from the M-25 spec),
//! - merging `.claude/settings.json` and `.claude/settings.local.json` into
//!   a read-only effective view, and
//! - rendering the advisory block (literal opening / closing phrases).
//!
//! No function in this module writes to either Claude Code settings file.

use std::collections::BTreeSet;
use std::path::Path;

/// Recommended Flow-related Claude Code allow rules. The order matches the
/// M-25 spec; the literal copy is preserved verbatim because the doctor
/// advisory text references it byte-for-byte.
pub const RECOMMENDED_RULES: &[&str] = &[
    "Bash(flow *)",
    "Bash(FLOW_HOST=* flow *)",
    "Edit(flow/**)",
    "Write(flow/**)",
    "Edit(.flow/**)",
    "Write(.flow/**)",
];

/// Read-only effective Claude Code permissions view, merged across
/// `.claude/settings.json` and `.claude/settings.local.json`.
#[derive(Debug, Default, Clone)]
pub struct UserPermissionsState {
    /// Set of literal `permissions.allow` strings present in either file.
    pub allow: BTreeSet<String>,
    /// True when either file declares `defaultMode == "bypassPermissions"`.
    pub bypass_mode: bool,
}

impl UserPermissionsState {
    /// Number of [`RECOMMENDED_RULES`] entries present in `allow`.
    #[must_use]
    pub fn recommended_present(&self) -> usize {
        RECOMMENDED_RULES
            .iter()
            .filter(|rule| self.allow.contains(**rule))
            .count()
    }

    /// True when the user's allow list looks "minimal," i.e., fewer than
    /// half of [`RECOMMENDED_RULES`] are present. Threshold: strictly less
    /// than half.
    #[must_use]
    pub fn is_minimal(&self) -> bool {
        let threshold = RECOMMENDED_RULES.len() / 2;
        self.recommended_present() < threshold
    }
}

/// Read `.claude/settings.json` and `.claude/settings.local.json` from `repo`
/// and return the merged effective view. Both files are optional; missing
/// or unreadable files are treated as absent (returning an empty contribution).
///
/// This function never writes to either file.
pub fn read_user_permissions_state(repo: &Path) -> UserPermissionsState {
    let mut state = UserPermissionsState::default();
    for file in [".claude/settings.json", ".claude/settings.local.json"] {
        let path = repo.join(file);
        if !path.exists() {
            continue;
        }
        let Ok(body) = std::fs::read_to_string(&path) else {
            continue;
        };
        let Ok(value) = serde_json::from_str::<serde_json::Value>(&body) else {
            continue;
        };
        if let Some(default_mode) = value.get("defaultMode").and_then(|v| v.as_str()) {
            if default_mode == "bypassPermissions" {
                state.bypass_mode = true;
            }
        }
        if let Some(allow) = value
            .get("permissions")
            .and_then(|p| p.get("allow"))
            .and_then(|a| a.as_array())
        {
            for entry in allow {
                if let Some(rule) = entry.as_str() {
                    state.allow.insert(rule.to_string());
                }
            }
        }
    }
    state
}

/// Outcome of [`advisory_block`].
#[derive(Debug, Clone)]
pub enum AdvisoryBlock {
    /// The advisory should not be printed (because the host is not
    /// Claude Code, the user already has a majority of recommended rules,
    /// or `defaultMode: bypassPermissions` is set).
    Suppressed,
    /// Print the multi-line advisory block verbatim.
    Print(String),
}

/// Compose the doctor advisory block for a Claude Code repo. The opening
/// phrase and closing line match the M-25 spec literally; the body lists
/// every rule from [`RECOMMENDED_RULES`] inside a JSON example block so
/// the user can copy-paste it directly into
/// `.claude/settings.local.json`.
#[must_use]
pub fn advisory_block(state: &UserPermissionsState) -> AdvisoryBlock {
    if state.bypass_mode {
        return AdvisoryBlock::Suppressed;
    }
    if !state.is_minimal() {
        return AdvisoryBlock::Suppressed;
    }
    let mut body = String::new();
    body.push_str("Note (claude-code): Flow detected that your `.claude/settings.local.json`\n");
    body.push_str("does not yet include rules that would let Flow run with fewer permission\n");
    body.push_str("prompts. Flow does not modify host configuration. If you'd like to reduce\n");
    body.push_str("prompts, add the following to your personal (gitignored)\n");
    body.push_str("`.claude/settings.local.json`:\n\n");
    body.push_str("  {\n");
    body.push_str("    \"permissions\": {\n");
    body.push_str("      \"allow\": [\n");
    for (idx, rule) in RECOMMENDED_RULES.iter().enumerate() {
        let suffix = if idx + 1 == RECOMMENDED_RULES.len() {
            ""
        } else {
            ","
        };
        body.push_str(&format!("        \"{rule}\"{suffix}\n"));
    }
    body.push_str("      ]\n");
    body.push_str("    }\n");
    body.push_str("  }\n\n");
    body.push_str("Skip this if you already manage Claude Code permissions another way.");
    AdvisoryBlock::Print(body)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;
    use tempfile::TempDir;

    #[test]
    fn t001_recommended_rules_match_spec_literally() {
        // Preserving the order is
        // important because the printed JSON example is rendered from
        // RECOMMENDED_RULES directly.
        assert_eq!(RECOMMENDED_RULES.len(), 6);
        assert_eq!(RECOMMENDED_RULES[0], "Bash(flow *)");
        assert_eq!(RECOMMENDED_RULES[1], "Bash(FLOW_HOST=* flow *)");
        assert_eq!(RECOMMENDED_RULES[2], "Edit(flow/**)");
        assert_eq!(RECOMMENDED_RULES[3], "Write(flow/**)");
        assert_eq!(RECOMMENDED_RULES[4], "Edit(.flow/**)");
        assert_eq!(RECOMMENDED_RULES[5], "Write(.flow/**)");
    }

    #[test]
    fn t001_read_user_permissions_state_returns_default_when_files_missing() {
        let td = TempDir::new().unwrap();
        let state = read_user_permissions_state(td.path());
        assert!(state.allow.is_empty());
        assert!(!state.bypass_mode);
        assert!(state.is_minimal());
    }

    #[test]
    fn t001_read_user_permissions_state_merges_local_and_main_settings() {
        let td = TempDir::new().unwrap();
        std::fs::create_dir_all(td.path().join(".claude")).unwrap();
        std::fs::write(
            td.path().join(".claude/settings.json"),
            "{\"permissions\":{\"allow\":[\"Bash(flow *)\"]}}",
        )
        .unwrap();
        std::fs::write(
            td.path().join(".claude/settings.local.json"),
            "{\"permissions\":{\"allow\":[\"Edit(flow/**)\",\"Bash(flow *)\"]}}",
        )
        .unwrap();
        let state = read_user_permissions_state(td.path());
        let expected: BTreeSet<String> = ["Bash(flow *)", "Edit(flow/**)"]
            .into_iter()
            .map(String::from)
            .collect();
        assert_eq!(state.allow, expected);
        assert!(!state.bypass_mode);
    }

    #[test]
    fn t001_read_user_permissions_state_detects_bypass_default_mode() {
        let td = TempDir::new().unwrap();
        std::fs::create_dir_all(td.path().join(".claude")).unwrap();
        std::fs::write(
            td.path().join(".claude/settings.local.json"),
            "{\"defaultMode\":\"bypassPermissions\"}",
        )
        .unwrap();
        let state = read_user_permissions_state(td.path());
        assert!(state.bypass_mode);
    }

    #[test]
    fn t002_advisory_suppressed_when_majority_present() {
        let mut state = UserPermissionsState::default();
        for rule in &RECOMMENDED_RULES[..4] {
            state.allow.insert((*rule).to_string());
        }
        // Threshold: < 4 of 8 triggers; >= 4 suppresses.
        assert!(matches!(advisory_block(&state), AdvisoryBlock::Suppressed));
    }

    #[test]
    fn t002_advisory_suppressed_when_bypass_mode_set() {
        let state = UserPermissionsState {
            bypass_mode: true,
            ..UserPermissionsState::default()
        };
        // Even with empty allow list, bypass_mode must suppress.
        assert!(matches!(advisory_block(&state), AdvisoryBlock::Suppressed));
    }

    #[test]
    fn t002_advisory_printed_when_allow_list_is_minimal() {
        let state = UserPermissionsState::default();
        let block = advisory_block(&state);
        let body = match block {
            AdvisoryBlock::Print(body) => body,
            AdvisoryBlock::Suppressed => panic!("expected advisory to be printed"),
        };
        assert!(body.starts_with("Note (claude-code): Flow detected that your"));
        assert!(body.contains("`.claude/settings.local.json`"));
        for rule in RECOMMENDED_RULES {
            assert!(
                body.contains(&format!("\"{rule}\"")),
                "advisory block should contain {rule:?}; body:\n{body}"
            );
        }
        assert!(
            body.ends_with("Skip this if you already manage Claude Code permissions another way."),
            "advisory block should end with the documented closing line; body:\n{body}"
        );
    }
}
