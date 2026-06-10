//! Parsers for every Flow artifact.
//!
//! Each sub-module exposes a `parse_*` function that reads a file (or string)
//! and returns a typed view of the artifact.

pub mod markdown;
pub mod plan;
pub mod roadmap;
pub mod spec;
pub mod status;
pub mod tasks;
