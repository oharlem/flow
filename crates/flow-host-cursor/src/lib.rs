//! Cursor host adapter for Flow (preview).

#![forbid(unsafe_code)]

use std::path::PathBuf;

/// Name of this host.
#[must_use]
pub fn name() -> &'static str {
    "cursor"
}

/// AGENTS.md fragment for Cursor.
#[must_use]
pub fn agents_fragment() -> &'static str {
    include_str!("assets/AGENTS.md.fragment")
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
    /// Paths written.
    pub written: Vec<PathBuf>,
}

/// Install the Cursor adapter (preview).
pub fn install(ctx: &InstallCtx) -> std::io::Result<Report> {
    let mut report = Report::default();
    let rules_dir = ctx.repo.join(".cursor").join("rules");
    std::fs::create_dir_all(&rules_dir)?;
    let rule_body = include_str!("assets/flow.mdc");
    let rule_path = rules_dir.join("flow.mdc");
    if !rule_path.exists() {
        std::fs::write(&rule_path, rule_body)?;
        report.written.push(rule_path);
    }
    let agents = ctx.repo.join("AGENTS.md");
    if !agents.exists() {
        std::fs::write(&agents, agents_fragment())?;
    } else {
        let text = std::fs::read_to_string(&agents)?;
        if !text.contains("## Cursor Notes") {
            std::fs::write(
                &agents,
                format!("{}\n{}\n", text.trim_end(), agents_fragment()),
            )?;
        }
    }
    Ok(report)
}

/// Refresh generated Cursor assets without touching root `AGENTS.md`.
pub fn refresh(ctx: &InstallCtx) -> std::io::Result<Report> {
    let mut report = Report::default();
    let rules_dir = ctx.repo.join(".cursor").join("rules");
    std::fs::create_dir_all(&rules_dir)?;
    let rule_path = rules_dir.join("flow.mdc");
    std::fs::write(&rule_path, include_str!("assets/flow.mdc"))?;
    report.written.push(rule_path);
    Ok(report)
}
