//! `flow` command-line interface.

#![forbid(unsafe_code)]

pub mod args;
pub mod cli_help;
pub mod cmd;
pub mod generated_docs;
pub mod logo;
pub mod output;
pub mod ownership;
pub mod public_command;
pub mod summary;

use args::{Cli, Commands};
use clap::error::ErrorKind;
use clap::Parser;
use std::ffi::{OsStr, OsString};

/// Run Flow with the given argv and return an exit code.
pub fn run<I, T>(argv: I) -> i32
where
    I: IntoIterator<Item = T>,
    T: Into<OsString>,
{
    flow_core::logging::init();
    let argv: Vec<OsString> = argv.into_iter().map(Into::into).collect();
    let top_level_help_requested = is_top_level_help_request(&argv);
    let cli = match Cli::try_parse_from(argv) {
        Ok(c) => c,
        Err(e) => {
            // Brand reveal: bare `flow` and top-level `flow --help` are the
            // genuine "load the tool" moments. Version probes, subcommand
            // help, and parse errors stay logo-free.
            if e.kind() == ErrorKind::DisplayVersion {
                println!("{}", args::version_line(logo::can_color()));
                return 0;
            }
            if e.kind() == ErrorKind::DisplayHelp && top_level_help_requested && logo::can_animate()
            {
                let _ = logo::show();
                println!();
            }
            if e.kind() == ErrorKind::MissingSubcommand && logo::can_animate() {
                let _ = logo::show();
                println!();
            }
            let exit_code = if e.use_stderr() { 64 } else { 0 };
            let _ = e.print();
            return exit_code;
        }
    };

    let result = match cli.command {
        Commands::Init(a) => cmd::init::run(a),
        Commands::Update(a) => cmd::update::run(a),
        Commands::Doctor(a) => cmd::doctor::run(a),
        Commands::ExportAssets(a) => cmd::export_assets::run(a),
        Commands::Start(a) => cmd::start::run(a),
        Commands::Amend(a) => cmd::amend::run(a),
        Commands::Plan(a) => cmd::plan::run(a),
        Commands::Build(a) => cmd::build::run(a),
        Commands::BuildTask(a) => cmd::build_task::run(a),
        Commands::Test(a) => cmd::test::run(a),
        Commands::Close(a) => cmd::close::run(a),
        Commands::Status(a) => cmd::status::run(a, cli.json),
        Commands::Set(a) => cmd::set::run(a),
        Commands::Settings(a) => cmd::settings::run(a),
        Commands::Setup(a) => cmd::setup::run(a),
        Commands::Roadmap(a) => cmd::roadmap::run(a),
        Commands::Run(a) => cmd::run::run(a),
    };

    match result {
        Ok(()) => 0,
        Err(e) => {
            eprintln!("[flow] ERROR: {e}");
            e.exit_code()
        }
    }
}

fn is_top_level_help_request(argv: &[OsString]) -> bool {
    let mut saw_help = false;
    for arg in argv.iter().skip(1) {
        if arg == OsStr::new("-h") || arg == OsStr::new("--help") {
            saw_help = true;
            continue;
        }
        if arg == OsStr::new("--json") {
            continue;
        }
        return false;
    }
    saw_help
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(items: &[&str]) -> Vec<OsString> {
        items.iter().map(OsString::from).collect()
    }

    #[test]
    fn top_level_help_request_matches_root_help_flags() {
        assert!(is_top_level_help_request(&args(&["flow", "--help"])));
        assert!(is_top_level_help_request(&args(&["flow", "-h"])));
        assert!(is_top_level_help_request(&args(&[
            "flow", "--json", "--help"
        ])));
    }

    #[test]
    fn top_level_help_request_excludes_subcommand_help_and_version() {
        assert!(!is_top_level_help_request(&args(&[
            "flow", "start", "--help"
        ])));
        assert!(!is_top_level_help_request(&args(&["flow", "--version"])));
        assert!(!is_top_level_help_request(&args(&["flow"])));
    }
}
