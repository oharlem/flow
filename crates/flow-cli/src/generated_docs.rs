//! Registry and helpers for Flow-owned generated documentation.

use crate::{cli_help, ownership, summary};
use flow_core::Result;
use std::path::{Path, PathBuf};

/// One Flow-owned generated documentation file.
pub struct GeneratedDoc {
    /// Repository-relative path to the generated file.
    pub rel_path: &'static str,
    marker: &'static str,
    render: fn(&Path) -> String,
}

impl GeneratedDoc {
    fn path(&self, repo: &Path) -> PathBuf {
        repo.join(self.rel_path)
    }

    fn expected(&self, repo: &Path) -> String {
        (self.render)(repo)
    }

    fn is_owned(&self, content: &str) -> bool {
        ownership::has_marker(content, self.marker)
    }
}

const GENERATED_DOCS: &[GeneratedDoc] = &[
    GeneratedDoc {
        rel_path: "docs/SUMMARY.md",
        marker: ownership::SUMMARY_MARKER,
        render: render_summary,
    },
    GeneratedDoc {
        rel_path: "docs/reference/cli.md",
        marker: ownership::CLI_REFERENCE_MARKER,
        render: render_cli_reference,
    },
];

/// Return all Flow-owned generated docs known to this binary.
pub fn all() -> &'static [GeneratedDoc] {
    GENERATED_DOCS
}

/// Return repository-relative generated docs that exist, are Flow-owned, and
/// differ from their current renderer output.
pub fn stale_paths(repo: &Path) -> Vec<&'static str> {
    let mut stale = Vec::new();
    for doc in all() {
        let path = doc.path(repo);
        if !path.is_file() {
            continue;
        }
        let Ok(current) = std::fs::read_to_string(&path) else {
            continue;
        };
        if doc.is_owned(&current) && current != doc.expected(repo) {
            stale.push(doc.rel_path);
        }
    }
    stale
}

/// Refresh existing Flow-owned generated docs and return the files changed.
///
/// Missing files and unmarked app-owned files are intentionally left alone.
pub fn refresh_existing(repo: &Path) -> Result<Vec<&'static str>> {
    let mut refreshed = Vec::new();
    for doc in all() {
        let path = doc.path(repo);
        if !path.is_file() {
            continue;
        }
        let current = std::fs::read_to_string(&path)?;
        if !doc.is_owned(&current) {
            continue;
        }
        let expected = doc.expected(repo);
        if current != expected {
            std::fs::write(&path, expected)?;
            refreshed.push(doc.rel_path);
        }
    }
    Ok(refreshed)
}

fn render_summary(repo: &Path) -> String {
    summary::render_full(repo)
}

fn render_cli_reference(_repo: &Path) -> String {
    cli_help::render_full()
}
