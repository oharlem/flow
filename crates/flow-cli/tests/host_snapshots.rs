//! Host-adapter tree snapshots.
//!
//! Lock the full installed file tree for every host adapter. Byte-stable host
//! assets are part of the Flow contract; this test fails loudly if the tree or
//! any asset body drifts.

use flow_host_claude_code as claude;
use flow_host_codex as codex;
use flow_host_cursor as cursor;
use std::path::Path;
use tempfile::TempDir;

fn init_git(path: &Path) {
    for args in [
        vec!["init", "-q", "-b", "main"],
        vec!["config", "user.email", "host-snap@example.com"],
        vec!["config", "user.name", "host-snap"],
        vec!["commit", "--allow-empty", "-q", "-m", "init"],
    ] {
        let ok = std::process::Command::new("git")
            .args(&args)
            .current_dir(path)
            .output()
            .unwrap();
        assert!(ok.status.success(), "git {args:?} failed: {ok:?}");
    }
}

fn tree_under(root: &Path, prefix: &str) -> Vec<String> {
    let mut out = Vec::new();
    for entry in walkdir::WalkDir::new(root.join(prefix)).sort_by_file_name() {
        let Ok(entry) = entry else { continue };
        if !entry.file_type().is_file() {
            continue;
        }
        let rel = entry.path().strip_prefix(root).unwrap();
        out.push(rel.to_string_lossy().replace('\\', "/"));
    }
    out
}

fn read(root: &Path, rel: &str) -> String {
    std::fs::read_to_string(root.join(rel)).unwrap_or_else(|_| panic!("missing {rel}"))
}

fn assert_contains_all(body: &str, needles: &[&str]) {
    for needle in needles {
        assert!(
            body.contains(needle),
            "expected skill body to contain {needle:?}; body:\n{body}"
        );
    }
}

fn assert_installed_flow_wording(body: &str, context: &str, host: &str) {
    assert!(
        body.contains(&format!("FLOW_HOST={host} flow")),
        "M-23: {context} should run flow directly from PATH for {host}; body:\n{body}"
    );
    assert!(
        !body.contains(".flow/bin/flow"),
        "M-23: {context} should not mention a project-local launcher; body:\n{body}"
    );
    assert!(
        !body.contains("if command -v flow >/dev/null 2>&1"),
        "M-23: {context} should not use a conditional shell wrapper around flow invocations; body:\n{body}"
    );
}

fn assert_claude_code_command_shape(body: &str, context: &str) {
    // M-23: Claude Code SKILL prints exactly one canonical command per Flow
    // subcommand.
    assert!(
        !body.contains("Run the simple PATH command first"),
        "M-23: Claude Code {context} should not advertise a two-step command shape; body:\n{body}"
    );
    assert!(
        !body.contains("as a separate fallback command"),
        "M-23: Claude Code {context} should not print a second runnable command; body:\n{body}"
    );
    assert!(
        !body.contains("if command -v flow"),
        "M-23: Claude Code {context} should not use inline shell conditionals in generated command guidance; body:\n{body}"
    );
    assert!(
        !body.contains("; then FLOW_HOST=claude-code"),
        "M-23: Claude Code {context} should not embed PATH lookup conditionals; body:\n{body}"
    );
}

// -- Claude Code --------------------------------------------------------------

#[test]
fn claude_code_tree_snapshot() {
    let td = TempDir::new().unwrap();
    init_git(td.path());
    claude::install(&claude::InstallCtx {
        repo: td.path().to_path_buf(),
    })
    .unwrap();

    let tree = tree_under(td.path(), ".claude");
    insta::assert_yaml_snapshot!("claude_code_tree", tree);
    // Also lock the skill bodies for status and close.
    insta::assert_snapshot!(
        "claude_code_skill_status",
        read(td.path(), ".claude/skills/flow-status/SKILL.md")
    );
    insta::assert_snapshot!(
        "claude_code_skill_close",
        read(td.path(), ".claude/skills/flow-close/SKILL.md")
    );
}

#[test]
fn claude_code_guard_phrases_present() {
    let td = TempDir::new().unwrap();
    init_git(td.path());
    claude::install(&claude::InstallCtx {
        repo: td.path().to_path_buf(),
    })
    .unwrap();
    for command in flow_core::assets::HOST_COMMANDS {
        let command = command.name;
        let body = read(
            td.path(),
            &format!(".claude/skills/flow-{command}/SKILL.md"),
        );
        assert_contains_all(
            &body,
            &[
                "yes",
                "do not add a `Next command: ...` footer yet",
                "Do not describe normal Flow state updates as worktree, dirty-file, or modified `status.md` issues",
                "Never run `git push`, `git pull`, `git fetch`, `gh`, or `glab`",
            ],
        );
        assert_installed_flow_wording(&body, &format!("claude-code flow-{command}"), "claude-code");
        assert_claude_code_command_shape(&body, &format!("flow-{command}"));
    }
}

// -- Codex --------------------------------------------------------------------

#[test]
fn codex_tree_snapshot() {
    let td = TempDir::new().unwrap();
    init_git(td.path());
    codex::install(&codex::InstallCtx {
        repo: td.path().to_path_buf(),
    })
    .unwrap();
    let tree = tree_under(td.path(), ".agents");
    insta::assert_yaml_snapshot!("codex_tree", tree);
    insta::assert_snapshot!(
        "codex_skill_test",
        read(td.path(), ".agents/skills/flow-test/SKILL.md")
    );
}

#[test]
fn codex_uses_dollar_skill_mentions() {
    let td = TempDir::new().unwrap();
    init_git(td.path());
    codex::install(&codex::InstallCtx {
        repo: td.path().to_path_buf(),
    })
    .unwrap();
    for command in flow_core::assets::HOST_COMMANDS {
        let command = command.name;
        let body = read(
            td.path(),
            &format!(".agents/skills/flow-{command}/SKILL.md"),
        );
        assert!(
            body.contains(&format!("$flow-{command}")),
            "Codex skill for {command} should mention $flow-{command}"
        );
        assert!(
            body.contains("FLOW_HOST=codex"),
            "Codex skill for {command} should tell the driver the active host"
        );
        assert!(
            body.contains("Codex `$flow-*` syntax"),
            "Codex skill for {command} should render public commands with Codex syntax"
        );
        assert!(
            body.contains("append it after the Flow subcommand"),
            "Codex skill for {command} should forward optional user arguments"
        );
        assert_installed_flow_wording(&body, &format!("codex flow-{command}"), "codex");
    }
}

#[test]
fn t011_slash_hosts_set_active_host_environment() {
    let claude_td = TempDir::new().unwrap();
    init_git(claude_td.path());
    claude::install(&claude::InstallCtx {
        repo: claude_td.path().to_path_buf(),
    })
    .unwrap();
    let claude_body = read(claude_td.path(), ".claude/skills/flow-plan/SKILL.md");
    assert!(claude_body.contains("FLOW_HOST=claude-code"));
    assert!(claude_body.contains("slash-command `/flow-*` syntax"));
    assert_installed_flow_wording(&claude_body, "claude-code flow-plan", "claude-code");
    assert_claude_code_command_shape(&claude_body, "flow-plan");
}

// -- Cursor -------------------------------------------------------------------

#[test]
fn cursor_tree_snapshot() {
    let td = TempDir::new().unwrap();
    init_git(td.path());
    cursor::install(&cursor::InstallCtx {
        repo: td.path().to_path_buf(),
    })
    .unwrap();
    let tree = tree_under(td.path(), ".cursor");
    insta::assert_yaml_snapshot!("cursor_tree", tree);
    let rule = read(td.path(), ".cursor/rules/flow.mdc");
    assert!(rule.contains("FLOW_HOST=cursor"));
    assert!(rule.contains("slash-command `/flow-*` syntax"));
    assert_installed_flow_wording(&rule, "cursor rule", "cursor");
    insta::assert_snapshot!("cursor_rule", rule);
}
