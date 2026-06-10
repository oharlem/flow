//! `flow start` — draft a new change spec.

use crate::args::StartArgs;
use flow_core::{
    config::Config, envelope, git, parse, paths, prompt, render, resume, roadmap, Error, Result,
};
use std::path::{Path, PathBuf};

/// Run `flow start`.
pub fn run(args: StartArgs) -> Result<()> {
    match super::finalize_mode(args.finalize) {
        super::FinalizeMode::Infer => {
            let repo = paths::repo_root(None)?;
            let dir = super::amend::resolve_feature_dir(&repo)?;
            return finalize(&dir);
        }
        super::FinalizeMode::Skip => {}
    }
    prepare(args)
}

fn prepare(args: StartArgs) -> Result<()> {
    let repo = paths::repo_root(None)?;

    // Parse milestone + description.
    // Scan all positional tokens for M-N; extract exactly one.
    let mut desc_tokens: Vec<String> = args.description.clone();
    let mut milestone_id: Option<String> = None;

    let matches: Vec<usize> = desc_tokens
        .iter()
        .enumerate()
        .filter(|(_, t)| looks_like_milestone_token(t))
        .map(|(i, _)| i)
        .collect();
    if matches.len() >= 2 {
        let ids: Vec<&str> = matches.iter().map(|&i| desc_tokens[i].as_str()).collect();
        return Err(Error::User(format!(
            "at most one M-N token; received: {}",
            ids.join(", ")
        )));
    } else if matches.len() == 1 {
        let idx = matches[0];
        if !regex_milestone().is_match(&desc_tokens[idx]) {
            return Err(Error::InvalidId {
                kind: "M".into(),
                input: desc_tokens[idx].clone(),
                reason: "expected M-N".into(),
            });
        }
        milestone_id = Some(desc_tokens.remove(idx));
    }

    let mut description = desc_tokens.join(" ");

    if envelope::looks_like_spec_amendment(&description) {
        return Err(Error::WrongCommand {
            suggested: format!("/flow-amend {}", description.trim()),
        });
    }

    let milestone = if let Some(id) = &milestone_id {
        if !regex_milestone().is_match(id) {
            return Err(Error::InvalidId {
                kind: "M".into(),
                input: id.clone(),
                reason: "expected M-N".into(),
            });
        }
        let ctx = crate::cmd::run::active_run_context(&repo)?.ok_or_else(|| {
            Error::User(format!(
                "Milestone {id} belongs to a roadmap run. Set FLOW_RUN_DIR to the run directory before running `flow start {id}`."
            ))
        })?;
        let m = roadmap::require_milestone_at_path(
            &crate::cmd::run::run_roadmap_path(&ctx.run_dir),
            id,
        )?;
        if description.trim().is_empty() {
            description = m.title.clone();
        }
        Some(m)
    } else {
        None
    };

    // Allocate feature name.
    let feature_name = if let Some(m) = milestone.as_ref() {
        milestone_feature_name(m)
    } else {
        let source = description.trim();
        if source.is_empty() || !source.chars().any(|ch| ch.is_ascii_alphanumeric()) {
            return Err(Error::User(
                "Provide a short description or an M-N milestone ID.".into(),
            ));
        }
        let slug = paths::slugify(source);
        allocate_unique_feature_name(&repo, &slug)?
    };
    let branch = paths::branch_name(&repo, &feature_name);
    let run_context = crate::cmd::run::active_run_context(&repo)?;

    // Worktree or branch.
    let cfg = Config::load_for_repo(&repo).unwrap_or_default();
    let working_dir: PathBuf = if let Some(ctx) = &run_context {
        let current = git::current_branch(&repo).unwrap_or_else(|_| "unknown".into());
        if !ctx.run_branch.is_empty() && ctx.run_branch != "(none)" && current != ctx.run_branch {
            return Err(Error::User(format!(
                "FLOW_RUN_DIR points to a run on branch '{}', but the current branch is '{}'. Switch to the run branch before starting the milestone.",
                ctx.run_branch, current
            )));
        }
        repo.clone()
    } else if cfg.git.worktrees {
        git::create_worktree(&repo, &branch, &feature_name)?
    } else {
        let current = git::current_branch(&repo).unwrap_or_else(|_| "main".into());
        if git::branch_is_protected(&current, &cfg.git.protected_branches) {
            let settings = flow_core::settings::Settings::load_for_repo(&repo).unwrap_or_default();
            match prompt::confirm_protected_branch(&current, settings.confirmation.is_disabled()) {
                prompt::Confirmation::Proceed => {}
                prompt::Confirmation::Abort => {
                    return Err(Error::User(format!(
                        "Aborted on protected branch '{current}'. Run `flow start` from a work branch, or set FLOW_FORCE_ON_PROTECTED=1."
                    )));
                }
            }
        }
        if !git::branch_exists(&repo, &branch)? {
            git::create_branch(&repo, &branch)?;
        } else {
            git::switch_branch(&repo, &branch)?;
        }
        repo.clone()
    };

    let current_branch = git::current_branch(&working_dir).unwrap_or_else(|_| "unknown".into());
    let run_dir = if let Some(ctx) = &run_context {
        ctx.run_dir.clone()
    } else {
        crate::cmd::run::create_one_off_run(&repo, &feature_name, &current_branch, &cfg)?
    };
    let feature_dir = crate::cmd::run::change_dir(&run_dir, &feature_name);

    // Seed files.
    render::seed_feature_files(&feature_dir, &feature_name, &current_branch)?;

    // Apply milestone to status.md.
    if let Some(m) = &milestone {
        roadmap::set_status_milestone(&feature_dir, &m.id)?;
    }

    crate::cmd::run::update_run_state_for_start(
        &run_dir,
        milestone.as_ref().map(|m| m.id.as_str()),
        &feature_dir,
        &repo,
    )?;

    resume::check(&feature_dir);

    // Envelope
    let extra = build_extra_context(&description, &milestone);
    let out = envelope::compose(&working_dir, "start", &feature_dir, extra.as_deref())?;
    print_start_summary(&feature_name, &current_branch, milestone.as_ref());
    println!();
    print!("{out}");
    crate::output::maybe_print_finalize_hint("start", &feature_dir);
    crate::output::print_next("/flow-plan", "after the spec state is saved.");
    Ok(())
}

fn finalize(feature_dir: &Path) -> Result<()> {
    let spec = parse::spec::parse_file(&feature_dir.join("spec.md"))?;
    parse::spec::validate(&spec)?;
    flow_core::parse::status::stamp(
        feature_dir,
        Some(flow_core::parse::status::State::Drafting),
        "spec-complete",
        "spec.md drafted and confirmed",
    )?;
    crate::cmd::run::update_run_state_for_feature_phase(
        feature_dir,
        "spec-complete",
        "spec-complete",
        "$flow-plan",
    )?;
    flow_core::logging::info("Spec finalized. Flow state saved.");
    println!();
    let next = crate::public_command::render_current("flow-plan");
    println!("Spec drafted. Next: run {next} to draft the implementation plan and task list.");
    crate::output::print_next("flow-plan", "draft the implementation plan and task list.");
    Ok(())
}

fn regex_milestone() -> &'static regex::Regex {
    static RE: once_cell::sync::Lazy<regex::Regex> =
        once_cell::sync::Lazy::new(|| regex::Regex::new(r"^M-[1-9]\d*$").unwrap());
    &RE
}

fn looks_like_milestone_token(token: &str) -> bool {
    static RE: once_cell::sync::Lazy<regex::Regex> =
        once_cell::sync::Lazy::new(|| regex::Regex::new(r"^M-\d+$").unwrap());
    RE.is_match(token)
}

fn milestone_feature_name(milestone: &flow_core::parse::roadmap::Milestone) -> String {
    let title = milestone.title.trim();
    if title.is_empty() {
        milestone.id.clone()
    } else {
        format!("{}-{}", milestone.id, paths::slugify(title))
    }
}

fn allocate_unique_feature_name(repo: &Path, slug: &str) -> Result<String> {
    let mut candidate = slug.to_string();
    for index in 2.. {
        if feature_name_available(repo, &candidate)? {
            return Ok(candidate);
        }
        candidate = format!("{slug}-{index}");
    }
    unreachable!("unbounded feature-name suffix search must return")
}

fn feature_name_available(repo: &Path, name: &str) -> Result<bool> {
    if change_exists(repo, name) {
        return Ok(false);
    }
    let branch = paths::branch_name(repo, name);
    Ok(!git::branch_exists(repo, &branch)?)
}

fn change_exists(repo: &Path, name: &str) -> bool {
    let runs = paths::runs_dir(repo);
    find_change_by_name(&runs, name)
}

fn find_change_by_name(root: &Path, name: &str) -> bool {
    let Ok(read) = std::fs::read_dir(root) else {
        return false;
    };
    read.flatten().any(|entry| {
        if !entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
            return false;
        }
        let path = entry.path();
        if path.file_name().and_then(|n| n.to_str()) == Some(name)
            && path.join("status.md").is_file()
        {
            return true;
        }
        find_change_by_name(&path, name)
    })
}

fn print_start_summary(
    feature_name: &str,
    branch: &str,
    milestone: Option<&flow_core::parse::roadmap::Milestone>,
) {
    println!("## Flow Start Summary");
    println!();
    println!("Created change `{feature_name}` on branch `{branch}`.");
    if let Some(m) = milestone {
        println!(
            "Linked existing roadmap milestone `{id}`: {title}.",
            id = m.id,
            title = m.title
        );
    } else {
        println!("No roadmap milestone linked; roadmap was left unchanged.");
    }
}

fn build_extra_context(
    description: &str,
    milestone: &Option<flow_core::parse::roadmap::Milestone>,
) -> Option<String> {
    let mut parts: Vec<String> = Vec::new();
    if !description.trim().is_empty() {
        parts.push(format!(
            "## User's Change Description\n\n{desc}",
            desc = description.trim()
        ));
    }
    if let Some(m) = milestone {
        let mut block = format!(
            "## Linked Milestone\n\nThis change is linked to roadmap milestone **{id}**: {title}.",
            id = m.id,
            title = m.title
        );
        if !m.description.trim().is_empty() {
            block.push_str(&format!(
                "\n\n> {desc}\n\nThe description has been seeded into `spec.md`'s `## What & Why`. Refine it as needed.",
                desc = m.description.trim()
            ));
        }
        parts.push(block);
    }
    if parts.is_empty() {
        None
    } else {
        Some(parts.join("\n\n"))
    }
}
