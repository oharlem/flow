//! Shared rendering of `flow --help` output.
//!
//! Both `examples/generate_cli_reference.rs` and `flow doctor`'s drift check
//! call `render_full()` so that "the on-disk reference matches what clap
//! would produce right now" has exactly one definition.

use clap::CommandFactory;

use crate::args::Cli;

const GENERATED_HELP_WIDTH: usize = 100;

/// Render the full `docs/reference/cli.md` body from clap introspection.
///
/// The output is byte-stable for a given build of `flow-cli`. It is the
/// single source of truth for both the regeneration example and the
/// doctor drift check (T-001, T-009).
pub fn render_full() -> String {
    let mut root = Cli::command();
    root.build();

    let mut out = String::new();
    out.push_str("# CLI reference\n\n");
    out.push_str(crate::ownership::CLI_REFERENCE_MARKER);
    out.push_str("\n\n");
    out.push_str("*Generated from `cargo run -p flow-cli --example generate_cli_reference`. Do not edit by hand.*\n\n");
    out.push_str("## `flow`\n\n");
    out.push_str("```\n");
    out.push_str(&render_help(&root.clone().disable_help_subcommand(true)));
    out.push_str("\n```\n\n");

    let mut subs: Vec<_> = root.get_subcommands().cloned().collect();
    subs.sort_by(|a, b| a.get_name().cmp(b.get_name()));

    for sub in &subs {
        if sub.get_name() == "help" || sub.is_hide_set() {
            continue;
        }
        out.push_str(&format!("## `flow {}`\n\n", sub.get_name()));
        out.push_str("```\n");
        out.push_str(&render_help(&sub.clone().disable_help_flag(false)));
        out.push_str("\n```\n\n");
    }

    while out.ends_with("\n\n") {
        out.pop();
    }
    out
}

fn render_help(cmd: &clap::Command) -> String {
    let mut cmd = cmd.clone();
    cmd = cmd.term_width(GENERATED_HELP_WIDTH);
    cmd.render_long_help().to_string()
}
