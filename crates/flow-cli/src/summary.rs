//! Shared rendering of `docs/SUMMARY.md`.

use std::fs;
use std::path::Path;

const INTRO_FILES: &[&str] = &["README.md"];
const WHY_FLOW_FILES: &[&str] = &["why-flow.md"];
const ARCHITECTURE_FILES: &[&str] = &["architecture.md", "drift-rules.md", "security.md"];
const HOST_FILES: &[&str] = &["hosts.md"];

#[derive(Clone, Copy)]
enum SectionSource {
    FixedFiles(&'static [&'static str]),
    DirectoryAlpha(&'static str),
    Decisions,
}

#[derive(Clone, Copy)]
struct Section {
    title: &'static str,
    source: SectionSource,
}

const SECTIONS: &[Section] = &[
    Section {
        title: "Summary",
        source: SectionSource::FixedFiles(INTRO_FILES),
    },
    Section {
        title: "Why Flow",
        source: SectionSource::FixedFiles(WHY_FLOW_FILES),
    },
    Section {
        title: "Start here",
        source: SectionSource::DirectoryAlpha("start-here"),
    },
    Section {
        title: "Architecture",
        source: SectionSource::FixedFiles(ARCHITECTURE_FILES),
    },
    Section {
        title: "Reference",
        source: SectionSource::DirectoryAlpha("reference"),
    },
    Section {
        title: "Hosts",
        source: SectionSource::FixedFiles(HOST_FILES),
    },
    Section {
        title: "Decisions",
        source: SectionSource::Decisions,
    },
];

/// Render the full `docs/SUMMARY.md` body from the repository docs tree.
///
/// The output is byte-stable for a given docs tree and is the single source of
/// truth for the regeneration example and doctor drift check.
pub fn render_full(repo: &Path) -> String {
    let docs = repo.join("docs");
    let mut out = String::new();
    let mut marker_inserted = false;

    for section in SECTIONS {
        let entries = section_entries(&docs, section.source);
        if entries.is_empty() {
            continue;
        }

        if !out.is_empty() {
            out.push('\n');
        }
        out.push_str("# ");
        out.push_str(section.title);
        out.push_str("\n\n");
        if !marker_inserted {
            out.push_str(crate::ownership::SUMMARY_MARKER);
            out.push_str("\n\n");
            marker_inserted = true;
        }

        for rel in entries {
            let path = docs.join(&rel);
            let title = link_title(&path, &rel);
            out.push_str("- [");
            out.push_str(&title);
            out.push_str("](./");
            out.push_str(&rel);
            out.push_str(")\n");
        }
    }

    out
}

fn section_entries(docs: &Path, source: SectionSource) -> Vec<String> {
    match source {
        SectionSource::FixedFiles(files) => files
            .iter()
            .filter(|rel| docs.join(rel).is_file())
            .map(|rel| (*rel).to_string())
            .collect(),
        SectionSource::DirectoryAlpha(dir) => directory_entries(docs, dir),
        SectionSource::Decisions => {
            let mut entries = directory_entries(docs, "decisions");
            entries.sort_by(|a, b| match (decision_rank(a), decision_rank(b)) {
                ((0, _), (rank, _)) if rank != 0 => std::cmp::Ordering::Less,
                ((rank, _), (0, _)) if rank != 0 => std::cmp::Ordering::Greater,
                ((1, a_num), (1, b_num)) => a_num.cmp(&b_num).then_with(|| a.cmp(b)),
                ((a_rank, _), (b_rank, _)) => a_rank.cmp(&b_rank).then_with(|| a.cmp(b)),
            });
            entries
        }
    }
}

fn directory_entries(docs: &Path, dir: &str) -> Vec<String> {
    let path = docs.join(dir);
    let Ok(read_dir) = fs::read_dir(path) else {
        return Vec::new();
    };

    let mut entries: Vec<_> = read_dir
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let file_type = entry.file_type().ok()?;
            if !file_type.is_file() {
                return None;
            }
            let name = entry.file_name();
            let name = name.to_str()?;
            if !name.ends_with(".md") {
                return None;
            }
            Some(format!("{dir}/{name}"))
        })
        .collect();
    entries.sort();
    entries
}

fn link_title(path: &Path, rel: &str) -> String {
    let Ok(contents) = fs::read_to_string(path) else {
        return filename_fallback(rel);
    };

    for line in contents.lines() {
        if let Some(title) = line.strip_prefix("# ") {
            let title = title.trim();
            if !title.is_empty() {
                return title.to_string();
            }
        }
    }

    eprintln!(
        "warning: docs/{rel} has no H1; using `{}` as SUMMARY link text",
        filename_fallback(rel)
    );
    filename_fallback(rel)
}

fn filename_fallback(rel: &str) -> String {
    Path::new(rel)
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or(rel)
        .to_string()
}

fn decision_rank(rel: &str) -> (u8, u32) {
    let Some(name) = Path::new(rel).file_name().and_then(|name| name.to_str()) else {
        return (2, 0);
    };

    if name == "README.md" {
        return (0, 0);
    }

    let Some((prefix, _)) = name.split_once('-') else {
        return (2, 0);
    };
    if prefix.len() == 4 && prefix.chars().all(|c| c.is_ascii_digit()) {
        return (1, prefix.parse().unwrap_or(0));
    }

    (2, 0)
}

#[cfg(test)]
mod tests {
    use super::render_full;
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::time::{SystemTime, UNIX_EPOCH};

    struct TestRepo {
        root: PathBuf,
    }

    impl TestRepo {
        fn new(name: &str) -> Self {
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system time before UNIX_EPOCH")
                .as_nanos();
            let root = std::env::temp_dir()
                .join(format!("flow-summary-{name}-{}-{now}", std::process::id()));
            fs::create_dir_all(root.join("docs")).expect("create test docs dir");
            Self { root }
        }

        fn write_doc(&self, rel: &str, body: &str) {
            let path = self.root.join("docs").join(rel);
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).expect("create doc parent dir");
            }
            fs::write(path, body).expect("write doc fixture");
        }

        fn path(&self) -> &Path {
            &self.root
        }
    }

    impl Drop for TestRepo {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }

    #[test]
    fn t001_renders_sections_in_schema_order() {
        let repo = TestRepo::new("section-order");
        repo.write_doc("decisions/README.md", "# ADR index\n");
        repo.write_doc("hosts.md", "# Host adapters overview\n");
        repo.write_doc("reference/commands.md", "# Commands\n");
        repo.write_doc("start-here/01-your-first-change.md", "# First change\n");
        repo.write_doc("architecture.md", "# Architecture overview\n");
        repo.write_doc("README.md", "# Introduction\n");

        let rendered = render_full(repo.path());

        assert!(
            rendered.find("# Summary").expect("summary section")
                < rendered.find("# Start here").expect("start section")
        );
        assert!(
            rendered.find("# Start here").expect("start section")
                < rendered
                    .find("# Architecture")
                    .expect("architecture section")
        );
        assert!(
            rendered
                .find("# Architecture")
                .expect("architecture section")
                < rendered.find("# Reference").expect("reference section")
        );
        assert!(
            rendered.find("# Reference").expect("reference section")
                < rendered.find("# Hosts").expect("hosts section")
        );
        assert!(
            rendered.find("# Hosts").expect("hosts section")
                < rendered.find("# Decisions").expect("decisions section")
        );
    }

    #[test]
    fn t001_sorts_files_alphabetically_within_schema_sections() {
        let repo = TestRepo::new("alphabetical");
        repo.write_doc("reference/glossary.md", "# Glossary\n");
        repo.write_doc("reference/artifacts.md", "# Artifacts\n");
        repo.write_doc("reference/commands.md", "# Commands\n");

        let rendered = render_full(repo.path());

        assert!(
            rendered
                .find("./reference/artifacts.md")
                .expect("artifacts")
                < rendered.find("./reference/commands.md").expect("commands")
        );
        assert!(
            rendered.find("./reference/commands.md").expect("commands")
                < rendered.find("./reference/glossary.md").expect("glossary")
        );
    }

    #[test]
    fn t001_uses_first_h1_as_link_text() {
        let repo = TestRepo::new("h1");
        repo.write_doc("README.md", "intro\n# Introduction\n# Later heading\n");

        let rendered = render_full(repo.path());

        assert!(rendered.contains("- [Introduction](./README.md)"));
        assert!(!rendered.contains("Later heading"));
    }

    #[test]
    fn t001_falls_back_to_filename_when_h1_is_missing() {
        let repo = TestRepo::new("missing-h1");
        repo.write_doc("reference/no-heading.md", "body without a heading\n");

        let rendered = render_full(repo.path());

        assert!(rendered.contains("- [no-heading](./reference/no-heading.md)"));
    }
}
