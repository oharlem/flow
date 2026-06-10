//! `clap`-based argument parser for every Flow command.
//!
//! Hidden `--finalize` flags are post-model state-save hooks printed by Flow
//! envelopes and executed by the agent after artifact edits are complete.
//!
//! Hidden `flow run --checkpoint <run-dir> --milestone M-N` is the one
//! internal roadmap-run path that may create local checkpoint commits when that
//! project setting is enabled. These flags are not public workflow commands.

use clap::{Args, Parser, Subcommand};
use std::path::PathBuf;

/// Version string shown by `flow --version`. Includes the workspace version,
/// the short git SHA baked at build time, and the build date.
pub const LONG_VERSION: &str = concat!(
    env!("CARGO_PKG_VERSION"),
    " (",
    env!("FLOW_GIT_SHA"),
    " built ",
    env!("FLOW_BUILD_DATE"),
    ")"
);

const ANSI_RESET: &str = "\x1B[0m";

/// Full line shown by `flow --version`.
pub(crate) fn version_line(color: bool) -> String {
    if color {
        let (r, g, b) = crate::logo::BRAND_BLUE;
        format!(
            "flow \x1B[38;2;{r};{g};{b}m{}{} ({} built {})",
            env!("CARGO_PKG_VERSION"),
            ANSI_RESET,
            env!("FLOW_GIT_SHA"),
            env!("FLOW_BUILD_DATE")
        )
    } else {
        format!("flow {LONG_VERSION}")
    }
}

/// Flow — spec-driven workflow toolkit for AI coding agents.
#[derive(Parser, Debug)]
#[command(name = "flow", version = LONG_VERSION, about, long_about = None)]
pub struct Cli {
    /// Emit machine-readable JSON where supported.
    #[arg(long, global = true)]
    pub json: bool,

    #[command(subcommand)]
    pub command: Commands,
}

/// The top-level Flow subcommands.
#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Install Flow into the current repository.
    Init(InitArgs),
    /// Update Flow templates and host assets in the current repository.
    Update(UpdateArgs),
    /// Check the local Flow installation.
    Doctor(DoctorArgs),
    /// Export embedded default conventions and base prompts.
    ExportAssets(ExportAssetsArgs),
    /// Draft a new change spec.
    Start(StartArgs),
    /// Update the active change spec.
    Amend(AmendArgs),
    /// Draft the implementation plan and task list.
    Plan(PlanArgs),
    /// Implement all remaining tasks.
    Build(BuildArgs),
    /// Implement one task.
    BuildTask(BuildTaskArgs),
    /// Run verification: tests and consistency checks.
    Test(TestArgs),
    /// Close a completed change in place.
    Close(CloseArgs),
    /// Show current status, consistency findings, and next action.
    Status(StatusArgs),
    /// Store a project setting.
    Set(SetArgs),
    /// Show current project settings.
    Settings(SettingsArgs),
    /// Install or upgrade Flow host assets.
    Setup(SetupArgs),
    /// Decompose a PRD or notes file into a planned roadmap run.
    Roadmap(RoadmapArgs),
    /// Start or continue a planned roadmap run.
    Run(RunArgs),
}

/// `flow init` arguments.
#[derive(Args, Debug, Clone)]
pub struct InitArgs {
    /// Comma-separated hosts to wire up (`claude-code,codex,cursor`).
    #[arg(long = "host", value_name = "HOSTS")]
    pub host: Option<String>,
}

/// `flow update` arguments.
#[derive(Args, Debug, Clone)]
pub struct UpdateArgs {
    /// Drop divergent generated default-asset copies under `.flow/conventions/`
    /// and `.flow/agents/*.base.md`, accepting the embedded defaults. Local
    /// prompt overrides under `.flow/agents/*.local.md` are preserved. Also
    /// allows the update to proceed when the running `flow` binary is older
    /// than the version recorded in `.flow/version`.
    #[arg(long)]
    pub force: bool,
}

/// `flow doctor` arguments.
#[derive(Args, Debug, Clone, Default)]
pub struct DoctorArgs {}

/// `flow export-assets` arguments.
#[derive(Args, Debug, Clone)]
pub struct ExportAssetsArgs {
    /// Directory where embedded conventions and base prompts should be written.
    #[arg(long, value_name = "DIR")]
    pub dir: PathBuf,
}

/// `flow start` arguments.
#[derive(Args, Debug, Clone)]
pub struct StartArgs {
    /// Free-form change description. May include a single positional `M-N` token to link a roadmap milestone.
    pub description: Vec<String>,

    /// Post-model finalize step. The change directory is inferred from
    /// `FLOW_CHANGE_DIR` / run-state / the current branch.
    #[arg(long, conflicts_with = "description")]
    pub finalize: bool,
}

/// `flow amend` arguments.
#[derive(Args, Debug, Clone)]
pub struct AmendArgs {
    /// Change request text.
    pub change: Vec<String>,

    /// Append a Q/A pair to `## Clarifications` in `spec.md`.
    ///
    /// Must be used with `--answer`.
    #[arg(long, value_name = "QUESTION")]
    pub ask: Option<String>,

    /// Answer text that pairs with `--ask`.
    #[arg(long, value_name = "ANSWER", requires = "ask")]
    pub answer: Option<String>,

    /// Post-model finalize step. The change directory is inferred from
    /// `FLOW_CHANGE_DIR` / run-state / the current branch.
    #[arg(long, conflicts_with_all = ["change", "ask", "answer"])]
    pub finalize: bool,
}

/// `flow plan` arguments.
#[derive(Args, Debug, Clone)]
pub struct PlanArgs {
    /// Post-model finalize step. The change directory is inferred from
    /// `FLOW_CHANGE_DIR` / run-state / the current branch.
    #[arg(long)]
    pub finalize: bool,
}

/// `flow build` arguments.
#[derive(Args, Debug, Clone)]
pub struct BuildArgs {
    /// Post-model finalize step. The change directory is inferred from
    /// `FLOW_CHANGE_DIR` / run-state / the current branch.
    #[arg(long)]
    pub finalize: bool,

    /// Accepted task IDs to mark complete during post-model finalization.
    #[arg(long, value_name = "T-NNN", hide = true)]
    pub completed: Vec<String>,
}

/// `flow build-task` arguments.
#[derive(Args, Debug, Clone)]
pub struct BuildTaskArgs {
    /// Optional task selector (e.g. `T-001`).
    #[arg(value_name = "T-NNN", value_parser = parse_task_selector)]
    pub task: Option<String>,

    /// Post-model finalize step. The change directory is inferred from
    /// `FLOW_CHANGE_DIR` / run-state / the current branch.
    #[arg(long)]
    pub finalize: bool,
}

/// `flow test` arguments.
#[derive(Args, Debug, Clone)]
pub struct TestArgs {
    /// Post-model finalize step. The change directory is inferred from
    /// `FLOW_CHANGE_DIR` / run-state / the current branch.
    #[arg(long)]
    pub finalize: bool,
}

/// `flow close` arguments.
#[derive(Args, Debug, Clone)]
pub struct CloseArgs {
    /// Post-model finalize step. The change directory is inferred from
    /// `FLOW_CHANGE_DIR` / run-state / the current branch.
    #[arg(long, hide = true)]
    pub finalize: bool,
}

/// `flow status` arguments.
#[derive(Args, Debug, Clone, Default)]
pub struct StatusArgs {
    /// Explicit change directory (default: resolve from branch or run state).
    #[arg(long = "change-dir", value_name = "CHANGE_DIR")]
    pub change_dir: Option<PathBuf>,
}

/// `flow set` arguments.
#[derive(Args, Debug, Clone)]
pub struct SetArgs {
    /// Setting assignment, such as `prefix=flow` or `confirmation=no`.
    pub assignment: String,
}

/// `flow settings` arguments.
#[derive(Args, Debug, Clone, Default)]
pub struct SettingsArgs {}

/// `flow setup` arguments.
#[derive(Args, Debug, Clone)]
pub struct SetupArgs {
    /// Comma-separated hosts to target (`claude-code,codex,cursor`).
    #[arg(long = "host", value_name = "HOSTS")]
    pub host: Option<String>,
}

/// `flow roadmap` arguments.
#[derive(Args, Debug, Clone)]
pub struct RoadmapArgs {
    /// Source: path to a PRD/notes file, or inline text. Reads stdin when empty and not a TTY.
    pub source: Vec<String>,

    /// Always append new milestones (never prompt).
    #[arg(long, conflicts_with = "replace")]
    pub append: bool,

    /// Replace existing milestones (always prompts when confirmation=required).
    #[arg(long, conflicts_with = "append")]
    pub replace: bool,

    /// Post-model finalize step for a planned roadmap run.
    #[arg(long, conflicts_with_all = ["source", "append", "replace"])]
    pub finalize: bool,
}

/// `flow run` arguments.
#[derive(Args, Debug, Clone)]
pub struct RunArgs {
    /// Optional milestone to target inside the active roadmap run (e.g. `M-1`).
    /// Omit to start or continue the run across every open milestone.
    pub target: Option<String>,

    /// Resume guidance for an interrupted run.
    #[arg(
        long,
        value_name = "RUN_DIR",
        num_args = 0..=1,
        conflicts_with_all = ["finalize", "rescan", "checkpoint", "target"]
    )]
    pub resume: Option<Option<PathBuf>>,

    /// Refresh run roadmap fingerprint and milestone snapshot from the run-local roadmap.
    #[arg(
        long,
        value_name = "RUN_DIR",
        num_args = 0..=1,
        conflicts_with_all = ["finalize", "resume", "checkpoint", "target"]
    )]
    pub rescan: Option<Option<PathBuf>>,

    /// Create a local checkpoint commit for a completed milestone in a roadmap run.
    #[arg(
        long,
        value_name = "RUN_DIR",
        hide = true,
        requires = "milestone",
        conflicts_with_all = ["finalize", "resume", "rescan", "target"]
    )]
    pub checkpoint: Option<PathBuf>,

    /// Milestone ID for `--checkpoint`.
    #[arg(long, value_name = "M-N", hide = true, requires = "checkpoint")]
    pub milestone: Option<String>,

    /// Post-run validation step. The run directory is inferred from
    /// `FLOW_RUN_DIR` / `active_run_context`.
    #[arg(
        long,
        hide = true,
        conflicts_with_all = ["resume", "rescan", "checkpoint", "target"]
    )]
    pub finalize: bool,
}

fn parse_task_selector(input: &str) -> std::result::Result<String, String> {
    input
        .parse::<flow_core::ids::TaskId>()
        .map(|_| input.to_string())
        .map_err(|err| err.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn colored_version_line_paints_version_number_brand_blue() {
        let (r, g, b) = crate::logo::BRAND_BLUE;
        let line = version_line(true);

        assert!(line.starts_with(&format!("flow \x1B[38;2;{r};{g};{b}m")));
        assert!(line.contains(env!("CARGO_PKG_VERSION")));
        assert!(line.contains(&format!("{ANSI_RESET} (")));
    }

    #[test]
    fn plain_version_line_matches_cli_shape() {
        assert!(version_line(false).starts_with(&format!("flow {} (", env!("CARGO_PKG_VERSION"))));
    }

    #[test]
    fn finalize_flag_accepts_bare_flag_for_every_subcommand() {
        let cases: &[&[&str]] = &[
            &["flow", "start", "--finalize"],
            &["flow", "amend", "--finalize"],
            &["flow", "plan", "--finalize"],
            &["flow", "build", "--finalize"],
            &["flow", "build-task", "--finalize"],
            &["flow", "test", "--finalize"],
            &["flow", "close", "--finalize"],
            &["flow", "roadmap", "--finalize"],
            &["flow", "run", "--finalize"],
        ];
        for argv in cases {
            let result = Cli::try_parse_from(argv.iter().copied());
            assert!(
                result.is_ok(),
                "expected `{}` to parse, got: {:?}",
                argv.join(" "),
                result.err().map(|e| e.to_string())
            );
        }
    }

    #[test]
    fn finalize_flag_rejects_path_values() {
        let cases: &[&[&str]] = &[
            &["flow", "start", "--finalize", "/tmp/x"],
            &["flow", "amend", "--finalize", "/tmp/x"],
            &["flow", "plan", "--finalize", "/tmp/x"],
            &["flow", "build", "--finalize", "/tmp/x"],
            &["flow", "build-task", "--finalize", "/tmp/x"],
            &["flow", "test", "--finalize", "/tmp/x"],
            &["flow", "close", "--finalize", "/tmp/x"],
            &["flow", "roadmap", "--finalize", "/tmp/x"],
            &["flow", "run", "--finalize", "/tmp/x"],
        ];
        for argv in cases {
            assert!(
                Cli::try_parse_from(argv.iter().copied()).is_err(),
                "expected `{}` to reject a finalize path",
                argv.join(" ")
            );
        }
    }

    #[test]
    fn finalize_flag_carries_boolean_value() {
        use Commands::*;
        let finalize = Cli::try_parse_from(["flow", "plan", "--finalize"]).unwrap();
        match finalize.command {
            Plan(args) => assert!(args.finalize),
            other => panic!("expected Plan, got {other:?}"),
        }
        let no_flag = Cli::try_parse_from(["flow", "plan"]).unwrap();
        match no_flag.command {
            Plan(args) => assert!(!args.finalize),
            other => panic!("expected Plan, got {other:?}"),
        }
    }

    #[test]
    fn t001_t008_run_resume_and_rescan_accept_optional_paths() {
        let resume_inferred = Cli::try_parse_from(["flow", "run", "--resume"]).unwrap();
        match resume_inferred.command {
            Commands::Run(args) => assert_eq!(args.resume, Some(None)),
            other => panic!("expected run command, got {other:?}"),
        }

        let resume_explicit = Cli::try_parse_from(["flow", "run", "--resume", "/tmp/run"]).unwrap();
        match resume_explicit.command {
            Commands::Run(args) => assert_eq!(args.resume, Some(Some(PathBuf::from("/tmp/run")))),
            other => panic!("expected run command, got {other:?}"),
        }

        let rescan_inferred = Cli::try_parse_from(["flow", "run", "--rescan"]).unwrap();
        match rescan_inferred.command {
            Commands::Run(args) => assert_eq!(args.rescan, Some(None)),
            other => panic!("expected run command, got {other:?}"),
        }

        let rescan_explicit = Cli::try_parse_from(["flow", "run", "--rescan", "/tmp/run"]).unwrap();
        match rescan_explicit.command {
            Commands::Run(args) => assert_eq!(args.rescan, Some(Some(PathBuf::from("/tmp/run")))),
            other => panic!("expected run command, got {other:?}"),
        }
    }
}
