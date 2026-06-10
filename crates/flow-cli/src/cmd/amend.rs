//! `flow amend` — update the active change spec in place.

use crate::args::AmendArgs;
use flow_core::{envelope, parse, paths, status as status_helpers, Error, Result};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// Run `flow amend`.
pub fn run(args: AmendArgs) -> Result<()> {
    match super::finalize_mode(args.finalize) {
        super::FinalizeMode::Infer => {
            let repo = paths::repo_root(None)?;
            let dir = resolve_feature_dir(&repo)?;
            return finalize(&dir);
        }
        super::FinalizeMode::Skip => {}
    }
    prepare(args)
}

fn prepare(args: AmendArgs) -> Result<()> {
    let repo = paths::repo_root(None)?;
    let change = args.change.join(" ");

    // If the user passed --ask / --answer, append to Clarifications and return.
    if let (Some(q), Some(a)) = (&args.ask, &args.answer) {
        let feature_dir = resolve_feature_dir(&repo)?;
        parse::spec::append_clarification(&feature_dir.join("spec.md"), q, a)?;
        flow_core::logging::info("Appended Q/A to spec.md ## Clarifications.");
        crate::output::print_next(
            "flow-amend",
            "continue refining the spec, or move to the planning phase when done.",
        );
        return Ok(());
    }

    if looks_like_roadmap_amend_intent(&change) {
        let roadmap = crate::public_command::render_current("flow-roadmap");
        let start = crate::public_command::render_current("flow-start");
        let plan = crate::public_command::render_current("flow-plan");
        let amend = crate::public_command::render_current("flow-amend");
        return Err(Error::WrongCommand {
            suggested: format!(
                "{roadmap} to edit roadmap milestones; {start} M-N to create an active change from a milestone, then {plan} for tasks.md — {amend} only updates the active change's spec.md"
            ),
        });
    }

    let feature_dir = resolve_feature_dir(&repo)?;
    let extra = if change.trim().is_empty() {
        None
    } else {
        Some(format!("## User's Change Request\n\n{change}"))
    };
    let out = envelope::compose(&repo, "amend", &feature_dir, extra.as_deref())?;
    print!("{out}");
    crate::output::maybe_print_finalize_hint("amend", &feature_dir);
    crate::output::print_next("/flow-plan", "after the spec state is saved.");
    Ok(())
}

fn finalize(feature_dir: &Path) -> Result<()> {
    let spec = parse::spec::parse_file(&feature_dir.join("spec.md"))?;
    parse::spec::validate(&spec)?;
    let current = status_helpers::read(feature_dir)?;
    parse::status::stamp(
        feature_dir,
        current.state, // unchanged
        "spec-amended",
        "spec.md updated in place",
    )?;
    flow_core::logging::info("Spec amended. Flow state saved.");
    crate::output::print_next("/flow-plan", "refresh the plan and tasks.");
    Ok(())
}

/// Resolve the active change directory for this invocation.
pub(crate) fn resolve_feature_dir(repo: &Path) -> Result<std::path::PathBuf> {
    if let Ok(explicit) = std::env::var("FLOW_CHANGE_DIR") {
        return Ok(normalize_feature_dir_arg(repo, Path::new(&explicit)));
    }
    let branch = flow_core::git::current_branch(repo).unwrap_or_default();
    if let Some(run_feature) = resolve_run_feature_dir(repo, &branch) {
        return Ok(run_feature);
    }
    if let Some(found) = find_feature_by_branch(&paths::runs_dir(repo), &branch) {
        return Ok(found);
    }
    let amend = crate::public_command::render_current("flow-amend");
    let start = crate::public_command::render_current("flow-start");
    let plan = crate::public_command::render_current("flow-plan");
    let roadmap = crate::public_command::render_current("flow-roadmap");
    Err(Error::User(format!(
        "Cannot resolve active change directory (current branch: {branch}). \
{amend} updates the active change's spec.md under flow/runs/<run>/changes/<change>. \
Set FLOW_CHANGE_DIR to an existing change directory, or check out a branch recorded in a child change status.md, \
or rely on FLOW_RUN_DIR / run-state mapping. \
To start from a roadmap milestone: {start} M-N, then {plan} for tasks.md. \
To edit roadmap milestones: {roadmap}."
    )))
}

/// True when the free-form change text is clearly about the roadmap, not revising `spec.md`.
fn looks_like_roadmap_amend_intent(change: &str) -> bool {
    let lower = change.to_ascii_lowercase();
    let trimmed = lower.trim();
    if trimmed.is_empty() {
        return false;
    }
    if trimmed.contains("roadmap") || trimmed.contains("milestones") {
        return true;
    }
    // "Milestone" alongside implementation/task language usually means roadmap workflow, not a spec edit.
    trimmed.contains("milestone")
        && (trimmed.contains("task") || trimmed.contains("tasks") || trimmed.contains("implement"))
}

fn resolve_run_feature_dir(repo: &Path, branch: &str) -> Option<PathBuf> {
    if branch.is_empty() {
        return None;
    }

    if let Ok(raw) = std::env::var("FLOW_RUN_DIR") {
        let run_dir = normalize_run_dir_arg(repo, Path::new(&raw));
        if let Some(feature_dir) = run_feature_from_state(repo, branch, &run_dir) {
            return Some(feature_dir);
        }
    }

    let runs_dir = paths::runs_dir(repo);
    let mut run_dirs = std::fs::read_dir(runs_dir)
        .ok()?
        .filter_map(|entry| {
            let entry = entry.ok()?;
            entry.file_type().ok()?.is_dir().then(|| entry.path())
        })
        .collect::<Vec<_>>();
    run_dirs.sort_by(|a, b| b.cmp(a));
    for run_dir in run_dirs {
        if let Some(feature_dir) = run_feature_from_state(repo, branch, &run_dir) {
            return Some(feature_dir);
        }
    }
    None
}

fn run_feature_from_state(repo: &Path, branch: &str, run_dir: &Path) -> Option<PathBuf> {
    let state = super::run::read_run_state(run_dir).ok()?;
    if state.get("Run branch").map(String::as_str) != Some(branch) {
        return None;
    }
    feature_dir_from_run_state(repo, &state)
}

fn feature_dir_from_run_state(repo: &Path, state: &BTreeMap<String, String>) -> Option<PathBuf> {
    let raw = state.get("Current change")?.trim();
    if raw.is_empty() || raw == "(none)" {
        return None;
    }
    let path = Path::new(raw);
    let feature_dir = if path.is_absolute() {
        path.to_path_buf()
    } else {
        repo.join(path)
    };
    feature_dir
        .join("status.md")
        .is_file()
        .then_some(feature_dir)
}

fn normalize_run_dir_arg(repo: &Path, run_dir: &Path) -> PathBuf {
    if run_dir.is_absolute() {
        run_dir.to_path_buf()
    } else {
        repo.join(run_dir)
    }
}

/// Normalize an explicit change directory argument into the canonical run changes directory.
pub(crate) fn normalize_feature_dir_arg(repo: &Path, feature_dir: &Path) -> std::path::PathBuf {
    if feature_dir.exists() {
        return if feature_dir.is_absolute() {
            feature_dir.to_path_buf()
        } else {
            repo.join(feature_dir)
        };
    }
    let by_name = feature_dir
        .file_name()
        .and_then(|name| find_change_by_name(&paths::runs_dir(repo), name));
    if let Some(path) = by_name {
        return path;
    }
    feature_dir.to_path_buf()
}

fn find_feature_by_branch(dir: &Path, branch: &str) -> Option<std::path::PathBuf> {
    if let Ok(read) = std::fs::read_dir(dir) {
        for entry in read.flatten() {
            let path = entry.path();
            if entry.file_type().map(|t| !t.is_dir()).unwrap_or(true) {
                continue;
            }
            let status_path = path.join("status.md");
            if let Ok(text) = std::fs::read_to_string(&status_path) {
                if text
                    .lines()
                    .any(|l| l.trim_start().starts_with("**Branch**:") && l.contains(branch))
                {
                    return Some(path);
                }
            }
            if let Some(found) = find_feature_by_branch(&path, branch) {
                return Some(found);
            }
        }
    }
    None
}

fn find_change_by_name(root: &Path, name: &std::ffi::OsStr) -> Option<PathBuf> {
    let read = std::fs::read_dir(root).ok()?;
    for entry in read.flatten() {
        if entry.file_type().map(|t| !t.is_dir()).unwrap_or(true) {
            continue;
        }
        let path = entry.path();
        if path.file_name() == Some(name) && path.join("status.md").is_file() {
            return Some(path);
        }
        if let Some(found) = find_change_by_name(&path, name) {
            return Some(found);
        }
    }
    None
}
