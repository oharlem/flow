//! Structural Markdown parser — ATX headings only.
//!
//! Pure stdlib-level logic; no external CommonMark library is required because
//! Flow's Markdown subset is small.

use indexmap::IndexMap;
use once_cell::sync::Lazy;
use regex::Regex;

static HEADING_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"^(#{1,6})\s+(.*?)\s*$").unwrap());

/// Split `text` into `{heading_text: body_text}`. Headings are ATX (`#`, `##`, …).
///
/// When the same heading appears multiple times, the last one wins.
#[must_use]
pub fn parse_sections(text: &str) -> IndexMap<String, String> {
    let mut sections: IndexMap<String, String> = IndexMap::new();
    let mut current_heading: Option<String> = None;
    let mut current_body = String::new();

    for line in text.split_inclusive('\n') {
        if let Some(caps) = HEADING_RE.captures(line.trim_end_matches('\n')) {
            if let Some(heading) = current_heading.take() {
                sections.insert(heading, std::mem::take(&mut current_body));
            }
            let heading = caps
                .get(2)
                .map(|m| m.as_str().to_string())
                .unwrap_or_default();
            current_heading = Some(heading);
        } else if current_heading.is_some() {
            current_body.push_str(line);
        }
    }
    if let Some(heading) = current_heading {
        sections.insert(heading, current_body);
    }
    sections
}

/// Slugify a section heading into a lookup key (lowercase, dashes).
#[must_use]
pub fn slug_heading(heading: &str) -> String {
    let mut out = String::new();
    let mut last_dash = true;
    for c in heading.chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c.to_ascii_lowercase());
            last_dash = false;
        } else if !last_dash {
            out.push('-');
            last_dash = true;
        }
    }
    while out.ends_with('-') {
        out.pop();
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_ordered_sections() {
        let text = "# Heading\n\nbody\n\n## Sub\n\nbody 2\n";
        let s = parse_sections(text);
        assert!(s.contains_key("Heading"));
        assert!(s.contains_key("Sub"));
    }

    #[test]
    fn last_duplicate_wins() {
        let text = "# Foo\n\nfirst\n\n# Foo\n\nsecond\n";
        let s = parse_sections(text);
        assert_eq!(s["Foo"].trim(), "second");
    }
}
