//! `flow settings` — show effective project-local CLI settings.

use crate::args::SettingsArgs;
use flow_core::{config::Config, paths, settings::Settings, Result};

/// Run `flow settings`.
pub fn run(_args: SettingsArgs) -> Result<()> {
    let repo = paths::repo_root(None)?;
    let settings = Settings::load_for_repo(&repo)?;
    let config = Config::load_for_repo(&repo)?;
    println!("prefix={}", config.prefix);
    println!("confirmation={}", settings.confirmation.as_str());
    println!("counter={}", settings.counter);
    println!("review.before_finalize={}", settings.review.before_finalize);
    if settings.review.per_command.is_empty() {
        println!("review.per_command=(none)");
    } else {
        for (cmd, value) in &settings.review.per_command {
            println!("review.per_command.{cmd}={value}");
        }
    }
    println!("run_branch={}", config.git.run_branch);
    println!(
        "run_checkpoint_commits={}",
        config.git.run_checkpoint_commits
    );
    Ok(())
}
