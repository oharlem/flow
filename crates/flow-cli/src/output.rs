//! Output helpers shared across command drivers.

use std::path::{Path, PathBuf};

/// Print the `Next command: …` footer.
pub fn print_next(command: &str, detail: &str) {
    let (command, detail) = if should_resume_roadmap_run(command) {
        (
            "flow-run".to_string(),
            roadmap_run_resume_detail(detail.trim()),
        )
    } else {
        (command.trim().to_string(), detail.to_string())
    };
    let rendered = crate::public_command::render_current(&command);
    flow_core::envelope::print_next_command(&rendered, &detail);
}

fn should_resume_roadmap_run(command: &str) -> bool {
    let canonical = crate::public_command::canonicalize(command);
    if !canonical.starts_with("flow-") || canonical == "flow-run" {
        return false;
    }
    active_roadmap_run_context()
}

fn active_roadmap_run_context() -> bool {
    let Some(raw) = std::env::var_os("FLOW_RUN_DIR") else {
        return false;
    };
    let Ok(repo) = flow_core::paths::repo_root(None) else {
        return false;
    };
    let run_dir = PathBuf::from(raw);
    let run_dir = if run_dir.is_absolute() {
        run_dir
    } else {
        repo.join(run_dir)
    };
    let Ok(state) = crate::cmd::run::read_run_state(&run_dir) else {
        return false;
    };
    state.get("Run type").map(String::as_str) == Some("roadmap")
        && state.get("Status").map(String::as_str) != Some("complete")
}

fn roadmap_run_resume_detail(detail: &str) -> String {
    if detail.is_empty() {
        return "continue this roadmap automation run.".to_string();
    }
    if detail.starts_with("after ") {
        return format!("continue this roadmap automation run {detail}");
    }
    format!("continue this roadmap automation run; {detail}")
}

/// Print finalize instructions when `review.before_finalize` (or a per-command
/// override) requires a two-stage protocol; otherwise suppress the footer.
pub fn maybe_print_finalize_hint(phase: &str, feature_dir: &Path) {
    if should_emit_finalize_footer(feature_dir, phase) {
        print_finalize_hint(phase, feature_dir);
    }
}

/// Print finalization instructions with an explicit command when review settings
/// require a two-stage protocol.
pub fn maybe_print_finalize_command(command: &str, feature_dir: &Path, phase: &str) {
    if should_emit_finalize_footer(feature_dir, phase) {
        print_finalize_command(command, feature_dir);
    }
}

/// Return `true` when the printed finalize footer should appear for `phase`.
#[must_use]
pub fn should_emit_finalize_footer(feature_dir: &Path, phase: &str) -> bool {
    let Ok(repo) = flow_core::paths::repo_root(Some(feature_dir)) else {
        return true;
    };
    let settings = flow_core::settings::Settings::load_for_repo(&repo).unwrap_or_default();
    !settings.review_skip_finalize_footer(phase)
}

/// Print the finalize instructions footer shared by pre-model envelopes.
///
/// Emits the stable shape `flow <phase> --finalize` with no per-change path
/// argument so that hosts which prompt per shell invocation see one matchable
/// command identity per Flow subcommand.
pub fn print_finalize_hint(phase: &str, feature_dir: &std::path::Path) {
    print_finalize_command(&format!("flow {phase} --finalize"), feature_dir);
}

/// Print finalization instructions with an explicit command.
pub fn print_finalize_command(command: &str, feature_dir: &Path) {
    println!();
    println!("---");
    println!();
    println!("## Finalization Instructions");
    println!();
    if confirmation_disabled(feature_dir) {
        println!("Project confirmation is disabled (`flow set confirmation=no`). When the work is ready, run:");
    } else {
        println!("Ask the user to reply `yes` or `y` to save Flow state. Then run:");
    }
    println!();
    println!("```sh");
    println!("{command}");
    println!("```");
    println!();
    println!("Change directory: `{}`", feature_dir.display());
}

fn confirmation_disabled(feature_dir: &Path) -> bool {
    let Ok(repo) = flow_core::paths::repo_root(Some(feature_dir)) else {
        return false;
    };
    flow_core::settings::Settings::load_for_repo(&repo)
        .map(|settings| settings.confirmation.is_disabled())
        .unwrap_or(false)
}
