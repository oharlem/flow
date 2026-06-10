//! `flow run` — automate one milestone or the full roadmap.

use crate::args::RunArgs;
use chrono::Utc;
use flow_core::{
    config::Config, envelope, git, parse, paths, prompt, render, settings::Settings, Error, Result,
};
use regex::Regex;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

pub(crate) const EMPTY_ROADMAP: &str = "# Roadmap\n\n## Milestones\n";

/// Run `flow run`.
pub fn run(args: RunArgs) -> Result<()> {
    let repo = paths::repo_root(None)?;
    match super::finalize_mode(args.finalize) {
        super::FinalizeMode::Infer => {
            let dir = active_run_dir_or_error(&repo, "flow run --finalize")?;
            return finalize(&repo, &dir);
        }
        super::FinalizeMode::Skip => {}
    }
    if let Some(dir) = args.resume {
        let dir = resolve_run_dir_arg(&repo, dir, "flow run --resume")?;
        return resume(&repo, &dir);
    }
    if let Some(dir) = args.rescan {
        let dir = resolve_run_dir_arg(&repo, dir, "flow run --rescan")?;
        return rescan(&repo, &dir);
    }
    if let Some(dir) = args.checkpoint {
        let dir = normalize_run_dir_arg(&repo, &dir);
        let milestone = args
            .milestone
            .as_deref()
            .ok_or_else(|| Error::User("flow run --checkpoint requires --milestone M-N".into()))?;
        return checkpoint(&repo, &dir, milestone);
    }
    prepare(args.target.as_deref())
}

/// Resolve the active run directory when `flow run --finalize` is invoked
/// without an explicit path. Falls back to `FLOW_RUN_DIR` via
/// `active_run_context`. Returns the FR-005-style actionable error when no
/// active run can be determined.
fn active_run_dir_or_error(repo: &Path, command: &str) -> Result<PathBuf> {
    if let Some(ctx) = active_run_context(repo)? {
        return Ok(ctx.run_dir);
    }
    Err(Error::User(format!(
        "Cannot resolve active run directory for `{command}`. Set FLOW_RUN_DIR to the run directory, or pass `{command} \"<run-dir>\"` explicitly."
    )))
}

fn resolve_run_dir_arg(repo: &Path, dir: Option<PathBuf>, command: &str) -> Result<PathBuf> {
    dir.map(|dir| normalize_run_dir_arg(repo, &dir))
        .map(Ok)
        .unwrap_or_else(|| active_run_dir_or_error(repo, command))
}

fn prepare(target: Option<&str>) -> Result<()> {
    let repo = paths::repo_root(None)?;
    let cfg = Config::load_for_repo(&repo).unwrap_or_default();
    let open = active_or_single_open_roadmap_run(&repo)?;
    let Some(mut open) = open else {
        return Err(Error::User(
            "No planned or running roadmap run found. Run `flow roadmap <source>` first, or set FLOW_RUN_DIR to an existing roadmap run.".into(),
        ));
    };
    if target.is_none() && open.state.get("Status").map(String::as_str) == Some("running") {
        return continue_running_run(&repo, &open.run_dir, &open.state);
    }
    let target = RunTarget::resolve(&repo, target.unwrap_or("all"), &open.run_dir)?;
    let was_planned = open.state.get("Status").map(String::as_str) == Some("planned");
    if was_planned {
        ensure_run_branch_for_start(&repo, &open.run_dir, &open.state, &cfg)?;
        open.state = read_run_state(&open.run_dir)?;
    }
    let summary_kind = if was_planned {
        RunSummaryKind::Started
    } else {
        RunSummaryKind::Attached
    };
    attach_to_run(&repo, &target, &open.run_dir, &open.state)?;
    update_run_state(&open.run_dir, &[("Status", "running")])?;
    let run_dir = open.run_dir;
    let extra = build_extra_context(&repo, &target, &run_dir)?;
    let out = envelope::compose(&repo, "run", &repo, Some(&extra))?;
    print_run_summary(&repo, &target, &run_dir, summary_kind);
    println!();
    print!("{out}");
    maybe_print_finalize_hint(&run_dir);
    crate::output::print_next("flow-run", "continue this roadmap automation run.");
    Ok(())
}

fn continue_running_run(
    repo: &Path,
    run_dir: &Path,
    state: &BTreeMap<String, String>,
) -> Result<()> {
    let run_branch = state.get("Run branch").map(String::as_str).unwrap_or("");
    ensure_on_run_branch(repo, run_branch)?;
    let target = running_run_target(run_dir, state)?;
    let extra = build_extra_context(repo, &target, run_dir)?;
    let out = envelope::compose(repo, "run", repo, Some(&extra))?;
    print_run_summary(repo, &target, run_dir, RunSummaryKind::Continuing);
    println!();
    print!("{out}");
    maybe_print_finalize_hint(run_dir);
    crate::output::print_next("flow-run", "continue this roadmap automation run.");
    Ok(())
}

fn running_run_target(run_dir: &Path, state: &BTreeMap<String, String>) -> Result<RunTarget> {
    let roadmap_path = run_roadmap_path(run_dir);
    let milestones = parse::roadmap::parse_file(&roadmap_path)
        .map_err(|e| Error::User(format!("Cannot read roadmap for run continuation: {e}")))?;
    if state.get("Run scope").map(String::as_str) == Some("single") {
        let current = state
            .get("Current milestone")
            .map(String::as_str)
            .unwrap_or("(none)");
        if milestone_re().is_match(current) {
            if let Some(milestone) = milestones
                .iter()
                .find(|milestone| milestone.id == current && !milestone.done)
            {
                return Ok(RunTarget::Milestone {
                    id: milestone.id.clone(),
                    title: milestone.title.clone(),
                    description: milestone.description.clone(),
                });
            }
        }
    }
    Ok(RunTarget::All {
        milestones: milestones
            .into_iter()
            .filter(|milestone| !milestone.done)
            .collect(),
    })
}

#[derive(Clone, Debug)]
enum RunTarget {
    Milestone {
        id: String,
        title: String,
        description: String,
    },
    All {
        milestones: Vec<parse::roadmap::Milestone>,
    },
}

impl RunTarget {
    fn resolve(_repo: &Path, raw: &str, run_dir: &Path) -> Result<Self> {
        let roadmap_path = run_roadmap_path(run_dir);
        let trimmed = raw.trim();
        if trimmed.eq_ignore_ascii_case("all") {
            let milestones = parse::roadmap::parse_file(&roadmap_path)
                .map_err(|e| Error::User(format!("Cannot read roadmap for full run: {e}")))?
                .into_iter()
                .filter(|m| !m.done)
                .collect::<Vec<_>>();
            if milestones.is_empty() {
                return Err(Error::User(format!(
                    "No open milestones found in {}.",
                    roadmap_path.display()
                )));
            }
            return Ok(Self::All { milestones });
        }

        if !milestone_re().is_match(trimmed) {
            return Err(Error::InvalidId {
                kind: "M".into(),
                input: trimmed.to_string(),
                reason: "expected M-N or the literal target all".into(),
            });
        }
        let milestone = flow_core::roadmap::require_milestone_at_path(&roadmap_path, trimmed)?;
        if milestone.done {
            return Err(Error::User(format!(
                "Milestone {} is already checked in the roadmap.",
                milestone.id
            )));
        }
        Ok(Self::Milestone {
            id: milestone.id,
            title: milestone.title,
            description: milestone.description,
        })
    }

    fn label(&self) -> String {
        match self {
            Self::Milestone { id, title, .. } => format!("{id}: {title}"),
            Self::All { milestones, .. } => {
                format!("all open roadmap milestones ({})", milestones.len())
            }
        }
    }

    fn first_milestone_id(&self) -> Option<&str> {
        match self {
            Self::Milestone { id, .. } => Some(id.as_str()),
            Self::All { milestones, .. } => milestones.first().map(|m| m.id.as_str()),
        }
    }

    fn single_milestone_id(&self) -> Option<&str> {
        match self {
            Self::Milestone { id, .. } => Some(id.as_str()),
            Self::All { .. } => None,
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct ActiveRunContext {
    pub(crate) run_dir: PathBuf,
    pub(crate) run_branch: String,
}

#[derive(Clone, Debug)]
pub struct OpenRoadmapRun {
    pub run_dir: PathBuf,
    pub state: BTreeMap<String, String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum RunKind {
    OneOff,
    Roadmap,
}

impl RunKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::OneOff => "one-off",
            Self::Roadmap => "roadmap",
        }
    }
}

pub fn roadmap_fingerprint_at_path(roadmap_path: &Path) -> Result<String> {
    let text = std::fs::read_to_string(roadmap_path).map_err(|_| Error::FileNotFound {
        kind: "roadmap.md".into(),
        path: roadmap_path.to_path_buf(),
    })?;
    Ok(flow_core::roadmap::fingerprint(&text))
}

pub fn find_open_roadmap_run(repo: &Path) -> Result<Option<OpenRoadmapRun>> {
    find_single_roadmap_run(repo)
}

fn active_or_single_open_roadmap_run(repo: &Path) -> Result<Option<OpenRoadmapRun>> {
    if let Some(ctx) = active_run_context(repo)? {
        let state = read_run_state(&ctx.run_dir)?;
        if state.get("Run type").map(String::as_str) != Some("roadmap") {
            return Err(Error::User(format!(
                "FLOW_RUN_DIR points to {}, but that run is not a roadmap run.",
                rel(repo, &ctx.run_dir)
            )));
        }
        ensure_run_roadmap_fingerprint_current(repo, &ctx.run_dir, &state)?;
        return Ok(Some(OpenRoadmapRun {
            run_dir: ctx.run_dir,
            state,
        }));
    }
    find_single_roadmap_run(repo)
}

fn find_single_roadmap_run(repo: &Path) -> Result<Option<OpenRoadmapRun>> {
    let runs_dir = paths::runs_dir(repo);
    if !runs_dir.exists() {
        return Ok(None);
    }
    let mut dirs = std::fs::read_dir(&runs_dir)?
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| path.is_dir() && path.join("run.md").is_file())
        .collect::<Vec<_>>();
    dirs.sort();

    let mut active = Vec::new();
    for run_dir in dirs {
        let state = read_run_state(&run_dir)?;
        if state.get("Run type").map(String::as_str) != Some("roadmap") {
            continue;
        }
        let status = state.get("Status").map(String::as_str).unwrap_or("");
        if !matches!(status, "planned" | "running") {
            continue;
        }
        ensure_run_roadmap_fingerprint_current(repo, &run_dir, &state)?;
        active.push(OpenRoadmapRun { run_dir, state });
    }
    match active.len() {
        0 => Ok(None),
        1 => Ok(active.pop()),
        _ => {
            let listing = active
                .iter()
                .map(|run| format!("- {}", rel(repo, &run.run_dir)))
                .collect::<Vec<_>>()
                .join("\n");
            Err(Error::User(format!(
                "Multiple planned or running roadmap runs found. Set FLOW_RUN_DIR to one run directory or pass an explicit run command from that run:\n{listing}"
            )))
        }
    }
}

fn ensure_run_roadmap_fingerprint_current(
    repo: &Path,
    run_dir: &Path,
    state: &BTreeMap<String, String>,
) -> Result<()> {
    let expected = roadmap_fingerprint_at_path(&run_roadmap_path(run_dir))?;
    let actual = state
        .get("Roadmap fingerprint")
        .map(String::as_str)
        .unwrap_or("(missing)");
    if actual != expected {
        return Err(Error::User(format!(
            "Open roadmap run {} has run-local roadmap fingerprint {actual}, but {} is {expected}. Run `flow run --rescan` if the roadmap edit belongs to this run, or `flow run --finalize` to finish the open run before starting a different roadmap.",
            rel(repo, run_dir),
            rel(repo, &run_roadmap_path(run_dir))
        )));
    }
    Ok(())
}

pub(crate) fn active_run_context(repo: &Path) -> Result<Option<ActiveRunContext>> {
    let Some(raw) = std::env::var_os("FLOW_RUN_DIR") else {
        return Ok(None);
    };
    let run_dir = normalize_run_dir_arg(repo, Path::new(&raw));
    let state = read_run_state(&run_dir)?;
    let run_branch = state.get("Run branch").cloned().unwrap_or_default();
    Ok(Some(ActiveRunContext {
        run_dir,
        run_branch,
    }))
}

pub(crate) fn create_one_off_run(
    repo: &Path,
    change_name: &str,
    run_branch: &str,
    cfg: &Config,
) -> Result<PathBuf> {
    let run_date = Utc::now().format("%Y%m%d").to_string();
    let title = change_name.replace('-', " ");
    let run_dir = create_run_workspace(
        repo,
        RunKind::OneOff,
        &title,
        change_name,
        change_name,
        Some(run_branch),
        cfg,
        &run_date,
        "running",
        "(none)",
        "$flow-start",
        "(none)".to_string(),
        "(none)".to_string(),
        None,
    )?;
    Ok(run_dir)
}

pub(crate) fn create_planned_roadmap_run(
    repo: &Path,
    run_title: &str,
    run_slug: &str,
    cfg: &Config,
    run_date: &str,
) -> Result<PathBuf> {
    let run_dir = create_run_workspace(
        repo,
        RunKind::Roadmap,
        run_title,
        "planned roadmap",
        &format!("roadmap-{run_slug}"),
        None,
        cfg,
        run_date,
        "planned",
        "(none)",
        "$flow-run",
        flow_core::roadmap::fingerprint(EMPTY_ROADMAP),
        "(none)".to_string(),
        Some(EMPTY_ROADMAP),
    )?;
    Ok(run_dir)
}

pub(crate) fn run_roadmap_path(run_dir: &Path) -> PathBuf {
    run_dir.join("roadmap.md")
}

pub(crate) fn change_dir(run_dir: &Path, change_name: &str) -> PathBuf {
    run_dir.join("changes").join(change_name)
}

fn attach_to_run(
    repo: &Path,
    target: &RunTarget,
    run_dir: &Path,
    state: &BTreeMap<String, String>,
) -> Result<()> {
    ensure_target_can_attach(state, target)?;
    let run_branch = state.get("Run branch").map(String::as_str).unwrap_or("");
    ensure_on_run_branch(repo, run_branch)?;
    let prior_milestone = state
        .get("Current milestone")
        .map(String::as_str)
        .unwrap_or("(none)");
    let prior_scope = state
        .get("Run scope")
        .map(String::as_str)
        .unwrap_or("(none)");
    let current_milestone = target.first_milestone_id().unwrap_or("(none)");
    let new_scope = derive_run_scope(prior_scope, prior_milestone, current_milestone, target);
    let next_command = current_milestone
        .strip_prefix("M-")
        .map(|_| format!("$flow-start {current_milestone}"))
        .unwrap_or_else(|| "$flow-run".to_string());
    update_run_state(
        run_dir,
        &[
            ("Current milestone", current_milestone),
            ("Current phase", "run-attached"),
            ("Last saved Flow action", "run-attached"),
            ("Next command", &next_command),
            ("Run scope", new_scope),
        ],
    )?;
    append_line(
        &run_dir.join("log.md"),
        &format!(
            "- {} — run-attached — Attached `{}` to the open roadmap run.\n",
            render::now_iso(),
            target.label()
        ),
    )?;
    Ok(())
}

/// Decide the `Run scope` value after an attach.
///
/// Once `all`, always `all`. A second single-milestone target that differs from
/// the current milestone escalates to `all` because the run is now covering
/// multiple milestones.
fn derive_run_scope<'a>(
    prior_scope: &str,
    prior_milestone: &str,
    new_milestone: &str,
    target: &RunTarget,
) -> &'a str {
    if matches!(target, RunTarget::All { .. }) {
        return "all";
    }
    match prior_scope {
        "all" => "all",
        "single" => {
            if prior_milestone == "(none)"
                || prior_milestone.is_empty()
                || prior_milestone == new_milestone
            {
                "single"
            } else {
                "all"
            }
        }
        _ => "single",
    }
}

fn ensure_target_can_attach(state: &BTreeMap<String, String>, target: &RunTarget) -> Result<()> {
    let Some(requested) = target.single_milestone_id() else {
        return Ok(());
    };
    let current = state
        .get("Current milestone")
        .map(String::as_str)
        .unwrap_or("(none)");
    if current == requested || current == "(none)" || current.is_empty() {
        return Ok(());
    }
    let phase = state
        .get("Current phase")
        .map(String::as_str)
        .unwrap_or("(unknown)");
    if matches!(phase, "checkpoint-complete" | "completed" | "run-complete") {
        return Ok(());
    }
    Err(Error::User(format!(
        "This roadmap run is already tracking {current} in phase `{phase}`; finish the in-progress milestone before starting {requested}."
    )))
}

pub(crate) fn update_run_state_for_start(
    run_dir: &Path,
    milestone: Option<&str>,
    change_dir: &Path,
    repo: &Path,
) -> Result<()> {
    let change = rel(repo, change_dir);
    update_run_state(
        run_dir,
        &[
            ("Current milestone", milestone.unwrap_or("(none)")),
            ("Current change", &change),
            ("Current phase", "start"),
            ("Last saved Flow action", "started"),
            ("Next command", "$flow-plan"),
        ],
    )?;
    append_change_index(run_dir, &change, milestone)
}

pub(crate) fn update_run_state_for_feature_phase(
    feature_dir: &Path,
    phase: &str,
    action: &str,
    next_command: &str,
) -> Result<()> {
    let repo = paths::repo_root(Some(feature_dir)).unwrap_or_else(|_| feature_dir.to_path_buf());
    update_active_run_state_for_path(&repo, feature_dir, phase, action, next_command)
}

pub(crate) fn update_run_state_after_close(
    repo: &Path,
    change_dir: &Path,
    milestone: Option<&str>,
) -> Result<()> {
    let Some(ctx) = active_run_context(repo)?.or_else(|| inferred_run_context(repo, change_dir))
    else {
        return Ok(());
    };
    ensure_on_run_branch(repo, &ctx.run_branch)?;
    let state = read_run_state(&ctx.run_dir)?;
    let is_roadmap = state.get("Run type").map(String::as_str) == Some("roadmap");
    let mut roadmap_fingerprint = state
        .get("Roadmap fingerprint")
        .cloned()
        .unwrap_or_else(|| "(none)".to_string());
    let next_command = if is_roadmap {
        let roadmap_path = run_roadmap_path(&ctx.run_dir);
        let roadmap_text = std::fs::read_to_string(&roadmap_path).unwrap_or_default();
        roadmap_fingerprint = flow_core::roadmap::fingerprint(&roadmap_text);
        if checkpoint_commits_enabled(&state) {
            milestone
                .map(|m| {
                    format!(
                        "flow run --checkpoint \"{}\" --milestone {m}",
                        rel(repo, &ctx.run_dir)
                    )
                })
                .unwrap_or_else(|| "$flow-run".to_string())
        } else if roadmap_milestones_complete_at_path(&roadmap_path)? {
            "flow run --finalize".to_string()
        } else {
            "$flow-run".to_string()
        }
    } else {
        "flow run --finalize".to_string()
    };
    let change = rel(repo, change_dir);
    update_run_state(
        &ctx.run_dir,
        &[
            ("Roadmap fingerprint", &roadmap_fingerprint),
            ("Current change", &change),
            ("Current phase", "closed"),
            ("Last saved Flow action", "closed"),
            ("Next command", &next_command),
        ],
    )?;
    if is_roadmap {
        let roadmap_text =
            std::fs::read_to_string(run_roadmap_path(&ctx.run_dir)).unwrap_or_default();
        refresh_milestone_snapshot(&ctx.run_dir, &format_milestone_snapshot(&roadmap_text))?;
    }
    mark_change_index_closed(&ctx.run_dir, &change)?;
    append_line(
        &ctx.run_dir.join("log.md"),
        &format!(
            "- {} — change-closed — Closed {}.\n",
            render::now_iso(),
            change
        ),
    )
}

fn update_active_run_state_for_path(
    repo: &Path,
    feature_dir: &Path,
    phase: &str,
    action: &str,
    next_command: &str,
) -> Result<()> {
    let Some(ctx) = active_run_context(repo)?.or_else(|| inferred_run_context(repo, feature_dir))
    else {
        return Ok(());
    };
    ensure_on_run_branch(repo, &ctx.run_branch)?;
    let change = rel(repo, feature_dir);
    update_run_state(
        &ctx.run_dir,
        &[
            ("Current change", &change),
            ("Current phase", phase),
            ("Last saved Flow action", action),
            ("Next command", next_command),
        ],
    )
}

fn inferred_run_context(repo: &Path, change_dir: &Path) -> Option<ActiveRunContext> {
    let run_dir = run_dir_for_change(repo, change_dir)?;
    let state = read_run_state(&run_dir).ok()?;
    Some(ActiveRunContext {
        run_dir,
        run_branch: state.get("Run branch").cloned().unwrap_or_default(),
    })
}

pub(crate) fn run_dir_for_change(repo: &Path, change_dir: &Path) -> Option<PathBuf> {
    let change_dir = if change_dir.is_absolute() {
        change_dir.to_path_buf()
    } else {
        repo.join(change_dir)
    };
    let change_dir = std::fs::canonicalize(&change_dir).unwrap_or(change_dir);
    let runs_dir = paths::runs_dir(repo);
    let runs_dir = std::fs::canonicalize(&runs_dir).unwrap_or(runs_dir);
    let rel = change_dir.strip_prefix(&runs_dir).ok()?;
    let mut components = rel.components();
    let run_name = components.next()?.as_os_str();
    if components.next()?.as_os_str() != "changes" {
        return None;
    }
    components.next()?;
    let run_dir = runs_dir.join(run_name);
    run_dir.join("run.md").is_file().then_some(run_dir)
}

pub(crate) fn roadmap_path_for_change(repo: &Path, change_dir: &Path) -> Result<Option<PathBuf>> {
    let Some(ctx) = active_run_context(repo)?.or_else(|| inferred_run_context(repo, change_dir))
    else {
        return Ok(None);
    };
    let state = read_run_state(&ctx.run_dir)?;
    if state.get("Run type").map(String::as_str) == Some("roadmap") {
        Ok(Some(run_roadmap_path(&ctx.run_dir)))
    } else {
        Ok(None)
    }
}

fn milestone_re() -> &'static Regex {
    static RE: once_cell::sync::Lazy<Regex> =
        once_cell::sync::Lazy::new(|| Regex::new(r"^M-[1-9]\d*$").unwrap());
    &RE
}

fn create_run_branch(
    repo: &Path,
    run_dir: &Path,
    run_slug: &str,
    cfg: &Config,
    run_date: &str,
) -> Result<Option<String>> {
    if !cfg.git.run_branch {
        return Ok(None);
    }
    if cfg.git.run_checkpoint_commits {
        ensure_clean_or_only_planned_run_changes(repo, run_dir)?;
    }

    let current = git::current_branch(repo).unwrap_or_else(|_| "main".into());
    if git::branch_is_protected(&current, &cfg.git.protected_branches) {
        let settings = Settings::load_for_repo(repo).unwrap_or_default();
        match prompt::confirm_protected_branch(&current, settings.confirmation.is_disabled()) {
            prompt::Confirmation::Proceed => {}
            prompt::Confirmation::Abort => {
                return Err(Error::User(format!(
                    "Aborted on protected branch '{current}'. Set FLOW_FORCE_ON_PROTECTED=1 or run from a non-protected branch."
                )));
            }
        }
    }

    let base = format!("{}/run-{run_date}-roadmap-{run_slug}", paths::prefix(repo));
    let branch = next_available_branch(repo, &base)?;
    git::create_branch(repo, &branch)?;
    Ok(Some(branch))
}

fn ensure_clean_or_only_planned_run_changes(repo: &Path, run_dir: &Path) -> Result<()> {
    let dirty_paths = git::dirty_paths(repo)?;
    if dirty_paths.is_empty() || dirty_paths_are_planned_run_bootstrap(repo, run_dir, &dirty_paths)
    {
        return Ok(());
    }
    Err(Error::User(
        "`flow run` requires a clean worktree when git.run_checkpoint_commits=true. Commit or stash unrelated changes, or set git.run_checkpoint_commits=false.".into(),
    ))
}

fn dirty_paths_are_planned_run_bootstrap(
    repo: &Path,
    run_dir: &Path,
    dirty_paths: &[String],
) -> bool {
    let run_rel = rel(repo, run_dir);
    let run_prefix = format!("{}/", run_rel.trim_end_matches('/'));
    dirty_paths
        .iter()
        .all(|path| path == ".flow/state.yaml" || path == &run_rel || path.starts_with(&run_prefix))
}

fn ensure_run_branch_for_start(
    repo: &Path,
    run_dir: &Path,
    state: &BTreeMap<String, String>,
    cfg: &Config,
) -> Result<()> {
    let existing = state.get("Run branch").map(String::as_str).unwrap_or("");
    if !existing.is_empty() && existing != "(none)" {
        return ensure_on_run_branch(repo, existing);
    }
    if !cfg.git.run_branch {
        update_run_state(run_dir, &[("Checkpoint commits", "disabled")])?;
        return Ok(());
    }
    let (run_date, run_slug) = run_date_and_slug(run_dir);
    let branch = create_run_branch(repo, run_dir, &run_slug, cfg, &run_date)?
        .unwrap_or_else(|| "(none)".to_string());
    let checkpoint_commits = if cfg.git.run_checkpoint_commits {
        "enabled"
    } else {
        "disabled"
    };
    update_run_state(
        run_dir,
        &[
            ("Run branch", &branch),
            ("Checkpoint commits", checkpoint_commits),
        ],
    )?;
    Ok(())
}

fn run_date_and_slug(run_dir: &Path) -> (String, String) {
    let name = run_dir
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();
    let mut parts = name.splitn(2, '-');
    let date = parts.next().unwrap_or_default();
    let rest = parts.next().unwrap_or(name.as_str());
    let slug = rest.strip_prefix("roadmap-").unwrap_or(rest);
    let run_date = if date.len() == 8 && date.chars().all(|ch| ch.is_ascii_digit()) {
        date.to_string()
    } else {
        Utc::now().format("%Y%m%d").to_string()
    };
    (run_date, slug.to_string())
}

#[allow(clippy::too_many_arguments)]
fn create_run_workspace(
    repo: &Path,
    kind: RunKind,
    run_title: &str,
    run_target: &str,
    slug: &str,
    run_branch: Option<&str>,
    cfg: &Config,
    run_date: &str,
    status: &str,
    current_milestone: &str,
    next_command: &str,
    roadmap_fingerprint: String,
    milestones: String,
    roadmap_text: Option<&str>,
) -> Result<PathBuf> {
    let runs_dir = paths::runs_dir(repo);
    std::fs::create_dir_all(&runs_dir)?;
    let base_name = format!("{run_date}-{slug}");
    let first_candidate_index = next_run_index_after_existing(&runs_dir, &base_name)?;
    cleanup_abandoned_run_skeletons(&runs_dir, &base_name)?;

    let now = render::now_iso();
    for index in first_candidate_index.. {
        let run_name = suffixed_run_name(&base_name, index);
        let run_dir = runs_dir.join(&run_name);
        if run_dir.exists() {
            continue;
        }

        let staging_dir = create_staging_run_dir(&runs_dir, &run_name)?;
        let write_result = write_run_workspace_files(
            &staging_dir,
            &run_name,
            kind,
            run_title,
            run_target,
            run_branch,
            cfg,
            status,
            current_milestone,
            next_command,
            &roadmap_fingerprint,
            &milestones,
            roadmap_text,
            &now,
        );
        if let Err(err) = write_result {
            let _ = std::fs::remove_dir_all(&staging_dir);
            return Err(err);
        }

        match std::fs::rename(&staging_dir, &run_dir) {
            Ok(()) => {
                let _ = std::fs::remove_dir(runs_dir.join(".tmp"));
                return Ok(run_dir);
            }
            Err(_err) if run_dir.exists() => {
                let _ = std::fs::remove_dir_all(&staging_dir);
                continue;
            }
            Err(err) => {
                let _ = std::fs::remove_dir_all(&staging_dir);
                return Err(err.into());
            }
        }
    }
    unreachable!("unbounded run directory suffix search must return")
}

#[allow(clippy::too_many_arguments)]
fn write_run_workspace_files(
    run_dir: &Path,
    run_name: &str,
    kind: RunKind,
    run_title: &str,
    run_target: &str,
    run_branch: Option<&str>,
    cfg: &Config,
    status: &str,
    current_milestone: &str,
    next_command: &str,
    roadmap_fingerprint: &str,
    milestones: &str,
    roadmap_text: Option<&str>,
    now: &str,
) -> Result<()> {
    std::fs::create_dir_all(run_dir.join("changes"))?;

    let mut vars: BTreeMap<&str, String> = BTreeMap::new();
    vars.insert("RUN_NAME", run_name.to_string());
    vars.insert("RUN_TYPE", kind.as_str().to_string());
    vars.insert("RUN_SCOPE", "(none)".to_string());
    vars.insert("STATUS", status.to_string());
    vars.insert("RUN_TITLE", run_title.to_string());
    vars.insert("RUN_TARGET", run_target.to_string());
    vars.insert("ISO_DATETIME", now.to_string());
    vars.insert("ROADMAP_FINGERPRINT", roadmap_fingerprint.to_string());
    vars.insert("RUN_BRANCH", run_branch.unwrap_or("(none)").to_string());
    vars.insert("CURRENT_MILESTONE", current_milestone.to_string());
    vars.insert("CURRENT_CHANGE", "(none)".to_string());
    vars.insert("CURRENT_PHASE", "run-started".to_string());
    vars.insert("LAST_SAVED_FLOW_ACTION", "run-started".to_string());
    vars.insert("NEXT_COMMAND", next_command.to_string());
    vars.insert("LAST_CHECKPOINT", "(none)".to_string());
    vars.insert(
        "CHECKPOINT_COMMITS",
        if kind == RunKind::Roadmap && run_branch.is_some() && cfg.git.run_checkpoint_commits {
            "enabled".to_string()
        } else {
            "disabled".to_string()
        },
    );
    vars.insert("CHANGES", "(none)".to_string());
    vars.insert("MILESTONES", milestones.to_string());

    let run = render::render_template("run.md.tmpl", &vars)
        .ok_or_else(|| Error::User("missing embedded run.md.tmpl".into()))?;
    let log = render::render_template("run-log.md.tmpl", &vars)
        .ok_or_else(|| Error::User("missing embedded run-log.md.tmpl".into()))?;
    let manual = render::render_template("run-manual.md.tmpl", &vars)
        .ok_or_else(|| Error::User("missing embedded run-manual.md.tmpl".into()))?;
    let release_notes = render::render_template("run-release-notes.md.tmpl", &vars)
        .ok_or_else(|| Error::User("missing embedded run-release-notes.md.tmpl".into()))?;
    std::fs::write(run_dir.join("run.md"), run)?;
    std::fs::write(run_dir.join("log.md"), log)?;
    std::fs::write(run_dir.join("manual.md"), manual)?;
    std::fs::write(run_dir.join("release-notes.md"), release_notes)?;
    if let Some(roadmap_text) = roadmap_text {
        std::fs::write(run_roadmap_path(run_dir), roadmap_text)?;
    }
    Ok(())
}

fn format_milestone_snapshot(roadmap_text: &str) -> String {
    let milestones = parse::roadmap::parse_str(roadmap_text);
    if milestones.is_empty() {
        return "(none)".to_string();
    }
    milestones
        .into_iter()
        .map(|m| {
            let state = if m.done {
                "[x]"
            } else if m.in_progress {
                "[~]"
            } else {
                "[ ]"
            };
            format!("- {state} {} — {}", m.id, m.title)
        })
        .collect::<Vec<_>>()
        .join("\n")
}

pub(crate) fn format_milestone_snapshot_from_path(roadmap_path: &Path) -> Result<String> {
    let roadmap_text = std::fs::read_to_string(roadmap_path)?;
    Ok(format_milestone_snapshot(&roadmap_text))
}

fn suffixed_run_name(base_name: &str, index: u64) -> String {
    if index == 1 {
        base_name.to_string()
    } else {
        format!("{base_name}-{index}")
    }
}

fn create_staging_run_dir(runs_dir: &Path, run_name: &str) -> Result<PathBuf> {
    let staging_root = runs_dir.join(".tmp");
    std::fs::create_dir_all(&staging_root)?;
    for attempt in 0.. {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let candidate = staging_root.join(format!(
            "{run_name}.{}.{}.tmp",
            std::process::id(),
            nanos + attempt
        ));
        match std::fs::create_dir(&candidate) {
            Ok(()) => return Ok(candidate),
            Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(err) => return Err(err.into()),
        }
    }
    unreachable!("unbounded run staging directory suffix search must return")
}

fn cleanup_abandoned_run_skeletons(runs_dir: &Path, base_name: &str) -> Result<()> {
    if !runs_dir.exists() {
        return Ok(());
    }
    for entry in std::fs::read_dir(runs_dir)? {
        let entry = entry?;
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().to_string();
        if !is_run_name_or_numeric_suffix(&name, base_name) {
            continue;
        }
        if is_abandoned_run_skeleton(&path)? {
            std::fs::remove_dir_all(path)?;
        }
    }
    Ok(())
}

fn is_run_name_or_numeric_suffix(name: &str, base_name: &str) -> bool {
    run_name_index(name, base_name).is_some()
}

fn run_name_index(name: &str, base_name: &str) -> Option<u64> {
    if name == base_name {
        return Some(1);
    }
    let suffix = name
        .strip_prefix(base_name)
        .and_then(|rest| rest.strip_prefix('-'))?;
    if suffix.is_empty() || !suffix.chars().all(|ch| ch.is_ascii_digit()) {
        return None;
    }
    suffix.parse::<u64>().ok()
}

fn next_run_index_after_existing(runs_dir: &Path, base_name: &str) -> Result<u64> {
    if !runs_dir.exists() {
        return Ok(1);
    }
    let mut highest = 0;
    for entry in std::fs::read_dir(runs_dir)? {
        let entry = entry?;
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().to_string();
        let Some(index) = run_name_index(&name, base_name) else {
            continue;
        };
        if is_abandoned_run_skeleton(&path)? {
            continue;
        }
        highest = highest.max(index);
    }
    Ok(highest + 1)
}

fn is_abandoned_run_skeleton(run_dir: &Path) -> Result<bool> {
    for required in [
        "run.md",
        "roadmap.md",
        "log.md",
        "manual.md",
        "release-notes.md",
    ] {
        if run_dir.join(required).exists() {
            return Ok(false);
        }
    }

    for entry in std::fs::read_dir(run_dir)? {
        let entry = entry?;
        let name = entry.file_name();
        if name == "changes" && entry.file_type()?.is_dir() && is_empty_dir(&entry.path())? {
            continue;
        }
        return Ok(false);
    }
    Ok(true)
}

fn is_empty_dir(path: &Path) -> Result<bool> {
    Ok(std::fs::read_dir(path)?.next().is_none())
}

fn next_available_branch(repo: &Path, base_name: &str) -> Result<String> {
    if !git::branch_exists(repo, base_name)? {
        return Ok(base_name.to_string());
    }
    for index in 2.. {
        let candidate = format!("{base_name}-{index}");
        if !git::branch_exists(repo, &candidate)? {
            return Ok(candidate);
        }
    }
    unreachable!("unbounded run branch suffix search must return")
}

fn build_extra_context(repo: &Path, target: &RunTarget, run_dir: &Path) -> Result<String> {
    let roadmap_path = run_roadmap_path(run_dir);
    let roadmap =
        std::fs::read_to_string(&roadmap_path).unwrap_or_else(|_| "(missing)".to_string());
    let target_block = match target {
        RunTarget::Milestone {
            id,
            title,
            description,
            ..
        } => format!(
            "## Run Target\n\n**Invocation**: milestone\n**Milestone**: {id}\n**Title**: {title}\n\n## Milestone Description\n\n{}",
            if description.trim().is_empty() {
                "(none)".to_string()
            } else {
                description.trim().to_string()
            }
        ),
        RunTarget::All { milestones, .. } => {
            let list = milestones
                .iter()
                .map(|m| {
                    let state = if m.in_progress { "[~]" } else { "[ ]" };
                    format!("- {state} {} — {}", m.id, m.title)
                })
            .collect::<Vec<_>>()
            .join("\n");
            format!(
                "## Run Target\n\n**Invocation**: all open milestones\n**Open milestones at run start**: {}\n\n{}",
                milestones.len(),
                list
            )
        }
    };
    let run_display = rel(repo, run_dir);
    let state_display = rel(repo, &run_dir.join("run.md"));
    let log_display = rel(repo, &run_dir.join("log.md"));
    let manual_display = rel(repo, &run_dir.join("manual.md"));
    let release_notes_display = rel(repo, &run_dir.join("release-notes.md"));
    let changes_display = rel(repo, &run_dir.join("changes"));
    let state = read_run_state(run_dir)?;
    let run_branch = state
        .get("Run branch")
        .cloned()
        .unwrap_or_else(|| "(none)".to_string());
    let checkpoint_commits = state
        .get("Checkpoint commits")
        .cloned()
        .unwrap_or_else(|| "disabled".to_string());
    let current_phase = state.get("Current phase").map(String::as_str).unwrap_or("");
    let next_command = state
        .get("Next command")
        .map(String::as_str)
        .unwrap_or("$flow-run");
    let should_print_first_child = current_phase == "run-attached"
        || matches!(flow_shell_command(next_command).as_str(), "flow run");
    let command_hint = match target {
        RunTarget::All { .. } if should_print_first_child => target
            .first_milestone_id()
            .map(|id| {
                format!("\n\nFirst child command: `FLOW_RUN_DIR=\"{run_display}\" flow start {id}`")
            })
            .unwrap_or_default(),
        _ if !is_no_next_command(next_command) => format!(
            "\n\nCurrent next command: `FLOW_RUN_DIR=\"{run_display}\" {}`",
            flow_shell_command(next_command)
        ),
        _ => String::new(),
    };
    let handoff_state = read_run_state(run_dir).unwrap_or_default();
    let handoff = run_handoff_requirements(&handoff_state);
    Ok(format!(
        "{target_block}\n\n## Run Workspace\n\n**Run directory**: {run_display}\n**Run state**: {state_display}\n**Changes**: {changes_display}\n**Log**: {log_display}\n**Manual**: {manual_display}\n**Release notes**: {release_notes_display}\n**Run branch**: {run_branch}\n**Checkpoint commits**: {checkpoint_commits}\n**Run finalize requires**: {handoff_summary}\n\nFor child Flow commands in this run, set `FLOW_RUN_DIR={run_display}` in the command environment so milestone work stays on the run branch.{command_hint}\n\n## Roadmap Snapshot\n\n{roadmap}",
        handoff_summary = handoff.summary
    ))
}

fn flow_shell_command(next_command: &str) -> String {
    let trimmed = next_command.trim();
    if let Some(rest) = trimmed
        .strip_prefix("$flow-")
        .or_else(|| trimmed.strip_prefix("/flow-"))
    {
        format!("flow {rest}")
    } else {
        trimmed.to_string()
    }
}

#[derive(Clone, Copy)]
enum RunSummaryKind {
    Started,
    Attached,
    Continuing,
}

fn print_run_summary(repo: &Path, target: &RunTarget, run_dir: &Path, kind: RunSummaryKind) {
    println!("## Flow Run Summary");
    println!();
    match kind {
        RunSummaryKind::Started => println!("Started run `{}`.", rel(repo, run_dir)),
        RunSummaryKind::Attached => println!("Attached to run `{}`.", rel(repo, run_dir)),
        RunSummaryKind::Continuing => println!("Continuing run `{}`.", rel(repo, run_dir)),
    }
    println!("Target: {}.", target.label());
    println!("Run state: {}.", rel(repo, &run_dir.join("run.md")));
    println!("Changes: {}.", rel(repo, &run_dir.join("changes")));
    println!("Log: {}.", rel(repo, &run_dir.join("log.md")));
    println!("Manual: {}.", rel(repo, &run_dir.join("manual.md")));
    println!(
        "Release notes: {}.",
        rel(repo, &run_dir.join("release-notes.md"))
    );
}

fn maybe_print_finalize_hint(run_dir: &Path) {
    if !crate::output::should_emit_finalize_footer(run_dir, "run") {
        return;
    }
    print_finalize_hint(run_dir);
}

fn print_finalize_hint(run_dir: &Path) {
    let state = read_run_state(run_dir).unwrap_or_default();
    let handoff = run_handoff_requirements(&state);
    println!();
    println!("---");
    println!();
    println!("## Finalization Instructions");
    println!();
    println!("When {} are complete, run:", handoff.summary);
    println!();
    println!("```sh");
    println!("flow run --finalize");
    println!("```");
    println!();
    println!("Run directory: `{}`", run_dir.display());
}

struct RunHandoffRequirements {
    summary: String,
}

fn run_handoff_requirements(state: &BTreeMap<String, String>) -> RunHandoffRequirements {
    let full = requires_full_run_handoff(state);
    RunHandoffRequirements {
        summary: if full {
            "`log.md`, `manual.md`, and `release-notes.md`".to_string()
        } else {
            "`release-notes.md`".to_string()
        },
    }
}

fn requires_full_run_handoff(state: &BTreeMap<String, String>) -> bool {
    if state.get("Run type").map(String::as_str) != Some("roadmap") {
        return false;
    }
    state
        .get("Run scope")
        .map(String::as_str)
        .map_or(true, |scope| scope == "all")
}

fn finalize(repo: &Path, run_dir: &Path) -> Result<()> {
    let run_path = run_dir.join("run.md");
    let log_path = run_dir.join("log.md");
    let manual_path = run_dir.join("manual.md");
    let release_notes_path = run_dir.join("release-notes.md");
    let log = std::fs::read_to_string(&log_path).map_err(|_| Error::FileNotFound {
        kind: "run log".into(),
        path: log_path.clone(),
    })?;
    let manual = std::fs::read_to_string(&manual_path).map_err(|_| Error::FileNotFound {
        kind: "run manual".into(),
        path: manual_path.clone(),
    })?;
    let release_notes =
        std::fs::read_to_string(&release_notes_path).map_err(|_| Error::FileNotFound {
            kind: "run release notes".into(),
            path: release_notes_path.clone(),
        })?;
    let state = read_run_state(run_dir)?;
    let full_handoff = requires_full_run_handoff(&state);
    if full_handoff {
        if !log.contains("## Event Log") || !log.contains("## Operations") {
            return Err(Error::ArtifactError {
                file: log_path.display().to_string(),
                message: "run log must contain Event Log and Operations sections".into(),
            });
        }
        if contains_run_template_placeholder(&manual) {
            return Err(Error::ArtifactError {
                file: manual_path.display().to_string(),
                message: "run manual still contains template placeholders".into(),
            });
        }
    }
    if contains_run_template_placeholder(&release_notes) {
        return Err(Error::ArtifactError {
            file: release_notes_path.display().to_string(),
            message: "run release notes still contain template placeholders".into(),
        });
    }
    let is_roadmap = state.get("Run type").map(String::as_str) == Some("roadmap");
    if is_roadmap {
        ensure_roadmap_fingerprint_matches(run_dir, &state)?;
        ensure_roadmap_complete(run_dir)?;
    }
    let closing_commit = checkpoint_commits_enabled(&state);
    let run_branch = state
        .get("Run branch")
        .cloned()
        .unwrap_or_else(|| "(none)".to_string());
    if closing_commit {
        ensure_on_run_branch(repo, &run_branch)?;
    }
    update_run_state(
        run_dir,
        &[
            ("Status", "complete"),
            ("Current phase", "completed"),
            ("Last saved Flow action", "run-complete"),
            ("Next command", "none"),
        ],
    )?;
    let finalized_at = render::now_iso();
    replace_first(&run_path, "**Status**: running", "**Status**: complete")?;
    replace_first(&log_path, "**Status**: running", "**Status**: complete")?;
    replace_first(&manual_path, "**Status**: draft", "**Status**: complete")?;
    replace_first(
        &release_notes_path,
        "**Status**: draft",
        "**Status**: complete",
    )?;
    append_line(
        &log_path,
        &format!(
            "- {finalized_at} — run-finalized — Log, owner's manual, and release notes completed.\n"
        ),
    )?;
    let closing_sha = if closing_commit {
        append_line(
            &log_path,
            &format!(
                "- {finalized_at} — run-finalize-commit — Closing commit for the finalized run state follows. The command output prints the exact SHA after Git creates it.\n"
            ),
        )?;
        let run_name = state
            .get("Run name")
            .cloned()
            .unwrap_or_else(|| rel(repo, run_dir));
        let run_pathspec = PathBuf::from(rel(repo, run_dir));
        git::stage_paths(repo, &[&run_pathspec])?;
        Some(git::commit_paths(
            repo,
            &format!("flow run finalize: {run_name}"),
            &[&run_pathspec],
        )?)
    } else {
        None
    };
    println!("Run finalized.");
    match &closing_sha {
        Some(sha) => println!("Closing commit: {sha}"),
        None => println!(
            "Run artifacts in {} are intentionally left uncommitted (checkpoint commits are disabled for this run); review and commit them yourself.",
            rel(repo, run_dir)
        ),
    }
    println!();
    println!("Verify this run:");
    println!(
        "   Release notes: {} — what changed and the user impact",
        rel(repo, &release_notes_path)
    );
    println!(
        "   Manual: {} — how to operate and verify the result",
        rel(repo, &manual_path)
    );
    println!("   Log:    {} — full event trail", rel(repo, &log_path));
    println!("   Run:    {} — final run state", rel(repo, &run_path));
    if is_roadmap {
        print_roadmap_archive_paths(repo, run_dir);
    }
    if closing_sha.is_some() {
        println!("   History: `git log --oneline` — one checkpoint commit per milestone, plus the closing commit");
    }
    println!();
    if run_branch != "(none)" && !run_branch.is_empty() {
        println!(
            "Next: review the files above, then merge run branch '{run_branch}' when satisfied. Flow never pushes or merges."
        );
    } else {
        println!("Next: review the files above.");
    }
    Ok(())
}

fn resume(repo: &Path, run_dir: &Path) -> Result<()> {
    let state = read_run_state(run_dir)?;
    let run_branch = state
        .get("Run branch")
        .cloned()
        .unwrap_or_else(|| "(none)".to_string());
    let current_branch = git::current_branch(repo).unwrap_or_else(|_| "unknown".to_string());
    println!("# Flow Run Resume");
    println!();
    println!("Run directory: `{}`", rel(repo, run_dir));
    println!("Run branch: `{run_branch}`");
    println!(
        "Current milestone: `{}`",
        state
            .get("Current milestone")
            .map(String::as_str)
            .unwrap_or("(unknown)")
    );
    println!(
        "Current phase: `{}`",
        state
            .get("Current phase")
            .map(String::as_str)
            .unwrap_or("(unknown)")
    );
    println!(
        "Current change: `{}`",
        state
            .get("Current change")
            .map(String::as_str)
            .unwrap_or("(unknown)")
    );
    println!(
        "Last checkpoint: `{}`",
        state
            .get("Last checkpoint")
            .map(String::as_str)
            .unwrap_or("(none)")
    );
    if run_branch != "(none)" && current_branch != run_branch {
        println!();
        println!("Switch to the run branch before resuming:");
        println!();
        println!("```sh");
        println!("git switch {run_branch}");
        println!("```");
    }
    let next_command = state
        .get("Next command")
        .map(String::as_str)
        .unwrap_or("$flow-run");
    if is_no_next_command(next_command) {
        println!();
        println!("No next command. The run is complete.");
    } else {
        println!();
        println!("Next command:");
        println!();
        println!("```sh");
        println!("FLOW_RUN_DIR=\"{}\" {next_command}", rel(repo, run_dir));
        println!("```");
    }
    Ok(())
}

fn rescan(repo: &Path, run_dir: &Path) -> Result<()> {
    let _state = read_run_state(run_dir)?;
    let roadmap_path = run_roadmap_path(run_dir);
    let roadmap = std::fs::read_to_string(&roadmap_path).map_err(|_| Error::FileNotFound {
        kind: "roadmap.md".into(),
        path: roadmap_path.clone(),
    })?;
    let fingerprint = flow_core::roadmap::fingerprint(&roadmap);
    let milestones = format_milestone_snapshot(&roadmap);
    update_run_state(
        run_dir,
        &[
            ("Roadmap fingerprint", &fingerprint),
            ("Last saved Flow action", "roadmap-rescan"),
        ],
    )?;
    refresh_milestone_snapshot(run_dir, &milestones)?;
    append_line(
        &run_dir.join("log.md"),
        &format!(
            "- {} — roadmap-rescan — Refreshed Roadmap fingerprint and Milestones snapshot from {}.\n",
            render::now_iso(),
            rel(repo, &roadmap_path)
        ),
    )?;
    println!("Run roadmap snapshot refreshed.");
    println!("   Run: {}", run_dir.display());
    println!("   Roadmap fingerprint: {fingerprint}");
    Ok(())
}

fn contains_run_template_placeholder(text: &str) -> bool {
    text.contains("To be completed before the roadmap delivery run is finalized.")
        || text.contains("To be filled during the run.")
        || text.contains("TODO")
}

fn checkpoint(repo: &Path, run_dir: &Path, milestone: &str) -> Result<()> {
    let state = read_run_state(run_dir)?;
    if !checkpoint_commits_enabled(&state) {
        println!("Run checkpoint commits are disabled for this run.");
        return Ok(());
    }
    let run_branch = state.get("Run branch").map(String::as_str).unwrap_or("");
    ensure_on_run_branch(repo, run_branch)?;
    if !milestone_re().is_match(milestone) {
        return Err(Error::InvalidId {
            kind: "M".into(),
            input: milestone.to_string(),
            reason: "expected M-N".into(),
        });
    }
    let roadmap_path = run_roadmap_path(run_dir);
    let milestone_record = flow_core::roadmap::require_milestone_at_path(&roadmap_path, milestone)?;
    let message = format!(
        "flow run checkpoint: {} {}",
        milestone_record.id, milestone_record.title
    );
    let final_checkpoint = !parse::roadmap::parse_file(&roadmap_path)?
        .into_iter()
        .any(|milestone| !milestone.done);
    let next_command = if final_checkpoint {
        "flow run --finalize".to_string()
    } else {
        "$flow-run".to_string()
    };
    update_run_state(
        run_dir,
        &[
            ("Current milestone", &milestone_record.id),
            ("Current phase", "checkpoint-complete"),
            ("Last saved Flow action", "close-finalized"),
            ("Next command", next_command.as_str()),
        ],
    )?;
    if final_checkpoint {
        ensure_roadmap_fingerprint_matches(run_dir, &state)?;
    }
    append_line(
        &run_dir.join("log.md"),
        &format!(
            "- {} — checkpoint — Preparing local checkpoint commit for {}. The command output prints the exact SHA after Git creates it.\n",
            render::now_iso(),
            milestone_record.id
        ),
    )?;
    git::stage_all(repo)?;
    match git::commit(repo, &message) {
        Ok(sha) => {
            let sha = CheckpointSha::new(sha)?;
            update_run_state(run_dir, &[("Last checkpoint", sha.as_str())])?;
            append_line(
                &run_dir.join("log.md"),
                &format!(
                    "- {} — checkpoint-complete — Local checkpoint commit for {} created as {}.\n",
                    render::now_iso(),
                    milestone_record.id,
                    sha.as_str()
                ),
            )?;
            println!("Checkpoint committed: {}", sha.as_str());
            if final_checkpoint {
                print_roadmap_archive_paths(repo, run_dir);
            }
            Ok(())
        }
        Err(err) => {
            let _ = update_run_state(
                run_dir,
                &[
                    ("Current phase", "checkpoint-failed"),
                    ("Next command", "$flow-run --resume"),
                ],
            );
            let _ = append_line(
                &run_dir.join("log.md"),
                &format!(
                    "- {} — checkpoint-failed — Local checkpoint commit for {} failed: {}.\n",
                    render::now_iso(),
                    milestone_record.id,
                    err
                ),
            );
            Err(err)
        }
    }
}

fn checkpoint_commits_enabled(state: &BTreeMap<String, String>) -> bool {
    state
        .get("Checkpoint commits")
        .is_some_and(|value| value == "enabled")
}

fn ensure_roadmap_complete(run_dir: &Path) -> Result<()> {
    let roadmap_path = run_roadmap_path(run_dir);
    let roadmap = std::fs::read_to_string(&roadmap_path).map_err(|_| Error::FileNotFound {
        kind: "roadmap.md".into(),
        path: roadmap_path.clone(),
    })?;
    let milestones = parse::roadmap::parse_str(&roadmap);
    if milestones.iter().any(|milestone| !milestone.done) {
        return Err(Error::User(format!(
            "Cannot finalize roadmap run while {} still has open milestones.",
            roadmap_path.display()
        )));
    }
    Ok(())
}

fn ensure_roadmap_fingerprint_matches(
    run_dir: &Path,
    state: &BTreeMap<String, String>,
) -> Result<()> {
    let expected = roadmap_fingerprint_at_path(&run_roadmap_path(run_dir))?;
    let actual = state
        .get("Roadmap fingerprint")
        .map(String::as_str)
        .unwrap_or("(missing)");
    if actual != expected {
        return Err(Error::User(format!(
            "Cannot finalize roadmap run because the run-local roadmap fingerprint is {expected}, but the run records {actual}. Run `flow run --rescan` if the roadmap edit belongs to this run."
        )));
    }
    Ok(())
}

fn roadmap_milestones_complete_at_path(roadmap_path: &Path) -> Result<bool> {
    let milestones = parse::roadmap::parse_file(roadmap_path)
        .map_err(|e| Error::User(format!("Cannot read roadmap for run finalization: {e}")))?;
    Ok(!milestones.is_empty() && milestones.iter().all(|m| m.done))
}

pub(crate) fn refresh_milestone_snapshot(run_dir: &Path, milestones: &str) -> Result<()> {
    let run_path = run_dir.join("run.md");
    let text = std::fs::read_to_string(&run_path)?;
    let lines = text.lines().collect::<Vec<_>>();
    let Some(start) = lines.iter().position(|line| line.trim() == "## Milestones") else {
        return Err(Error::ArtifactError {
            file: run_path.display().to_string(),
            message: "run.md is missing ## Milestones".into(),
        });
    };
    let end = lines
        .iter()
        .enumerate()
        .skip(start + 1)
        .find_map(|(index, line)| line.starts_with("## ").then_some(index))
        .unwrap_or(lines.len());

    let mut next = String::new();
    for line in &lines[..=start] {
        next.push_str(line);
        next.push('\n');
    }
    next.push('\n');
    next.push_str(milestones);
    next.push_str("\n\n");
    for line in &lines[end..] {
        next.push_str(line);
        next.push('\n');
    }
    std::fs::write(run_path, next)?;
    Ok(())
}

fn append_change_index(run_dir: &Path, change: &str, milestone: Option<&str>) -> Result<()> {
    let run_path = run_dir.join("run.md");
    let text = std::fs::read_to_string(&run_path)?;
    let lines = text.lines().collect::<Vec<_>>();
    let Some(start) = lines.iter().position(|line| line.trim() == "## Changes") else {
        return Err(Error::ArtifactError {
            file: run_path.display().to_string(),
            message: "run.md is missing ## Changes".into(),
        });
    };
    let end = lines
        .iter()
        .enumerate()
        .skip(start + 1)
        .find_map(|(index, line)| line.starts_with("## ").then_some(index))
        .unwrap_or(lines.len());
    let milestone = milestone.unwrap_or("(none)");
    let entry = format!("- [ ] {change} — milestone: {milestone}");

    let mut existing = lines[start + 1..end]
        .iter()
        .map(|line| line.trim())
        .filter(|line| !line.is_empty() && *line != "(none)")
        .map(ToOwned::to_owned)
        .collect::<Vec<_>>();
    if existing.iter().any(|line| line.contains(change)) {
        return Ok(());
    }
    existing.push(entry);

    let mut next = String::new();
    for line in &lines[..=start] {
        next.push_str(line);
        next.push('\n');
    }
    next.push('\n');
    next.push_str(&existing.join("\n"));
    next.push_str("\n\n");
    for line in &lines[end..] {
        next.push_str(line);
        next.push('\n');
    }
    std::fs::write(run_path, next)?;
    Ok(())
}

fn mark_change_index_closed(run_dir: &Path, change: &str) -> Result<()> {
    let run_path = run_dir.join("run.md");
    let text = std::fs::read_to_string(&run_path)?;
    let mut next = String::with_capacity(text.len());
    for line in text.lines() {
        if line.contains(change) && line.trim_start().starts_with("- [ ]") {
            next.push_str(&line.replacen("- [ ]", "- [x]", 1));
        } else {
            next.push_str(line);
        }
        next.push('\n');
    }
    std::fs::write(run_path, next)?;
    Ok(())
}

fn print_roadmap_archive_paths(repo: &Path, run_dir: &Path) {
    println!("   Roadmap: {}", rel(repo, &run_roadmap_path(run_dir)));
}

struct CheckpointSha(String);

impl CheckpointSha {
    fn new(value: String) -> Result<Self> {
        if value.len() == 40 && value.chars().all(|ch| ch.is_ascii_hexdigit()) {
            Ok(Self(value))
        } else {
            Err(Error::User(format!(
                "git commit returned invalid checkpoint SHA {value:?}; expected 40 hex characters"
            )))
        }
    }

    fn as_str(&self) -> &str {
        &self.0
    }
}

fn ensure_on_run_branch(repo: &Path, run_branch: &str) -> Result<()> {
    if run_branch.is_empty() || run_branch == "(none)" {
        return Ok(());
    }
    let current = git::current_branch(repo).unwrap_or_else(|_| "unknown".into());
    if current != run_branch {
        return Err(Error::User(format!(
            "This run is on branch '{run_branch}', but the current branch is '{current}'. Switch to the run branch before continuing."
        )));
    }
    Ok(())
}

pub(crate) fn read_run_state(run_dir: &Path) -> Result<BTreeMap<String, String>> {
    let run_path = run_dir.join("run.md");
    let text = std::fs::read_to_string(&run_path).map_err(|_| Error::FileNotFound {
        kind: "run state".into(),
        path: run_path.clone(),
    })?;
    let mut state = BTreeMap::new();
    for line in text.lines() {
        if line.starts_with("## ") {
            break;
        }
        if let Some((name, value)) = parse_state_line(line) {
            state.insert(name.to_string(), value.to_string());
        }
    }
    if state.is_empty() {
        return Err(Error::ArtifactError {
            file: run_path.display().to_string(),
            message: "run.md is missing run state fields".into(),
        });
    }
    let required = [
        "Run name",
        "Run type",
        "Status",
        "Run branch",
        "Roadmap fingerprint",
        "Checkpoint commits",
        "Current milestone",
        "Current change",
        "Current phase",
        "Last saved Flow action",
        "Next command",
        "Last checkpoint",
    ];
    if let Some(missing) = required.iter().find(|name| !state.contains_key(**name)) {
        return Err(Error::ArtifactError {
            file: run_path.display().to_string(),
            message: format!("run.md is missing field {missing:?}"),
        });
    }
    Ok(state)
}

pub(crate) fn update_run_state(run_dir: &Path, updates: &[(&str, &str)]) -> Result<()> {
    let run_path = run_dir.join("run.md");
    let mut text = std::fs::read_to_string(&run_path)?;
    for (name, value) in updates {
        let needle = format!("**{name}**:");
        let mut replaced = false;
        let mut next = String::with_capacity(text.len() + value.len());
        for line in text.lines() {
            if line.trim_start().starts_with(&needle) {
                next.push_str(&format!("**{name}**: {value}\n"));
                replaced = true;
            } else {
                next.push_str(line);
                next.push('\n');
            }
        }
        if !replaced {
            return Err(Error::ArtifactError {
                file: run_path.display().to_string(),
                message: format!("run.md is missing field {name:?}"),
            });
        }
        text = next;
    }
    std::fs::write(run_path, text)?;
    Ok(())
}

fn parse_state_line(line: &str) -> Option<(&str, &str)> {
    let trimmed = line.trim();
    let rest = trimmed.strip_prefix("**")?;
    let (name, rest) = rest.split_once("**:")?;
    Some((name.trim(), rest.trim()))
}

fn is_no_next_command(command: &str) -> bool {
    let normalized = command.trim().to_ascii_lowercase();
    normalized.is_empty()
        || matches!(
            normalized.as_str(),
            "none" | "(none)" | "no next command" | "complete"
        )
}

fn replace_first(path: &Path, from: &str, to: &str) -> Result<()> {
    let text = std::fs::read_to_string(path)?;
    if !text.contains(from) {
        return Ok(());
    }
    std::fs::write(path, text.replacen(from, to, 1))?;
    Ok(())
}

fn append_line(path: &Path, line: &str) -> Result<()> {
    let mut text = std::fs::read_to_string(path)?;
    if !text.ends_with('\n') {
        text.push('\n');
    }
    text.push_str(line);
    std::fs::write(path, text)?;
    Ok(())
}

fn normalize_run_dir_arg(repo: &Path, run_dir: &Path) -> PathBuf {
    if run_dir.is_absolute() {
        run_dir.to_path_buf()
    } else {
        repo.join(run_dir)
    }
}

fn rel(repo: &Path, path: &Path) -> String {
    let relative = path
        .strip_prefix(repo)
        .ok()
        .map(Path::to_path_buf)
        .or_else(|| {
            let repo = repo.canonicalize().ok()?;
            let path = path.canonicalize().ok()?;
            path.strip_prefix(repo).ok().map(Path::to_path_buf)
        });
    relative
        .as_deref()
        .unwrap_or(path)
        .display()
        .to_string()
        .replace('\\', "/")
}

#[cfg(test)]
mod tests {
    use super::CheckpointSha;
    use super::{
        cleanup_abandoned_run_skeletons, derive_run_scope, next_run_index_after_existing,
        requires_full_run_handoff, RunTarget,
    };
    use std::collections::BTreeMap;

    #[test]
    fn t002_checkpoint_sha_accepts_full_hex_sha() {
        let sha = "0123456789abcdef0123456789ABCDEF01234567".to_string();
        assert_eq!(CheckpointSha::new(sha).unwrap().as_str().len(), 40);
    }

    #[test]
    fn t002_checkpoint_sha_rejects_placeholders_and_short_sha() {
        assert!(CheckpointSha::new("HEAD".to_string()).is_err());
        assert!(CheckpointSha::new("75b5f41".to_string()).is_err());
    }

    fn milestone_target(id: &str) -> RunTarget {
        RunTarget::Milestone {
            id: id.to_string(),
            title: "t".to_string(),
            description: "d".to_string(),
        }
    }

    fn all_target() -> RunTarget {
        RunTarget::All {
            milestones: Vec::new(),
        }
    }

    #[test]
    fn run_scope_first_attach_records_user_intent() {
        assert_eq!(
            derive_run_scope("(none)", "(none)", "M-1", &milestone_target("M-1")),
            "single"
        );
        assert_eq!(
            derive_run_scope("(none)", "(none)", "M-1", &all_target()),
            "all"
        );
    }

    #[test]
    fn run_scope_repeat_attach_to_same_milestone_stays_single() {
        assert_eq!(
            derive_run_scope("single", "M-1", "M-1", &milestone_target("M-1")),
            "single"
        );
    }

    #[test]
    fn run_scope_attach_to_different_milestone_escalates_to_all() {
        assert_eq!(
            derive_run_scope("single", "M-1", "M-2", &milestone_target("M-2")),
            "all"
        );
    }

    #[test]
    fn run_scope_all_is_sticky_across_subsequent_attaches() {
        assert_eq!(
            derive_run_scope("all", "M-1", "M-2", &milestone_target("M-2")),
            "all"
        );
        assert_eq!(derive_run_scope("all", "M-1", "M-1", &all_target()), "all");
    }

    #[test]
    fn full_run_handoff_required_for_roadmap_all() {
        let mut state = BTreeMap::new();
        state.insert("Run type".to_string(), "roadmap".to_string());
        state.insert("Run scope".to_string(), "all".to_string());
        assert!(requires_full_run_handoff(&state));
    }

    #[test]
    fn full_run_handoff_not_required_for_single_milestone_roadmap() {
        let mut state = BTreeMap::new();
        state.insert("Run type".to_string(), "roadmap".to_string());
        state.insert("Run scope".to_string(), "single".to_string());
        assert!(!requires_full_run_handoff(&state));
    }

    #[test]
    fn full_run_handoff_not_required_for_one_off_runs() {
        let mut state = BTreeMap::new();
        state.insert("Run type".to_string(), "one-off".to_string());
        // Even if scope happens to be "all" on a one-off run, type wins.
        state.insert("Run scope".to_string(), "all".to_string());
        assert!(!requires_full_run_handoff(&state));
    }

    #[test]
    fn full_run_handoff_defaults_to_full_when_scope_missing() {
        let mut state = BTreeMap::new();
        state.insert("Run type".to_string(), "roadmap".to_string());
        // Missing `Run scope` (e.g. a run created before this field existed)
        // should fail safe to the full handoff so we do not silently drop
        // historical validation.
        assert!(requires_full_run_handoff(&state));
    }

    #[test]
    fn abandoned_skeleton_cleanup_preserves_highest_valid_suffix() {
        let td = tempfile::TempDir::new().unwrap();
        let runs = td.path();
        let base = "20260526-roadmap-demo";
        std::fs::create_dir_all(runs.join(base).join("changes")).unwrap();
        std::fs::create_dir_all(runs.join(format!("{base}-2")).join("changes")).unwrap();
        let valid = runs.join(format!("{base}-3"));
        std::fs::create_dir_all(valid.join("changes")).unwrap();
        std::fs::write(valid.join("run.md"), "# Run: Demo\n").unwrap();

        assert_eq!(next_run_index_after_existing(runs, base).unwrap(), 4);
        cleanup_abandoned_run_skeletons(runs, base).unwrap();

        assert!(!runs.join(base).exists());
        assert!(!runs.join(format!("{base}-2")).exists());
        assert!(valid.join("run.md").is_file());
        assert_eq!(next_run_index_after_existing(runs, base).unwrap(), 4);
    }
}
