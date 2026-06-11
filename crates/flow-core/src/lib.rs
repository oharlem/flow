//! Flow — spec-driven workflow engine (core library).
//!
//! This crate is host-agnostic and has no knowledge of Claude Code, Codex,
//! Cursor, or any other agent host. It exposes typed artifact parsers,
//! renderers, drift-check engines, and an envelope composer. Host adapters
//! and the CLI live in sibling crates.

#![forbid(unsafe_code)]

pub mod assets;
pub mod config;
pub mod drift;
pub mod envelope;
pub mod error;
pub mod git;
pub mod ids;
pub mod logging;
pub mod parse;
pub mod paths;
pub mod preflight;
pub mod principles;
pub mod prompt;
pub mod render;
pub mod resume;
pub mod roadmap;
pub mod settings;
pub mod status;
pub mod verify;

pub use error::{Error, Result};

/// Build source stamp used to keep Cargo git-install fingerprints commit-aware.
#[doc(hidden)]
pub const BUILD_GIT_SHA: &str = env!("FLOW_CORE_GIT_SHA");
