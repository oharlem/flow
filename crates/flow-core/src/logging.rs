//! Logging initialization for `[flow]`-prefixed messages.
//!
//! `flow-core` intentionally exposes a minimal, stderr-based logger so the
//! library is free of any `tracing-subscriber` dependency. The CLI crate may
//! add richer subscribers on top.

use std::sync::Once;

static INIT: Once = Once::new();

/// Initialize the Flow logger.
///
/// Idempotent: safe to call from `main()` and from tests (first caller wins).
pub fn init() {
    INIT.call_once(|| {
        // Library-level init is a no-op; the CLI wires up `tracing-subscriber`.
    });
}

/// Log an `[flow] …` info message to stderr.
pub fn info(message: impl AsRef<str>) {
    eprintln!("[flow] {}", message.as_ref());
}

/// Log an `[flow] WARN: …` warning message to stderr.
pub fn warn(message: impl AsRef<str>) {
    eprintln!("[flow] WARN: {}", message.as_ref());
}

/// Log an `[flow] ERROR: …` error message to stderr.
pub fn error(message: impl AsRef<str>) {
    eprintln!("[flow] ERROR: {}", message.as_ref());
}
