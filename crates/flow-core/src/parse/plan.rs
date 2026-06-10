//! `plan.md` parser (lightweight).

use crate::parse::markdown;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// In-memory view of a `plan.md`.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Plan {
    /// Raw Markdown source.
    pub raw: String,
    /// Heading → body map.
    pub sections: IndexMap<String, String>,
}

impl Plan {
    /// Returns `true` iff required plan sections exist.
    #[must_use]
    pub fn is_complete(&self) -> bool {
        self.sections.contains_key("Summary")
            && self.sections.contains_key("Technical Context")
            && self.sections.contains_key("Documentation Impact")
    }

    /// Returns true when `## Documentation Impact` explicitly opts out of
    /// central Flow docs updates.
    #[must_use]
    pub fn declares_no_documentation_impact(&self) -> bool {
        let Some(section) = self.sections.get("Documentation Impact") else {
            return false;
        };
        section.lines().any(|line| {
            let line = line
                .trim()
                .trim_start_matches(['-', '*'])
                .trim()
                .trim_matches('`')
                .trim()
                .to_ascii_lowercase();
            matches!(line.as_str(), "impact: none" | "documentation impact: none")
        })
    }
}

/// Parse a `plan.md` file from disk.
pub fn parse_file(path: &Path) -> crate::Result<Plan> {
    let text = std::fs::read_to_string(path).map_err(|_| crate::Error::FileNotFound {
        kind: "plan.md".into(),
        path: path.to_path_buf(),
    })?;
    Ok(parse_str(&text))
}

/// Parse a `plan.md` string directly.
#[must_use]
pub fn parse_str(text: &str) -> Plan {
    Plan {
        raw: text.to_string(),
        sections: markdown::parse_sections(text),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn t001_plan_detects_explicit_no_documentation_impact() {
        let plan = parse_str(
            "## Summary\n\nReady.\n\n## Technical Context\n\nRust.\n\n## Documentation Impact\n\nImpact: none\n\nDocs already current because this change only touches internal tests.\n",
        );
        assert!(plan.declares_no_documentation_impact());
    }

    #[test]
    fn t001_plan_does_not_infer_no_documentation_impact_from_rationale_only() {
        let plan = parse_str(
            "## Summary\n\nReady.\n\n## Technical Context\n\nRust.\n\n## Documentation Impact\n\nDocs already current because this change only touches internal tests.\n",
        );
        assert!(!plan.declares_no_documentation_impact());
    }
}
