//! Install routine for the Claude Code adapter.

use flow_core::assets;
use std::path::PathBuf;

const AGENTS_MARKER_START: &str = "<!-- FLOW:CLAUDE-CODE-NOTES:START -->";
const AGENTS_MARKER_END: &str = "<!-- FLOW:CLAUDE-CODE-NOTES:END -->";
/// Static capabilities of a host adapter.
#[derive(Clone, Copy, Debug, Default)]
pub struct Capabilities {
    /// `.claude/skills/` — set of SKILL.md files.
    pub supports_skills: bool,
    /// `.claude/settings.json` installed.
    pub supports_settings_json: bool,
    /// Commands directory layout (e.g. host-specific `commands/*.md`).
    pub supports_commands_dir: bool,
}

/// Context supplied to the install routine.
#[derive(Clone, Debug)]
pub struct InstallCtx {
    /// Target git repository root.
    pub repo: PathBuf,
}

/// Install report.
#[derive(Clone, Debug, Default)]
pub struct Report {
    /// Paths written by the install routine.
    pub written: Vec<PathBuf>,
}

/// Install the Claude Code adapter into `ctx.repo`.
pub fn install(ctx: &InstallCtx) -> std::io::Result<Report> {
    let mut report = Report::default();
    let claude_dir = ctx.repo.join(".claude");
    let skills_dir = claude_dir.join("skills");
    std::fs::create_dir_all(&skills_dir)?;

    // settings.json
    let settings_path = claude_dir.join("settings.json");
    if !settings_path.exists() {
        std::fs::write(&settings_path, super::settings_json())?;
        report.written.push(settings_path);
    }

    report.written.extend(refresh(ctx)?.written);
    report.written.extend(refresh_agents_fragment(ctx)?.written);
    Ok(report)
}

/// Refresh generated Claude Code skill assets without touching `AGENTS.md` or
/// `.claude/settings.json`.
pub fn refresh(ctx: &InstallCtx) -> std::io::Result<Report> {
    let mut report = Report::default();
    let skills_dir = ctx.repo.join(".claude").join("skills");
    std::fs::create_dir_all(&skills_dir)?;

    let body_tmpl = super::skill_body_template();
    for command in assets::HOST_COMMANDS {
        let skill_dir = skills_dir.join(format!("flow-{}", command.name));
        std::fs::create_dir_all(&skill_dir)?;
        let body = body_tmpl
            .replace("{{CMD}}", command.name)
            .replace("{{FLOW_SUBCOMMAND}}", command.flow_subcommand)
            .replace(
                "{{DESCRIPTION}}",
                &command.description_for_host("Claude Code"),
            );
        let skill_path = skill_dir.join("SKILL.md");
        std::fs::write(&skill_path, body)?;
        report.written.push(skill_path);
    }

    Ok(report)
}

/// Refresh the Flow-owned Claude Code notes in `AGENTS.md`.
///
/// Modern installs are bounded by Flow ownership markers. Older installs only
/// had the generic `## Claude Code Notes` heading, so we replace that section
/// only when it has Flow's generated Claude Code wording.
pub fn refresh_agents_fragment(ctx: &InstallCtx) -> std::io::Result<Report> {
    let fragment = super::agents_fragment();
    let mut report = Report::default();
    let path = ctx.repo.join("AGENTS.md");
    if !path.exists() {
        std::fs::write(&path, ensure_trailing_newline(fragment))?;
        report.written.push(path);
        return Ok(report);
    }

    let text = std::fs::read_to_string(&path)?;
    let Some(next_text) = upsert_agents_fragment(&text, fragment) else {
        return Ok(report);
    };
    std::fs::write(&path, next_text)?;
    report.written.push(path);
    Ok(report)
}

fn upsert_agents_fragment(text: &str, fragment: &str) -> Option<String> {
    let fragment = ensure_trailing_newline(fragment);
    if let Some((start, end)) = marked_range(text) {
        return replace_range(text, start, end, &fragment);
    }

    let appended = if text.trim().is_empty() {
        fragment
    } else {
        format!("{}\n{}", text.trim_end(), fragment)
    };
    (appended != text).then_some(appended)
}

fn marked_range(text: &str) -> Option<(usize, usize)> {
    let start = text.find(AGENTS_MARKER_START)?;
    let end = text[start..].find(AGENTS_MARKER_END)? + start + AGENTS_MARKER_END.len();
    Some((start, end))
}

fn replace_range(text: &str, start: usize, end: usize, replacement: &str) -> Option<String> {
    let before = text[..start].trim_end();
    let after = text[end..].trim_start_matches(['\r', '\n']);
    let mut out = String::new();
    if !before.is_empty() {
        out.push_str(before);
        out.push_str("\n\n");
    }
    out.push_str(replacement.trim_end());
    out.push('\n');
    if !after.is_empty() {
        out.push('\n');
        out.push_str(after);
        if !out.ends_with('\n') {
            out.push('\n');
        }
    }
    (out != text).then_some(out)
}

fn ensure_trailing_newline(text: &str) -> String {
    let mut out = text.trim_end().to_string();
    out.push('\n');
    out
}
