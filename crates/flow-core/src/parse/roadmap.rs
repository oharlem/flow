//! Flow roadmap parser.

use once_cell::sync::Lazy;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Parsed milestone bullet.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Milestone {
    /// Milestone ID (e.g. `"M-1"`).
    pub id: String,
    /// 1-based line number.
    pub line_number: usize,
    /// Title (text before the em-dash separator).
    pub title: String,
    /// One-line description (text after the em-dash separator).
    pub description: String,
    /// Whether the checkbox is ticked.
    pub done: bool,
    /// Whether the checkbox is marked in progress (`[~]`).
    pub in_progress: bool,
}

static MILESTONE_HEADING: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^\s*###\s+\[(?P<state>[ xX~])\]\s+(?P<id>M-[1-9]\d*)\s*:?\s*(?P<title>.*?)\s*$")
        .unwrap()
});

/// Parse a roadmap into a list of milestones.
pub fn parse_file(path: &Path) -> crate::Result<Vec<Milestone>> {
    let text = std::fs::read_to_string(path).map_err(|_| crate::Error::FileNotFound {
        kind: "roadmap.md".into(),
        path: path.to_path_buf(),
    })?;
    Ok(parse_str(&text))
}

/// Parse `roadmap.md` content directly.
#[must_use]
pub fn parse_str(text: &str) -> Vec<Milestone> {
    let mut out = Vec::new();
    let mut in_fence = false;
    let mut current_heading: Option<Milestone> = None;
    let mut current_description: Vec<String> = Vec::new();

    for (idx, line) in text.lines().enumerate() {
        if line.trim_start().starts_with("```") {
            in_fence = !in_fence;
        }

        if !in_fence {
            if let Some(caps) = MILESTONE_HEADING.captures(line) {
                if let Some(mut milestone) = current_heading.take() {
                    milestone.description = current_description.join("\n");
                    out.push(milestone);
                    current_description.clear();
                }
                let state = caps.name("state").map(|m| m.as_str()).unwrap_or(" ");
                current_heading = Some(Milestone {
                    id: caps["id"].to_string(),
                    line_number: idx + 1,
                    title: caps
                        .name("title")
                        .map(|m| m.as_str())
                        .unwrap_or("")
                        .trim()
                        .to_string(),
                    description: String::new(),
                    done: matches!(state, "x" | "X"),
                    in_progress: state == "~",
                });
                continue;
            }
        }

        if current_heading.is_some() {
            current_description.push(line.to_string());
        }
    }

    if let Some(mut milestone) = current_heading {
        milestone.description = current_description.join("\n");
        out.push(milestone);
    }

    out
}

/// Find a specific milestone by ID.
#[must_use]
pub fn find(milestones: &[Milestone], id: &str) -> Option<Milestone> {
    milestones.iter().find(|m| m.id == id).cloned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_milestones() {
        let text = "# Roadmap\n\n## Milestones\n\n### [ ] M-1: First\n\nShort desc.\n\n### [x] M-2: Second\n\nDone desc.\n";
        let ms = parse_str(text);
        assert_eq!(ms.len(), 2);
        assert_eq!(ms[0].id, "M-1");
        assert_eq!(ms[0].title, "First");
        assert_eq!(ms[0].description, "\nShort desc.\n");
        assert!(!ms[0].done);
        assert!(ms[1].done);
        assert_eq!(ms[1].description, "\nDone desc.");
    }

    #[test]
    fn ignores_fenced_lines() {
        let text = "```markdown\n### [ ] M-999: never\n```\n\n### [ ] M-1: real\n\nYes.\n";
        let ms = parse_str(text);
        assert_eq!(ms.len(), 1);
        assert_eq!(ms[0].id, "M-1");
    }

    #[test]
    fn t004_parses_heading_milestones_with_full_body_description() {
        let text = "# Roadmap\n\n## Milestones\n\n### [~] M-1: Release flow\n\n#### Description\n\nFirst paragraph.\n\n- keep this list\n\n### [ ] M-2: Later\n\nSecond body.";
        let ms = parse_str(text);
        assert_eq!(ms.len(), 2);
        assert_eq!(ms[0].id, "M-1");
        assert_eq!(ms[0].title, "Release flow");
        assert_eq!(
            ms[0].description,
            "\n#### Description\n\nFirst paragraph.\n\n- keep this list\n"
        );
        assert!(!ms[0].done);
        assert_eq!(ms[1].id, "M-2");
        assert_eq!(ms[1].description, "\nSecond body.");
    }

    #[test]
    fn ignores_bullet_and_padded_milestones() {
        let text = "# Roadmap\n\n## Milestones\n\n### [ ] M-1: First\n\nBody.\n\n### [ ] M-0002: Ignored\n\nNo.\n\n- [x] **M-2**: Ignored.\n";
        let ms = parse_str(text);
        assert_eq!(ms.len(), 1);
        assert_eq!(ms[0].id, "M-1");
        assert_eq!(ms[0].title, "First");
    }

    #[test]
    fn preserves_current_heading_body() {
        let text = "\
# Roadmap

## Milestones

### [ ] M-1: Source-preserving example

Source: `flow/drafts/x.md`, section \"Milestone Shape\".

Outcome: Demonstrates that the new shape parses.

Must preserve:
- Exact label order.
- Copy strings `In review` and `Ready`.

Done when:
- Parser preserves the body.

Do not include:
- Any out-of-scope work.

### [ ] M-2: Next milestone

One paragraph describing the deliverable and its user value.
";
        let ms = parse_str(text);
        assert_eq!(ms.len(), 2);

        assert_eq!(ms[0].id, "M-1");
        assert_eq!(ms[0].title, "Source-preserving example");
        let m1_body = &ms[0].description;
        for label in [
            "Source:",
            "Outcome:",
            "Must preserve:",
            "Done when:",
            "Do not include:",
        ] {
            assert!(
                m1_body.contains(label),
                "M-1 description must preserve the `{label}` field label verbatim"
            );
        }
        // Literal copy strings inside the five-field body must survive.
        assert!(m1_body.contains("In review"));
        assert!(m1_body.contains("Ready"));

        assert_eq!(ms[1].id, "M-2");
        assert_eq!(ms[1].title, "Next milestone");
        assert!(
            ms[1]
                .description
                .contains("One paragraph describing the deliverable and its user value."),
            "heading milestone bodies must keep their description"
        );
    }
}
