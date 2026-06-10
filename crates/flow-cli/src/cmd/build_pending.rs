//! Per-change `<change_dir>/.flow/build-pending.yaml` state for `flow build`.
//!
//! M-22: the printed `flow build` footer collapses to the single stable
//! string `flow build --finalize`, regardless of which task IDs were queued
//! for completion in the current round. The queue is persisted to a small
//! per-change YAML state file at `<change_dir>/.flow/build-pending.yaml`
//! during `flow build` prepare and consumed (and cleared) during finalize.
//!
//! Stale state from an interrupted prior run is treated as in-flight: the
//! next `flow build` prepare logs a clear warning and overwrites the file
//! with the new queue. The file's purpose is "queued for the current round,"
//! so the next prepare is authoritative.

use flow_core::Result;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

const SCHEMA_VERSION: u32 = 1;

#[derive(Serialize, Deserialize, Debug)]
struct State {
    schema_version: u32,
    pending: Vec<String>,
}

fn state_path(feature_dir: &Path) -> PathBuf {
    feature_dir.join(".flow").join("build-pending.yaml")
}

/// Write the queued task IDs for the current `flow build` round.
///
/// Creates `<change_dir>/.flow/` if it does not exist. Overwrites any
/// existing state file (stale-state-recovery is the caller's concern via
/// [`read`]).
pub(crate) fn write(feature_dir: &Path, ids: &[String]) -> Result<()> {
    let path = state_path(feature_dir);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|err| {
            flow_core::Error::User(format!(
                "failed to create directory {}: {err}",
                parent.display()
            ))
        })?;
    }
    let state = State {
        schema_version: SCHEMA_VERSION,
        pending: ids.to_vec(),
    };
    let body = serde_yaml::to_string(&state).map_err(|err| {
        flow_core::Error::User(format!("failed to serialize build-pending state: {err}"))
    })?;
    std::fs::write(&path, body).map_err(|err| {
        flow_core::Error::User(format!("failed to write {}: {err}", path.display()))
    })?;
    Ok(())
}

/// Read the queued task IDs from `<change_dir>/.flow/build-pending.yaml`.
///
/// Returns `Ok(None)` when the file is absent. Returns `Err` when the file
/// exists but is unreadable or malformed; callers must surface that error
/// rather than silently clearing the file.
pub(crate) fn read(feature_dir: &Path) -> Result<Option<Vec<String>>> {
    let path = state_path(feature_dir);
    if !path.exists() {
        return Ok(None);
    }
    let body = std::fs::read_to_string(&path).map_err(|err| {
        flow_core::Error::User(format!("failed to read {}: {err}", path.display()))
    })?;
    let state: State = serde_yaml::from_str(&body).map_err(|err| {
        flow_core::Error::User(format!(
            "failed to parse {} as build-pending YAML: {err}",
            path.display()
        ))
    })?;
    Ok(Some(state.pending))
}

/// Remove the state file, if any, and the `<change_dir>/.flow/` directory
/// when that leaves it empty. Idempotent.
pub(crate) fn clear(feature_dir: &Path) -> Result<()> {
    let path = state_path(feature_dir);
    if path.exists() {
        std::fs::remove_file(&path).map_err(|err| {
            flow_core::Error::User(format!("failed to remove {}: {err}", path.display()))
        })?;
    }
    if let Some(parent) = path.parent() {
        // remove_dir refuses non-empty directories, so anything else that
        // lands under <change_dir>/.flow/ keeps the directory alive.
        let _ = std::fs::remove_dir(parent);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn t001_round_trips_pending_task_ids_through_yaml_state() {
        let td = TempDir::new().unwrap();
        let feature_dir = td.path();
        write(feature_dir, &["T-001".into(), "T-002".into()]).unwrap();
        let read_back = read(feature_dir).unwrap().unwrap();
        assert_eq!(read_back, vec!["T-001".to_string(), "T-002".to_string()]);
    }

    #[test]
    fn t001_clear_removes_state_file_idempotently() {
        let td = TempDir::new().unwrap();
        let feature_dir = td.path();
        write(feature_dir, &["T-001".into()]).unwrap();
        clear(feature_dir).unwrap();
        assert!(read(feature_dir).unwrap().is_none());
        // The emptied .flow/ directory is removed along with the file.
        assert!(!feature_dir.join(".flow").exists());
        // Idempotent: clearing an already-absent file is fine.
        clear(feature_dir).unwrap();
    }

    #[test]
    fn t001_clear_keeps_flow_dir_with_other_contents() {
        let td = TempDir::new().unwrap();
        let feature_dir = td.path();
        write(feature_dir, &["T-001".into()]).unwrap();
        let other = feature_dir.join(".flow").join("other.txt");
        std::fs::write(&other, "keep me").unwrap();
        clear(feature_dir).unwrap();
        assert!(read(feature_dir).unwrap().is_none());
        assert!(other.is_file());
    }

    #[test]
    fn t001_read_returns_none_when_state_file_missing() {
        let td = TempDir::new().unwrap();
        assert!(read(td.path()).unwrap().is_none());
    }

    #[test]
    fn t005_overwrite_replaces_stale_pending_with_new_queue() {
        let td = TempDir::new().unwrap();
        let feature_dir = td.path();
        write(feature_dir, &["T-099".into()]).unwrap();
        write(feature_dir, &["T-001".into(), "T-002".into()]).unwrap();
        let read_back = read(feature_dir).unwrap().unwrap();
        assert_eq!(read_back, vec!["T-001".to_string(), "T-002".to_string()]);
    }
}
