//! All top-level Flow command drivers.
//!
//! Artifact-mutating commands use a two-function layout:
//! - `prepare` — print the host envelope (no state stamp)
//! - `finalize` — validate artifacts and stamp `status.md` (CLI `--finalize`)
//!
//! Internal phase chains (e.g. `build` → `test::run_and_finalize` when all
//! tasks are complete) reuse a phase's `finalize` body via a `run_and_finalize`
//! helper exposed only on the chained-into command. New chains should add a
//! helper on the destination phase only when actually wired up; do not
//! pre-export speculative chain entry points.
//!
//! The agent green path is unchanged: `flow <cmd>` → edit artifacts →
//! `flow <cmd> --finalize`. `review.before_finalize: false` only suppresses the
//! printed footer checkpoint, not the separate finalize invocation.

pub mod amend;
pub mod build;
mod build_pending;
pub mod build_task;
pub mod close;
pub mod doctor;
pub mod export_assets;
pub mod init;
pub mod plan;
mod plan_gate;
pub mod roadmap;
pub mod run;
pub mod set;
pub mod settings;
pub mod setup;
pub mod start;
pub mod status;
mod task_state;
pub mod test;
pub mod update;
mod version_marker;

/// Mode of a `--finalize` argument after clap parsing.
pub(crate) enum FinalizeMode {
    /// `--finalize` flag was not provided; not a finalize call.
    Skip,
    /// `--finalize` was provided with no value; the handler infers.
    Infer,
}

pub(crate) fn finalize_mode(arg: bool) -> FinalizeMode {
    match arg {
        false => FinalizeMode::Skip,
        true => FinalizeMode::Infer,
    }
}
