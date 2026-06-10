use flow_core::{parse, Error, Result};
use once_cell::sync::Lazy;
use regex::Regex;
use std::collections::{HashMap, HashSet};
use std::path::Path;

static TASK_LINE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r"^(?P<prefix>\s*-\s+\[)(?P<mark>[ xX~])(?P<middle>\]\s+(?:\*\*)?)(?P<id>T-[A-Z]?\d{1,4})(?P<suffix>(?:\*\*)?[:\s].*)$",
    )
    .unwrap()
});

pub(crate) fn mark_done(feature_dir: &Path, ids: &[String]) -> Result<Vec<String>> {
    let tasks_file = feature_dir.join("tasks.md");
    let text = std::fs::read_to_string(&tasks_file).map_err(|_| Error::FileNotFound {
        kind: "tasks.md".into(),
        path: tasks_file.clone(),
    })?;
    let wanted = normalize_ids(ids);
    if wanted.is_empty() {
        return Ok(Vec::new());
    }

    let mut found: HashMap<String, String> = HashMap::new();
    let mut out = String::with_capacity(text.len());
    let mut changed = false;

    for line in text.split_inclusive('\n') {
        let newline = if line.ends_with("\r\n") {
            "\r\n"
        } else if line.ends_with('\n') {
            "\n"
        } else {
            ""
        };
        let body = line.trim_end_matches(['\n', '\r']);

        if let Some(caps) = TASK_LINE.captures(body) {
            let id = caps.name("id").unwrap().as_str();
            let normalized = normalize_id(id);
            if wanted.contains(&normalized) {
                found.insert(normalized, id.to_string());
                if !caps
                    .name("mark")
                    .unwrap()
                    .as_str()
                    .eq_ignore_ascii_case("x")
                {
                    out.push_str(caps.name("prefix").unwrap().as_str());
                    out.push('x');
                    out.push_str(caps.name("middle").unwrap().as_str());
                    out.push_str(id);
                    out.push_str(caps.name("suffix").unwrap().as_str());
                    out.push_str(newline);
                    changed = true;
                    continue;
                }
            }
        }

        out.push_str(line);
    }

    let missing = wanted
        .iter()
        .find(|id| !found.contains_key(*id))
        .map(String::as_str);
    if let Some(id) = missing {
        return Err(Error::ArtifactError {
            file: "tasks.md".into(),
            message: format!("task '{id}' was not found"),
        });
    }

    if changed {
        let tmp = tasks_file.with_extension(format!("md.tmp.{}", std::process::id()));
        std::fs::write(&tmp, out)?;
        std::fs::rename(&tmp, &tasks_file)?;
    }

    Ok(ids
        .iter()
        .map(|id| found[&normalize_id(id)].clone())
        .collect())
}

pub(crate) fn ensure_all_accepted(feature_dir: &Path, next_command: &str) -> Result<()> {
    let tasks_file = feature_dir.join("tasks.md");
    if !tasks_file.exists() {
        return Ok(());
    }
    let tasks = parse::tasks::parse_file(&tasks_file)?;
    if let Some(task) = tasks.iter().find(|t| !t.done) {
        let message = if task.state.is_awaiting_acceptance() {
            format!(
                "Task {} is awaiting user acceptance. Save Flow state before running `{next_command}`.",
                task.id
            )
        } else {
            format!(
                "Task {} is not complete. Run `/flow-build-task` before `{next_command}`.",
                task.id
            )
        };
        return Err(Error::User(message));
    }
    Ok(())
}

pub(crate) fn normalize_id(id: &str) -> String {
    id.trim().to_uppercase()
}

fn normalize_ids(ids: &[String]) -> HashSet<String> {
    ids.iter()
        .map(|id| normalize_id(id))
        .filter(|id| !id.is_empty())
        .collect()
}
