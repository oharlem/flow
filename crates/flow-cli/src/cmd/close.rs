//! Closeout finalization — finalize and document a completed change.

use crate::args::CloseArgs;
use chrono::Utc;
use flow_core::{
    config::Config,
    drift::{self, render::Mode, Severity},
    envelope, git, parse, paths, roadmap, status as status_helpers, Error, Result,
};
use std::collections::HashSet;
use std::path::Path;

/// Run the close command.
pub fn run(args: CloseArgs) -> Result<()> {
    let repo = paths::repo_root(None)?;
    match super::finalize_mode(args.finalize) {
        super::FinalizeMode::Infer => {
            let dir = super::amend::resolve_feature_dir(&repo)?;
            return finalize(&dir);
        }
        super::FinalizeMode::Skip => {}
    }
    prepare()
}

fn prepare() -> Result<()> {
    let repo = paths::repo_root(None)?;
    let feature_dir = super::amend::resolve_feature_dir(&repo)?;

    // Guard: tasks.md must show build-complete in the history. Matches the
    // closeout readiness gate used by older Flow drivers.
    if !status_helpers::history_contains(&feature_dir, "build-complete") {
        return Err(Error::User(format!(
            "Build verification is not complete. Run `{}` before `{}`.",
            crate::public_command::render_current("flow-test"),
            crate::public_command::render_current("flow-close")
        )));
    }
    super::task_state::ensure_all_accepted(
        &feature_dir,
        &crate::public_command::render_current("flow-close"),
    )?;

    // Pre-flight drift check (D1/D2/D3 are error; advisory checks stay warn).
    let findings = drift::check_artifacts(&feature_dir, Some(&repo))?;
    let promote: HashSet<String> = ["D1", "D2", "D3"]
        .iter()
        .map(|s| (*s).to_string())
        .collect();
    let findings = drift::promote_severity(findings, &promote);
    let report = drift::build_report(findings);
    let has_error = report.has_error;
    let findings_count = report.findings.len();

    if !report.findings.is_empty() {
        let next = if has_error {
            crate::public_command::render_current("flow-test")
        } else {
            "git status".to_string()
        };
        println!(
            "{}",
            drift::render::render(&report, Mode::Close, &next, false)
        );
    }
    if has_error {
        return Err(Error::DriftErrors {
            errors: report
                .findings
                .iter()
                .filter(|f| matches!(f.severity, Severity::Error))
                .count(),
            warns: report
                .findings
                .iter()
                .filter(|f| matches!(f.severity, Severity::Warn))
                .count(),
        });
    }

    // T-009: touch-map warnings (advisory, non-blocking — close proceeds regardless).
    let config = Config::load_for_repo(&repo).unwrap_or_default();
    for warn in crate::cmd::doctor::check_touch_map(&repo, &config.git, &config.docs) {
        println!("[warning] {warn}");
    }

    let consistency_summary = if findings_count > 0 {
        let word = if findings_count == 1 {
            "warning"
        } else {
            "warnings"
        };
        format!(
            "All tasks are complete and all must-fix consistency checks passed. Flow found {findings_count} non-blocking consistency {word}; closing is allowed, but review them before closing unless you intentionally accept the advisory findings."
        )
    } else {
        "All tasks are complete and the consistency check is clean.".to_string()
    };
    let extra = format!(
        "## Ready to close\n\n{consistency_summary}\n\nBefore finalizing, ensure current documentation under the configured Flow docs directory is updated, or that `plan.md` has `## Documentation Impact` with `Impact: none` and a docs-current rationale.\n\nFlow will stamp the change as closed in place and update the linked roadmap milestone. Flow will not bump application versions, create commits, create tags, merge branches, or push.\n"
    );

    let out = envelope::compose(&repo, "close", &feature_dir, Some(&extra))?;
    print!("{out}");
    crate::output::maybe_print_finalize_hint("close", &feature_dir);
    Ok(())
}

fn finalize(feature_dir: &Path) -> Result<()> {
    let repo = paths::repo_root(Some(feature_dir)).unwrap_or_else(|_| feature_dir.to_path_buf());
    let today = Utc::now().format("%Y-%m-%d").to_string();
    let feature_name = feature_dir
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();

    // Guard: tasks.md must show build-complete in the history before closeout.
    if !status_helpers::history_contains(feature_dir, "build-complete") {
        return Err(Error::User(format!(
            "Build verification is not complete. Run `{}` before `{}`.",
            crate::public_command::render_current("flow-test"),
            crate::public_command::render_current("flow-close")
        )));
    }
    super::task_state::ensure_all_accepted(
        feature_dir,
        &crate::public_command::render_current("flow-close"),
    )?;

    // Pre-flight (block on errors only).
    let findings = drift::check_artifacts(feature_dir, Some(&repo))?;
    let promote: HashSet<String> = ["D1", "D2", "D3"]
        .iter()
        .map(|s| (*s).to_string())
        .collect();
    let findings = drift::promote_severity(findings, &promote);
    let report = drift::build_report(findings);
    if report.has_error {
        return Err(Error::DriftErrors {
            errors: report
                .findings
                .iter()
                .filter(|f| matches!(f.severity, Severity::Error))
                .count(),
            warns: report
                .findings
                .iter()
                .filter(|f| matches!(f.severity, Severity::Warn))
                .count(),
        });
    }

    let already_closed = close_marked_closed(feature_dir);
    ensure_documentation_evidence(&repo, feature_dir)?;

    if !already_closed {
        // Tick milestones.
        let feature_link = flow_relative_link(&repo, feature_dir);
        if let Some(roadmap_path) = crate::cmd::run::roadmap_path_for_change(&repo, feature_dir)? {
            let _ = roadmap::tick_milestones_at_path(
                feature_dir,
                &roadmap_path,
                &feature_link,
                &today,
            )?;
        }

        stamp_closed_header(feature_dir, &today)?;
        parse::status::stamp(
            feature_dir,
            Some(parse::status::State::Closed),
            "closed",
            "change closed",
        )?;
    }
    let closed_milestone = parse::status::parse_file(&feature_dir.join("status.md"))
        .ok()
        .and_then(|status| status.milestones.first().cloned());
    crate::cmd::run::update_run_state_after_close(&repo, feature_dir, closed_milestone.as_deref())?;

    // Closed changes keep only durable artifacts. The consistency cache and
    // build-pending state are intra-run scratch; nothing reads them after
    // close, so drop them instead of leaving noise in the change directory.
    status_helpers::remove_cache(feature_dir)?;
    super::build_pending::clear(feature_dir)?;

    println!();
    println!("Change '{feature_name}' closed.");
    println!("   Change path:     {}", rel_path(&repo, feature_dir));
    crate::output::print_next(
        "git status",
        "review closeout changes before committing through your normal workflow.",
    );
    Ok(())
}

fn close_marked_closed(feature_dir: &Path) -> bool {
    let status = parse::status::parse_file(&feature_dir.join("status.md")).ok();
    matches!(
        status.as_ref().and_then(|status| status.state),
        Some(parse::status::State::Closed)
    ) && status_helpers::history_contains(feature_dir, "closed")
}

fn flow_relative_link(repo: &Path, feature_dir: &Path) -> String {
    let flow_root = paths::work_dir(repo);
    let rel = feature_dir
        .strip_prefix(&flow_root)
        .unwrap_or(feature_dir)
        .display()
        .to_string()
        .replace('\\', "/");
    format!("./{rel}/")
}

fn stamp_closed_header(feature_dir: &Path, today: &str) -> Result<()> {
    let spec_path = feature_dir.join("spec.md");
    let text = std::fs::read_to_string(&spec_path)?;
    if text.contains("**Closed**:") {
        return Ok(());
    }
    // Insert after `**Created**:` when present, otherwise after the title block.
    let marker = "**Created**:";
    let new = if let Some(idx) = text.find(marker) {
        let after = &text[idx..];
        let line_end = after.find('\n').map_or(text.len(), |n| idx + n);
        let (head, tail) = text.split_at(line_end);
        format!("{head}\n**Closed**: {today}{tail}")
    } else if let Some(idx) = text.find("\n---\n") {
        let (head, tail) = text.split_at(idx);
        format!("{head}\n**Closed**: {today}{tail}")
    } else {
        format!("**Closed**: {today}\n\n{text}")
    };
    std::fs::write(&spec_path, new)?;
    Ok(())
}

fn ensure_documentation_evidence(repo: &Path, feature_dir: &Path) -> Result<()> {
    let docs_dir = paths::documentation_dir(repo);
    if documentation_changed(repo, &docs_dir) {
        return Ok(());
    }
    if plan_declares_docs_current(feature_dir)? {
        return Ok(());
    }
    Err(Error::User(format!(
        "Central documentation evidence is missing. Update files under {}, or add `## Documentation Impact` to plan.md with `Impact: none` and a docs-current rationale.",
        docs_dir.display()
    )))
}

fn documentation_changed(repo: &Path, docs_dir: &Path) -> bool {
    let docs_rel = rel_path(repo, docs_dir);
    changed_paths(repo)
        .into_iter()
        .any(|path| path == docs_rel || path.starts_with(&format!("{docs_rel}/")))
}

fn changed_paths(repo: &Path) -> Vec<String> {
    let mut out = Vec::new();
    let cfg = Config::load_for_repo(repo).unwrap_or_default();
    if let Some(default_branch) = crate::cmd::doctor::resolve_default_branch(repo, &cfg.git) {
        if let Ok(base) = git::merge_base(repo, "HEAD", &default_branch) {
            if let Ok(files) = git::diff_files(repo, &base) {
                out.extend(
                    files
                        .into_iter()
                        .map(|p| p.to_string_lossy().replace('\\', "/")),
                );
            }
        }
    }
    if let Ok(status) = std::process::Command::new("git")
        .args(["status", "--porcelain", "--untracked-files=all"])
        .current_dir(repo)
        .output()
    {
        out.extend(parse_porcelain_paths(&String::from_utf8_lossy(
            &status.stdout,
        )));
    }
    out.sort();
    out.dedup();
    out
}

fn parse_porcelain_paths(text: &str) -> Vec<String> {
    text.lines()
        .filter_map(|line| {
            let path = line.get(3..)?.trim();
            let path = path.rsplit_once(" -> ").map_or(path, |(_, new)| new);
            Some(path.trim_matches('"').replace('\\', "/"))
        })
        .filter(|path| !path.is_empty())
        .collect()
}

fn rel_path(repo: &Path, path: &Path) -> String {
    path.strip_prefix(repo)
        .unwrap_or(path)
        .display()
        .to_string()
        .replace('\\', "/")
}

fn plan_declares_docs_current(feature_dir: &Path) -> Result<bool> {
    let plan = parse::plan::parse_file(&feature_dir.join("plan.md"))?;
    if !plan.declares_no_documentation_impact() {
        return Ok(false);
    }
    let Some(section) = plan.sections.get("Documentation Impact") else {
        return Ok(false);
    };
    let lower = section.to_lowercase();
    if !lower.contains("docs already current") {
        return Ok(false);
    }
    let rationale = lower
        .replace("docs already current", "")
        .replace("because", "")
        .replace("rationale", "")
        .replace("reason", "");
    Ok(rationale.chars().filter(|ch| ch.is_alphanumeric()).count() >= 10)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    // T-002: docs-current rationale is accepted when no-impact intent is explicit.
    #[test]
    fn t004_plan_docs_current_rationale_is_evidence() {
        let td = tempdir().unwrap();
        let feature_dir = td.path().join("feat");
        std::fs::create_dir_all(&feature_dir).unwrap();
        std::fs::write(
            feature_dir.join("plan.md"),
            "## Summary\n\nReady.\n\n## Technical Context\n\nRust.\n\n## Documentation Impact\n\nImpact: none\n\nDocs already current because this change only affects internal path resolution.\n",
        )
        .unwrap();
        assert!(plan_declares_docs_current(&feature_dir).unwrap());
    }

    // T-002: docs-current rationale alone is not enough to bypass docs evidence.
    #[test]
    fn t004_plan_docs_current_requires_no_impact_declaration() {
        let td = tempdir().unwrap();
        let feature_dir = td.path().join("feat");
        std::fs::create_dir_all(&feature_dir).unwrap();
        std::fs::write(
            feature_dir.join("plan.md"),
            "## Summary\n\nReady.\n\n## Technical Context\n\nRust.\n\n## Documentation Impact\n\nDocs already current because this change only affects internal path resolution.\n",
        )
        .unwrap();
        assert!(!plan_declares_docs_current(&feature_dir).unwrap());
    }

    // T-004: docs-current without rationale is not accepted.
    #[test]
    fn t004_plan_docs_current_requires_rationale() {
        let td = tempdir().unwrap();
        let feature_dir = td.path().join("feat");
        std::fs::create_dir_all(&feature_dir).unwrap();
        std::fs::write(
            feature_dir.join("plan.md"),
            "## Summary\n\nReady.\n\n## Technical Context\n\nRust.\n\n## Documentation Impact\n\nDocs already current.\n",
        )
        .unwrap();
        assert!(!plan_declares_docs_current(&feature_dir).unwrap());
    }

    // T-002: impact declarations keep the close gate strict even with a rationale.
    #[test]
    fn t004_plan_docs_current_rejects_declared_docs_impact() {
        let td = tempdir().unwrap();
        let feature_dir = td.path().join("feat");
        std::fs::create_dir_all(&feature_dir).unwrap();
        std::fs::write(
            feature_dir.join("plan.md"),
            "## Summary\n\nReady.\n\n## Technical Context\n\nRust.\n\n## Documentation Impact\n\nImpact: update\n\nDocs already current because this change only affects internal path resolution.\n",
        )
        .unwrap();
        assert!(!plan_declares_docs_current(&feature_dir).unwrap());
    }

    // T-004: git porcelain paths include renamed and untracked docs paths.
    #[test]
    fn t004_parse_porcelain_paths() {
        let paths = parse_porcelain_paths(
            " M flow/docs/guide.md\n?? flow/docs/new.md\nR  old.md -> flow/docs/moved.md\n",
        );
        assert_eq!(
            paths,
            vec![
                "flow/docs/guide.md",
                "flow/docs/new.md",
                "flow/docs/moved.md"
            ]
        );
    }
}
