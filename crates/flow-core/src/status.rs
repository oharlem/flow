//! `status.md` helpers (reads, validation, state).
//!
//! The parser itself lives in [`crate::parse::status`]; this module wires a
//! few common helpers that command drivers call directly.

use crate::error::Result;
use crate::parse::{
    status::{self, Status},
    tasks,
};
use once_cell::sync::Lazy;
use regex::Regex;
use std::path::Path;

/// Read + parse `status.md`.
pub fn read(feature_dir: &Path) -> Result<Status> {
    status::parse_file(&feature_dir.join("status.md"))
}

/// Return `true` when the history contains an entry with the given action slug.
pub fn history_contains(feature_dir: &Path, action: &str) -> bool {
    let Ok(s) = read(feature_dir) else {
        return false;
    };
    s.history.iter().any(|h| h.action == action)
}

/// Human-readable gate describing what must happen before the next phase advance.
pub fn effective_gate(feature_dir: &Path) -> Result<String> {
    let status = read(feature_dir)?;
    let gate = match status.state {
        Some(status::State::Drafting) => {
            if history_contains(feature_dir, "spec-complete") {
                if feature_dir.join("plan.md").exists() && feature_dir.join("tasks.md").exists() {
                    "ready for flow plan --finalize (plan.md and tasks.md present)".to_string()
                } else {
                    "ready for flow plan (draft plan.md and tasks.md)".to_string()
                }
            } else {
                "ready for flow start --finalize (complete spec.md)".to_string()
            }
        }
        Some(status::State::Building) => {
            let tasks_file = feature_dir.join("tasks.md");
            if tasks_file.exists() {
                let has_unaccepted = tasks::parse_file(&tasks_file)
                    .map(|tasks| tasks.iter().any(|t| !t.done))
                    .unwrap_or(true);
                if has_unaccepted {
                    return Ok(
                        "blocked: accept all tasks ([x]) via flow build / flow build-task"
                            .to_string(),
                    );
                }
            }
            if history_contains(feature_dir, "build-complete") {
                "ready for flow close".to_string()
            } else {
                "blocked: run flow test to record build-complete".to_string()
            }
        }
        Some(status::State::Closed) => "closed".to_string(),
        None => "unknown — run flow status".to_string(),
    };
    Ok(gate)
}

/// Return the next recommended public command based on state + completion.
#[must_use]
pub fn next_command(feature_dir: &Path) -> &'static str {
    let Ok(status) = read(feature_dir) else {
        return "flow-status";
    };
    match status.state {
        Some(status::State::Drafting) => {
            let has_plan_artifacts =
                feature_dir.join("plan.md").exists() && feature_dir.join("tasks.md").exists();
            if has_plan_artifacts && history_contains(feature_dir, "plan-complete") {
                "flow-build-task"
            } else {
                "flow-plan"
            }
        }
        Some(status::State::Building) => {
            let tasks_file = feature_dir.join("tasks.md");
            if tasks_file.exists() {
                let has_unaccepted = tasks::parse_file(&tasks_file)
                    .map(|tasks| tasks.iter().any(|t| !t.done))
                    .unwrap_or(true);
                if has_unaccepted {
                    return "flow-build-task";
                }
            }
            if history_contains(feature_dir, "build-complete") {
                "flow-close"
            } else {
                "flow-test"
            }
        }
        Some(status::State::Closed) | None => "flow-status",
    }
}

/// Parse entries from the optional `## Known Regressions` section.
///
/// Format per line: `- <test_name> — <reason> (<YYYY-MM-DD>)`.
pub fn list_known_regressions(feature_dir: &Path) -> Vec<String> {
    static ITEM: Lazy<Regex> =
        Lazy::new(|| Regex::new(r"^\s*-\s+(\S+)\s+—\s+.*?\(\d{4}-\d{2}-\d{2}\)\s*$").unwrap());
    let Ok(text) = std::fs::read_to_string(feature_dir.join("status.md")) else {
        return Vec::new();
    };
    let Some(section) = section_body(&text, "## Known Regressions") else {
        return Vec::new();
    };
    section
        .lines()
        .filter_map(|line| ITEM.captures(line).map(|c| c[1].to_string()))
        .collect()
}

/// Append a known-regression entry to `status.md`.
pub fn add_known_regression(feature_dir: &Path, test_name: &str, reason: &str) -> Result<()> {
    let path = feature_dir.join("status.md");
    let text = std::fs::read_to_string(&path)?;
    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
    let entry = format!("- {test_name} — {reason} ({today})\n");
    let new_text = if text.contains("## Known Regressions") {
        text.replacen(
            "## Known Regressions\n",
            &format!("## Known Regressions\n{entry}"),
            1,
        )
    } else {
        let mut out = text.trim_end().to_string();
        out.push('\n');
        out.push_str("\n## Known Regressions\n\n");
        out.push_str(&entry);
        out
    };
    std::fs::write(&path, new_text)?;
    Ok(())
}

/// Write the consistency-check cache at `<change_dir>/.flow-test.last.md`.
///
/// Consumed by the envelope composer so subsequent phases can surface the
/// most recent findings without re-running the drift engine.
pub fn write_cache(feature_dir: &Path, report_md: &str) -> Result<()> {
    let path = feature_dir.join(".flow-test.last.md");
    std::fs::write(path, report_md)?;
    Ok(())
}

/// Read the cached consistency-check report, if present.
#[must_use]
pub fn read_cache(feature_dir: &Path) -> Option<String> {
    std::fs::read_to_string(feature_dir.join(".flow-test.last.md")).ok()
}

/// Remove the consistency-check cache, if present. Idempotent.
///
/// Called at close finalize: the cache only carries findings between phases
/// of an open change, so nothing reads it once the change is closed.
pub fn remove_cache(feature_dir: &Path) -> Result<()> {
    let path = feature_dir.join(".flow-test.last.md");
    if path.exists() {
        std::fs::remove_file(path)?;
    }
    Ok(())
}

fn section_body<'a>(text: &'a str, heading: &str) -> Option<&'a str> {
    let start = text.find(heading)?;
    let rest = &text[start + heading.len()..];
    let end = rest.find("\n## ").unwrap_or(rest.len());
    Some(&rest[..end])
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn next_command_drafting_without_plan_complete_routes_to_plan() {
        let td = TempDir::new().unwrap();
        let feature = td.path().join("feat");
        std::fs::create_dir_all(&feature).unwrap();
        std::fs::write(feature.join("plan.md"), "## Summary\n\nx\n").unwrap();
        std::fs::write(feature.join("tasks.md"), "## Tasks\n\n- [ ] **T-001**: a\n").unwrap();
        std::fs::write(
            feature.join("status.md"),
            "# Status: feat\n\n**Change**: feat\n**Started**: 2026-01-01\n**Updated**: 2026-01-01T00:00:00Z\n**State**: drafting\n**Branch**: flow/feat\n\n## History\n\n- 2026-01-01T00:00:00Z — started — seeded\n",
        )
        .unwrap();
        assert_eq!(next_command(&feature), "flow-plan");
    }

    #[test]
    fn next_command_drafting_with_plan_complete_routes_to_build_task() {
        let td = TempDir::new().unwrap();
        let feature = td.path().join("feat");
        std::fs::create_dir_all(&feature).unwrap();
        std::fs::write(feature.join("plan.md"), "## Summary\n\nx\n").unwrap();
        std::fs::write(feature.join("tasks.md"), "## Tasks\n\n- [ ] **T-001**: a\n").unwrap();
        std::fs::write(
            feature.join("status.md"),
            "# Status: feat\n\n**Change**: feat\n**Started**: 2026-01-01\n**Updated**: 2026-01-01T00:00:00Z\n**State**: drafting\n**Branch**: flow/feat\n\n## History\n\n- 2026-01-01T00:00:00Z — plan-complete — ok\n",
        )
        .unwrap();
        assert_eq!(next_command(&feature), "flow-build-task");
    }

    #[test]
    fn effective_gate_drafting_before_spec_complete() {
        let td = TempDir::new().unwrap();
        let feature = td.path().join("feat");
        std::fs::create_dir_all(&feature).unwrap();
        std::fs::write(
            feature.join("status.md"),
            "# Status: feat\n\n**Change**: feat\n**Started**: 2026-01-01\n**Updated**: 2026-01-01T00:00:00Z\n**State**: drafting\n**Branch**: flow/feat\n\n## History\n\n- 2026-01-01T00:00:00Z — started — seeded\n",
        )
        .unwrap();
        let gate = effective_gate(&feature).unwrap();
        assert!(gate.contains("flow start --finalize"));
    }

    #[test]
    fn effective_gate_building_blocked_without_build_complete() {
        let td = TempDir::new().unwrap();
        let feature = td.path().join("feat");
        std::fs::create_dir_all(&feature).unwrap();
        std::fs::write(
            feature.join("tasks.md"),
            "## Tasks\n\n- [x] **T-001**: done\n",
        )
        .unwrap();
        std::fs::write(
            feature.join("status.md"),
            "# Status: feat\n\n**Change**: feat\n**Started**: 2026-01-01\n**Updated**: 2026-01-01T00:00:00Z\n**State**: building\n**Branch**: flow/feat\n\n## History\n\n- 2026-01-01T00:00:00Z — plan-complete — ok\n",
        )
        .unwrap();
        let gate = effective_gate(&feature).unwrap();
        assert!(gate.contains("flow test"));
    }

    #[test]
    fn effective_gate_building_ready_for_close_after_build_complete() {
        let td = TempDir::new().unwrap();
        let feature = td.path().join("feat");
        std::fs::create_dir_all(&feature).unwrap();
        std::fs::write(
            feature.join("tasks.md"),
            "## Tasks\n\n- [x] **T-001**: done\n",
        )
        .unwrap();
        std::fs::write(
            feature.join("status.md"),
            "# Status: feat\n\n**Change**: feat\n**Started**: 2026-01-01\n**Updated**: 2026-01-01T00:00:00Z\n**State**: building\n**Branch**: flow/feat\n\n## History\n\n- 2026-01-01T00:00:00Z — build-complete — ok\n",
        )
        .unwrap();
        let gate = effective_gate(&feature).unwrap();
        assert_eq!(gate, "ready for flow close");
    }
}
