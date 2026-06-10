//! `flow test` — run tests and consistency checks.

use crate::args::TestArgs;
use flow_core::{
    drift::{self, render::Mode, Report, Severity},
    envelope, parse, paths, verify, Error, Result,
};
use std::collections::HashSet;
use std::path::Path;

const TEST_RUNNER_ENV_REMOVE: &[&str] = &[
    crate::public_command::FLOW_HOST_ENV,
    "FLOW_RUN_DIR",
    "FLOW_CHANGE_DIR",
];

/// Run `flow test`.
pub fn run(args: TestArgs) -> Result<()> {
    let repo = paths::repo_root(None)?;
    match super::finalize_mode(args.finalize) {
        super::FinalizeMode::Infer => {
            let dir = super::amend::resolve_feature_dir(&repo)?;
            return finalize(&dir);
        }
        super::FinalizeMode::Skip => {}
    }
    let feature_dir = super::amend::resolve_feature_dir(&repo)?;
    prepare(&repo, &feature_dir)
}

fn prepare(repo: &Path, feature_dir: &Path) -> Result<()> {
    super::task_state::ensure_all_accepted(feature_dir, "/flow-test")?;

    let verification = verify_feature(repo, feature_dir)?;
    let out = envelope::compose(repo, "test", feature_dir, Some(&verification.extra))?;
    print!("{out}");
    if verification.passed() {
        crate::output::maybe_print_finalize_hint("test", feature_dir);
        Ok(())
    } else {
        crate::output::print_next("flow-test", "after fixing verification failures.");
        Err(verification.failure_error())
    }
}

pub(crate) fn run_and_finalize(feature_dir: &Path) -> Result<()> {
    super::task_state::ensure_all_accepted(feature_dir, "/flow-test")?;
    let repo = paths::repo_root(Some(feature_dir)).unwrap_or_else(|_| feature_dir.to_path_buf());
    let verification = verify_feature(&repo, feature_dir)?;
    print!("{}", verification.extra);
    if !verification.passed() {
        crate::output::print_next("flow-test", "after fixing verification failures.");
        return Err(verification.failure_error());
    }
    stamp_build_complete(feature_dir)
}

struct Verification {
    extra: String,
    report: Report,
    tests_passed: bool,
}

impl Verification {
    fn passed(&self) -> bool {
        self.tests_passed && !self.report.has_error
    }

    fn failure_error(&self) -> Error {
        if !self.tests_passed {
            return Error::User(
                "Verification failed: test runner did not pass. Fix the failures and rerun `flow test`."
                    .to_string(),
            );
        }
        Error::DriftErrors {
            errors: self
                .report
                .findings
                .iter()
                .filter(|f| matches!(f.severity, Severity::Error))
                .count(),
            warns: self
                .report
                .findings
                .iter()
                .filter(|f| matches!(f.severity, Severity::Warn))
                .count(),
        }
    }
}

fn verify_feature(repo: &Path, feature_dir: &Path) -> Result<Verification> {
    let cfg = flow_core::config::Config::load_for_repo(repo).unwrap_or_default();
    let runner = verify::detect(repo, &cfg);
    let mut test_summary = String::new();
    let tests_passed = if let Some(r) = runner {
        test_summary.push_str(&format!("## Detected test runner\n\n`{}`\n", r.command));
        match verify::run_with_env_removed(repo, &r, TEST_RUNNER_ENV_REMOVE) {
            Ok(status) if status.success() => {
                test_summary.push_str("\nTests: PASS\n");
                true
            }
            Ok(_) => {
                test_summary.push_str("\nTests: FAIL\n");
                false
            }
            Err(e) => {
                test_summary.push_str(&format!("\nTest runner error: {e}\n"));
                false
            }
        }
    } else {
        test_summary.push_str(
            "## Detected test runner\n\nNo configured or auto-detected test runner.\n\nTests: NOT RUN\n",
        );
        true
    };

    // Drift
    let findings = drift::check_artifacts(feature_dir, Some(repo))?;
    let promote: HashSet<String> = ["D1", "D2", "D3"]
        .iter()
        .map(|s| (*s).to_string())
        .collect();
    let findings = drift::promote_severity(findings, &promote);
    let report = drift::build_report(findings);
    let consistency = drift::render::render(
        &report,
        Mode::Test,
        &crate::public_command::render_current("flow-test"),
        false,
    );

    // Cache the consistency report so later envelopes can surface it.
    flow_core::status::write_cache(feature_dir, &consistency)?;

    let extra = format!("{test_summary}\n{consistency}");
    Ok(Verification {
        extra,
        report,
        tests_passed,
    })
}

fn finalize(feature_dir: &Path) -> Result<()> {
    run_and_finalize(feature_dir)
}

fn stamp_build_complete(feature_dir: &Path) -> Result<()> {
    parse::status::stamp(
        feature_dir,
        Some(parse::status::State::Building),
        "build-complete",
        "verification passed",
    )?;
    crate::cmd::run::update_run_state_for_feature_phase(
        feature_dir,
        "build-complete",
        "build-complete",
        "$flow-close",
    )?;
    flow_core::logging::info("Verification complete. Build phase closed.");
    println!("Verification complete. Build phase closed.");
    crate::output::print_next("flow-close", "close the change.");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn t004_verification_strips_flow_run_context_from_test_runner() {
        assert!(TEST_RUNNER_ENV_REMOVE.contains(&crate::public_command::FLOW_HOST_ENV));
        assert!(TEST_RUNNER_ENV_REMOVE.contains(&"FLOW_RUN_DIR"));
        assert!(TEST_RUNNER_ENV_REMOVE.contains(&"FLOW_CHANGE_DIR"));
    }
}
