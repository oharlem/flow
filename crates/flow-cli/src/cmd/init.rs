//! `flow init` — install Flow into the current repository.

use crate::args::InitArgs;
use flow_core::{assets, paths, settings::Settings, Result};
use std::path::{Path, PathBuf};

pub(crate) const EMBEDDED_DEFAULTS_HINT: &str =
    "run `flow export-assets --dir <DIR>` to inspect embedded defaults";

/// Disposition for an on-disk generated default-asset copy after a sweep.
#[derive(Clone, Copy, Debug)]
pub(crate) enum DivergenceMode {
    /// Preserve divergent copies and warn (used by `flow init` and plain `flow update`).
    Preserve,
    /// Delete divergent copies to align with embedded defaults (used by `flow update --force`).
    Reset,
}

/// Run `flow init`.
pub fn run(args: InitArgs) -> Result<()> {
    let repo = paths::repo_root(None)?;
    let flow_dir = paths::flow_dir(&repo);
    let hosts = requested_hosts(args)?;
    std::fs::create_dir_all(flow_dir.join("agents"))?;

    // Write config.yaml if missing before resolving the visible layout.
    let config_path = flow_dir.join("config.yaml");
    if !config_path.exists() {
        if let Some(tmpl) = assets::template("config.yaml.tmpl") {
            std::fs::write(&config_path, seed_config(tmpl, &hosts))?;
        }
    }

    let layout = paths::layout(&repo);
    std::fs::create_dir_all(&layout.workspace_dir)?;
    std::fs::create_dir_all(&layout.runs_dir)?;
    ensure_flow_docs_readme(&layout.documentation_dir)?;
    remove_generated_default_asset_copies(
        &flow_dir,
        &layout.conventions_dir,
        DivergenceMode::Preserve,
    )?;
    ensure_flow_readme(&layout.workspace_dir)?;
    crate::cmd::version_marker::write(&flow_dir)?;

    // AGENTS.md (root)
    let agents_md = repo.join("AGENTS.md");
    if !agents_md.exists() {
        if let Some(tmpl) = assets::template("AGENTS.md.tmpl") {
            std::fs::write(&agents_md, tmpl)?;
        }
    }

    ensure_settings(&repo)?;

    // Host adapter install, if requested
    for host in hosts {
        install_host(&repo, &host)?;
    }

    flow_core::logging::info(format!(
        "Flow initialized in {}. Use `flow doctor` to verify; {EMBEDDED_DEFAULTS_HINT}.",
        repo.display(),
    ));
    Ok(())
}

pub(crate) fn ensure_settings(repo: &Path) -> Result<()> {
    let path = flow_core::settings::state_path(repo);
    if !path.exists() {
        Settings::default().save_for_repo(repo)?;
    }
    Ok(())
}

pub(crate) fn ensure_flow_readme(workspace: &Path) -> Result<()> {
    let path = workspace.join("README.md");
    if !path.exists() {
        std::fs::write(
            path,
            "# Flow\n\n`flow/` is your Flow workspace. `.flow/` is Flow's runtime control plane.\n\nRoadmap runs keep milestones in `runs/<run>/roadmap.md` and child work artifacts in `runs/<run>/changes/<change>/`. See `docs/reference/artifacts.md` for the schema contract. Keep current documentation in `docs/`. Do not edit generated host assets by hand.\n",
        )?;
    }
    Ok(())
}

pub(crate) fn remove_generated_default_asset_copies(
    flow_dir: &Path,
    conventions_dir: &Path,
    mode: DivergenceMode,
) -> Result<()> {
    let mut divergent_conventions = Vec::new();
    let mut divergent_agents = Vec::new();
    let mut reset_paths = Vec::new();

    for name in assets::CONVENTIONS_SHARD_NAMES {
        let body = assets::conventions_shard(name).unwrap_or_else(|| {
            panic!("embedded conventions shard missing: {name} — rebuild the binary")
        });
        let path = conventions_dir.join(format!("{name}.md"));
        match handle_default_asset_copy(&path, body, mode)? {
            DefaultAssetOutcome::Absent | DefaultAssetOutcome::AlignedRemoved => {}
            DefaultAssetOutcome::DivergentPreserved => divergent_conventions.push(path),
            DefaultAssetOutcome::DivergentRemoved => reset_paths.push(path),
        }
    }
    match std::fs::remove_dir(conventions_dir) {
        Ok(()) => {}
        Err(err)
            if matches!(
                err.kind(),
                std::io::ErrorKind::NotFound | std::io::ErrorKind::DirectoryNotEmpty
            ) => {}
        Err(err) => return Err(err.into()),
    }

    let agents_dir = flow_dir.join("agents");
    for phase in assets::PHASES {
        let body = assets::agent_base(phase)
            .unwrap_or_else(|| panic!("embedded agent base missing: {phase} — rebuild the binary"));
        let path = agents_dir.join(format!("{phase}.base.md"));
        match handle_default_asset_copy(&path, body, mode)? {
            DefaultAssetOutcome::Absent | DefaultAssetOutcome::AlignedRemoved => {}
            DefaultAssetOutcome::DivergentPreserved => divergent_agents.push(path),
            DefaultAssetOutcome::DivergentRemoved => reset_paths.push(path),
        }
    }

    emit_default_asset_summary(&divergent_conventions, &divergent_agents, &reset_paths);
    Ok(())
}

#[derive(Debug)]
enum DefaultAssetOutcome {
    Absent,
    AlignedRemoved,
    DivergentPreserved,
    DivergentRemoved,
}

fn handle_default_asset_copy(
    path: &Path,
    embedded_body: &str,
    mode: DivergenceMode,
) -> Result<DefaultAssetOutcome> {
    let current = match std::fs::read(path) {
        Ok(current) => current,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            return Ok(DefaultAssetOutcome::Absent)
        }
        Err(err) => return Err(err.into()),
    };

    if current == embedded_body.as_bytes() {
        std::fs::remove_file(path)?;
        return Ok(DefaultAssetOutcome::AlignedRemoved);
    }

    match mode {
        DivergenceMode::Reset => {
            std::fs::remove_file(path)?;
            Ok(DefaultAssetOutcome::DivergentRemoved)
        }
        DivergenceMode::Preserve => Ok(DefaultAssetOutcome::DivergentPreserved),
    }
}

fn emit_default_asset_summary(
    divergent_conventions: &[PathBuf],
    divergent_agents: &[PathBuf],
    reset_paths: &[PathBuf],
) {
    if !reset_paths.is_empty() {
        let count = reset_paths.len();
        let listing = reset_paths
            .iter()
            .map(|p| format!("  - {}", p.display()))
            .collect::<Vec<_>>()
            .join("\n");
        let copy_word = if count == 1 { "copy" } else { "copies" };
        flow_core::logging::info(format!(
            "removed {count} divergent default-asset {copy_word}; the embedded defaults are now authoritative:\n{listing}"
        ));
    }

    let total_preserved = divergent_conventions.len() + divergent_agents.len();
    if total_preserved == 0 {
        return;
    }
    let listing = divergent_conventions
        .iter()
        .chain(divergent_agents.iter())
        .map(|p| format!("  - {}", p.display()))
        .collect::<Vec<_>>()
        .join("\n");
    let copy_word = if total_preserved == 1 {
        "copy"
    } else {
        "copies"
    };
    let mut advice = format!(
        "preserved {total_preserved} modified default-asset {copy_word}; they differ from the embedded defaults:\n{listing}\n\nReview the listed files. Re-run `flow update --force` to drop them and accept the embedded versions."
    );
    if !divergent_agents.is_empty() {
        advice
            .push_str(" Customizations to agent base prompts belong in `.flow/agents/*.local.md`.");
    }
    advice.push_str(" Use `flow export-assets --dir <DIR>` to inspect embedded defaults.");
    flow_core::logging::info(advice);
}

pub(crate) fn ensure_flow_docs_readme(docs: &Path) -> Result<()> {
    std::fs::create_dir_all(docs)?;
    let path = docs.join("README.md");
    if !path.exists() {
        std::fs::write(
            path,
            "# Flow Documentation\n\nThis directory is maintained as part of Flow changes. Keep these pages focused on current behavior, not closeout summaries.\n",
        )?;
    }
    Ok(())
}

fn requested_hosts(args: InitArgs) -> Result<Vec<String>> {
    let Some(host_list) = args.host else {
        return Ok(Vec::new());
    };

    let mut hosts = Vec::new();
    for host in host_list.split(',').map(str::trim) {
        if host.is_empty() {
            return Err(flow_core::Error::User(
                "--host expects comma-separated host names, e.g. claude-code,codex".into(),
            ));
        }
        if !is_known_host(host) {
            return Err(unknown_host(host));
        }
        if !hosts.iter().any(|known| known == host) {
            hosts.push(host.to_string());
        }
    }
    Ok(hosts)
}

pub(crate) fn seed_config(tmpl: &str, hosts: &[String]) -> String {
    if hosts.is_empty() {
        return tmpl.to_string();
    }

    let rendered_hosts = hosts
        .iter()
        .map(|host| format!("  - {host}"))
        .collect::<Vec<_>>()
        .join("\n");
    tmpl.replace(
        "hosts:\n  - claude-code",
        &format!("hosts:\n{rendered_hosts}"),
    )
}

fn install_host(repo: &Path, host: &str) -> Result<()> {
    match host {
        "claude-code" => {
            flow_host_claude_code::install::install(&flow_host_claude_code::install::InstallCtx {
                repo: repo.to_path_buf(),
            })?;
        }
        "codex" => {
            flow_host_codex::install(&flow_host_codex::InstallCtx {
                repo: repo.to_path_buf(),
            })?;
        }
        "cursor" => {
            flow_host_cursor::install(&flow_host_cursor::InstallCtx {
                repo: repo.to_path_buf(),
            })?;
        }
        other => return Err(unknown_host(other)),
    }
    Ok(())
}

fn is_known_host(host: &str) -> bool {
    matches!(host, "claude-code" | "codex" | "cursor")
}

fn unknown_host(host: &str) -> flow_core::Error {
    flow_core::Error::User(format!(
        "unknown host: {host} (expected claude-code, codex, cursor)"
    ))
}
