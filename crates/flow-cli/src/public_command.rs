//! User-facing Flow command rendering.

use flow_core::assets;

/// Environment variable set by host adapters before launching the Flow driver.
pub const FLOW_HOST_ENV: &str = "FLOW_HOST";

/// Supported host command syntaxes.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Host {
    /// Codex skill mentions use `$flow-*`.
    Codex,
    /// Claude Code slash commands use `/flow-*`.
    ClaudeCode,
    /// Cursor rules guide users to slash command syntax.
    Cursor,
}

impl Host {
    fn from_env_value(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "codex" => Some(Self::Codex),
            "claude" | "claude-code" | "claude_code" => Some(Self::ClaudeCode),
            "cursor" => Some(Self::Cursor),
            _ => None,
        }
    }
}

/// Return the active host declared by the process environment, if any.
#[must_use]
pub fn active_host() -> Option<Host> {
    std::env::var(FLOW_HOST_ENV)
        .ok()
        .and_then(|value| Host::from_env_value(&value))
}

/// Normalize a public Flow command into its host-neutral identity.
#[must_use]
pub fn canonicalize(command: &str) -> String {
    let trimmed = command.trim();
    let Some(stripped) = trimmed
        .strip_prefix("$flow-")
        .or_else(|| trimmed.strip_prefix("/flow-"))
    else {
        if let Some(rest) = trimmed.strip_prefix("flow ") {
            return format!("flow-{}", rest.trim().replace(' ', "-"));
        }
        return trimmed.to_string();
    };
    format!("flow-{stripped}")
}

fn flow_name(command: &str) -> Option<String> {
    let canonical = canonicalize(command);
    canonical
        .strip_prefix("flow-")
        .filter(|name| !name.is_empty())
        .map(ToOwned::to_owned)
}

/// Render a Flow command using a specific host's public syntax.
#[must_use]
pub fn render_for_host(command: &str, host: Host) -> String {
    let Some(name) = flow_name(command) else {
        return command.trim().to_string();
    };
    match host {
        Host::Codex => format!("$flow-{name}"),
        Host::ClaudeCode | Host::Cursor => format!("/flow-{name}"),
    }
}

/// Render a compact cross-host mapping for generic CLI contexts.
#[must_use]
pub fn render_universal(command: &str) -> String {
    let Some(name) = flow_name(command) else {
        return command.trim().to_string();
    };
    let subcommand = assets::host_command(&name)
        .map(|command| command.flow_subcommand)
        .unwrap_or(&name);
    format!("flow {subcommand}")
}

/// Render a command for the active host, or use the universal mapping when no
/// host is known.
#[must_use]
pub fn render_current(command: &str) -> String {
    match active_host() {
        Some(host) => render_for_host(command, host),
        None => render_universal(command),
    }
}
