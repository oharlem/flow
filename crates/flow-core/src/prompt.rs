//! Small host-facing prompt helpers.
//!
//! Flow only uses terminal prompts for the protected-branch warning that
//! `flow start` surfaces before creating a Flow branch. Tests set
//! `FLOW_FORCE_ON_PROTECTED=1` to skip the interactive step.

use std::io::{self, BufRead, IsTerminal, Write};

/// Outcome of a protected-branch confirmation prompt.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Confirmation {
    /// The user affirmed (y/yes) or `FLOW_FORCE_ON_PROTECTED=1` was set.
    Proceed,
    /// The user declined (anything else).
    Abort,
}

/// Ask for a `y`/`n`-style confirmation on stderr; read from stdin.
///
/// - Returns [`Confirmation::Proceed`] unconditionally when
///   `FLOW_FORCE_ON_PROTECTED=1` is set in the environment.
/// - Returns [`Confirmation::Proceed`] when project settings have
///   `confirmation=no`.
/// - Returns [`Confirmation::Proceed`] when stdin is not a TTY (non-interactive
///   runs default to "skip" to avoid hanging CI scripts).
/// - Otherwise reads a line from stdin and returns Proceed for
///   `y` / `yes` (case-insensitive).
pub fn confirm_protected_branch(branch: &str, confirmation_disabled: bool) -> Confirmation {
    if std::env::var_os("FLOW_FORCE_ON_PROTECTED").is_some_and(|v| v == "1") {
        return Confirmation::Proceed;
    }

    let stderr = io::stderr();
    let _ = writeln!(
        stderr.lock(),
        "[flow] WARN: Starting from protected branch '{branch}'. A Flow branch will be created."
    );

    if confirmation_disabled {
        let _ = writeln!(
            stderr.lock(),
            "[flow] confirmation=no; proceeding without prompt."
        );
        return Confirmation::Proceed;
    }

    let stdin = io::stdin();
    if !stdin.is_terminal() {
        // Non-interactive: warn once and proceed (tests, CI).
        let _ = writeln!(
            stderr.lock(),
            "[flow] stdin is not a TTY; proceeding without prompt. Set FLOW_FORCE_ON_PROTECTED=1 to silence."
        );
        return Confirmation::Proceed;
    }

    let _ = write!(
        stderr.lock(),
        "[flow] Continue? Type `y` or `yes` to proceed: "
    );
    let _ = io::stderr().flush();
    let mut line = String::new();
    if stdin.lock().read_line(&mut line).is_err() {
        return Confirmation::Abort;
    }
    let trimmed = line.trim().to_lowercase();
    if matches!(trimmed.as_str(), "y" | "yes") {
        Confirmation::Proceed
    } else {
        Confirmation::Abort
    }
}
