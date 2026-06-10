//! Claude Code host adapter for Flow.

#![forbid(unsafe_code)]

pub mod install;
pub mod recommendations;

pub use install::{install, Capabilities, InstallCtx, Report};
pub use recommendations::{
    advisory_block, read_user_permissions_state, AdvisoryBlock, UserPermissionsState,
    RECOMMENDED_RULES,
};

/// Name of this host.
#[must_use]
pub fn name() -> &'static str {
    "claude-code"
}

/// Capabilities of this host adapter.
#[must_use]
pub fn capabilities() -> Capabilities {
    Capabilities {
        supports_skills: true,
        supports_settings_json: true,
        supports_commands_dir: false,
    }
}

/// Fragment appended to the project's `AGENTS.md`.
#[must_use]
pub fn agents_fragment() -> &'static str {
    include_str!("assets/AGENTS.md.fragment")
}

/// Per-skill body template.
#[must_use]
pub fn skill_body_template() -> &'static str {
    include_str!("assets/SKILL.body.md.tmpl")
}

/// `.claude/settings.json` fragment.
#[must_use]
pub fn settings_json() -> &'static str {
    include_str!("assets/settings.json.tmpl")
}
