//! Typed Flow identifiers (FR/SC/T/P/D/M/R).
//!
//! Grammar follows `core/conventions/conventions.md` §3. All IDs are case-sensitive.

use crate::error::Error;
use once_cell::sync::Lazy;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

/// Shared regex for every ID prefix recognized by Flow.
///
/// Matches: `FR-[A]?NNNN[a]?`, `SC-[A]?NNNN[a]?`, `T-[A]?NNNN`, `M-N`,
/// `P-NNNN`, `R-NNNN`, `D[1-9]\d?`.
pub static ANY_ID: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r"\b(?:FR-[A-Z]?\d{1,4}[a-z]?|SC-[A-Z]?\d{1,4}[a-z]?|T-[A-Z]?\d{1,4}|M-[1-9]\d*|P-\d{1,4}|R-[A-Z]?\d{1,4}|D[1-9]\d?)\b",
    )
    .expect("shared ID regex compiles")
});

macro_rules! id_struct_simple {
    ($name:ident, $prefix:literal, $re_str:literal) => {
        /// Flow identifier.
        #[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name(pub String);

        impl $name {
            /// Return the underlying identifier text.
            #[must_use]
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(&self.0)
            }
        }

        impl FromStr for $name {
            type Err = Error;

            fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
                static RE: Lazy<Regex> =
                    Lazy::new(|| Regex::new(&format!("^{}$", $re_str)).expect("ID regex compiles"));
                if RE.is_match(s) {
                    Ok(Self(s.to_string()))
                } else {
                    Err(Error::InvalidId {
                        kind: $prefix.to_string(),
                        input: s.to_string(),
                        reason: format!("expected grammar {}", $re_str),
                    })
                }
            }
        }
    };
}

id_struct_simple!(FrId, "FR", r"FR-[A-Z]?\d{1,4}[a-z]?");
id_struct_simple!(ScId, "SC", r"SC-[A-Z]?\d{1,4}[a-z]?");
id_struct_simple!(TaskId, "T", r"T-[A-Z]?\d{1,4}");
id_struct_simple!(MilestoneId, "M", r"M-[1-9]\d*");
id_struct_simple!(PId, "P", r"P-\d{1,4}");
id_struct_simple!(RId, "R", r"R-[A-Z]?\d{1,4}");
id_struct_simple!(DId, "D", r"D[1-9]\d?");

/// Scan `text` for every occurrence of an ID with the given prefix.
///
/// Returns `(id, 1-based-line-number, trimmed-line)` tuples in occurrence order.
/// Only the **first** line containing a given ID is returned.
#[must_use]
pub fn first_locations(text: &str, prefix: &str) -> Vec<(String, usize, String)> {
    let pattern_source = match prefix {
        "FR" => r"\bFR-[A-Z]?\d{1,4}[a-z]?\b",
        "SC" => r"\bSC-[A-Z]?\d{1,4}[a-z]?\b",
        "T" => r"\bT-[A-Z]?\d{1,4}\b",
        "M" => r"\bM-[1-9]\d*\b",
        "P" => r"\bP-\d{1,4}\b",
        "R" => r"\bR-[A-Z]?\d{1,4}\b",
        "D" => r"\bD[1-9]\d?\b",
        _ => return Vec::new(),
    };
    let re = Regex::new(pattern_source).expect("prefix regex compiles");
    let mut seen = std::collections::HashSet::new();
    let mut out = Vec::new();
    for (lineno, line) in text.lines().enumerate() {
        for m in re.find_iter(line) {
            let id = m.as_str().to_string();
            if seen.insert(id.clone()) {
                out.push((id, lineno + 1, line.trim().chars().take(120).collect()));
            }
        }
    }
    out
}

/// Return every occurrence (definitions + references) of the given-prefix IDs
/// in `text`, as `(id, 1-based-line, trimmed-line)` tuples.
#[must_use]
pub fn find_references(text: &str, prefix: &str) -> Vec<(String, usize, String)> {
    let pattern_source = match prefix {
        "FR" => r"\bFR-[A-Z]?\d{1,4}[a-z]?\b",
        "SC" => r"\bSC-[A-Z]?\d{1,4}[a-z]?\b",
        "T" => r"\bT-[A-Z]?\d{1,4}\b",
        "M" => r"\bM-[1-9]\d*\b",
        "P" => r"\bP-\d{1,4}\b",
        "R" => r"\bR-[A-Z]?\d{1,4}\b",
        "D" => r"\bD[1-9]\d?\b",
        _ => return Vec::new(),
    };
    let re = Regex::new(pattern_source).expect("prefix regex compiles");
    let mut out = Vec::new();
    for (lineno, line) in text.lines().enumerate() {
        for m in re.find_iter(line) {
            out.push((
                m.as_str().to_string(),
                lineno + 1,
                line.trim().chars().take(120).collect(),
            ));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_valid_fr_ids() {
        assert!("FR-001".parse::<FrId>().is_ok());
        assert!("FR-V12".parse::<FrId>().is_ok());
        assert!("FR-007a".parse::<FrId>().is_ok());
    }

    #[test]
    fn rejects_invalid_fr_ids() {
        assert!("FR-".parse::<FrId>().is_err());
        assert!("fr-001".parse::<FrId>().is_err());
        assert!("FR-99999".parse::<FrId>().is_err());
        assert!("FR-1ab".parse::<FrId>().is_err());
    }

    #[test]
    fn parses_valid_milestone_ids() {
        assert!("M-1".parse::<MilestoneId>().is_ok());
        assert!("M-9999".parse::<MilestoneId>().is_ok());
    }

    #[test]
    fn rejects_invalid_milestone_ids() {
        assert!("M-".parse::<MilestoneId>().is_err());
        assert!("M-0".parse::<MilestoneId>().is_err());
        assert!("M-0001".parse::<MilestoneId>().is_err());
        assert!("M-V01".parse::<MilestoneId>().is_err());
        assert!("m-001".parse::<MilestoneId>().is_err());
    }

    #[test]
    fn parses_valid_task_ids() {
        assert!("T-001".parse::<TaskId>().is_ok());
        assert!("T-V07".parse::<TaskId>().is_ok());
    }

    #[test]
    fn display_round_trip() {
        let id: FrId = "FR-007a".parse().unwrap();
        assert_eq!(id.to_string(), "FR-007a");
    }

    #[test]
    fn first_locations_dedups() {
        let text = "line\nFR-001\nFR-001 again\nFR-002\n";
        let got = first_locations(text, "FR");
        assert_eq!(got.len(), 2);
        assert_eq!(got[0].0, "FR-001");
        assert_eq!(got[0].1, 2);
        assert_eq!(got[1].0, "FR-002");
        assert_eq!(got[1].1, 4);
    }
}
