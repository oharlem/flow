//! JSON output helpers for drift reports.
//!
//! The [`Report`](super::Report) and [`Finding`](super::Finding) types already
//! derive `serde::Serialize`; this module exposes convenience functions so
//! callers don't need to depend on `serde_json` directly.

use super::Report;

/// Serialize a drift [`Report`] as pretty JSON (2-space indent).
///
/// # Errors
///
/// Returns [`serde_json::Error`] when the report cannot be serialized — in
/// practice this never happens because `Report` owns only `String` / `usize`
/// / `bool` / `Option<usize>` fields.
pub fn to_pretty_json(report: &Report) -> serde_json::Result<String> {
    serde_json::to_string_pretty(report)
}

/// Serialize a drift [`Report`] as compact JSON (single line).
pub fn to_compact_json(report: &Report) -> serde_json::Result<String> {
    serde_json::to_string(report)
}

#[cfg(test)]
mod tests {
    use super::super::{build_report, Finding, Severity};
    use super::*;

    fn sample() -> Report {
        build_report(vec![Finding {
            id: "D2".into(),
            severity: Severity::Warn,
            message: "task T-001 covers 'FR-999' which is not defined in spec.md".into(),
            title: "Task points to a missing requirement".into(),
            cause: "T-001 says it covers FR-999, but FR-999 is not listed in spec.md.".into(),
            file: "tasks.md".into(),
            line: Some(17),
            subject: "T-001".into(),
            fix_options: vec!["Remove the stale FR-999 reference from T-001.".into()],
        }])
    }

    #[test]
    fn pretty_json_round_trips() {
        let report = sample();
        let text = to_pretty_json(&report).unwrap();
        assert!(text.contains(r#""id": "D2""#));
        assert!(text.contains(r#""severity": "warn""#));
        let parsed: serde_json::Value = serde_json::from_str(&text).unwrap();
        assert_eq!(parsed["findings"][0]["id"], "D2");
        assert_eq!(parsed["has_warn"], true);
        assert_eq!(parsed["has_error"], false);
    }

    #[test]
    fn compact_json_is_single_line() {
        let text = to_compact_json(&sample()).unwrap();
        assert_eq!(text.lines().count(), 1);
    }
}
