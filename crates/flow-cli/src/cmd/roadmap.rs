//! `flow roadmap` — decompose a PRD or notes file into Flow roadmap milestones.

use crate::args::RoadmapArgs;
use chrono::Utc;
use flow_core::{config::Config, envelope, paths, roadmap, settings::Settings, Error, Result};
use std::io::Read;
use std::path::{Path, PathBuf};

/// Run `flow roadmap`.
pub fn run(args: RoadmapArgs) -> Result<()> {
    match super::finalize_mode(args.finalize) {
        super::FinalizeMode::Infer => {
            let repo = paths::repo_root(None)?;
            let run_dir = crate::cmd::run::active_run_context(&repo)?
                .map(|ctx| ctx.run_dir)
                .ok_or_else(|| {
                    Error::User(
                        "Cannot resolve roadmap run for `flow roadmap --finalize`. Set FLOW_RUN_DIR to the run directory.".into(),
                    )
                })?;
            return finalize(&run_dir);
        }
        super::FinalizeMode::Skip => {}
    }
    prepare(args)
}

fn prepare(args: RoadmapArgs) -> Result<()> {
    let repo = paths::repo_root(None)?;
    if crate::public_command::active_host().is_none() {
        return Err(Error::User(
            "flow roadmap is a host-assisted command and does not modify the roadmap when run directly from a shell. Run the Flow roadmap command from your configured host (Claude Code, Codex, Cursor), or set FLOW_HOST for the host that will consume the generated roadmap prompt."
                .to_string(),
        ));
    }

    // Resolve source content.
    let source_joined = args.source.join(" ");
    let (source_content, source_label) = if source_joined.trim().is_empty() {
        // Try stdin when not a TTY.
        use std::io::IsTerminal;
        if std::io::stdin().is_terminal() {
            return Err(Error::User(
                "Provide a source: 'flow roadmap path/to/prd.md' or pipe text via stdin"
                    .to_string(),
            ));
        }
        let mut buf = String::new();
        std::io::stdin().read_to_string(&mut buf)?;
        if buf.trim().is_empty() {
            return Err(Error::User(
                "Provide a source: 'flow roadmap path/to/prd.md' or pipe text via stdin"
                    .to_string(),
            ));
        }
        (buf, "stdin".to_string())
    } else {
        // Check if the joined string is a readable file path.
        let candidate = std::path::Path::new(source_joined.trim());
        if candidate.is_file() {
            let content = std::fs::read_to_string(candidate)
                .map_err(|e| Error::User(format!("Cannot read {}: {e}", candidate.display())))?;
            let label = format!("file: {}", candidate.display());
            (content, label)
        } else if looks_like_source_path(source_joined.trim()) {
            return Err(Error::User(format!(
                "Source file not found: {}",
                candidate.display()
            )));
        } else {
            (source_joined, "inline text".to_string())
        }
    };

    if source_content.trim().is_empty() {
        return Err(Error::User(
            "Provide a source: 'flow roadmap path/to/prd.md' or pipe text via stdin".to_string(),
        ));
    }

    let descriptor = roadmap_descriptor_from_source(&source_content, &source_label);
    let cfg = Config::load_for_repo(&repo).unwrap_or_default();
    let run_date = Utc::now().format("%Y%m%d").to_string();
    let run_dir = crate::cmd::run::create_planned_roadmap_run(
        &repo,
        &descriptor.title,
        &descriptor.slug,
        &cfg,
        &run_date,
    )?;
    let roadmap_path = crate::cmd::run::run_roadmap_path(&run_dir);
    let mode = if args.append { "append" } else { "replace" };

    // Compute next free milestone from the persistent counter setting.
    let settings = Settings::load_for_repo(&repo)?;
    let next_num = roadmap::next_available_milestone_number_at_paths(
        existing_run_roadmap_paths(&repo),
        settings.counter,
    )?;
    let next_free = format!("M-{next_num}");

    let current_roadmap =
        std::fs::read_to_string(&roadmap_path).unwrap_or_else(|_| "(empty)".to_string());
    let current_roadmap_display = if current_roadmap.trim().is_empty() {
        "(empty)".to_string()
    } else {
        current_roadmap
    };

    let extra = format!(
        "## Roadmap Operation\n\n**Operation**: {mode}\n**Run directory**: {}\n**Roadmap file**: {}\n**Next free milestone**: {next_free}\n**Source**: {source_label}\n\n## Source Content\n\n{source_content}\n\n## Current Roadmap\n\n{current_roadmap_display}",
        rel(&repo, &run_dir),
        rel(&repo, &roadmap_path),
    );

    // `--finalize` is a bare flag that conflicts with the positional source;
    // the run directory must travel via `FLOW_RUN_DIR` because the run is not
    // yet active when roadmap finalize executes.
    let finalize_command = format!(
        "FLOW_RUN_DIR=\"{}\" flow roadmap --finalize",
        rel(&repo, &run_dir)
    );
    let out = envelope::compose_with_save_command(
        &repo,
        "roadmap",
        &run_dir,
        Some(&extra),
        &finalize_command,
    )?;
    print!("{out}");
    crate::output::maybe_print_finalize_command(&finalize_command, &run_dir, "roadmap");
    crate::output::print_next("flow-run", "after the roadmap run is saved.");
    Ok(())
}

fn looks_like_source_path(source: &str) -> bool {
    let source = source.trim();
    if source.is_empty() {
        return false;
    }

    let path = Path::new(source);
    if path.is_absolute()
        || source.starts_with("./")
        || source.starts_with("../")
        || source.starts_with(".\\")
        || source.starts_with("..\\")
    {
        return true;
    }

    if let Some(separator_index) = source.find(['/', '\\']) {
        let first_segment = &source[..separator_index];
        return !first_segment.is_empty() && !first_segment.chars().any(char::is_whitespace);
    }

    if source.chars().any(char::is_whitespace) {
        return false;
    }

    let extension = path
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    matches!(
        extension.as_str(),
        "md" | "markdown" | "txt" | "rst" | "adoc" | "asciidoc" | "doc" | "docx"
    )
}

fn finalize(run_dir_arg: &Path) -> Result<()> {
    let repo = paths::repo_root(Some(run_dir_arg))?;
    let run_dir = normalize_run_dir_arg(&repo, run_dir_arg);
    let roadmap_path = crate::cmd::run::run_roadmap_path(&run_dir);
    if !roadmap_path.exists() {
        return Err(Error::User(format!(
            "{} not found — the agent must write it before finalize",
            roadmap_path.display()
        )));
    }
    // Validate the roadmap by parsing it.
    flow_core::parse::roadmap::parse_file(&roadmap_path)
        .map_err(|e| Error::User(format!("{} parse error: {e}", roadmap_path.display())))?;
    let count = roadmap::count_milestones_at_path(&roadmap_path);
    let fingerprint = crate::cmd::run::roadmap_fingerprint_at_path(&roadmap_path)?;
    crate::cmd::run::update_run_state(
        &run_dir,
        &[
            ("Roadmap fingerprint", &fingerprint),
            ("Current phase", "roadmap-ready"),
            ("Last saved Flow action", "roadmap-finalized"),
            ("Next command", "$flow-run"),
        ],
    )?;
    crate::cmd::run::refresh_milestone_snapshot(
        &run_dir,
        &crate::cmd::run::format_milestone_snapshot_from_path(&roadmap_path)?,
    )?;
    let mut settings = Settings::load_for_repo(&repo)?;
    let next_counter = roadmap::next_available_milestone_number_at_paths(
        existing_run_roadmap_paths(&repo),
        settings.counter,
    )?;
    if settings.counter != next_counter {
        settings.counter = next_counter;
        settings.save_for_repo(&repo)?;
    }
    flow_core::logging::info(format!(
        "Roadmap finalized. {count} milestones saved. Next counter: {next_counter}."
    ));
    Ok(())
}

struct RoadmapDescriptor {
    title: String,
    slug: String,
}

fn roadmap_descriptor_from_source(source_content: &str, source_label: &str) -> RoadmapDescriptor {
    let title = first_source_heading(source_content)
        .and_then(normalize_descriptor_candidate)
        .or_else(|| file_stem_descriptor(source_label))
        .or_else(|| first_meaningful_source_line(source_content))
        .unwrap_or_else(|| "Planned Work".to_string());
    let slug = paths::slugify(&title);
    RoadmapDescriptor { title, slug }
}

fn first_source_heading(source_content: &str) -> Option<&str> {
    source_content
        .lines()
        .find_map(|line| line.trim().strip_prefix("# ").map(str::trim))
        .filter(|title| !title.is_empty())
}

fn file_stem_descriptor(source_label: &str) -> Option<String> {
    source_label
        .strip_prefix("file: ")
        .and_then(|path| Path::new(path).file_stem())
        .and_then(|stem| stem.to_str())
        .and_then(normalize_descriptor_candidate)
}

fn first_meaningful_source_line(source_content: &str) -> Option<String> {
    source_content.lines().find_map(|line| {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            return None;
        }
        let candidate = trimmed.trim_start_matches('#').trim();
        normalize_descriptor_candidate(candidate)
    })
}

fn normalize_descriptor_candidate(candidate: &str) -> Option<String> {
    let candidate = candidate.replace(['_', '-'], " ");
    let candidate = collapse_whitespace(&candidate);
    let candidate = trim_roadmap_affixes(&candidate);
    let candidate = trim_descriptor_edges(&candidate);
    let slug = paths::slugify(&candidate);
    (!is_generic_descriptor_slug(&slug)).then_some(candidate)
}

fn collapse_whitespace(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn trim_roadmap_affixes(value: &str) -> String {
    let mut descriptor = value.trim().to_string();
    let lower = descriptor.to_ascii_lowercase();
    for prefix in ["roadmap:", "roadmap -", "roadmap "] {
        if lower.starts_with(prefix) {
            descriptor = descriptor[prefix.len()..].trim().to_string();
            break;
        }
    }
    let lower = descriptor.to_ascii_lowercase();
    if lower.ends_with(" roadmap") {
        descriptor.truncate(descriptor.len() - " roadmap".len());
    }
    descriptor.trim().to_string()
}

fn trim_descriptor_edges(value: &str) -> String {
    value
        .trim_matches(|ch: char| !ch.is_alphanumeric())
        .trim()
        .to_string()
}

fn is_generic_descriptor_slug(slug: &str) -> bool {
    matches!(
        slug,
        "" | "roadmap"
            | "full-roadmap"
            | "feature"
            | "project"
            | "implementation"
            | "build"
            | "add"
            | "fix"
    )
}

fn existing_run_roadmap_paths(repo: &Path) -> Vec<PathBuf> {
    let runs_dir = paths::runs_dir(repo);
    let mut paths = Vec::new();
    if let Ok(entries) = std::fs::read_dir(runs_dir) {
        for entry in entries.flatten() {
            let path = entry.path().join("roadmap.md");
            if path.is_file() {
                paths.push(path);
            }
        }
    }
    paths.sort();
    paths
}

fn normalize_run_dir_arg(repo: &Path, dir: &Path) -> PathBuf {
    if dir.is_absolute() {
        dir.to_path_buf()
    } else {
        repo.join(dir)
    }
}

fn rel(repo: &Path, path: &Path) -> String {
    path.strip_prefix(repo)
        .unwrap_or(path)
        .display()
        .to_string()
        .replace('\\', "/")
}
