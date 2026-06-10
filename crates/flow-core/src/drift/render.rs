//! User-facing consistency report rendering. Port of `render-consistency.py`.

use super::{Finding, Report, Severity};

/// Mode that shapes the rendered message.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    /// `/flow-status` — everything is advisory.
    Status,
    /// `/flow-test` — D1/D2/D3 block.
    Test,
    /// `/flow-close` — D1/D2/D3 block.
    Close,
    /// `/flow-plan` — D2/D3 block.
    Plan,
}

/// Render a consistency report as Markdown.
///
/// `next_command` is the bash-fenced command printed at the bottom.
/// `missing_tasks` is set when `tasks.md` does not yet exist.
#[must_use]
pub fn render(report: &Report, mode: Mode, next_command: &str, missing_tasks: bool) -> String {
    let findings = &report.findings;
    let has_error = report.has_error;
    let (what_happened, detail_heading, detail) = intro(mode, findings, has_error, missing_tasks);

    let mut lines: Vec<String> = vec!["## Consistency Check".into(), String::new()];
    lines.extend([
        "### What happened".into(),
        String::new(),
        what_happened,
        String::new(),
    ]);
    lines.extend([
        format!("### {detail_heading}"),
        String::new(),
        detail,
        String::new(),
    ]);
    lines.extend(["### What to fix".into(), String::new()]);

    if missing_tasks {
        lines.push(format!(
            "Run `{next_command}` to create `tasks.md`, then run the command again."
        ));
    } else if findings.is_empty() {
        lines.push("Nothing.".into());
    } else {
        for (i, f) in findings.iter().enumerate() {
            let severity = if f.severity == Severity::Error {
                "must fix"
            } else {
                "should fix"
            };
            lines.push(format!(
                "{n}. **{title}** ({severity}; developer detail: {id})",
                n = i + 1,
                title = f.title,
                id = f.id,
            ));
            lines.push(format!(
                "   - Why: {}",
                if f.cause.is_empty() {
                    &f.message
                } else {
                    &f.cause
                }
            ));
            if let Some(where_text) = where_text(f) {
                lines.push(format!("   - Where: {where_text}"));
            }
            lines.push(format!("   - Fix: {}", fix_text(f)));
        }
    }
    lines.push(String::new());

    lines.extend([
        "### Nothing was changed automatically".into(),
        String::new(),
    ]);
    if mode == Mode::Status {
        lines.push("This was only a status check; no planning files were changed.".into());
    } else if has_error || missing_tasks {
        lines.push(
            "Flow did not close verification, close the change, or change these planning files."
                .into(),
        );
    } else {
        lines.push("Flow did not change these planning files.".into());
    }
    lines.push(String::new());

    lines.extend([
        "### Next command".into(),
        String::new(),
        format!("`{next_command}`"),
        String::new(),
    ]);

    lines.join("\n")
}

fn where_text(f: &Finding) -> Option<String> {
    match (f.file.as_str(), f.line) {
        ("", _) => None,
        (file, Some(line)) => Some(format!("`{file}:{line}`")),
        (file, None) => Some(format!("`{file}`")),
    }
}

fn fix_text(f: &Finding) -> String {
    if f.fix_options.is_empty() {
        "Review the planning files and correct the out-of-sync reference.".into()
    } else if f.fix_options.len() == 1 {
        f.fix_options[0].clone()
    } else {
        f.fix_options.join("; or ")
    }
}

fn plural(count: usize, singular: &str) -> String {
    if count == 1 {
        format!("{count} {singular}")
    } else {
        format!("{count} {singular}s")
    }
}

fn intro(
    mode: Mode,
    findings: &[Finding],
    has_error: bool,
    missing_tasks: bool,
) -> (String, &'static str, String) {
    let count = findings.len();
    if missing_tasks {
        return (
            "Flow could not run the consistency check yet.".into(),
            "Why Flow stopped",
            "`tasks.md` does not exist yet. Flow needs it before it can compare the spec and task list.".into(),
        );
    }
    if count == 0 {
        return (
            "Flow checked the planning files and found no out-of-sync items.".into(),
            "Impact",
            "Flow did not stop for this check.".into(),
        );
    }

    if mode == Mode::Status {
        return (
            format!("Flow found {} that needs attention.", plural(count, "item")),
            "Impact",
            "`spec.md` and `tasks.md` no longer agree.".to_string(),
        );
    }

    if mode == Mode::Close {
        if !has_error {
            return (
                format!(
                    "Flow found {}.",
                    plural(count, "non-blocking consistency warning")
                ),
                "Impact",
                "These warnings do not block closing. Review them before closing so any remaining drift is intentional.".into(),
            );
        }
        return (
            "Flow stopped before closing.".into(),
            "Why Flow stopped",
            "Your spec and task list do not match. Closing now could archive work that misses a requirement or points to stale tasks.".into(),
        );
    }

    if has_error {
        return (
            "Flow stopped before closing verification.".into(),
            "Why Flow stopped",
            "Your spec and task list do not match. Closing now could mark the change ready while some requirement or task is missing.".into(),
        );
    }

    (
        format!(
            "Flow found {}.",
            plural(count, "non-blocking consistency warning")
        ),
        "Impact",
        "Flow did not stop for this check, but these references should still be cleaned up.".into(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_clean() {
        let report = Report {
            clean: true,
            ..Default::default()
        };
        let out = render(&report, Mode::Status, "/flow-status", false);
        assert!(out.contains("no out-of-sync items"));
    }

    #[test]
    fn renders_missing_tasks() {
        let r = Report::default();
        let out = render(&r, Mode::Plan, "/flow-plan", true);
        assert!(out.contains("Why Flow stopped"));
        assert!(out.contains("tasks.md"));
    }
}
