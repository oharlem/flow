//! `tasks.md` parser.

use crate::error::{Error, Result};
use once_cell::sync::Lazy;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

/// Flow-owned task checkbox state.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TaskState {
    /// `- [ ]` — not implemented or not ready for acceptance.
    #[default]
    Open,
    /// `- [~]` — implemented locally and awaiting user acceptance.
    AwaitingAcceptance,
    /// `- [x]` — accepted and saved into Flow state.
    Done,
}

impl TaskState {
    /// Parse the single-character marker inside a task checkbox.
    #[must_use]
    pub fn from_marker(marker: &str) -> Self {
        match marker {
            "x" | "X" => Self::Done,
            "~" => Self::AwaitingAcceptance,
            _ => Self::Open,
        }
    }

    /// Return true when the task is accepted and saved into Flow state.
    #[must_use]
    pub fn is_done(self) -> bool {
        matches!(self, Self::Done)
    }

    /// Return true when the task is not started or needs more work.
    #[must_use]
    pub fn is_open(self) -> bool {
        matches!(self, Self::Open)
    }

    /// Return true when the task is implemented but not accepted.
    #[must_use]
    pub fn is_awaiting_acceptance(self) -> bool {
        matches!(self, Self::AwaitingAcceptance)
    }
}

/// Parsed task bullet from `tasks.md`.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Task {
    /// Task ID text (e.g. `"T-001"`).
    pub id: String,
    /// 1-based line number of the first line of the bullet.
    pub line_number: usize,
    /// First line of the bullet (trimmed, ≤ 120 chars).
    pub summary: String,
    /// Flow-owned checkbox state.
    pub state: TaskState,
    /// Whether the bullet is checked (`[x]`).
    pub done: bool,
    /// `Covers: FR-…` IDs.
    pub covers: Vec<String>,
    /// `Verifies: SC-…` IDs.
    pub verifies: Vec<String>,
    /// `Depends-On: T-…` IDs.
    pub depends_on: Vec<String>,
    /// `Requires: <preflight requirement IDs>`.
    pub requires: Vec<String>,
}

static TASK_BULLET: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^\s*-\s+\[(?P<mark>[ xX~])\]\s+(?:\*\*)?(?P<id>T-[A-Z]?\d{1,4})(?:\*\*)?[:\s]")
        .unwrap()
});
static COVERS_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?i)^\s*[-*]?\s*Covers:\s*(.*)").unwrap());
static VERIFIES_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?i)^\s*[-*]?\s*Verifies:\s*(.*)").unwrap());
static DEPENDS_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?i)^\s*[-*]?\s*Depends-On:\s*(.*)").unwrap());
static REQUIRES_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?i)^\s*[-*]?\s*Requires:\s*(.*)").unwrap());
static CONTINUATION: Lazy<Regex> = Lazy::new(|| Regex::new(r"^\s{2,}").unwrap());

/// Parse `tasks.md` from disk.
pub fn parse_file(path: &Path) -> Result<Vec<Task>> {
    let text = std::fs::read_to_string(path).map_err(|_| Error::FileNotFound {
        kind: "tasks.md".into(),
        path: path.to_path_buf(),
    })?;
    Ok(parse_str(&text))
}

/// Parse `tasks.md` content directly.
#[must_use]
pub fn parse_str(text: &str) -> Vec<Task> {
    let mut tasks = Vec::new();
    let mut current_lines: Vec<String> = Vec::new();
    let mut current_start = 0usize;

    for (idx, line) in text.lines().enumerate() {
        let lineno = idx + 1;
        let is_task = TASK_BULLET.is_match(line);
        let is_continuation = CONTINUATION.is_match(line);

        if is_task {
            if let Some(t) = finalize(current_start, &current_lines) {
                tasks.push(t);
            }
            current_lines = vec![line.to_string()];
            current_start = lineno;
        } else if is_continuation && !current_lines.is_empty() {
            current_lines.push(line.to_string());
        } else {
            if let Some(t) = finalize(current_start, &current_lines) {
                tasks.push(t);
            }
            current_lines.clear();
        }
    }
    if let Some(t) = finalize(current_start, &current_lines) {
        tasks.push(t);
    }
    tasks
}

fn finalize(start: usize, lines: &[String]) -> Option<Task> {
    let first = lines.first()?;
    let caps = TASK_BULLET.captures(first)?;
    let id = caps.name("id")?.as_str().to_string();
    let state = TaskState::from_marker(caps.name("mark")?.as_str());
    let done = state.is_done();
    let summary = first.trim().chars().take(120).collect::<String>();

    let mut task = Task {
        id,
        line_number: start,
        summary,
        state,
        done,
        ..Default::default()
    };

    for line in lines {
        if let Some(c) = COVERS_RE.captures(line) {
            task.covers = split_ids(&c[1]);
        } else if let Some(c) = VERIFIES_RE.captures(line) {
            task.verifies = split_ids(&c[1]);
        } else if let Some(c) = DEPENDS_RE.captures(line) {
            task.depends_on = split_ids(&c[1]);
        } else if let Some(c) = REQUIRES_RE.captures(line) {
            task.requires = split_requirements(&c[1]);
        }
    }

    Some(task)
}

fn split_ids(list: &str) -> Vec<String> {
    list.split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty() && !s.eq_ignore_ascii_case("(none)"))
        .collect()
}

/// Map each task ID (uppercase) to whether it is accepted (`[x]`).
#[must_use]
pub fn acceptance_map(tasks: &[Task]) -> HashMap<String, bool> {
    let mut m = HashMap::new();
    for t in tasks {
        m.insert(t.id.to_uppercase(), t.done);
    }
    m
}

fn normalize_dep_id(raw: &str) -> Option<String> {
    let s = raw.trim();
    if s.is_empty() || s.eq_ignore_ascii_case("(none)") {
        return None;
    }
    Some(s.to_uppercase())
}

/// True when every `Depends-On` entry names an accepted (`[x]`) task.
#[must_use]
pub fn dependencies_satisfied(task: &Task, accepted: &HashMap<String, bool>) -> bool {
    for raw in &task.depends_on {
        let Some(dep) = normalize_dep_id(raw) else {
            continue;
        };
        if !accepted.get(&dep).copied().unwrap_or(false) {
            return false;
        }
    }
    true
}

/// First open task in file order whose dependencies are all accepted.
#[must_use]
pub fn first_runnable_open_task(tasks: &[Task]) -> Option<&Task> {
    let accepted = acceptance_map(tasks);
    tasks
        .iter()
        .find(|t| t.state.is_open() && dependencies_satisfied(t, &accepted))
}

/// Up to `limit` open tasks in runnable order (deps satisfied; simulates
/// completing each queued task when finding the next runnable).
#[must_use]
pub fn runnable_open_task_queue(tasks: &[Task], limit: usize) -> Vec<&Task> {
    if limit == 0 {
        return Vec::new();
    }
    let mut out: Vec<&Task> = Vec::new();
    let mut accepted = acceptance_map(tasks);
    while out.len() < limit {
        let mut picked: Option<&Task> = None;
        for t in tasks {
            if !t.state.is_open() {
                continue;
            }
            if out.iter().any(|u| u.id == t.id) {
                continue;
            }
            if dependencies_satisfied(t, &accepted) {
                picked = Some(t);
                break;
            }
        }
        let Some(t) = picked else {
            break;
        };
        out.push(t);
        accepted.insert(t.id.to_uppercase(), true);
    }
    out
}

fn split_requirements(list: &str) -> Vec<String> {
    list.split(',')
        .map(|s| s.trim().to_ascii_lowercase())
        .filter(|s| !s.is_empty() && !s.eq_ignore_ascii_case("(none)"))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_done_task() {
        let text = "- [x] **T-001**: first task\n    - Covers: FR-1, FR-2\n    - Verifies: SC-001\n    - Depends-On: (none)\n";
        let tasks = parse_str(text);
        assert_eq!(tasks.len(), 1);
        assert!(tasks[0].done);
        assert_eq!(tasks[0].covers, vec!["FR-1", "FR-2"]);
        assert_eq!(tasks[0].verifies, vec!["SC-001"]);
        assert!(tasks[0].depends_on.is_empty());
        assert!(tasks[0].requires.is_empty());
    }

    #[test]
    fn parses_multiple_tasks() {
        let text = "- [ ] **T-001**: a\n- [~] **T-002**: b\n";
        let tasks = parse_str(text);
        assert_eq!(tasks.len(), 2);
        assert_eq!(tasks[0].state, TaskState::Open);
        assert_eq!(tasks[1].state, TaskState::AwaitingAcceptance);
        assert!(!tasks[1].done);
    }

    #[test]
    fn parses_preflight_requirements() {
        let text = "- [ ] **T-001**: x\n    - Requires: Docker, postgres\n";
        let tasks = parse_str(text);
        assert_eq!(tasks[0].requires, vec!["docker", "postgres"]);
    }

    #[test]
    fn runnable_queue_respects_depends_on() {
        let text = "- [ ] **T-001**: first\n    - Depends-On: (none)\n\n- [ ] **T-002**: second\n    - Depends-On: T-001\n";
        let tasks = parse_str(text);
        let q = runnable_open_task_queue(&tasks, 20);
        assert_eq!(q.len(), 2);
        assert_eq!(q[0].id, "T-001");
        assert_eq!(q[1].id, "T-002");
        let q2 = runnable_open_task_queue(&tasks, 1);
        assert_eq!(q2.len(), 1);
        assert_eq!(q2[0].id, "T-001");
    }

    #[test]
    fn first_runnable_skips_blocked_task() {
        let text = "- [ ] **T-002**: second\n    - Depends-On: T-001\n\n- [ ] **T-001**: first\n    - Depends-On: (none)\n";
        let tasks = parse_str(text);
        assert_eq!(first_runnable_open_task(&tasks).unwrap().id, "T-001");
    }

    #[test]
    fn no_runnable_task_when_all_open_tasks_are_blocked() {
        let text = "- [ ] **T-002**: second\n    - Depends-On: T-001\n";
        let tasks = parse_str(text);
        assert!(first_runnable_open_task(&tasks).is_none());
        assert!(runnable_open_task_queue(&tasks, 20).is_empty());
    }
}
