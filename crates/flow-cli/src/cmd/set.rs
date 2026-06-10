//! `flow set` — persist a project-local CLI setting.

use crate::args::SetArgs;
use flow_core::{
    config::{self, Config},
    paths, roadmap,
    settings::{ConfirmationSetting, Settings},
    Error, Result,
};
use std::path::PathBuf;

/// Run `flow set`.
pub fn run(args: SetArgs) -> Result<()> {
    let repo = paths::repo_root(None)?;
    let (name, value) = parse_assignment(&args.assignment)?;
    if name == "confirmation" {
        let confirmation = ConfirmationSetting::parse(value).map_err(Error::User)?;
        config::save_confirmation_for_repo(&repo, confirmation)?;

        println!("Set confirmation={}", confirmation.as_str());
        return Ok(());
    }

    if name == "prefix" {
        let prefix = paths::validate_prefix(value)?;
        let mut config = Config::load_for_repo(&repo)?;
        config.prefix = prefix.clone();
        config.save_for_repo(&repo)?;

        println!("Set prefix={prefix}");
        return Ok(());
    }

    if name == "counter" {
        let counter = Settings::parse_counter(value).map_err(Error::User)?;
        if roadmap::milestone_number_exists_at_paths(run_roadmap_paths(&repo), counter) {
            return Err(Error::User(format!(
                "counter={counter} would collide with an existing roadmap milestone"
            )));
        }
        let mut settings = Settings::load_for_repo(&repo)?;
        settings.counter = counter;
        settings.save_for_repo(&repo)?;

        println!("Set counter={counter}");
        return Ok(());
    }

    if name == "run_checkpoint_commits" {
        let enabled = parse_bool(value)?;
        let mut config = Config::load_for_repo(&repo)?;
        config.git.run_checkpoint_commits = enabled;
        config.save_for_repo(&repo)?;

        println!("Set run_checkpoint_commits={enabled}");
        return Ok(());
    }

    if name == "run_branch" {
        let enabled = parse_bool(value)?;
        let mut config = Config::load_for_repo(&repo)?;
        config.git.run_branch = enabled;
        config.save_for_repo(&repo)?;

        println!("Set run_branch={enabled}");
        return Ok(());
    }

    Err(Error::User(format!(
        "unknown setting {name:?}; supported settings: confirmation, counter, prefix, run_branch, run_checkpoint_commits"
    )))
}

fn run_roadmap_paths(repo: &std::path::Path) -> Vec<PathBuf> {
    let runs = paths::runs_dir(repo);
    let Ok(read) = std::fs::read_dir(runs) else {
        return Vec::new();
    };
    read.flatten()
        .map(|entry| entry.path().join("roadmap.md"))
        .filter(|path| path.is_file())
        .collect()
}

fn parse_assignment(input: &str) -> Result<(&str, &str)> {
    let Some((name, value)) = input.split_once('=') else {
        return Err(Error::User(
            "expected setting assignment NAME=VALUE, e.g. `flow set prefix=flow`".into(),
        ));
    };
    let name = name.trim();
    let value = value.trim();
    if name.is_empty() || value.is_empty() {
        return Err(Error::User(
            "expected setting assignment NAME=VALUE, e.g. `flow set prefix=flow`".into(),
        ));
    }
    Ok((name, value))
}

fn parse_bool(input: &str) -> Result<bool> {
    match input.trim().to_ascii_lowercase().as_str() {
        "true" | "yes" | "on" | "1" => Ok(true),
        "false" | "no" | "off" | "0" => Ok(false),
        _ => Err(Error::User(format!(
            "expected boolean value, got {input:?}"
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn t002_parse_assignment_requires_name_value_shape() {
        assert_eq!(
            parse_assignment("confirmation=no").unwrap(),
            ("confirmation", "no")
        );
        assert!(parse_assignment("confirmation").is_err());
        assert!(parse_assignment("confirmation=").is_err());
        assert!(parse_assignment("=no").is_err());
    }
}
