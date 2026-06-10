//! `spec.md` parser + mutation helpers.

use crate::error::{Error, Result};
use crate::parse::markdown;
use chrono::Utc;
use indexmap::IndexMap;
use once_cell::sync::Lazy;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Matches `**Capability**: <value>` in the spec preamble.
static CAPABILITY_LINE_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?m)^\*\*Capability\*\*:\s*(.+?)\s*$").unwrap());

/// Valid capability slug: starts with a lowercase letter, followed by lowercase
/// letters, digits, or hyphens. No trailing hyphen (enforced implicitly by the
/// leading-letter requirement on any char following a hyphen).
static CAPABILITY_SLUG_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"^[a-z][a-z0-9-]*$").unwrap());

/// In-memory view of a `spec.md`.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Spec {
    /// Full Markdown source.
    pub raw: String,
    /// Every `## …` (and deeper) heading → body map, in document order.
    pub sections: IndexMap<String, String>,
}

impl Spec {
    /// Return `true` when the required `## What & Why` section is present.
    #[must_use]
    pub fn has_what_and_why(&self) -> bool {
        self.sections.keys().any(|h| h == "What & Why")
    }

    /// Return the text content of `## What & Why` (trimmed), or empty string.
    #[must_use]
    pub fn what_and_why(&self) -> String {
        self.sections
            .get("What & Why")
            .map(|s| s.trim().to_string())
            .unwrap_or_default()
    }

    /// Return the optional `**Capability**: <slug>` value from the preamble.
    ///
    /// Returns `Ok(None)` when the field is absent. Returns an error when the
    /// line is present but the value does not match `[a-z][a-z0-9-]*`.
    pub fn capability(&self) -> Result<Option<String>> {
        let preamble = preamble(&self.raw);
        if let Some(caps) = CAPABILITY_LINE_RE.captures(preamble) {
            let value = caps[1].to_string();
            validate_capability_slug(&value)?;
            Ok(Some(value))
        } else {
            Ok(None)
        }
    }
}

/// Extract the preamble — text before the first `## ` heading.
fn preamble(text: &str) -> &str {
    // Find the first occurrence of a level-2+ ATX heading on its own line.
    if let Some(pos) = text.find("\n## ") {
        &text[..pos]
    } else if let Some(pos) = text.find("\n### ") {
        &text[..pos]
    } else {
        text
    }
}

/// Validate that `slug` matches `[a-z][a-z0-9-]*` and does not end with `-`.
pub fn validate_capability_slug(slug: &str) -> Result<()> {
    if !CAPABILITY_SLUG_RE.is_match(slug) || slug.ends_with('-') {
        return Err(Error::ArtifactError {
            file: "spec.md".into(),
            message: format!(
                "invalid **Capability** value '{slug}': must match [a-z][a-z0-9-]* \
                 (lowercase letters, digits, hyphens; no leading digit or trailing hyphen)"
            ),
        });
    }
    Ok(())
}

/// Parse a `spec.md` file at `path`.
pub fn parse_file(path: &Path) -> Result<Spec> {
    let text = std::fs::read_to_string(path).map_err(|_| Error::FileNotFound {
        kind: "spec.md".into(),
        path: path.to_path_buf(),
    })?;
    Ok(parse_str(&text))
}

/// Parse a `spec.md` string directly (no I/O).
#[must_use]
pub fn parse_str(text: &str) -> Spec {
    Spec {
        raw: text.to_string(),
        sections: markdown::parse_sections(text),
    }
}

/// Validate that the spec contains every mandatory section.
pub fn validate(spec: &Spec) -> Result<()> {
    if !spec.has_what_and_why() {
        return Err(Error::ArtifactError {
            file: "spec.md".into(),
            message: "missing required section: ## What & Why".into(),
        });
    }
    Ok(())
}

/// Append a Q/A pair to `## Clarifications` under today's session.
///
/// Creates `## Clarifications` and/or `### Session YYYY-MM-DD` when absent.
/// The section is inserted just after `## What & Why` so the clarifications
/// stay near the top of `spec.md` per conventions §7.
pub fn append_clarification(spec_path: &Path, question: &str, answer: &str) -> Result<()> {
    let today = Utc::now().format("%Y-%m-%d").to_string();
    let text = std::fs::read_to_string(spec_path)?;
    let entry = format!("- Q: {question} → A: {answer}\n");

    let new_text = if text.contains("## Clarifications") {
        // Look for today's session; append entry to it; else add a new subsection.
        let header = format!("### Session {today}");
        if text.contains(&header) {
            insert_under_session(&text, &header, &entry)
        } else {
            insert_new_session(&text, &header, &entry)
        }
    } else {
        // Insert a fresh `## Clarifications` block after `## What & Why`.
        let block = format!("\n## Clarifications\n\n### Session {today}\n\n{entry}");
        insert_after_what_and_why(&text, &block)
    };

    std::fs::write(spec_path, new_text)?;
    Ok(())
}

fn insert_under_session(text: &str, session_header: &str, entry: &str) -> String {
    let idx = text.find(session_header).expect("session_header present");
    let after = &text[idx..];
    // Find the next `### ` or `## ` header (or EOF).
    let rel_end = after[session_header.len()..]
        .find("\n## ")
        .or_else(|| after[session_header.len()..].find("\n### "))
        .map_or(text.len() - idx, |n| session_header.len() + n);
    let insertion_point = idx + rel_end;
    let mut out = text[..insertion_point].trim_end().to_string();
    out.push('\n');
    out.push_str(entry);
    if insertion_point < text.len() {
        out.push_str(&text[insertion_point..]);
    } else {
        out.push('\n');
    }
    out
}

fn insert_new_session(text: &str, session_header: &str, entry: &str) -> String {
    // Find the `## Clarifications` section and append a new subsection after any existing sessions.
    let clar_idx = text
        .find("## Clarifications")
        .expect("clarifications present");
    // End of the Clarifications section = next `## ` header after clar_idx.
    let rel = text[clar_idx + 1..].find("\n## ").map(|n| clar_idx + 1 + n);
    let insertion_point = rel.unwrap_or(text.len());
    let mut out = text[..insertion_point].trim_end().to_string();
    out.push_str("\n\n");
    out.push_str(session_header);
    out.push_str("\n\n");
    out.push_str(entry);
    out.push('\n');
    if insertion_point < text.len() {
        out.push_str(&text[insertion_point..]);
    }
    out
}

fn insert_after_what_and_why(text: &str, block: &str) -> String {
    let marker = "## What & Why";
    let Some(start) = text.find(marker) else {
        let mut out = text.trim_end().to_string();
        out.push('\n');
        out.push_str(block);
        return out;
    };
    let rel = text[start + marker.len()..]
        .find("\n## ")
        .map(|n| start + marker.len() + n);
    let insertion_point = rel.unwrap_or(text.len());
    let mut out = text[..insertion_point].trim_end().to_string();
    out.push('\n');
    out.push_str(block);
    if insertion_point < text.len() {
        out.push('\n');
        out.push_str(&text[insertion_point..]);
    } else {
        out.push('\n');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    // T-001: Spec::capability() and kebab-case validator

    #[test]
    fn t001_capability_absent_returns_none() {
        let text = "# Spec\n\n**Change**: 001-foo\n\n## What & Why\n\nBecause.\n";
        let spec = parse_str(text);
        assert_eq!(spec.capability().unwrap(), None);
    }

    #[test]
    fn t001_capability_valid_slug() {
        let text =
            "# Spec\n\n**Change**: 001-foo\n**Capability**: my-feature\n\n## What & Why\n\nBecause.\n";
        let spec = parse_str(text);
        assert_eq!(spec.capability().unwrap(), Some("my-feature".to_string()));
    }

    #[test]
    fn t001_capability_single_word() {
        let text = "# Spec\n\n**Capability**: docs\n\n## What & Why\n\nBecause.\n";
        let spec = parse_str(text);
        assert_eq!(spec.capability().unwrap(), Some("docs".to_string()));
    }

    #[test]
    fn t001_capability_with_digits() {
        let text = "# Spec\n\n**Capability**: flow2\n\n## What & Why\n\nBecause.\n";
        let spec = parse_str(text);
        assert_eq!(spec.capability().unwrap(), Some("flow2".to_string()));
    }

    #[test]
    fn t001_capability_uppercase_rejected() {
        let text = "# Spec\n\n**Capability**: MyFeature\n\n## What & Why\n\nBecause.\n";
        let spec = parse_str(text);
        assert!(spec.capability().is_err());
        let msg = spec.capability().unwrap_err().to_string();
        assert!(msg.contains("MyFeature"));
    }

    #[test]
    fn t001_capability_underscore_rejected() {
        let text = "# Spec\n\n**Capability**: my_feature\n\n## What & Why\n\nBecause.\n";
        let spec = parse_str(text);
        assert!(spec.capability().is_err());
    }

    #[test]
    fn t001_capability_leading_digit_rejected() {
        let text = "# Spec\n\n**Capability**: 1feature\n\n## What & Why\n\nBecause.\n";
        let spec = parse_str(text);
        assert!(spec.capability().is_err());
    }

    #[test]
    fn t001_capability_trailing_hyphen_rejected() {
        let text = "# Spec\n\n**Capability**: feature-\n\n## What & Why\n\nBecause.\n";
        let spec = parse_str(text);
        assert!(spec.capability().is_err());
    }

    #[test]
    fn t001_capability_only_in_preamble_not_body() {
        // **Capability** appearing inside a section body must not be picked up.
        let text = "# Spec\n\n**Change**: 001-foo\n\n## What & Why\n\n**Capability**: in-body\n";
        let spec = parse_str(text);
        assert_eq!(spec.capability().unwrap(), None);
    }

    #[test]
    fn parses_what_and_why() {
        let text = "# Spec\n\n## What & Why\n\nBecause.\n";
        let spec = parse_str(text);
        assert!(spec.has_what_and_why());
        assert_eq!(spec.what_and_why(), "Because.");
    }

    #[test]
    fn rejects_missing_what_and_why() {
        let spec = parse_str("# Spec\n\n## Summary\n\nfoo\n");
        assert!(validate(&spec).is_err());
    }

    #[test]
    fn appends_clarification_to_new_section() {
        let td = tempfile::tempdir().unwrap();
        let p = td.path().join("spec.md");
        std::fs::write(&p, "# Spec\n\n## What & Why\n\nBecause.\n").unwrap();
        append_clarification(&p, "Who?", "Us.").unwrap();
        let out = std::fs::read_to_string(&p).unwrap();
        assert!(out.contains("## Clarifications"));
        assert!(out.contains("### Session "));
        assert!(out.contains("- Q: Who? → A: Us."));
    }

    #[test]
    fn appends_to_existing_session() {
        let td = tempfile::tempdir().unwrap();
        let p = td.path().join("spec.md");
        let today = Utc::now().format("%Y-%m-%d").to_string();
        std::fs::write(
            &p,
            format!("## What & Why\n\nFoo\n\n## Clarifications\n\n### Session {today}\n\n- Q: A → A: B\n\n## Edge Cases\n\n- None\n"),
        )
        .unwrap();
        append_clarification(&p, "C", "D").unwrap();
        let out = std::fs::read_to_string(&p).unwrap();
        assert!(out.contains("- Q: A → A: B"));
        assert!(out.contains("- Q: C → A: D"));
        assert!(out.contains("## Edge Cases"));
    }
}
