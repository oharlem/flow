//! Template rendering helpers.

use crate::assets;
use chrono::{DateTime, NaiveDate, Utc};
use std::collections::BTreeMap;
use std::path::Path;

/// Render a single template by name, substituting `{{KEY}}` placeholders.
#[must_use]
pub fn render_template(name: &str, vars: &BTreeMap<&str, String>) -> Option<String> {
    let tmpl = assets::template(name)?;
    Some(substitute(tmpl, vars))
}

/// Substitute `{{KEY}}` placeholders in `tmpl` with the given vars.
#[must_use]
pub fn substitute(tmpl: &str, vars: &BTreeMap<&str, String>) -> String {
    let mut out = tmpl.to_string();
    for (key, value) in vars {
        let placeholder = format!("{{{{{}}}}}", key);
        out = out.replace(&placeholder, value);
    }
    out
}

/// Seed `spec.md` + `status.md` in `feature_dir`.
///
/// Does not overwrite existing files.
pub fn seed_feature_files(
    feature_dir: &Path,
    feature_name: &str,
    branch: &str,
) -> crate::Result<()> {
    std::fs::create_dir_all(feature_dir)?;
    let today = Utc::now().format("%Y-%m-%d").to_string();
    let now = Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();
    let artifact_root = feature_dir
        .parent()
        .and_then(|path| path.file_name())
        .and_then(|name| name.to_str())
        .unwrap_or("flow")
        .to_string();

    let mut vars: BTreeMap<&str, String> = BTreeMap::new();
    vars.insert("ARTIFACT_ROOT", artifact_root);
    vars.insert("FEATURE_NAME", feature_name.to_string());
    vars.insert("ISO_DATE", today.clone());
    vars.insert("ISO_DATETIME", now.clone());
    vars.insert("branch", branch.to_string());
    vars.insert("WHAT_AND_WHY", String::new());

    let spec_path = feature_dir.join("spec.md");
    if !spec_path.exists() {
        let body =
            render_template("spec.md.tmpl", &vars).unwrap_or_else(|| default_spec(feature_name));
        std::fs::write(&spec_path, body)?;
    }

    let status_path = feature_dir.join("status.md");
    if !status_path.exists() {
        let body = render_template("status.md.tmpl", &vars)
            .unwrap_or_else(|| default_status(feature_name, &today, &now, branch));
        std::fs::write(&status_path, body)?;
    }

    Ok(())
}

/// Seed `plan.md` and `tasks.md` in `feature_dir` when absent.
///
/// Does not overwrite existing files.
pub fn seed_plan_files(feature_dir: &Path, feature_name: &str, branch: &str) -> crate::Result<()> {
    std::fs::create_dir_all(feature_dir)?;
    let today = Utc::now().format("%Y-%m-%d").to_string();
    let artifact_root = feature_dir
        .parent()
        .and_then(|path| path.file_name())
        .and_then(|name| name.to_str())
        .unwrap_or("flow")
        .to_string();

    let mut vars: BTreeMap<&str, String> = BTreeMap::new();
    vars.insert("ARTIFACT_ROOT", artifact_root.clone());
    vars.insert("FEATURE_NAME", feature_name.to_string());
    vars.insert("ISO_DATE", today.clone());
    vars.insert("branch", branch.to_string());
    vars.insert("SUMMARY", String::new());
    vars.insert("language_version", "(detect from repo)".to_string());
    vars.insert("dependencies", "(detect from repo)".to_string());
    vars.insert("storage_approach", "(detect from repo)".to_string());
    vars.insert("testing_approach", "(detect from repo)".to_string());
    vars.insert("target_platform", "(detect from repo)".to_string());
    vars.insert("project_type", "(detect from repo)".to_string());
    vars.insert("performance_goals", "(none stated)".to_string());
    vars.insert("constraints", "(none stated)".to_string());
    vars.insert("scale_scope", "(none stated)".to_string());

    let plan_path = feature_dir.join("plan.md");
    if !plan_path.exists() {
        let body = render_template("plan.md.tmpl", &vars)
            .unwrap_or_else(|| default_plan(feature_name, branch, &today));
        std::fs::write(&plan_path, body)?;
    }

    let tasks_path = feature_dir.join("tasks.md");
    if !tasks_path.exists() {
        let body = render_template("tasks.md.tmpl", &vars)
            .unwrap_or_else(|| default_tasks(feature_name, &artifact_root));
        std::fs::write(&tasks_path, body)?;
    }

    Ok(())
}

fn default_plan(feature_name: &str, branch: &str, today: &str) -> String {
    format!(
        "# Implementation Plan: {feature_name}\n\n**Branch**: `{branch}` | **Date**: {today} | **Spec**: [spec.md](./spec.md)\n\n## Summary\n\n## Technical Context\n\n## Documentation Impact\n\n"
    )
}

fn default_tasks(feature_name: &str, artifact_root: &str) -> String {
    format!(
        "# Tasks: {feature_name}\n\n**Input**: Design documents from `/{artifact_root}/{feature_name}/`\n\n## Tasks\n\n"
    )
}

fn default_spec(feature_name: &str) -> String {
    format!("# Spec: {feature_name}\n\n## What & Why\n\n")
}

fn default_status(feature_name: &str, today: &str, now: &str, branch: &str) -> String {
    format!(
        "# Status: {feature_name}\n\n**Change**: {feature_name}\n**Started**: {today}\n**Updated**: {now}\n**State**: drafting\n**Branch**: {branch}\n\n## History\n\n- {now} — started — change seeded\n"
    )
}

/// Parse an ISO-8601 datetime produced by Flow.
///
/// Accepts `YYYY-MM-DDTHH:MM:SSZ` and `YYYY-MM-DDTHH:MM:SS+HH:MM`.
#[must_use]
pub fn parse_iso_datetime(s: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(s)
        .ok()
        .map(|dt| dt.with_timezone(&Utc))
}

/// Return today's date in ISO-8601 (`YYYY-MM-DD`) in UTC.
#[must_use]
pub fn today_iso() -> String {
    let today = Utc::now();
    today.format("%Y-%m-%d").to_string()
}

/// Return now in ISO-8601 UTC (`YYYY-MM-DDTHH:MM:SSZ`).
#[must_use]
pub fn now_iso() -> String {
    Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string()
}

/// Parse a bare ISO-8601 date.
#[must_use]
pub fn parse_iso_date(s: &str) -> Option<NaiveDate> {
    NaiveDate::parse_from_str(s, "%Y-%m-%d").ok()
}
