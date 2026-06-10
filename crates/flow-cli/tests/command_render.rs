use flow_cli::public_command::{render_for_host, render_universal, Host};

#[test]
fn t009_t010_render_codex_skill_mentions() {
    assert_eq!(render_for_host("flow-plan", Host::Codex), "$flow-plan");
    assert_eq!(render_for_host("/flow-build", Host::Codex), "$flow-build");
}

#[test]
fn t009_t010_render_slash_host_commands() {
    assert_eq!(
        render_for_host("flow-close", Host::ClaudeCode),
        "/flow-close"
    );
    assert_eq!(render_for_host("flow-status", Host::Cursor), "/flow-status");
}

#[test]
fn t009_t010_render_universal_mapping_when_host_is_unknown() {
    let rendered = render_universal("flow-plan");
    assert_eq!(rendered, "flow plan");
}
