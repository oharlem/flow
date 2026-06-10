//! `flow build` — implement all remaining tasks.

use crate::args::BuildArgs;
use flow_core::{envelope, parse, paths, preflight, Result};
use std::path::Path;

/// Run `flow build`.
pub fn run(args: BuildArgs) -> Result<()> {
    let repo = paths::repo_root(None)?;
    match super::finalize_mode(args.finalize) {
        super::FinalizeMode::Infer => {
            let dir = super::amend::resolve_feature_dir(&repo)?;
            return finalize(&dir, &args.completed);
        }
        super::FinalizeMode::Skip => {}
    }
    let feature_dir = super::amend::resolve_feature_dir(&repo)?;
    prepare(&repo, &feature_dir)
}

fn prepare(repo: &Path, feature_dir: &Path) -> Result<()> {
    super::plan_gate::require_plan_complete(feature_dir)?;
    let tasks_file = feature_dir.join("tasks.md");
    let tasks = parse::tasks::parse_file(&tasks_file)?;
    let awaiting: Vec<_> = tasks
        .iter()
        .filter(|t| t.state.is_awaiting_acceptance())
        .collect();
    let queued: Vec<_> = if awaiting.is_empty() {
        parse::tasks::runnable_open_task_queue(&tasks, 20)
    } else {
        Vec::new()
    };
    if awaiting.is_empty() && queued.is_empty() && tasks.iter().any(|t| t.state.is_open()) {
        return Err(flow_core::Error::User(
            "No runnable open tasks. Open tasks are blocked by Depends-On entries that are not accepted ([x]) in tasks.md."
                .into(),
        ));
    }
    let extra = if !awaiting.is_empty() {
        Some(format!(
            "## Build Task Queue\n\nThe following tasks are implemented and awaiting user acceptance:\n\n{}\n\nSave Flow state for accepted tasks before continuing the build.",
            format_task_list(&awaiting)
        ))
    } else if queued.is_empty() {
        Some(
            "## Build Task Queue\n\nAll tasks are checked. Flow will route to verification."
                .to_string(),
        )
    } else {
        Some(format!(
            "## Build Task Queue\n\n{}",
            format_task_list(&queued)
        ))
    };
    if awaiting.is_empty() && !queued.is_empty() {
        let cfg = flow_core::config::Config::load_for_repo(repo)?;
        let report = preflight::run_for_tasks(repo, &cfg, &queued)?;
        if report.is_blocked() {
            print!("{}", preflight::render_blocked(&report));
            crate::output::print_next("/flow-build", "after required resources are available.");
            return Ok(());
        }
    }
    let finalizable: Vec<&parse::tasks::Task> = if awaiting.is_empty() {
        queued.clone()
    } else {
        awaiting.clone()
    };

    // M-22: stale-state recovery. If a build-pending state file is left over
    // from an interrupted prior run, log a clear warning and overwrite it
    // with the new queue. The state file represents "queued for the current
    // round," so the next prepare is authoritative.
    if let Some(stale) = super::build_pending::read(feature_dir)? {
        if !stale.is_empty() {
            flow_core::logging::warn(format!(
                "Stale build-pending state found at {}/.flow/build-pending.yaml ({}). Overwriting with the new queue for this round.",
                feature_dir.display(),
                stale.join(", ")
            ));
        }
    }

    // M-22: persist the queue to per-change state so the printed footer can
    // collapse to the single stable string `flow build --finalize`.
    let pending_ids: Vec<String> = finalizable.iter().map(|t| t.id.clone()).collect();
    super::build_pending::write(feature_dir, &pending_ids)?;

    let out = envelope::compose(repo, "build", feature_dir, extra.as_deref())?;
    print!("{out}");
    let finalize_command = "flow build --finalize".to_string();
    crate::output::maybe_print_finalize_command(&finalize_command, feature_dir, "build");
    crate::output::print_next("/flow-test", "after all tasks are checked.");
    Ok(())
}

fn format_task_list(tasks: &[&parse::tasks::Task]) -> String {
    tasks
        .iter()
        .map(|t| {
            let requires = if t.requires.is_empty() {
                String::new()
            } else {
                format!(" (requires: {})", t.requires.join(", "))
            };
            format!("- {} — {}{}", t.id, t.summary, requires)
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn finalize(feature_dir: &Path, completed: &[String]) -> Result<()> {
    super::plan_gate::require_plan_complete(feature_dir)?;

    // M-22: when no `--completed` overrides are passed, fall back to the
    // per-change state file written during the most recent `flow build`
    // prepare. This collapses the printed footer to one stable string while
    // preserving the scripted-override path.
    let resolved_ids: Vec<String> = if completed.is_empty() {
        super::build_pending::read(feature_dir)?.unwrap_or_default()
    } else {
        completed.to_vec()
    };
    let completed_ids = super::task_state::mark_done(feature_dir, &resolved_ids)?;

    // Clear the state file unconditionally on a successful finalize, even
    // when scripted overrides were used; we don't want stale state to leak
    // into the next round.
    super::build_pending::clear(feature_dir)?;

    let tasks_file = feature_dir.join("tasks.md");
    let tasks = parse::tasks::parse_file(&tasks_file)?;
    let has_unfinished_tasks = tasks.iter().any(|t| !t.done);
    let completed_summary = completed_ids.join(", ");
    let summary = if completed_summary.is_empty() {
        if has_unfinished_tasks {
            "build progress saved".to_string()
        } else {
            "all tasks complete".to_string()
        }
    } else if has_unfinished_tasks {
        format!("{completed_summary} build progress saved")
    } else {
        format!("{completed_summary} build progress saved; all tasks implemented")
    };
    parse::status::stamp(
        feature_dir,
        Some(parse::status::State::Building),
        "build-progress",
        &summary,
    )?;
    flow_core::logging::info("Build state saved.");
    if has_unfinished_tasks {
        crate::cmd::run::update_run_state_for_feature_phase(
            feature_dir,
            "build-progress",
            "build-progress",
            "$flow-build-task",
        )?;
        crate::output::print_next("/flow-build-task", "continue with the next task.");
        Ok(())
    } else {
        super::test::run_and_finalize(feature_dir)
    }
}
