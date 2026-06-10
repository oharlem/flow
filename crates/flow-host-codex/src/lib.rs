//! Codex host adapter for Flow.

#![forbid(unsafe_code)]

use flow_core::assets;
use std::path::PathBuf;

/// Name of this host.
#[must_use]
pub fn name() -> &'static str {
    "codex"
}

/// Fragment appended to the project's `AGENTS.md`.
#[must_use]
pub fn agents_fragment() -> &'static str {
    include_str!("assets/AGENTS.md.fragment")
}

/// Skill body template.
#[must_use]
pub fn skill_body_template() -> &'static str {
    include_str!("assets/SKILL.body.md.tmpl")
}

/// Context for the installer.
#[derive(Clone, Debug)]
pub struct InstallCtx {
    /// Repository root.
    pub repo: PathBuf,
}

/// Install report.
#[derive(Clone, Debug, Default)]
pub struct Report {
    /// Paths that were (re)written.
    pub written: Vec<PathBuf>,
}

/// Install the Codex adapter into `ctx.repo`.
pub fn install(ctx: &InstallCtx) -> std::io::Result<Report> {
    let report = refresh(ctx)?;
    ensure_fragment(&ctx.repo, agents_fragment())?;
    Ok(report)
}

/// Refresh generated Codex assets without touching root `AGENTS.md`.
pub fn refresh(ctx: &InstallCtx) -> std::io::Result<Report> {
    let mut report = Report::default();
    let skills_dir = ctx.repo.join(".agents").join("skills");
    std::fs::create_dir_all(&skills_dir)?;
    let body_tmpl = skill_body_template();
    for command in assets::HOST_COMMANDS {
        let skill_dir = skills_dir.join(format!("flow-{}", command.name));
        std::fs::create_dir_all(&skill_dir)?;
        let body = body_tmpl
            .replace("{{CMD}}", command.name)
            .replace("{{FLOW_SUBCOMMAND}}", command.flow_subcommand)
            .replace("{{DESCRIPTION}}", &command.description_for_host("Codex"));
        let skill_path = skill_dir.join("SKILL.md");
        std::fs::write(&skill_path, body)?;
        report.written.push(skill_path);
    }
    Ok(report)
}

fn ensure_fragment(repo: &std::path::Path, fragment: &str) -> std::io::Result<()> {
    let path = repo.join("AGENTS.md");
    if !path.exists() {
        std::fs::write(&path, fragment)?;
        return Ok(());
    }
    let text = std::fs::read_to_string(&path)?;
    if text.contains("## Codex Notes") {
        return Ok(());
    }
    let new_text = format!("{}\n{}\n", text.trim_end(), fragment);
    std::fs::write(&path, new_text)?;
    Ok(())
}
