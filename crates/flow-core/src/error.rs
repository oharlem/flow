//! Unified error type for `flow-core`.

use std::path::PathBuf;

/// The result type returned by most `flow-core` entry points.
pub type Result<T> = std::result::Result<T, Error>;

/// Errors produced anywhere inside `flow-core`.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Current working directory is not inside a git checkout.
    #[error("not a git repository; run Flow inside a git checkout")]
    NotAGitRepository,

    /// A required file is missing.
    #[error("{kind} not found at {path}")]
    FileNotFound {
        /// Human description of the missing file (e.g. `"spec.md"`).
        kind: String,
        /// Absolute path that was checked.
        path: PathBuf,
    },

    /// Workflow guard: working tree has unstaged changes that would conflict.
    #[error("working tree has uncommitted changes in a path Flow would touch: {0}")]
    DirtyWorkingTree(String),

    /// A Flow identifier string was malformed.
    #[error("invalid {kind} identifier {input:?}: {reason}")]
    InvalidId {
        /// ID kind being parsed (e.g. `"FR"`, `"T"`, `"M"`).
        kind: String,
        /// The raw input string that failed to parse.
        input: String,
        /// Human-readable reason.
        reason: String,
    },

    /// An artifact section is missing or malformed.
    #[error("{file}: {message}")]
    ArtifactError {
        /// File that contained the problem.
        file: String,
        /// Message describing the problem.
        message: String,
    },

    /// A drift-check run surfaced blocking findings.
    #[error("consistency check failed: {errors} error(s), {warns} warning(s)")]
    DriftErrors {
        /// Number of `severity="error"` findings.
        errors: usize,
        /// Number of `severity="warn"` findings.
        warns: usize,
    },

    /// Wrong command for the intent (e.g. `/flow-start` when user wants `/flow-amend`).
    #[error("use {suggested} instead")]
    WrongCommand {
        /// The command the user probably meant.
        suggested: String,
    },

    /// Configuration parsing failed.
    #[error("config error: {0}")]
    Config(String),

    /// Arbitrary user-error with a one-liner message.
    #[error("{0}")]
    User(String),

    /// Underlying I/O error.
    #[error(transparent)]
    Io(#[from] std::io::Error),

    /// Underlying YAML parse error.
    #[error(transparent)]
    Yaml(#[from] serde_yaml::Error),

    /// Underlying JSON parse error.
    #[error(transparent)]
    Json(#[from] serde_json::Error),

    /// Regex compilation error.
    #[error(transparent)]
    Regex(#[from] regex::Error),
}

impl Error {
    /// Return the POSIX-style exit code this error maps to.
    #[must_use]
    pub fn exit_code(&self) -> i32 {
        match self {
            Self::NotAGitRepository => 1,
            Self::DriftErrors { errors: 0, .. } => 1,
            Self::DriftErrors { .. } => 2,
            Self::InvalidId { .. }
            | Self::ArtifactError { .. }
            | Self::WrongCommand { .. }
            | Self::DirtyWorkingTree(_)
            | Self::User(_) => 2,
            Self::FileNotFound { .. } | Self::Config(_) => 64,
            Self::Io(_) | Self::Yaml(_) | Self::Json(_) | Self::Regex(_) => 64,
        }
    }
}
