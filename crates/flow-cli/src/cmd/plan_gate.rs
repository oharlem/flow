//! Require finalized plan (`plan-complete` in status history) before build.

use flow_core::{status, Error, Result};
use std::path::Path;

/// Return `Err` when `tasks.md` exists but plan was never finalized.
pub(crate) fn require_plan_complete(feature_dir: &Path) -> Result<()> {
    if !feature_dir.join("tasks.md").is_file() {
        return Ok(());
    }
    if status::history_contains(feature_dir, "plan-complete") {
        return Ok(());
    }
    let cmd = crate::public_command::render_current("flow-plan");
    Err(Error::User(format!(
        "Plan is not finalized (no `plan-complete` entry in status history). Run `{cmd}` with `--finalize` after saving plan.md and tasks.md, then continue with build."
    )))
}
