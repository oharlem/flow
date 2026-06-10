//! `docs/principles.md` loader.
//!
//! Flow reads principles **live** — the file is never pinned to a commit. When
//! it is absent or empty the principles section is skipped by every phase.

use crate::parse::markdown;
use once_cell::sync::Lazy;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// A single engineering principle.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Principle {
    /// Optional `P-NNN` identifier.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// Directive text.
    pub directive: String,
    /// Rationale, if the bullet ends with `Rationale: …`.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub rationale: String,
}

/// Load `docs/principles.md` for a repository. Missing or empty → empty list.
pub fn load(repo: &Path) -> Vec<Principle> {
    let path = repo.join("docs").join("principles.md");
    let Ok(text) = std::fs::read_to_string(&path) else {
        return Vec::new();
    };
    if text.trim().is_empty() {
        return Vec::new();
    }
    parse_str(&text)
}

/// Parse principles from a string.
#[must_use]
pub fn parse_str(text: &str) -> Vec<Principle> {
    static BULLET: Lazy<Regex> = Lazy::new(|| {
        Regex::new(r"^\s*-\s+(?:\*\*(?P<id>P-\d{1,4})\*\*:?\s*)?(?P<body>.*)$").unwrap()
    });
    static RATIONALE: Lazy<Regex> = Lazy::new(|| Regex::new(r"(?i)\s+Rationale:\s*(.*)$").unwrap());

    let sections = markdown::parse_sections(text);
    let Some(body) = sections.get("Engineering Principles") else {
        return Vec::new();
    };

    let mut out = Vec::new();
    for line in body.lines() {
        let Some(caps) = BULLET.captures(line) else {
            continue;
        };
        let id = caps.name("id").map(|m| m.as_str().to_string());
        let mut directive = caps["body"].trim().to_string();
        let mut rationale = String::new();
        if let Some(r) = RATIONALE.captures(&directive.clone()) {
            rationale = r[1].trim().to_string();
            directive = RATIONALE.replace(&directive, "").trim().to_string();
        }
        if !directive.is_empty() || id.is_some() {
            out.push(Principle {
                id,
                directive,
                rationale,
            });
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_bullets_with_ids() {
        let text = "## Engineering Principles\n\n- **P-001**: Be explicit. Rationale: less surprise.\n- Prefer composition over inheritance.\n";
        let p = parse_str(text);
        assert_eq!(p.len(), 2);
        assert_eq!(p[0].id.as_deref(), Some("P-001"));
        assert_eq!(p[0].directive, "Be explicit.");
        assert_eq!(p[0].rationale, "less surprise.");
        assert!(p[1].id.is_none());
    }

    #[test]
    fn missing_section_yields_empty() {
        assert!(parse_str("# No principles\n").is_empty());
    }
}
