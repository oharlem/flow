use flow_core::Result;
use std::path::Path;

pub(crate) const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");

pub(crate) fn read(flow_dir: &Path) -> Option<String> {
    std::fs::read_to_string(flow_dir.join("version"))
        .ok()
        .map(|version| version.trim().to_string())
        .filter(|version| !version.is_empty())
}

pub(crate) fn write(flow_dir: &Path) -> Result<()> {
    std::fs::write(flow_dir.join("version"), format!("{CURRENT_VERSION}\n"))?;
    Ok(())
}

pub(crate) fn display_previous(previous: Option<&str>) -> &str {
    previous.unwrap_or("(none)")
}

/// Parse the leading `MAJOR.MINOR.PATCH` triplet from a version string.
///
/// Anything after a `-` (pre-release) or `+` (build metadata) is ignored. The
/// goal is a tiny, dependency-free comparator suitable for the recorded
/// `.flow/version` marker, which is always written from `CARGO_PKG_VERSION`.
fn parse_triplet(version: &str) -> Option<(u64, u64, u64)> {
    let core = version.split(['-', '+']).next().unwrap_or(version).trim();
    let mut parts = core.split('.');
    let major = parts.next()?.parse().ok()?;
    let minor = parts.next()?.parse().ok()?;
    let patch = parts.next()?.parse().ok()?;
    if parts.next().is_some() {
        return None;
    }
    Some((major, minor, patch))
}

/// Relationship between the recorded `.flow/version` marker and the
/// running Flow binary's `CARGO_PKG_VERSION`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum VersionDelta {
    /// No marker recorded yet (fresh install).
    FirstInstall,
    /// Marker matches the running binary (or one side is unparseable; we
    /// treat that as "same" so garbage markers never strand the user).
    Same,
    /// Running binary is strictly newer than the recorded marker.
    Upgrade,
    /// Running binary is strictly older than the recorded marker.
    Downgrade,
}

/// Classify how the running binary's version relates to the recorded marker.
///
/// Unparseable versions on either side collapse to [`VersionDelta::Same`] so
/// malformed markers never cause refusal or noisy warnings — flow update
/// must never strand a user on a garbage marker.
pub(crate) fn classify(previous: Option<&str>, current: &str) -> VersionDelta {
    let Some(previous) = previous else {
        return VersionDelta::FirstInstall;
    };
    match (parse_triplet(previous), parse_triplet(current)) {
        (Some(prev), Some(curr)) => match prev.cmp(&curr) {
            std::cmp::Ordering::Less => VersionDelta::Upgrade,
            std::cmp::Ordering::Equal => VersionDelta::Same,
            std::cmp::Ordering::Greater => VersionDelta::Downgrade,
        },
        _ => VersionDelta::Same,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_triplet_accepts_plain_semver() {
        assert_eq!(parse_triplet("0.11.2"), Some((0, 11, 2)));
        assert_eq!(parse_triplet("1.0.0"), Some((1, 0, 0)));
    }

    #[test]
    fn parse_triplet_strips_pre_release_and_build_metadata() {
        assert_eq!(parse_triplet("0.11.2-pre.3"), Some((0, 11, 2)));
        assert_eq!(parse_triplet("0.11.2+build.7"), Some((0, 11, 2)));
    }

    #[test]
    fn parse_triplet_rejects_extras_and_garbage() {
        assert_eq!(parse_triplet(""), None);
        assert_eq!(parse_triplet("0.11"), None);
        assert_eq!(parse_triplet("0.11.2.4"), None);
        assert_eq!(parse_triplet("zero.eleven.two"), None);
    }

    #[test]
    fn classify_returns_first_install_when_no_marker_recorded() {
        assert_eq!(classify(None, "0.12.0"), VersionDelta::FirstInstall);
    }

    #[test]
    fn classify_distinguishes_upgrade_same_and_downgrade() {
        assert_eq!(classify(Some("0.11.2"), "0.12.0"), VersionDelta::Upgrade);
        assert_eq!(classify(Some("0.12.0"), "0.12.0"), VersionDelta::Same);
        assert_eq!(classify(Some("0.12.0"), "0.11.2"), VersionDelta::Downgrade);
    }

    #[test]
    fn classify_detects_strictly_older_running_binary() {
        // Regression coverage for the dogfooding incident: a 0.11.2 binary
        // running against a 0.12.0 marker (or 0.99.99 vs 1.0.0) must
        // classify as a downgrade, never as Same or Upgrade.
        assert_eq!(classify(Some("0.11.2"), "0.9.0"), VersionDelta::Downgrade);
        assert_eq!(classify(Some("1.0.0"), "0.99.99"), VersionDelta::Downgrade);
    }

    #[test]
    fn classify_treats_strictly_newer_running_binary_as_upgrade() {
        assert_eq!(classify(Some("0.9.0"), "0.10.0"), VersionDelta::Upgrade);
        assert_eq!(classify(Some("0.9.0"), "1.0.0"), VersionDelta::Upgrade);
    }

    #[test]
    fn classify_ignores_pre_release_suffix_when_core_is_equal() {
        // Core triplets are identical → Same. We deliberately ignore
        // pre-release ordering to keep the comparator dependency-free,
        // which means e.g. `0.11.2-pre` and `0.11.2` are not a downgrade
        // either way.
        assert_eq!(classify(Some("0.11.2-pre"), "0.11.2"), VersionDelta::Same);
        assert_eq!(classify(Some("0.11.2"), "0.11.2-pre"), VersionDelta::Same);
    }

    #[test]
    fn classify_collapses_unparseable_to_same() {
        // Garbage on either side must never strand the user — there is no
        // refusal path through `Same`, so an unparseable marker simply
        // gets refreshed to the running binary's version.
        assert_eq!(classify(Some("garbage"), "0.12.0"), VersionDelta::Same);
        assert_eq!(classify(Some("0.12.0"), "garbage"), VersionDelta::Same);
        assert_eq!(classify(Some("garbage"), "garbage"), VersionDelta::Same);
    }
}
