//! Resume check — warn when a change directory already has artifacts.

use crate::logging;
use std::path::Path;

/// Emit a `[flow] WARN: …` message when the change dir already has planning
/// files that a new Flow phase would normally create. Never errors.
pub fn check(feature_dir: &Path) {
    if feature_dir.join("tasks.md").exists() && feature_dir.join("plan.md").exists() {
        logging::warn(format!(
            "Change directory already has plan.md and tasks.md: {}. Continuing.",
            feature_dir.display()
        ));
    }
}
