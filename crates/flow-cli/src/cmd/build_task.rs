//! `flow build-task` — implement one task.

use crate::args::BuildTaskArgs;
use flow_core::{envelope, parse, paths, preflight, Error, Result};
use std::path::Path;

/// Run `flow build-task`.
pub fn run(args: BuildTaskArgs) -> Result<()> {
    let repo = paths::repo_root(None)?;
    match super::finalize_mode(args.finalize) {
        super::FinalizeMode::Infer => {
            let dir = super::amend::resolve_feature_dir(&repo)?;
            return finalize(&dir, args.task.as_deref());
        }
        super::FinalizeMode::Skip => {}
    }
    let feature_dir = super::amend::resolve_feature_dir(&repo)?;
    prepare(&repo, &feature_dir, args.task.as_deref())
}

fn prepare(repo: &Path, feature_dir: &Path, task_selector: Option<&str>) -> Result<()> {
    super::plan_gate::require_plan_complete(feature_dir)?;
    let tasks_file = feature_dir.join("tasks.md");
    let tasks = parse::tasks::parse_file(&tasks_file)?;
    let selected = task_selector
        .and_then(|sel| find_task(&tasks, sel))
        .cloned();
    let awaiting = selected
        .as_ref()
        .filter(|t| t.state.is_awaiting_acceptance())
        .or_else(|| {
            if task_selector.is_none() {
                tasks.iter().find(|t| t.state.is_awaiting_acceptance())
            } else {
                None
            }
        })
        .cloned();
    let task = if awaiting.is_none() {
        pick_task(&tasks, task_selector)?
    } else {
        None
    };

    let extra = if let Some(t) = awaiting.as_ref() {
        Some(format!(
            "## Active Task\n\n**{}** — {}\n\nThis task is marked `[~]`: implemented locally and waiting for user acceptance. Save Flow state for this task before continuing the build.",
            t.id, t.summary
        ))
    } else {
        task.as_ref().map(|t| {
            let mut block = format!("## Active Task\n\n**{}** — {}", t.id, t.summary);
            if !t.covers.is_empty() {
                block.push_str(&format!("\n\nCovers: {}", t.covers.join(", ")));
            }
            if !t.verifies.is_empty() {
                block.push_str(&format!("\nVerifies: {}", t.verifies.join(", ")));
            }
            if !t.requires.is_empty() {
                block.push_str(&format!("\nRequires: {}", t.requires.join(", ")));
            }
            block.push_str(
                "\n\nImplement test-first: add an automated test that references the task ID.",
            );
            block
        })
    };

    if awaiting.is_none() {
        if let Some(t) = task {
            let cfg = flow_core::config::Config::load_for_repo(repo)?;
            let task_refs = [t];
            let report = preflight::run_for_tasks(repo, &cfg, &task_refs)?;
            if report.is_blocked() {
                print!("{}", preflight::render_blocked(&report));
                crate::output::print_next(
                    "/flow-build-task",
                    "after required resources are available.",
                );
                return Ok(());
            }
        }
    }

    let finalizable = awaiting.as_ref().or(task).or(selected.as_ref());
    let finalize_command = if let Some(task) = finalizable {
        format!("flow build-task {} --finalize", task.id)
    } else {
        "flow build-task --finalize".to_string()
    };
    let out = envelope::compose_with_save_command(
        repo,
        "build-task",
        feature_dir,
        extra.as_deref(),
        &finalize_command,
    )?;
    print!("{out}");
    crate::output::maybe_print_finalize_command(&finalize_command, feature_dir, "build-task");
    crate::output::print_next("/flow-build-task", "after the task state is saved.");
    Ok(())
}

fn finalize(feature_dir: &Path, selector: Option<&str>) -> Result<()> {
    super::plan_gate::require_plan_complete(feature_dir)?;
    let completed_ids = completion_ids(feature_dir, selector)?;
    let completed_ids = super::task_state::mark_done(feature_dir, &completed_ids)?;

    // Re-read tasks; stamp task-complete on each finalize. `build-complete` is
    // exclusively owned by `/flow-test --finalize`; the verification phase is
    // what closes the build state.
    let tasks = parse::tasks::parse_file(&feature_dir.join("tasks.md"))?;
    let has_unfinished_tasks = tasks.iter().any(|t| !t.done);
    let completed = completed_ids.join(", ");
    let summary = if has_unfinished_tasks {
        format!("{completed} task state saved")
    } else {
        format!("{completed} task state saved; all tasks implemented")
    };
    parse::status::stamp(
        feature_dir,
        Some(parse::status::State::Building),
        "task-complete",
        &summary,
    )?;
    flow_core::logging::info("Task state saved.");
    if has_unfinished_tasks {
        crate::output::print_next("/flow-build-task", "continue implementing.");
        Ok(())
    } else {
        super::test::run_and_finalize(feature_dir)
    }
}

fn pick_task<'a>(
    tasks: &'a [parse::tasks::Task],
    selector: Option<&str>,
) -> Result<Option<&'a parse::tasks::Task>> {
    if let Some(sel) = selector {
        let wanted = super::task_state::normalize_id(sel);
        let Some(t) = find_task(tasks, sel).filter(|t| t.state.is_open()) else {
            return Err(Error::ArtifactError {
                file: "tasks.md".into(),
                message: format!("task '{wanted}' is not open or was not found"),
            });
        };
        let accepted = parse::tasks::acceptance_map(tasks);
        if !parse::tasks::dependencies_satisfied(t, &accepted) {
            return Err(Error::User(format!(
                "Task {} cannot start until Depends-On tasks are accepted ([x]) in tasks.md.",
                t.id
            )));
        }
        return Ok(Some(t));
    }
    let task = parse::tasks::first_runnable_open_task(tasks);
    if task.is_none() && tasks.iter().any(|t| t.state.is_open()) {
        return Err(Error::User(
            "No runnable open tasks. Open tasks are blocked by Depends-On entries that are not accepted ([x]) in tasks.md."
                .into(),
        ));
    }
    Ok(task)
}

fn find_task<'a>(
    tasks: &'a [parse::tasks::Task],
    selector: &str,
) -> Option<&'a parse::tasks::Task> {
    let wanted = selector.to_uppercase();
    tasks.iter().find(|t| t.id.eq_ignore_ascii_case(&wanted))
}

fn completion_ids(feature_dir: &Path, selector: Option<&str>) -> Result<Vec<String>> {
    let tasks = parse::tasks::parse_file(&feature_dir.join("tasks.md"))?;
    if let Some(selector) = selector {
        let wanted = super::task_state::normalize_id(selector);
        let task = tasks
            .iter()
            .find(|t| t.id.eq_ignore_ascii_case(&wanted))
            .ok_or_else(|| flow_core::Error::ArtifactError {
                file: "tasks.md".into(),
                message: format!("task '{wanted}' was not found"),
            })?;
        return Ok(vec![task.id.clone()]);
    }

    if let Some(task) = tasks
        .iter()
        .find(|t| t.state.is_awaiting_acceptance())
        .or_else(|| parse::tasks::first_runnable_open_task(&tasks))
    {
        return Ok(vec![task.id.clone()]);
    }

    if tasks.iter().all(|t| t.done) {
        if let Some(task) = tasks.last() {
            return Ok(vec![task.id.clone()]);
        }
    }

    Err(flow_core::Error::ArtifactError {
        file: "tasks.md".into(),
        message: "no tasks awaiting acceptance or runnable open tasks found".into(),
    })
}
