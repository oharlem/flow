//! `flow plan` — draft the implementation plan and task list.

use crate::args::PlanArgs;
use flow_core::{
    drift::{self, render::Mode},
    envelope, parse, paths, preflight, Result,
};
use std::path::Path;

/// Run `flow plan`.
pub fn run(args: PlanArgs) -> Result<()> {
    let repo = paths::repo_root(None)?;
    match super::finalize_mode(args.finalize) {
        super::FinalizeMode::Infer => {
            let dir = super::amend::resolve_feature_dir(&repo)?;
            return finalize(&dir);
        }
        super::FinalizeMode::Skip => {}
    }
    let feature_dir = super::amend::resolve_feature_dir(&repo)?;
    prepare(&repo, &feature_dir)
}

fn prepare(repo: &Path, feature_dir: &Path) -> Result<()> {
    let feature_name = feature_dir
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default();
    let branch = flow_core::git::current_branch(repo).unwrap_or_else(|_| "unknown".to_string());
    flow_core::render::seed_plan_files(feature_dir, &feature_name, &branch)?;
    let out = envelope::compose(repo, "plan", feature_dir, None)?;
    print!("{out}");
    crate::output::maybe_print_finalize_hint("plan", feature_dir);
    crate::output::print_next("/flow-build", "after the plan is saved.");
    Ok(())
}

fn finalize(feature_dir: &Path) -> Result<()> {
    // Plan must exist and have required sections.
    let plan = parse::plan::parse_file(&feature_dir.join("plan.md"))?;
    if !plan.is_complete() {
        return Err(flow_core::Error::ArtifactError {
            file: "plan.md".into(),
            message:
                "missing required sections (## Summary, ## Technical Context, ## Documentation Impact)"
                    .into(),
        });
    }
    // tasks.md must exist (warn via drift if absent)
    let tasks_file = feature_dir.join("tasks.md");
    if !tasks_file.exists() {
        return Err(flow_core::Error::FileNotFound {
            kind: "tasks.md".into(),
            path: tasks_file,
        });
    }
    let repo = paths::repo_root(Some(feature_dir)).unwrap_or_else(|_| feature_dir.to_path_buf());
    let cfg = flow_core::config::Config::load_for_repo(&repo)?;
    let tasks = parse::tasks::parse_file(&feature_dir.join("tasks.md"))?;
    preflight::validate_task_requirements(&tasks, &cfg)?;
    // Drift check — promote D2/D3 to error at plan finalize.
    let findings = drift::check_artifacts(feature_dir, Some(&repo))?;
    let promote: std::collections::HashSet<String> =
        ["D2", "D3"].iter().map(|s| (*s).to_string()).collect();
    let findings = drift::promote_severity(findings, &promote);
    let report = drift::build_report(findings);
    if !report.findings.is_empty() {
        println!(
            "{}",
            drift::render::render(
                &report,
                Mode::Plan,
                &crate::public_command::render_current("flow-plan"),
                false,
            )
        );
        if report.has_error {
            return Err(flow_core::Error::DriftErrors {
                errors: report
                    .findings
                    .iter()
                    .filter(|f| matches!(f.severity, drift::Severity::Error))
                    .count(),
                warns: report
                    .findings
                    .iter()
                    .filter(|f| matches!(f.severity, drift::Severity::Warn))
                    .count(),
            });
        }
    }
    parse::status::stamp(
        feature_dir,
        Some(parse::status::State::Building),
        "plan-complete",
        "plan and tasks finalized",
    )?;
    crate::cmd::run::update_run_state_for_feature_phase(
        feature_dir,
        "plan-complete",
        "plan-complete",
        "$flow-build",
    )?;
    flow_core::logging::info("Plan finalized. Flow state saved.");
    crate::output::print_next("/flow-build", "implement the tasks.");
    Ok(())
}
