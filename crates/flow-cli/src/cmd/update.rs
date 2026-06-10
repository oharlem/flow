//! `flow update` — refresh Flow templates in a repo.

use crate::args::UpdateArgs;
use flow_core::{assets, paths, Error, Result};
use serde_yaml::Value;
use std::collections::BTreeSet;
use std::path::Path;

/// Run `flow update`.
pub fn run(args: UpdateArgs) -> Result<()> {
    let repo = paths::repo_root(None)?;
    let flow_dir = paths::flow_dir(&repo);
    let previous_version = crate::cmd::version_marker::read(&flow_dir);
    let delta = crate::cmd::version_marker::classify(
        previous_version.as_deref(),
        crate::cmd::version_marker::CURRENT_VERSION,
    );
    if matches!(delta, crate::cmd::version_marker::VersionDelta::Downgrade) {
        let previous = previous_version.as_deref().unwrap_or("(none)");
        let current = crate::cmd::version_marker::CURRENT_VERSION;
        if !args.force {
            return Err(Error::User(format!(
                "refusing to downgrade Flow: .flow/version is {previous} but the running `flow` binary is {current}. \
                 Upgrade your `flow` binary (e.g. `cargo install --git https://github.com/oharlem/flow --locked --force flow-cli`) and re-run, \
                 or pass `flow update --force` to overwrite the marker and reinstall the older embedded assets.",
            )));
        }
        flow_core::logging::warn(format!(
            "forcing downgrade of Flow: .flow/version is {previous} but the running `flow` binary is {current}. \
             Embedded assets will be reverted to the older versions and the marker rewritten to {current}. \
             Reinstall a newer `flow` binary (e.g. `cargo install --git https://github.com/oharlem/flow --locked --force flow-cli` or `make up`) to recover.",
        ));
    }
    std::fs::create_dir_all(flow_dir.join("agents"))?;
    let mode = if args.force {
        super::init::DivergenceMode::Reset
    } else {
        super::init::DivergenceMode::Preserve
    };

    let hosts = detected_hosts(&repo);
    refresh_config(&flow_dir, &hosts)?;
    super::init::ensure_settings(&repo)?;

    // Embedded defaults are served from the running binary. Clean up old
    // generated copies so they do not shadow the version-coupled assets.
    let layout = paths::layout(&repo);
    std::fs::create_dir_all(&layout.workspace_dir)?;
    std::fs::create_dir_all(&layout.runs_dir)?;
    super::init::remove_generated_default_asset_copies(&flow_dir, &layout.conventions_dir, mode)?;
    super::init::ensure_flow_readme(&layout.workspace_dir)?;
    super::init::ensure_flow_docs_readme(&layout.documentation_dir)?;
    for host in &hosts {
        refresh_host(&repo, host)?;
    }
    let refreshed_docs = crate::generated_docs::refresh_existing(&repo)?;
    crate::cmd::version_marker::write(&flow_dir)?;

    if !refreshed_docs.is_empty() {
        flow_core::logging::info(format!(
            "Refreshed generated docs: {}",
            refreshed_docs.join(", ")
        ));
    }
    if hosts.is_empty() {
        flow_core::logging::info(
            "Flow refreshed. No configured host assets were found. Run `flow doctor` to verify.",
        );
    } else {
        flow_core::logging::info(format!(
            "Flow refreshed, including host assets for {}. Run `flow doctor` to verify.",
            hosts.join(", ")
        ));
    }
    log_version_outcome(delta, previous_version.as_deref());
    Ok(())
}

/// Emit the final `[flow]` line summarising what just happened to
/// `.flow/version`. The phrasing is branched on `delta` so a downgrade is
/// never reported as if it were an install.
fn log_version_outcome(delta: crate::cmd::version_marker::VersionDelta, previous: Option<&str>) {
    use crate::cmd::version_marker::VersionDelta;
    let current = crate::cmd::version_marker::CURRENT_VERSION;
    let previous = crate::cmd::version_marker::display_previous(previous);
    let line = match delta {
        VersionDelta::FirstInstall => format!("Installed Flow version: {current}"),
        VersionDelta::Same => format!("Refreshed Flow at version {current}"),
        VersionDelta::Upgrade => format!("Upgraded Flow version: {previous} -> {current}"),
        VersionDelta::Downgrade => {
            format!("Downgraded Flow version: {previous} -> {current} (forced)")
        }
    };
    flow_core::logging::info(line);
}

fn refresh_config(flow_dir: &Path, hosts: &[String]) -> Result<()> {
    let config_path = flow_dir.join("config.yaml");
    let Some(tmpl) = assets::template("config.yaml.tmpl") else {
        return Ok(());
    };
    let default_text = super::init::seed_config(tmpl, hosts);

    if !config_path.exists() {
        std::fs::write(&config_path, default_text)?;
        return Ok(());
    }

    let _validated = flow_core::config::Config::load(&config_path)?;
    let current_text = std::fs::read_to_string(&config_path)?;
    let mut current = parse_config_value(&config_path, &current_text)?;
    if current.is_null() {
        current = Value::Mapping(Default::default());
    }
    let defaults = parse_config_value(
        Path::new("assets/templates/config.yaml.tmpl"),
        &default_text,
    )?;

    let mut changed = merge_missing_defaults(&mut current, &defaults);
    if !hosts.is_empty() {
        changed |= set_detected_hosts(&mut current, hosts)?;
    }

    if changed {
        write_config_value(&config_path, &current)?;
    }

    Ok(())
}

fn parse_config_value(path: &Path, text: &str) -> Result<Value> {
    serde_yaml::from_str(text).map_err(|e| Error::Config(format!("{}: {e}", path.display())))
}

fn merge_missing_defaults(current: &mut Value, defaults: &Value) -> bool {
    let (Value::Mapping(current), Value::Mapping(defaults)) = (current, defaults) else {
        return false;
    };

    let mut changed = false;
    for (key, default_value) in defaults {
        if let Some(current_value) = current.get_mut(key) {
            changed |= merge_missing_defaults(current_value, default_value);
        } else {
            current.insert(key.clone(), default_value.clone());
            changed = true;
        }
    }
    changed
}

fn set_detected_hosts(config: &mut Value, hosts: &[String]) -> Result<bool> {
    let Value::Mapping(root) = config else {
        return Err(Error::Config(
            ".flow/config.yaml must contain a YAML mapping at the root".into(),
        ));
    };

    let host_values = hosts.iter().cloned().map(Value::String).collect::<Vec<_>>();
    let desired = Value::Sequence(host_values);
    let key = Value::String("hosts".into());
    if root.get(&key) == Some(&desired) {
        return Ok(false);
    }
    root.insert(key, desired);
    Ok(true)
}

fn write_config_value(path: &Path, config: &Value) -> Result<()> {
    let text = serde_yaml::to_string(config)
        .map_err(|e| Error::Config(format!("could not render {}: {e}", path.display())))?;
    let tmp = path.with_file_name(format!(
        "{}.tmp.{}",
        path.file_name().unwrap_or_default().to_string_lossy(),
        std::process::id()
    ));
    std::fs::write(&tmp, text)?;
    std::fs::rename(tmp, path)?;
    Ok(())
}

fn detected_hosts(repo: &Path) -> Vec<String> {
    let mut hosts = BTreeSet::new();
    if has_flow_skill(&repo.join(".claude").join("skills")) {
        hosts.insert("claude-code".to_string());
    }
    if has_flow_skill(&repo.join(".agents").join("skills")) {
        hosts.insert("codex".to_string());
    }
    if repo
        .join(".cursor")
        .join("rules")
        .join("flow.mdc")
        .is_file()
    {
        hosts.insert("cursor".to_string());
    }
    hosts.into_iter().collect()
}

fn has_flow_skill(skills_dir: &Path) -> bool {
    assets::HOST_COMMANDS
        .iter()
        .map(|command| {
            skills_dir
                .join(format!("flow-{}", command.name))
                .join("SKILL.md")
        })
        .any(|path| path.is_file())
        || has_flow_prefixed_entry(skills_dir)
}

fn has_flow_prefixed_entry(dir: &Path) -> bool {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return false;
    };
    entries.filter_map(std::result::Result::ok).any(|entry| {
        entry
            .file_name()
            .to_str()
            .is_some_and(|name| name.starts_with("flow-"))
    })
}

fn refresh_host(repo: &Path, host: &str) -> Result<()> {
    match host {
        "claude-code" => {
            let ctx = flow_host_claude_code::install::InstallCtx {
                repo: repo.to_path_buf(),
            };
            flow_host_claude_code::install::refresh(&ctx)?;
            flow_host_claude_code::install::refresh_agents_fragment(&ctx)?;
        }
        "codex" => {
            flow_host_codex::refresh(&flow_host_codex::InstallCtx {
                repo: repo.to_path_buf(),
            })?;
        }
        "cursor" => {
            flow_host_cursor::refresh(&flow_host_cursor::InstallCtx {
                repo: repo.to_path_buf(),
            })?;
        }
        _ => {}
    }
    Ok(())
}
