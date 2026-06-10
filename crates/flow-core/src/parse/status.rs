//! `status.md` parser + stamping.

use crate::error::{Error, Result};
use chrono::{DateTime, Utc};
use indexmap::IndexMap;
use once_cell::sync::Lazy;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::path::Path;
use std::str::FromStr;

/// Current state enum for Flow changes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum State {
    /// Change is being specced.
    Drafting,
    /// Change is being implemented.
    Building,
    /// Change has been closed.
    Closed,
}

impl fmt::Display for State {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Drafting => "drafting",
            Self::Building => "building",
            Self::Closed => "closed",
        })
    }
}

impl FromStr for State {
    type Err = Error;
    fn from_str(s: &str) -> Result<Self> {
        match s.trim() {
            "drafting" => Ok(Self::Drafting),
            "building" => Ok(Self::Building),
            "closed" => Ok(Self::Closed),
            other => Err(Error::ArtifactError {
                file: "status.md".into(),
                message: format!("State '{other}' is not one of: drafting, building, closed"),
            }),
        }
    }
}

/// In-memory view of `status.md`.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Status {
    /// `**Change**:` value.
    pub feature: String,
    /// `**Started**:` value (ISO date).
    pub started: String,
    /// `**Updated**:` value (ISO datetime).
    pub updated: String,
    /// `**State**:` value.
    pub state: Option<State>,
    /// `**Branch**:` value.
    pub branch: String,
    /// `**Milestone**:` values (split on comma).
    pub milestones: Vec<String>,
    /// Entries under `## History`, newest first.
    pub history: Vec<HistoryEntry>,
    /// Raw Markdown source.
    pub raw: String,
}

/// Single `## History` bullet parsed into structured fields.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HistoryEntry {
    /// Raw ISO timestamp prefix.
    pub timestamp: String,
    /// Action slug (e.g. `"spec-complete"`).
    pub action: String,
    /// One-line summary after the second em-dash.
    pub summary: String,
}

static FIELD_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^\*\*([A-Za-z-]+)\*\*:\s*(.*?)\s*$").unwrap());
static HISTORY_ITEM: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^\s*-\s+(\S+)\s+—\s+([^\s—][^—]*?)\s+—\s+(.*)$").unwrap());

/// Parse a `status.md` file.
pub fn parse_file(path: &Path) -> Result<Status> {
    let text = std::fs::read_to_string(path).map_err(|_| Error::FileNotFound {
        kind: "status.md".into(),
        path: path.to_path_buf(),
    })?;
    Ok(parse_str(&text))
}

/// Parse a `status.md` string.
#[must_use]
pub fn parse_str(text: &str) -> Status {
    let mut status = Status {
        raw: text.to_string(),
        ..Default::default()
    };
    let mut in_history = false;
    let mut fields: IndexMap<String, String> = IndexMap::new();

    for line in text.lines() {
        if line.trim_start().starts_with("## History") {
            in_history = true;
            continue;
        }
        if line.trim_start().starts_with("## ") && in_history {
            in_history = false;
        }
        if in_history {
            if let Some(caps) = HISTORY_ITEM.captures(line) {
                status.history.push(HistoryEntry {
                    timestamp: caps[1].to_string(),
                    action: caps[2].trim().to_string(),
                    summary: caps[3].trim().to_string(),
                });
            }
        } else if let Some(caps) = FIELD_RE.captures(line) {
            fields.insert(caps[1].to_string(), caps[2].to_string());
        }
    }

    status.feature = fields.get("Change").cloned().unwrap_or_default();
    status.started = fields.get("Started").cloned().unwrap_or_default();
    status.updated = fields.get("Updated").cloned().unwrap_or_default();
    status.state = fields.get("State").and_then(|s| State::from_str(s).ok());
    status.branch = fields.get("Branch").cloned().unwrap_or_default();
    if let Some(ms) = fields.get("Milestone") {
        status.milestones = ms
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
    }
    status
}

/// Validate that a `Status` has every mandatory field.
pub fn validate(status: &Status) -> Result<()> {
    if status.feature.is_empty() {
        return Err(Error::ArtifactError {
            file: "status.md".into(),
            message: "missing **Change**".into(),
        });
    }
    if status.started.is_empty() {
        return Err(Error::ArtifactError {
            file: "status.md".into(),
            message: "missing **Started**".into(),
        });
    }
    if status.updated.is_empty() {
        return Err(Error::ArtifactError {
            file: "status.md".into(),
            message: "missing **Updated**".into(),
        });
    }
    if status.state.is_none() {
        return Err(Error::ArtifactError {
            file: "status.md".into(),
            message: "missing or invalid **State**".into(),
        });
    }
    if status.branch.is_empty() {
        return Err(Error::ArtifactError {
            file: "status.md".into(),
            message: "missing **Branch**".into(),
        });
    }
    if !status.raw.contains("## History") {
        return Err(Error::ArtifactError {
            file: "status.md".into(),
            message: "missing '## History' section".into(),
        });
    }
    Ok(())
}

/// Stamp `status.md` by updating State, Updated, and prepending a history entry.
///
/// If `new_state` is `None`, the State field is left unchanged.
pub fn stamp(
    feature_dir: &Path,
    new_state: Option<State>,
    action: &str,
    summary: &str,
) -> Result<()> {
    let status_file = feature_dir.join("status.md");
    let text = std::fs::read_to_string(&status_file).map_err(|_| Error::FileNotFound {
        kind: "status.md".into(),
        path: status_file.clone(),
    })?;
    let now: DateTime<Utc> = Utc::now();
    let now_str = now.format("%Y-%m-%dT%H:%M:%SZ").to_string();
    let entry = format!("- {now_str} — {action} — {summary}\n");

    let mut out = String::with_capacity(text.len() + entry.len());
    let mut in_history = false;
    let mut history_inserted = false;

    for line in text.split_inclusive('\n') {
        let raw = line.trim_end_matches(['\n', '\r']);
        if let Some(state) = new_state {
            if raw.starts_with("**State**:") {
                out.push_str(&format!("**State**: {state}\n"));
                continue;
            }
        }
        if raw.starts_with("**Updated**:") {
            out.push_str(&format!("**Updated**: {now_str}\n"));
            continue;
        }
        if raw.starts_with("## History") {
            in_history = true;
            out.push_str(line);
            continue;
        }
        if in_history && !history_inserted && raw.trim_start().starts_with("- ") {
            out.push_str(&entry);
            history_inserted = true;
        }
        out.push_str(line);
    }

    if in_history && !history_inserted {
        if !out.ends_with('\n') {
            out.push('\n');
        }
        out.push_str(&entry);
    }

    // Atomic write: tmp file + rename
    let tmp = status_file.with_extension(format!("md.tmp.{}", std::process::id()));
    std::fs::write(&tmp, &out)?;
    std::fs::rename(&tmp, &status_file)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn example() -> &'static str {
        "# Status: 001-foo\n\n**Change**: 001-foo\n**Started**: 2026-05-06\n**Updated**: 2026-05-06T12:00:00Z\n**State**: drafting\n**Branch**: flow/001-foo\n\n## History\n\n- 2026-05-06T12:00:00Z — started — seeded\n"
    }

    #[test]
    fn parses_all_fields() {
        let s = parse_str(example());
        assert_eq!(s.feature, "001-foo");
        assert_eq!(s.state, Some(State::Drafting));
        assert_eq!(s.history.len(), 1);
    }

    #[test]
    fn validates_clean() {
        let s = parse_str(example());
        assert!(validate(&s).is_ok());
    }

    #[test]
    fn validates_missing_feature() {
        let mut s = parse_str(example());
        s.feature.clear();
        assert!(validate(&s).is_err());
    }

    #[test]
    fn state_parsing() {
        assert_eq!(State::from_str("drafting").unwrap(), State::Drafting);
        assert_eq!(State::from_str("building").unwrap(), State::Building);
        assert_eq!(State::from_str("closed").unwrap(), State::Closed);
        assert!(State::from_str("nope").is_err());
    }
}
