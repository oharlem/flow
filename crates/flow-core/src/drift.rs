//! Drift engine.
//!
//! Rule set:
//! - **D1**: every `FR-NNN` in `spec.md` is covered by at least one task (warn)
//! - **D2**: every `FR-NNN` in a task's `Covers:` exists in `spec.md` (warn)
//! - **D3**: every `SC-NNN` in a task's `Verifies:` exists in `spec.md` (warn)

pub mod json;
pub mod render;

use crate::{ids, parse::tasks::parse_file as parse_tasks_file};
use once_cell::sync::Lazy;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::Path;

static FR_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"\bFR-[A-Z]?\d{1,4}[a-z]?\b").unwrap());
static SC_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"\bSC-[A-Z]?\d{1,4}[a-z]?\b").unwrap());

/// Severity level for a finding.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    /// Non-blocking (exit code 1).
    Warn,
    /// Blocking (exit code 2).
    Error,
}

impl std::fmt::Display for Severity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Warn => "warn",
            Self::Error => "error",
        })
    }
}

/// Single drift finding.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Finding {
    /// Drift rule ID (e.g. `"D1"`).
    pub id: String,
    /// Severity level.
    pub severity: Severity,
    /// Short machine-readable message.
    pub message: String,
    /// User-facing title.
    pub title: String,
    /// User-facing cause.
    pub cause: String,
    /// File the finding points at.
    pub file: String,
    /// Line number in `file`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line: Option<usize>,
    /// ID or text the finding is about.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub subject: String,
    /// Suggested fix options.
    pub fix_options: Vec<String>,
}

/// Full drift report.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Report {
    /// List of findings (order preserved).
    pub findings: Vec<Finding>,
    /// `true` when `findings` is empty.
    pub clean: bool,
    /// `true` when any finding has `severity=error`.
    pub has_error: bool,
    /// `true` when any finding has `severity=warn`.
    pub has_warn: bool,
}

/// Run D1/D2/D3 against `feature_dir`.
///
/// `repo_root` is reserved for repository-wide checks; it is currently unused.
pub fn check_artifacts(
    feature_dir: &Path,
    _repo_root: Option<&Path>,
) -> crate::Result<Vec<Finding>> {
    let mut findings: Vec<Finding> = Vec::new();
    let spec_file = feature_dir.join("spec.md");
    if !spec_file.exists() {
        return Err(crate::Error::FileNotFound {
            kind: "spec.md".into(),
            path: spec_file,
        });
    }
    let spec_text = std::fs::read_to_string(&spec_file)?;
    let spec_frs: HashSet<String> = FR_RE
        .find_iter(&spec_text)
        .map(|m| m.as_str().to_string())
        .collect();
    let spec_scs: HashSet<String> = SC_RE
        .find_iter(&spec_text)
        .map(|m| m.as_str().to_string())
        .collect();
    let spec_fr_locations = first_locations(&spec_text, &FR_RE);

    let tasks_file = feature_dir.join("tasks.md");
    if !tasks_file.exists() {
        return Ok(findings);
    }
    let tasks = parse_tasks_file(&tasks_file)?;

    // D2/D3
    let mut covered_frs: HashSet<String> = HashSet::new();
    for t in &tasks {
        for reference in &t.covers {
            if spec_frs.contains(reference) {
                covered_frs.insert(reference.clone());
            } else {
                findings.push(finding_d2(&t.id, reference, t.line_number));
            }
        }
        for reference in &t.verifies {
            if !spec_scs.contains(reference) {
                findings.push(finding_d3(&t.id, reference, t.line_number));
            }
        }
    }

    // D1
    let mut uncovered: Vec<&String> = spec_frs.difference(&covered_frs).collect();
    uncovered.sort();
    for fr in uncovered {
        let line = spec_fr_locations.get(fr).copied();
        findings.push(finding_d1(fr, line));
    }

    Ok(findings)
}

/// Promote the severity of every finding whose `id` is in `promote_ids`.
#[must_use]
pub fn promote_severity(findings: Vec<Finding>, promote_ids: &HashSet<String>) -> Vec<Finding> {
    findings
        .into_iter()
        .map(|f| {
            if promote_ids.contains(&f.id) {
                Finding {
                    severity: Severity::Error,
                    ..f
                }
            } else {
                f
            }
        })
        .collect()
}

/// Build a [`Report`] from a list of findings.
#[must_use]
pub fn build_report(findings: Vec<Finding>) -> Report {
    let has_error = findings.iter().any(|f| f.severity == Severity::Error);
    let has_warn = findings.iter().any(|f| f.severity == Severity::Warn);
    let clean = findings.is_empty();
    Report {
        findings,
        clean,
        has_error,
        has_warn,
    }
}

fn first_locations(text: &str, re: &Regex) -> HashMap<String, usize> {
    let mut out: HashMap<String, usize> = HashMap::new();
    for (idx, line) in text.lines().enumerate() {
        for m in re.find_iter(line) {
            out.entry(m.as_str().to_string()).or_insert(idx + 1);
        }
    }
    out
}

// ---------- finding constructors ---------------------------------------------

fn finding_d1(fr: &str, line: Option<usize>) -> Finding {
    Finding {
        id: "D1".into(),
        severity: Severity::Warn,
        message: format!("FR '{fr}' is defined in spec.md but not covered by any task"),
        title: "Requirement has no task".into(),
        cause: format!("{fr} is in spec.md, but no task in tasks.md lists it under Covers."),
        file: "spec.md".into(),
        line,
        subject: fr.to_string(),
        fix_options: vec![
            format!("Add a task in tasks.md with Covers: {fr}."),
            format!("Remove {fr} from spec.md if it is no longer needed."),
        ],
    }
}

fn finding_d2(tid: &str, reference: &str, line: usize) -> Finding {
    Finding {
        id: "D2".into(),
        severity: Severity::Warn,
        message: format!("task {tid} covers '{reference}' which is not defined in spec.md"),
        title: "Task points to a missing requirement".into(),
        cause: format!(
            "{tid} says it covers {reference}, but {reference} is not listed in spec.md."
        ),
        file: "tasks.md".into(),
        line: Some(line),
        subject: tid.to_string(),
        fix_options: vec![
            format!("Fix the requirement ID in tasks.md if {reference} is a typo."),
            format!("Add {reference} to spec.md if this is a real requirement."),
            format!("Remove the stale {reference} reference from {tid}."),
        ],
    }
}

fn finding_d3(tid: &str, reference: &str, line: usize) -> Finding {
    Finding {
        id: "D3".into(),
        severity: Severity::Warn,
        message: format!("task {tid} verifies '{reference}' which is not defined in spec.md"),
        title: "Task points to a missing success criterion".into(),
        cause: format!(
            "{tid} says it verifies {reference}, but {reference} is not listed in spec.md."
        ),
        file: "tasks.md".into(),
        line: Some(line),
        subject: tid.to_string(),
        fix_options: vec![
            format!("Fix the success-criterion ID in tasks.md if {reference} is a typo."),
            format!("Add {reference} to spec.md if this is a real success criterion."),
            format!("Remove the stale {reference} reference from {tid}."),
        ],
    }
}

/// Convenience: also exported for callers who want the raw IDs list.
#[must_use]
pub fn ids_in_text(text: &str) -> Vec<String> {
    ids::ANY_ID
        .find_iter(text)
        .map(|m| m.as_str().to_string())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn d1_flags_uncovered_frs() {
        let td = tempfile::TempDir::new().unwrap();
        let feat = td.path().join("feat");
        std::fs::create_dir_all(&feat).unwrap();
        std::fs::write(
            feat.join("spec.md"),
            "# Spec\n\n## What & Why\n\n## Requirements\n### Functional Requirements\n- **FR-001**: One.\n- **FR-002**: Two.\n",
        )
        .unwrap();
        std::fs::write(
            feat.join("tasks.md"),
            "## Tasks\n\n- [ ] **T-001**: a\n    - Covers: FR-001\n    - Verifies: SC-001\n",
        )
        .unwrap();
        let got = check_artifacts(&feat, None).unwrap();
        assert!(got.iter().any(|f| f.id == "D1" && f.subject == "FR-002"));
    }

    #[test]
    fn d2_flags_missing_covers() {
        let td = tempfile::TempDir::new().unwrap();
        let feat = td.path().join("feat");
        std::fs::create_dir_all(&feat).unwrap();
        std::fs::write(
            feat.join("spec.md"),
            "## What & Why\n\n## Requirements\n### Functional Requirements\n- **FR-001**: One.\n",
        )
        .unwrap();
        std::fs::write(
            feat.join("tasks.md"),
            "- [ ] **T-001**: a\n    - Covers: FR-999\n    - Verifies: SC-001\n",
        )
        .unwrap();
        let got = check_artifacts(&feat, None).unwrap();
        assert!(got.iter().any(|f| f.id == "D2"));
    }
}
