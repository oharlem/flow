//! Envelope composer — builds the `/flow-<cmd>` stdout blob that the host agent
//! reads as instructions.
//!
//! Mirrors `core/scripts/lib/flow-compose.sh`.

use crate::{assets, config::Config, git, paths, preflight, settings::Settings};
use std::path::{Path, PathBuf};

const SEPARATOR: &str = "\n---\n";

/// Name of the phase whose envelope is being composed.
///
/// Kept as a string to match the existing prompt file naming convention.
pub type Phase = &'static str;

/// Compose an envelope for `phase` in `feature_dir`.
///
/// The envelope concatenates (in order):
/// 1. Conventions shards (`core.md` always, plus the phase-specific shard
///    resolved via [`conventions_shards_for`]). Each shard is read from
///    `.flow/conventions/<name>.md` or falls back to the binary-embedded
///    asset.
/// 2. `docs/principles.md` (if present and non-empty).
/// 3. The phase base prompt.
/// 4. Optional local override: `.flow/agents/<phase>.local.md`.
/// 5. Runtime context block.
/// 6. Optional `extra_context` (e.g. task text).
pub fn compose(
    repo: &Path,
    phase: Phase,
    feature_dir: &Path,
    extra_context: Option<&str>,
) -> crate::Result<String> {
    compose_full(repo, phase, feature_dir, extra_context, None, None)
}

/// Compose an envelope with an optional destructive-action annotation.
///
/// When `destructive_reason` is `Some`, the runtime context block will contain
/// `**Destructive action**: <reason>` immediately after `**Confirmation**:`.
/// Phase agents read this line to decide whether to prompt even when
/// `**Confirmation**: disabled`.
pub fn compose_with_destructive(
    repo: &Path,
    phase: Phase,
    feature_dir: &Path,
    extra_context: Option<&str>,
    destructive_reason: Option<&str>,
) -> crate::Result<String> {
    compose_full(
        repo,
        phase,
        feature_dir,
        extra_context,
        destructive_reason,
        None,
    )
}

/// Compose an envelope with an explicit `Save state with` command.
///
/// Use when the canonical finalize command for `phase` takes a dynamic
/// argument the runtime cannot derive from the phase name alone — for example
/// `flow build-task T-001 --finalize` or
/// `FLOW_RUN_DIR="<run-dir>" flow roadmap --finalize`.
/// When omitted (callers using [`compose`] or [`compose_with_destructive`]),
/// the runtime context defaults to `flow <phase> --finalize`.
pub fn compose_with_save_command(
    repo: &Path,
    phase: Phase,
    feature_dir: &Path,
    extra_context: Option<&str>,
    save_command: &str,
) -> crate::Result<String> {
    compose_full(
        repo,
        phase,
        feature_dir,
        extra_context,
        None,
        Some(save_command),
    )
}

fn compose_full(
    repo: &Path,
    phase: Phase,
    feature_dir: &Path,
    extra_context: Option<&str>,
    destructive_reason: Option<&str>,
    save_command: Option<&str>,
) -> crate::Result<String> {
    let mut out = String::new();

    // [1] Conventions — always load the core shard, plus the phase-specific
    // shard when one applies. Both reads fall back to the embedded constant
    // so the composer never fails when the filesystem copy is missing.
    for shard in conventions_shards_for(phase) {
        let text = load_conventions_shard(repo, shard);
        if !text.is_empty() {
            out.push_str(&text);
            out.push_str(SEPARATOR);
        }
    }

    // [2] Principles (live)
    let principles_file = repo.join("docs").join("principles.md");
    if let Ok(text) = std::fs::read_to_string(&principles_file) {
        if !text.trim().is_empty() {
            out.push_str(&text);
            out.push_str(SEPARATOR);
        }
    }

    // [3] Phase base
    let flow_base = paths::flow_dir(repo)
        .join("agents")
        .join(format!("{phase}.base.md"));
    let base_text = if let Ok(text) = std::fs::read_to_string(&flow_base) {
        text
    } else if let Some(builtin) = assets::agent_base(phase) {
        builtin.to_string()
    } else {
        String::new()
    };
    if !base_text.is_empty() {
        out.push_str(&base_text);
        out.push_str(SEPARATOR);
    }

    // [4] Phase local
    let flow_local = paths::flow_dir(repo)
        .join("agents")
        .join(format!("{phase}.local.md"));
    if let Ok(text) = std::fs::read_to_string(&flow_local) {
        if !text.trim().is_empty() {
            out.push_str(&text);
            out.push_str(SEPARATOR);
        }
    }

    // [5] Runtime context
    out.push_str(&runtime_context_full(
        repo,
        phase,
        feature_dir,
        destructive_reason,
        save_command,
    )?);

    // [6] Extra
    if let Some(extra) = extra_context {
        if !extra.trim().is_empty() {
            out.push_str(SEPARATOR);
            out.push_str(extra);
            if !extra.ends_with('\n') {
                out.push('\n');
            }
        }
    }

    Ok(out)
}

/// Build the `# Runtime Context` section of the envelope.
pub fn runtime_context(repo: &Path, phase: &str, feature_dir: &Path) -> crate::Result<String> {
    runtime_context_full(repo, phase, feature_dir, None, None)
}

fn runtime_context_full(
    repo: &Path,
    phase: &str,
    feature_dir: &Path,
    destructive_reason: Option<&str>,
    save_command: Option<&str>,
) -> crate::Result<String> {
    let mut out = String::new();
    let cfg = Config::load_for_repo(repo).unwrap_or_default();
    out.push_str("# Runtime Context\n\n");

    let feature_name = feature_dir
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default();
    out.push_str(&format!("**Change**: {feature_name}\n"));

    let branch = git::current_branch(repo).unwrap_or_else(|_| "unknown".to_string());
    out.push_str(&format!("**Branch**: {branch}\n"));
    out.push_str(&format!("**Phase being invoked**: {phase}\n"));

    // Auto-inject confirmation setting so every phase agent can read it.
    let settings = Settings::load_for_repo(repo).unwrap_or_default();
    let confirmation_label = if settings.confirmation.is_disabled() {
        "disabled"
    } else {
        "required"
    };
    out.push_str(&format!("**Confirmation**: {confirmation_label}\n"));
    let review_label = if settings.review_skip_finalize_footer(phase) {
        "collapsed"
    } else {
        "two-stage"
    };
    out.push_str(&format!("**Review**: {review_label}\n"));
    let default_save_command = format!("flow {phase} --finalize");
    let save_cmd = save_command.unwrap_or(default_save_command.as_str());
    out.push_str(&format!("**Save state with**: `{save_cmd}`\n"));
    if review_label == "collapsed" {
        out.push_str(
            "**Finalize**: run the `Save state with` command in this session when the work is ready (no separate footer checkpoint).\n",
        );
    }
    if let Some(reason) = destructive_reason {
        out.push_str(&format!("**Destructive action**: {reason}\n"));
    }

    let status_path = feature_dir.join("status.md");
    if status_path.exists() {
        if let Ok(status) = crate::parse::status::parse_file(&status_path) {
            out.push_str(&format!(
                "**State (per status.md)**: {}\n",
                status
                    .state
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| "unknown".into())
            ));
            out.push_str(&format!("**Updated**: {}\n", status.updated));
            if let Ok(gate) = crate::status::effective_gate(feature_dir) {
                out.push_str(&format!("**Effective gate**: {gate}\n"));
            }
            out.push('\n');
            out.push_str("## Recent Phase History (last 5)\n\n");
            for entry in status.history.iter().take(5) {
                out.push_str(&format!(
                    "- {} — {} — {}\n",
                    entry.timestamp, entry.action, entry.summary
                ));
            }
        }
    }
    out.push('\n');

    out.push_str("## Current Planning Files (size, last-modified)\n\n");
    if let Ok(read) = std::fs::read_dir(feature_dir) {
        let mut md_files: Vec<PathBuf> = read
            .flatten()
            .filter(|e| {
                e.path()
                    .extension()
                    .is_some_and(|x| x.eq_ignore_ascii_case("md"))
            })
            .map(|e| e.path())
            .collect();
        md_files.sort();
        for p in md_files {
            let fname = p.file_name().unwrap().to_string_lossy();
            if fname == ".flow-test.last.md" {
                continue;
            }
            let meta = std::fs::metadata(&p).ok();
            let size = meta.as_ref().map_or(0, |m| m.len());
            out.push_str(&format!("- {fname} — {size} B\n"));
        }
    }

    out.push_str("\n## Latest Consistency Check (most recent /flow-test or auto-check)\n\n");
    if let Some(cache) = crate::status::read_cache(feature_dir) {
        // Include only the `## Consistency Check` block + its immediate content.
        if let Some(idx) = cache.find("## Consistency Check") {
            let slice = &cache[idx..];
            // Keep it compact: first ~30 lines.
            let trimmed: String = slice.lines().take(30).collect::<Vec<_>>().join("\n");
            out.push_str(&trimmed);
            out.push('\n');
        } else {
            out.push_str("(cached report has no findings)\n");
        }
    } else {
        out.push_str("(no cached consistency report — run `flow test`)\n");
    }

    if matches!(phase, "plan" | "build" | "build-task") {
        out.push('\n');
        out.push_str(&preflight::render_known_requirements(&cfg)?);
    }

    out.push_str("\n## Per-Phase Model & Effort\n\n");
    let (effort, model) = phase_override(&cfg, phase);
    out.push_str(&format!(
        "**Model**: {}\n",
        model.unwrap_or_else(|| "host default".into())
    ));
    out.push_str(&format!(
        "**Effort**: {}\n",
        effort.unwrap_or_else(|| "host default".into())
    ));

    Ok(out)
}

fn phase_override(cfg: &Config, phase: &str) -> (Option<String>, Option<String>) {
    let pc = match phase {
        "setup" => &cfg.phases.setup,
        "start" => &cfg.phases.start,
        "amend" => &cfg.phases.amend,
        "plan" => &cfg.phases.plan,
        "build" | "build-task" => &cfg.phases.build,
        "test" | "check" => &cfg.phases.check,
        "close" => &cfg.phases.close,
        _ => return (None, None),
    };
    (pc.effort.clone(), pc.model.clone())
}

/// Print the canonical `Next command: …` footer to stdout.
pub fn print_next_command(command: &str, detail: &str) {
    if detail.is_empty() {
        println!("\nNext command: `{command}`");
    } else {
        println!("\nNext command: `{command}` - {detail}");
    }
}

/// Return `true` if the passed description looks like a spec-amendment request
/// (contains phrases such as "adjust the spec", "update the requirements", etc.).
///
/// Used by `/flow-start` to route the user to `/flow-amend` on mistakes.
#[must_use]
pub fn looks_like_spec_amendment(text: &str) -> bool {
    let lower = text.to_lowercase();
    let patterns = [
        r"\b(adjust|update|revise|amend|edit|modify|change|fix)\s+(the\s+)?(current\s+)?spec\b",
        r"\b(add|append|include)\s+([a-z0-9 ,'-]+\s+)?requirements?\b",
        r"\bchange\s+(current\s+)?spec\b",
        r"\bcurrent\s+spec(ification)?\b",
    ];
    for p in patterns {
        if let Ok(re) = regex::Regex::new(p) {
            if re.is_match(&lower) {
                return true;
            }
        }
    }
    false
}

/// Map a phase name to the ordered list of conventions shard names that
/// should be prepended to its envelope.
///
/// `core` is always first. A single phase-specific shard is appended when the
/// phase class has one. Phases without a dedicated shard (`status`, `setup`,
/// `doctor`, …) receive only `core`.
///
/// `amend` shares the `spec` shard with `start` because it edits the same
/// artifact class (`spec.md`). `build-task` shares the `build` shard with
/// `build`.
#[must_use]
pub fn conventions_shards_for(phase: &str) -> Vec<&'static str> {
    let mut shards = vec!["core"];
    match phase {
        "start" | "amend" => shards.push("spec"),
        "plan" => shards.push("plan"),
        "build" | "build-task" => shards.push("build"),
        "test" | "check" => shards.push("test"),
        "close" => shards.push("close"),
        "run" => shards.push("run"),
        "roadmap" => shards.push("roadmap"),
        _ => {}
    }
    shards
}

/// Read a conventions shard: prefer the on-disk copy under
/// `.flow/conventions/<name>.md`, falling back to the binary-embedded
/// constant when the file cannot be read. Returns an empty string only when
/// both lookups fail.
fn load_conventions_shard(repo: &Path, shard_name: &str) -> String {
    for path in paths::conventions_shard_paths(repo, shard_name) {
        if let Ok(text) = std::fs::read_to_string(path) {
            return text;
        }
    }
    assets::conventions_shard(shard_name)
        .map(std::string::ToString::to_string)
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn detects_amendment_requests() {
        assert!(looks_like_spec_amendment("adjust the spec to add X"));
        assert!(looks_like_spec_amendment("change the current spec"));
        assert!(looks_like_spec_amendment("add requirements for login"));
        assert!(!looks_like_spec_amendment("add login form"));
    }

    fn seed_repo(confirmation: Option<&str>) -> TempDir {
        let td = TempDir::new().unwrap();
        let flow_dir = td.path().join(".flow");
        std::fs::create_dir_all(&flow_dir).unwrap();
        if let Some(value) = confirmation {
            std::fs::write(
                flow_dir.join("config.yaml"),
                format!("schema_version: 1.0\nconfirmation: \"{value}\"\n"),
            )
            .unwrap();
        }
        td
    }

    #[test]
    fn t012_confirmation_required_emitted_when_setting_required() {
        let td = seed_repo(Some("yes"));
        let ctx = runtime_context(td.path(), "start", td.path()).unwrap();
        assert!(
            ctx.contains("**Confirmation**: required"),
            "expected required marker:\n{ctx}"
        );
    }

    #[test]
    fn t012_confirmation_disabled_emitted_when_setting_no() {
        let td = seed_repo(Some("no"));
        let ctx = runtime_context(td.path(), "start", td.path()).unwrap();
        assert!(
            ctx.contains("**Confirmation**: disabled"),
            "expected disabled marker:\n{ctx}"
        );
    }

    #[test]
    fn t012_confirmation_disabled_when_settings_missing() {
        let td = TempDir::new().unwrap();
        let ctx = runtime_context(td.path(), "start", td.path()).unwrap();
        // Default ConfirmationSetting is No (disabled).
        assert!(
            ctx.contains("**Confirmation**: disabled"),
            "expected disabled marker by default:\n{ctx}"
        );
    }

    #[test]
    fn t012_destructive_action_line_present_when_reason_supplied() {
        let td = seed_repo(Some("no"));
        let ctx = runtime_context_full(
            td.path(),
            "roadmap",
            td.path(),
            Some("replacing existing milestones in the roadmap"),
            None,
        )
        .unwrap();
        assert!(
            ctx.contains("**Destructive action**: replacing existing milestones in the roadmap"),
            "expected destructive action marker:\n{ctx}"
        );
    }

    #[test]
    fn review_collapsed_emitted_when_before_finalize_false() {
        let td = seed_repo(Some("no"));
        std::fs::write(
            td.path().join(".flow/config.yaml"),
            "schema_version: 1.0\nconfirmation: no\nreview:\n  before_finalize: false\n",
        )
        .unwrap();
        let ctx = runtime_context(td.path(), "plan", td.path()).unwrap();
        assert!(ctx.contains("**Review**: collapsed"));
        assert!(ctx.contains("**Save state with**: `flow plan --finalize`"));
    }

    #[test]
    fn review_two_stage_emitted_when_before_finalize_true() {
        let td = seed_repo(Some("no"));
        std::fs::write(
            td.path().join(".flow/config.yaml"),
            "schema_version: 1.0\nconfirmation: no\nreview:\n  before_finalize: true\n",
        )
        .unwrap();
        let ctx = runtime_context(td.path(), "plan", td.path()).unwrap();
        assert!(ctx.contains("**Review**: two-stage"));
        assert!(ctx.contains("**Save state with**: `flow plan --finalize`"));
        assert!(!ctx.contains("no separate footer checkpoint"));
    }

    #[test]
    fn save_state_with_uses_explicit_command_when_provided() {
        let td = seed_repo(Some("no"));
        let ctx = runtime_context_full(
            td.path(),
            "build-task",
            td.path(),
            None,
            Some("flow build-task T-007 --finalize"),
        )
        .unwrap();
        assert!(
            ctx.contains("**Save state with**: `flow build-task T-007 --finalize`"),
            "expected explicit save command:\n{ctx}"
        );
    }

    #[test]
    fn t012_destructive_action_line_absent_when_reason_none() {
        let td = seed_repo(Some("no"));
        let ctx = runtime_context_full(td.path(), "start", td.path(), None, None).unwrap();
        assert!(
            !ctx.contains("**Destructive action**:"),
            "did not expect destructive action marker:\n{ctx}"
        );
    }

    // T-011: the composer falls back to the embedded shard constants when
    // the on-disk conventions files are absent. Both core and the phase
    // shard end up in the envelope.
    #[test]
    fn t011_compose_uses_embedded_fallback_when_shards_missing() {
        let td = seed_repo(Some("no"));
        // Intentionally do NOT create .flow/conventions/ — both reads must fall back.
        let envelope = compose(td.path(), "plan", td.path(), None).unwrap();
        assert!(
            envelope.contains("Conventions-Version: 1.1"),
            "core shard missing from envelope fallback output"
        );
        assert!(
            envelope.contains("## 3. ID grammar"),
            "core shard body missing:\n{envelope}"
        );
        assert!(
            envelope.contains("## 4.6 Engineering Principle"),
            "plan shard body missing from fallback envelope:\n{envelope}"
        );
    }

    // T-011: phases without a dedicated shard load only core.
    #[test]
    fn t011_shards_for_default_phase_loads_only_core() {
        assert_eq!(conventions_shards_for("status"), vec!["core"]);
        assert_eq!(conventions_shards_for("learn"), vec!["core"]);
        assert_eq!(conventions_shards_for("next"), vec!["core"]);
    }

    // T-011: start, amend, plan, build, build-task, test, close, run, roadmap
    // each add their shard.
    #[test]
    fn t011_shards_for_phase_dispatch_table() {
        assert_eq!(conventions_shards_for("start"), vec!["core", "spec"]);
        assert_eq!(conventions_shards_for("amend"), vec!["core", "spec"]);
        assert_eq!(conventions_shards_for("plan"), vec!["core", "plan"]);
        assert_eq!(conventions_shards_for("build"), vec!["core", "build"]);
        assert_eq!(conventions_shards_for("build-task"), vec!["core", "build"]);
        assert_eq!(conventions_shards_for("test"), vec!["core", "test"]);
        assert_eq!(conventions_shards_for("close"), vec!["core", "close"]);
        assert_eq!(conventions_shards_for("run"), vec!["core", "run"]);
        assert_eq!(conventions_shards_for("roadmap"), vec!["core", "roadmap"]);
    }

    // T-011: on-disk shard files take precedence over the embedded fallback.
    #[test]
    fn t011_on_disk_shard_overrides_embedded_fallback() {
        let td = seed_repo(Some("no"));
        let shard_dir = td.path().join(".flow").join("conventions");
        std::fs::create_dir_all(&shard_dir).unwrap();
        std::fs::write(
            shard_dir.join("core.md"),
            "CUSTOM-CORE-SENTINEL\nConventions-Version: 1.1\n",
        )
        .unwrap();
        std::fs::write(
            shard_dir.join("plan.md"),
            "CUSTOM-PLAN-SENTINEL\nConventions-Version: 1.1\n",
        )
        .unwrap();
        let envelope = compose(td.path(), "plan", td.path(), None).unwrap();
        assert!(envelope.contains("CUSTOM-CORE-SENTINEL"));
        assert!(envelope.contains("CUSTOM-PLAN-SENTINEL"));
    }
}
