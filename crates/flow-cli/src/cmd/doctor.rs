//! `flow doctor` — sanity check the local Flow installation.

use crate::args::DoctorArgs;
use crate::generated_docs;
use flow_core::{
    assets,
    config::{Config, DocsConfig, GitConfig},
    git, paths, Result,
};
use std::path::Path;

/// Run `flow doctor`.
pub fn run(_args: DoctorArgs) -> Result<()> {
    let repo = paths::repo_root(None)?;
    let flow_dir = paths::flow_dir(&repo);
    let config = Config::load_for_repo(&repo)?;
    let layout = paths::layout(&repo);
    let installed_version = crate::cmd::version_marker::read(&flow_dir);

    let mut ok = true;
    let mut checks = vec![
        (".flow/".to_string(), flow_dir.is_dir()),
        (".flow/version".to_string(), installed_version.is_some()),
        (
            ".flow/agents/".to_string(),
            flow_dir.join("agents").is_dir(),
        ),
        (
            "embedded conventions".to_string(),
            embedded_conventions_available(),
        ),
        (
            "embedded base prompts".to_string(),
            embedded_base_prompts_available(),
        ),
        ("AGENTS.md".to_string(), repo.join("AGENTS.md").is_file()),
    ];
    checks.push((
        rel(&repo, &layout.workspace_dir),
        layout.workspace_dir.is_dir(),
    ));
    checks.push((rel(&repo, &layout.runs_dir), layout.runs_dir.is_dir()));
    checks.push((
        rel(&repo, &layout.documentation_dir),
        layout.documentation_dir.is_dir(),
    ));
    println!("Flow doctor — repo: {}", repo.display());
    for (name, present) in &checks {
        let mark = if *present { "ok" } else { "MISSING" };
        println!("  [{mark:>7}] {name}");
        if !present {
            ok = false;
        }
    }
    println!(
        "  [     ok] Embedded asset version: {}",
        assets::CONVENTIONS_VERSION
    );
    if let Some(version) = installed_version.as_deref() {
        println!("\nInstalled Flow version: {version}");
        let current = crate::cmd::version_marker::CURRENT_VERSION;
        if version != current {
            use crate::cmd::version_marker::VersionDelta;
            match crate::cmd::version_marker::classify(Some(version), current) {
                VersionDelta::Downgrade => println!(
                    "  warning: running Flow binary is {current} (older than recorded {version}). \
                     Reinstall Flow (e.g. `cargo install --git https://github.com/oharlem/flow --locked --force flow-cli` or `make up`) to recover, \
                     or run `flow update --force` to accept the downgrade."
                ),
                // Includes the rare case where strings differ but the
                // triplet classifier collapses them (e.g. pre-release suffixes).
                VersionDelta::Upgrade
                | VersionDelta::Same
                | VersionDelta::FirstInstall => println!(
                    "  warning: running Flow binary is {current} (newer than recorded {version}); run `flow update` to refresh this repo."
                ),
            }
        }
    }

    check_generated_doc_drift(&repo);

    // T-005: review-marker warnings (advisory, non-blocking).
    for warn in check_review_markers(&repo, &config.docs) {
        println!("[warning] {warn}");
    }

    // T-008: touch-map warnings (advisory, non-blocking).
    for warn in check_touch_map(&repo, &config.git, &config.docs) {
        println!("[warning] {warn}");
    }

    // M-25 (T-002): Claude Code permissions advisory. Read-only — Flow
    // never writes to `.claude/settings.json` or `.claude/settings.local.json`.
    print_claude_code_permissions_advisory(&repo);

    if ok {
        println!("\nFlow is installed. Use `flow roadmap <source>` to plan a roadmap run, or `flow start` to draft a one-off change.");
        println!("{}.", super::init::EMBEDDED_DEFAULTS_HINT);
        Ok(())
    } else {
        Err(flow_core::Error::User(
            "Flow appears incomplete. Run `flow init` (or `flow setup`) to repair.".into(),
        ))
    }
}

fn embedded_conventions_available() -> bool {
    assets::CONVENTIONS_SHARD_NAMES
        .iter()
        .all(|name| assets::conventions_shard(name).is_some_and(|body| !body.is_empty()))
}

fn embedded_base_prompts_available() -> bool {
    assets::PHASES
        .iter()
        .all(|phase| assets::agent_base(phase).is_some_and(|body| !body.is_empty()))
}

fn rel(repo: &Path, path: &Path) -> String {
    path.strip_prefix(repo)
        .unwrap_or(path)
        .display()
        .to_string()
        .replace('\\', "/")
}

/// M-25 (T-002): print the Claude Code permissions advisory block when the
/// active host is `claude-code` and the user's allow list looks minimal.
/// The block is text only; Flow never writes `.claude/settings*.json`.
fn print_claude_code_permissions_advisory(repo: &Path) {
    use crate::public_command::Host;
    if !matches!(crate::public_command::active_host(), Some(Host::ClaudeCode)) {
        return;
    }
    let state = flow_host_claude_code::read_user_permissions_state(repo);
    match flow_host_claude_code::advisory_block(&state) {
        flow_host_claude_code::AdvisoryBlock::Suppressed => {}
        flow_host_claude_code::AdvisoryBlock::Print(body) => {
            println!();
            println!("{body}");
        }
    }
}

/// Warn (never fail) when Flow-owned generated docs are out of date. The check
/// uses only local state — no network, no git operations.
fn check_generated_doc_drift(repo: &Path) {
    let stale = generated_docs::stale_paths(repo);
    if stale.is_empty() {
        return;
    }
    println!("\nwarning: generated docs are stale: {}", stale.join(", "));
    println!("Run `flow update` to refresh Flow-owned generated docs.");
}

/// Scan docs directories for pages missing or stale `**Reviewed**: YYYY-MM-DD`
/// markers. Returns advisory warning strings. (T-004)
///
/// Monitored: `docs/start-here/`, `docs/how-to/`, `docs/explanation/`,
/// `docs/features/`. Exempt: `docs/reference/`, `docs/decisions/`, generated
/// files. Missing directories are silently skipped.
fn check_review_markers(repo: &Path, docs: &DocsConfig) -> Vec<String> {
    let monitored = ["start-here", "how-to", "explanation", "features"];
    let mut warnings = Vec::new();

    for dir_name in &monitored {
        let dir = repo.join("docs").join(dir_name);
        if !dir.is_dir() {
            continue;
        }
        let entries = match std::fs::read_dir(&dir) {
            Ok(e) => e,
            Err(_) => continue,
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("md") {
                continue;
            }
            let content = match std::fs::read_to_string(&path) {
                Ok(s) => s,
                Err(_) => continue,
            };
            let rel = path
                .strip_prefix(repo)
                .unwrap_or(&path)
                .display()
                .to_string()
                .replace('\\', "/");
            match find_review_date(&content) {
                None => {
                    warnings.push(format!(
                        "docs freshness: {rel} lacks **Reviewed**: YYYY-MM-DD marker"
                    ));
                }
                Some(date_str) => {
                    if let Some(max_age) = docs.review_max_age_days {
                        if is_stale(&date_str, max_age) {
                            warnings.push(format!(
                                "docs freshness: {rel} review marker ({date_str}) is older than {max_age} days"
                            ));
                        }
                    }
                }
            }
        }
    }
    warnings
}

/// Extract the `YYYY-MM-DD` date from a `**Reviewed**: YYYY-MM-DD` line, if present.
fn find_review_date(content: &str) -> Option<String> {
    for line in content.lines() {
        if let Some(rest) = line.trim().strip_prefix("**Reviewed**:") {
            let date = rest.trim().to_string();
            if !date.is_empty() {
                return Some(date);
            }
        }
    }
    None
}

/// Return `true` when the ISO date string is older than `max_age_days` days.
/// Returns `false` for unparseable dates (don't warn on malformed markers).
fn is_stale(date_str: &str, max_age_days: u64) -> bool {
    use chrono::{NaiveDate, Utc};
    let Ok(review_date) = NaiveDate::parse_from_str(date_str, "%Y-%m-%d") else {
        return false;
    };
    let today = Utc::now().date_naive();
    today.signed_duration_since(review_date).num_days() > max_age_days as i64
}

/// Resolve the local default branch for touch-map diff computation. (T-007)
///
/// Resolution order: (1) `git.default_branch` from config; (2) `"main"` if
/// that branch exists locally; (3) `"master"` if that branch exists locally;
/// (4) `None` — caller emits a note and skips the check.
pub(crate) fn resolve_default_branch(repo: &Path, git_cfg: &GitConfig) -> Option<String> {
    if let Some(b) = &git_cfg.default_branch {
        return Some(b.clone());
    }
    if git::branch_exists(repo, "main").unwrap_or(false) {
        return Some("main".to_string());
    }
    if git::branch_exists(repo, "master").unwrap_or(false) {
        return Some("master".to_string());
    }
    None
}

/// Evaluate `docs.touch_map` entries against the current branch diff and
/// return advisory warning strings. (T-007)
///
/// All diff operations are local-only. No network access. Returns an empty vec
/// when `touch_map` is empty, `touch_map_warnings` is false, or the default
/// branch cannot be resolved.
pub(crate) fn check_touch_map(
    repo: &Path,
    git_cfg: &GitConfig,
    docs_cfg: &DocsConfig,
) -> Vec<String> {
    if !docs_cfg.touch_map_warnings || docs_cfg.touch_map.is_empty() {
        return Vec::new();
    }

    let default_branch = match resolve_default_branch(repo, git_cfg) {
        Some(b) => b,
        None => {
            println!(
                "[note] docs touch-map: could not resolve a local default branch; skipping touch-map check"
            );
            return Vec::new();
        }
    };

    let base = match git::merge_base(repo, "HEAD", &default_branch) {
        Ok(sha) => sha,
        Err(_) => {
            println!(
                "[note] docs touch-map: git merge-base HEAD {default_branch} failed; skipping touch-map check"
            );
            return Vec::new();
        }
    };

    let changed = match git::diff_files(repo, &base) {
        Ok(files) => files,
        Err(_) => return Vec::new(),
    };

    let changed_strs: Vec<String> = changed
        .iter()
        .map(|p| p.to_string_lossy().replace('\\', "/"))
        .collect();

    let mut warnings = Vec::new();

    for entry in &docs_cfg.touch_map {
        if entry.suppress {
            continue;
        }

        // Find changed files that match any source-path glob in this entry.
        let triggering: Vec<&str> = changed_strs
            .iter()
            .filter(|path| {
                entry.paths.iter().any(|pat| {
                    glob::Pattern::new(pat)
                        .map(|p| p.matches(path))
                        .unwrap_or(false)
                })
            })
            .map(String::as_str)
            .collect();

        if triggering.is_empty() {
            continue;
        }

        // Check whether any of the mapped docs appear in the diff.
        let docs_touched = entry.docs.iter().any(|doc| {
            let norm = doc.replace('\\', "/");
            changed_strs.iter().any(|c| c == &norm)
        });

        if docs_touched {
            continue;
        }

        // Warn about mapped doc paths that don't exist on disk (FR-007 edge case).
        for doc in &entry.docs {
            if !repo.join(doc).exists() {
                warnings.push(format!(
                    "docs touch-map: mapped doc path '{doc}' does not exist on disk"
                ));
            }
        }

        // Main advisory warning.
        let shown: Vec<&str> = triggering.iter().take(3).copied().collect();
        let suffix = if triggering.len() > 3 {
            format!(" (+{})", triggering.len() - 3)
        } else {
            String::new()
        };
        warnings.push(format!(
            "docs touch-map: {} changed{suffix} — consider updating {}",
            shown.join(", "),
            entry.docs.join(", ")
        ));
    }

    warnings
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn tmp_dir() -> TempDir {
        TempDir::new().unwrap()
    }

    // T-004: find_review_date returns None when no marker present.
    #[test]
    fn t004_find_review_date_absent() {
        assert!(find_review_date("# Title\n\nSome content.\n").is_none());
    }

    // T-004: find_review_date extracts the date string.
    #[test]
    fn t004_find_review_date_present() {
        let content = "# Guide\n\n**Reviewed**: 2026-01-15\n\nContent here.\n";
        assert_eq!(find_review_date(content), Some("2026-01-15".to_string()));
    }

    // T-004: find_review_date ignores malformed markers (no date).
    #[test]
    fn t004_find_review_date_empty_value() {
        let content = "**Reviewed**:  \n\nContent.\n";
        assert!(find_review_date(content).is_none());
    }

    // T-004: is_stale returns false for a very recent date.
    #[test]
    fn t004_is_stale_recent_date_is_not_stale() {
        // 2099-01-01 is in the future relative to any real run date.
        assert!(!is_stale("2099-01-01", 180));
    }

    // T-004: is_stale returns true for a very old date.
    #[test]
    fn t004_is_stale_old_date_is_stale() {
        assert!(is_stale("2000-01-01", 180));
    }

    // T-004: is_stale returns false for an unparseable date string.
    #[test]
    fn t004_is_stale_bad_date_returns_false() {
        assert!(!is_stale("not-a-date", 180));
    }

    // T-004: check_review_markers returns empty when no docs dirs exist.
    #[test]
    fn t004_check_review_markers_no_docs_dirs() {
        let td = tmp_dir();
        let docs_cfg = DocsConfig::default();
        let warnings = check_review_markers(td.path(), &docs_cfg);
        assert!(warnings.is_empty());
    }

    // T-004: check_review_markers warns when marker absent.
    #[test]
    fn t004_check_review_markers_missing_marker() {
        let td = tmp_dir();
        let dir = td.path().join("docs").join("how-to");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("guide.md"), "# Guide\n\nContent.\n").unwrap();
        let docs_cfg = DocsConfig::default();
        let warnings = check_review_markers(td.path(), &docs_cfg);
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].contains("lacks **Reviewed**"));
    }

    // T-004: check_review_markers warns when marker is stale.
    #[test]
    fn t004_check_review_markers_stale_marker() {
        let td = tmp_dir();
        let dir = td.path().join("docs").join("explanation");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("arch.md"),
            "# Arch\n\n**Reviewed**: 2000-01-01\n\nContent.\n",
        )
        .unwrap();
        let docs_cfg = DocsConfig {
            review_max_age_days: Some(30),
            ..DocsConfig::default()
        };
        let warnings = check_review_markers(td.path(), &docs_cfg);
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].contains("older than 30 days"));
    }

    // T-004: check_review_markers does not warn for a fresh marker.
    #[test]
    fn t004_check_review_markers_fresh_marker_no_warn() {
        let td = tmp_dir();
        let dir = td.path().join("docs").join("start-here");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("install.md"),
            "# Install\n\n**Reviewed**: 2099-01-01\n\nContent.\n",
        )
        .unwrap();
        let docs_cfg = DocsConfig {
            review_max_age_days: Some(180),
            ..DocsConfig::default()
        };
        let warnings = check_review_markers(td.path(), &docs_cfg);
        assert!(warnings.is_empty());
    }

    // T-004: files outside monitored dirs are not scanned.
    #[test]
    fn t004_check_review_markers_reference_dir_exempt() {
        let td = tmp_dir();
        // reference/ and decisions/ are exempt.
        for exempt in &["reference", "decisions"] {
            let dir = td.path().join("docs").join(exempt);
            std::fs::create_dir_all(&dir).unwrap();
            std::fs::write(dir.join("foo.md"), "# Foo\n\nNo marker.\n").unwrap();
        }
        let docs_cfg = DocsConfig::default();
        let warnings = check_review_markers(td.path(), &docs_cfg);
        assert!(
            warnings.is_empty(),
            "exempt dirs produced warnings: {warnings:?}"
        );
    }

    // T-007: check_touch_map returns empty when touch_map is empty.
    #[test]
    fn t007_check_touch_map_empty_config_returns_empty() {
        let td = tmp_dir();
        let git_cfg = GitConfig::default();
        let docs_cfg = DocsConfig::default();
        let warnings = check_touch_map(td.path(), &git_cfg, &docs_cfg);
        assert!(warnings.is_empty());
    }

    // T-007: check_touch_map returns empty when touch_map_warnings is false.
    #[test]
    fn t007_check_touch_map_warnings_disabled_returns_empty() {
        let td = tmp_dir();
        let git_cfg = GitConfig::default();
        let docs_cfg = DocsConfig {
            touch_map_warnings: false,
            touch_map: vec![flow_core::config::TouchMapEntry {
                paths: vec!["src/**".to_string()],
                docs: vec!["docs/foo.md".to_string()],
                suppress: false,
            }],
            ..DocsConfig::default()
        };
        let warnings = check_touch_map(td.path(), &git_cfg, &docs_cfg);
        assert!(warnings.is_empty());
    }
}
