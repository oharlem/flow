//! `flow status` — read-only report. Honors `--json`.

use crate::args::StatusArgs;
use flow_core::{
    drift::{self, render::Mode},
    parse, paths, status as status_helpers, Result,
};
use serde::Serialize;

/// JSON-serializable status payload.
#[derive(Serialize)]
struct StatusJson<'a> {
    change: &'a str,
    state: Option<String>,
    branch: &'a str,
    milestones: &'a [String],
    updated: &'a str,
    started: &'a str,
    history: &'a [parse::status::HistoryEntry],
    next_command: &'a str,
    drift: &'a drift::Report,
}

/// Run `flow status`.
pub fn run(args: StatusArgs, json: bool) -> Result<()> {
    let repo = paths::repo_root(None)?;
    let feature_dir = match args.change_dir {
        Some(d) => d,
        None => super::amend::resolve_feature_dir(&repo)
            .ok()
            .unwrap_or_else(|| paths::runs_dir(&repo)),
    };

    if feature_dir.join("status.md").is_file() {
        let status = status_helpers::read(&feature_dir)?;
        let report = if feature_dir.join("tasks.md").exists() {
            drift::build_report(drift::check_artifacts(&feature_dir, Some(&repo))?)
        } else {
            drift::Report::default()
        };
        let next = status_helpers::next_command(&feature_dir);
        let next_display = crate::public_command::render_current(next);

        if json {
            let payload = StatusJson {
                change: &status.feature,
                state: status.state.map(|s| s.to_string()),
                branch: &status.branch,
                milestones: &status.milestones,
                updated: &status.updated,
                started: &status.started,
                history: &status.history,
                next_command: next,
                drift: &report,
            };
            println!("{}", serde_json::to_string_pretty(&payload)?);
            return Ok(());
        }

        print_human(&status, &report, &feature_dir, &next_display);
    } else if json {
        println!(r#"{{"change": null, "next_command": "flow-start"}}"#);
    } else {
        println!("# Flow Status");
        println!();
        let next = crate::public_command::render_current("flow-start");
        println!("No active change detected. Run `{next} <description>` to begin.");
    }
    Ok(())
}

fn print_human(
    status: &parse::status::Status,
    report: &drift::Report,
    feature_dir: &std::path::Path,
    next: &str,
) {
    println!("# Flow Status");
    println!();
    println!("**Change**: {}", status.feature);
    if let Some(state) = status.state {
        println!("**State**: {state}");
    }
    if !status.branch.is_empty() {
        println!("**Branch**: {}", status.branch);
    }
    if !status.milestones.is_empty() {
        println!("**Milestone**: {}", status.milestones.join(", "));
    }
    println!();
    println!("## Recent History");
    for entry in status.history.iter().take(5) {
        println!(
            "- {} — {} — {}",
            entry.timestamp, entry.action, entry.summary
        );
    }
    println!();

    if feature_dir.join("tasks.md").exists() {
        println!(
            "{}",
            drift::render::render(report, Mode::Status, next, false)
        );
    } else {
        println!(
            "{}",
            drift::render::render(
                &drift::Report::default(),
                Mode::Status,
                &crate::public_command::render_current("flow-plan"),
                true,
            )
        );
    }
}
