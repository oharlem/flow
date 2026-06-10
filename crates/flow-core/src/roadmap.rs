//! Flow roadmap mutation.
//!
//! Current roadmaps are user-authored milestone lists. Flow only ticks linked
//! milestone checkboxes on close.

use crate::error::{Error, Result};
use crate::parse::roadmap::{parse_str, Milestone};
use crate::paths;
use once_cell::sync::Lazy;
use regex::Regex;
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::path::Path;

/// Seed the configured roadmap path from the bundled template when absent.
pub fn ensure_roadmap(repo: &Path) -> Result<()> {
    let path = paths::roadmap_path(repo);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    if !path.exists() {
        if let Some(tmpl) = crate::assets::template("roadmap.md.tmpl") {
            std::fs::write(&path, tmpl)?;
        } else {
            std::fs::write(&path, "# Roadmap\n\n## Milestones\n")?;
        }
    }
    Ok(())
}

/// Return a stable short SHA-256 identity for the exact roadmap text.
pub fn fingerprint(text: &str) -> String {
    let digest = Sha256::digest(text.as_bytes());
    let full = format!("{digest:x}");
    format!("sha256:{}", &full[..12])
}

/// Return `true` when the roadmap contains at least one milestone.
pub fn has_milestones(repo: &Path) -> bool {
    let path = readable_roadmap_path(repo);
    has_milestones_at_path(&path)
}

/// Return `true` when the roadmap at `path` contains at least one milestone.
pub fn has_milestones_at_path(path: &Path) -> bool {
    let text = match std::fs::read_to_string(path) {
        Ok(t) => t,
        Err(_) => return false,
    };
    !parse_str(&text).is_empty()
}

/// Return the number of milestones (any state) in the roadmap.
pub fn count_milestones(repo: &Path) -> usize {
    let path = readable_roadmap_path(repo);
    count_milestones_at_path(&path)
}

/// Return the number of milestones (any state) in the roadmap at `path`.
pub fn count_milestones_at_path(path: &Path) -> usize {
    let text = match std::fs::read_to_string(path) {
        Ok(t) => t,
        Err(_) => return 0,
    };
    parse_str(&text).len()
}

/// Return the numeric portion of the highest milestone ID in the roadmap, or
/// `None` when the roadmap is missing or contains no milestones.
pub fn highest_milestone_id(repo: &Path) -> Option<u32> {
    let path = readable_roadmap_path(repo);
    highest_milestone_id_at_path(&path)
}

/// Return the numeric portion of the highest milestone ID in the roadmap at
/// `path`, or `None` when the roadmap is missing or contains no milestones.
pub fn highest_milestone_id_at_path(path: &Path) -> Option<u32> {
    let text = std::fs::read_to_string(path).ok()?;
    parse_str(&text)
        .into_iter()
        .filter_map(|m| milestone_number(&m.id))
        .max()
}

/// Return the first milestone number at or after `start` that is not already
/// present in the roadmap.
pub fn next_available_milestone_number(repo: &Path, start: u32) -> Result<u32> {
    next_available_milestone_number_at_paths([readable_roadmap_path(repo)], start)
}

/// Return the first milestone number at or after `start` that is absent from
/// every roadmap in `paths`.
pub fn next_available_milestone_number_at_paths<I, P>(paths: I, start: u32) -> Result<u32>
where
    I: IntoIterator<Item = P>,
    P: AsRef<Path>,
{
    if start == 0 {
        return Err(Error::User(
            "counter must be a positive integer".to_string(),
        ));
    }
    let used = milestone_numbers_at_paths(paths);
    let mut candidate = start;
    while used.contains(&candidate) {
        candidate = candidate.checked_add(1).ok_or_else(|| {
            Error::User("No milestone IDs remain; counter overflowed".to_string())
        })?;
    }
    Ok(candidate)
}

/// Return true when the roadmap already contains this numeric milestone ID.
pub fn milestone_number_exists(repo: &Path, number: u32) -> bool {
    milestone_numbers_at_paths([readable_roadmap_path(repo)]).contains(&number)
}

/// Return true when any roadmap in `paths` already contains this numeric
/// milestone ID.
pub fn milestone_number_exists_at_paths<I, P>(paths: I, number: u32) -> bool
where
    I: IntoIterator<Item = P>,
    P: AsRef<Path>,
{
    milestone_numbers_at_paths(paths).contains(&number)
}

/// Return the milestone whose ID matches `id`, if any.
pub fn get_milestone(repo: &Path, id: &str) -> Option<Milestone> {
    let path = readable_roadmap_path(repo);
    get_milestone_at_path(&path, id)
}

/// Return the milestone whose ID matches `id` in the roadmap at `path`, if any.
pub fn get_milestone_at_path(path: &Path, id: &str) -> Option<Milestone> {
    let text = std::fs::read_to_string(path).ok()?;
    find_milestone(parse_str(&text), id)
}

/// Return the milestone whose ID matches `id`, failing when the roadmap cannot
/// be read or the milestone is absent.
pub fn require_milestone(repo: &Path, id: &str) -> Result<Milestone> {
    let path = readable_roadmap_path(repo);
    require_milestone_at_path(&path, id)
}

/// Return the milestone whose ID matches `id` in the roadmap at `path`, failing
/// when the roadmap cannot be read or the milestone is absent.
pub fn require_milestone_at_path(path: &Path, id: &str) -> Result<Milestone> {
    let text = std::fs::read_to_string(path).map_err(|e| {
        Error::User(format!(
            "Cannot read {} while resolving milestone {id}: {e}",
            path.display()
        ))
    })?;
    find_milestone(parse_str(&text), id).ok_or_else(|| {
            Error::User(format!(
                "Milestone {id} not found in {}. Run `flow roadmap` to create it, or omit the milestone link.",
                path.display()
            ))
        })
}

/// Return the `**Milestone**:` IDs recorded on `status.md`, or empty.
pub fn status_milestones(feature_dir: &Path) -> Vec<String> {
    let text = match std::fs::read_to_string(feature_dir.join("status.md")) {
        Ok(t) => t,
        Err(_) => return Vec::new(),
    };
    let re = Regex::new(r"(?m)^\*\*Milestone\*\*:\s*(.+)$").unwrap();
    if let Some(c) = re.captures(&text) {
        Regex::new(r"M-\d+")
            .unwrap()
            .find_iter(&c[1])
            .filter_map(|m| milestone_number(m.as_str()).map(|_| m.as_str().to_string()))
            .collect()
    } else {
        Vec::new()
    }
}

/// Write or replace the `**Milestone**:` line on a status.md.
pub fn set_status_milestone(feature_dir: &Path, value: &str) -> Result<()> {
    let path = feature_dir.join("status.md");
    let text = std::fs::read_to_string(&path)?;
    let new_line = format!("**Milestone**: {value}");

    static HAS_MILESTONE: Lazy<Regex> =
        Lazy::new(|| Regex::new(r"(?m)^\*\*Milestone\*\*:.*$").unwrap());
    static HAS_BRANCH: Lazy<Regex> =
        Lazy::new(|| Regex::new(r"(?m)^(\*\*Branch\*\*:.*)$").unwrap());

    let updated = if HAS_MILESTONE.is_match(&text) {
        HAS_MILESTONE.replace(&text, new_line.as_str()).to_string()
    } else if HAS_BRANCH.is_match(&text) {
        HAS_BRANCH
            .replace(&text, format!("$1\n{new_line}").as_str())
            .to_string()
    } else if text.contains("## History") {
        text.replacen("## History", &format!("{new_line}\n\n## History"), 1)
    } else {
        let mut t = text;
        if !t.ends_with('\n') {
            t.push('\n');
        }
        t.push_str(&new_line);
        t.push('\n');
        t
    };
    std::fs::write(&path, updated)?;
    Ok(())
}

/// Tick every milestone referenced by `feature_dir/status.md`.
pub fn tick_milestones(
    feature_dir: &Path,
    repo: &Path,
    feature_link: &str,
    today: &str,
) -> Result<Vec<String>> {
    let path = readable_roadmap_path(repo);
    tick_milestones_at_path(feature_dir, &path, feature_link, today)
}

/// Tick every milestone referenced by `feature_dir/status.md` in the roadmap
/// at `path`.
pub fn tick_milestones_at_path(
    feature_dir: &Path,
    path: &Path,
    _feature_link: &str,
    _today: &str,
) -> Result<Vec<String>> {
    let ids = status_milestones(feature_dir);
    if ids.is_empty() {
        return Ok(Vec::new());
    }
    if !path.exists() {
        return Err(Error::User(format!(
            "Cannot tick milestones because {} does not exist",
            path.display()
        )));
    }
    let mut text = std::fs::read_to_string(path)?;
    let original_text = text.clone();
    let feature_name = feature_dir
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();
    let mut missing = Vec::new();
    for mid in &ids {
        let heading_pattern = format!(
            r"(?m)^(\s*###\s+\[)([ xX~])(\]\s+{}\s*:?\s*.*?)(\r?\n?)$",
            regex::escape(mid)
        );
        let heading_re = Regex::new(&heading_pattern)?;
        if let Some(caps) = heading_re.captures(&text) {
            if in_code_fence(&text, caps.get(0).unwrap().start()) {
                return Err(Error::User(format!(
                    "Milestone {mid} referenced by {feature_name} is inside a code fence in {}",
                    path.display()
                )));
            }
            if caps[2].eq_ignore_ascii_case("x") {
                continue;
            }
            let m = caps.get(0).unwrap();
            let replacement = format!("{}x{}{}", &caps[1], &caps[3], &caps[4]);
            let mut new_text = String::with_capacity(text.len());
            new_text.push_str(&text[..m.start()]);
            new_text.push_str(&replacement);
            new_text.push_str(&text[m.end()..]);
            text = new_text;
            continue;
        }
        missing.push(mid.clone());
    }
    ensure_milestone_count_not_decreased(&original_text, &text)?;
    if !missing.is_empty() {
        return Err(Error::User(format!(
            "Milestone(s) referenced by {feature_name} not found in {}: {}",
            path.display(),
            missing.join(", ")
        )));
    }
    std::fs::write(path, text)?;
    Ok(missing)
}

fn readable_roadmap_path(repo: &Path) -> std::path::PathBuf {
    paths::roadmap_path(repo)
}

fn find_milestone(milestones: Vec<Milestone>, id: &str) -> Option<Milestone> {
    milestones.into_iter().find(|milestone| milestone.id == id)
}

pub fn milestone_numbers_at_paths<I, P>(paths: I) -> BTreeSet<u32>
where
    I: IntoIterator<Item = P>,
    P: AsRef<Path>,
{
    let mut numbers = BTreeSet::new();
    for path in paths {
        let text = match std::fs::read_to_string(path.as_ref()) {
            Ok(t) => t,
            Err(_) => continue,
        };
        numbers.extend(
            parse_str(&text)
                .into_iter()
                .filter_map(|m| milestone_number(&m.id)),
        );
    }
    numbers
}

fn milestone_number(id: &str) -> Option<u32> {
    let raw = id.strip_prefix("M-")?;
    if raw.starts_with('0') {
        return None;
    }
    let number = raw.parse::<u32>().ok()?;
    if number == 0 {
        return None;
    }
    Some(number)
}

fn in_code_fence(text: &str, offset: usize) -> bool {
    let mut fence = false;
    let mut pos = 0usize;
    for line in text.split_inclusive('\n') {
        if pos >= offset {
            break;
        }
        if line.trim_start().starts_with("```") {
            fence = !fence;
        }
        pos += line.len();
    }
    fence
}

fn ensure_milestone_count_not_decreased(before: &str, after: &str) -> Result<()> {
    let before_count = parse_str(before).len();
    let after_count = parse_str(after).len();
    if after_count < before_count {
        return Err(Error::User(format!(
            "refusing to update roadmap because the milestone count would decrease from {before_count} to {after_count}"
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn make_repo(roadmap: &str) -> TempDir {
        let td = TempDir::new().unwrap();
        let flow = td.path().join("flow");
        std::fs::create_dir_all(&flow).unwrap();
        std::fs::write(flow.join("roadmap.md"), roadmap).unwrap();
        td
    }

    #[test]
    fn gets_milestone() {
        let td = make_repo("# Roadmap\n\n## Milestones\n\n### [ ] M-1: One\n\nDesc.\n");
        let m = get_milestone(td.path(), "M-1").unwrap();
        assert_eq!(m.title, "One");
    }

    #[test]
    fn t003_fingerprint_is_sha256_short_stable_and_change_sensitive() {
        let text = "hello\n";
        let first = fingerprint(text);
        let second = fingerprint(text);

        assert_eq!(first, "sha256:5891b5b522d5");
        assert_eq!(first, second);
        assert_ne!(first, fingerprint("hello"));
    }

    #[test]
    fn t003_count_milestones_empty_or_missing_returns_zero() {
        let td = TempDir::new().unwrap();
        // Missing file.
        assert_eq!(count_milestones(td.path()), 0);
        // Empty roadmap.
        let flow = td.path().join("flow");
        std::fs::create_dir_all(&flow).unwrap();
        std::fs::write(flow.join("roadmap.md"), "# Roadmap\n\n## Milestones\n").unwrap();
        assert_eq!(count_milestones(td.path()), 0);
    }

    #[test]
    fn t003_count_milestones_counts_all_states() {
        let td = make_repo(
            "# Roadmap\n\n## Milestones\n\n### [ ] M-1: One\n\n### [~] M-2: Two\n\n### [x] M-3: Three\n",
        );
        assert_eq!(count_milestones(td.path()), 3);
    }

    #[test]
    fn t003_highest_milestone_id_returns_max() {
        let td = make_repo(
            "# Roadmap\n\n## Milestones\n\n### [ ] M-1: One\n\n### [~] M-5: Five\n\n### [x] M-3: Three\n",
        );
        assert_eq!(highest_milestone_id(td.path()), Some(5));
    }

    #[test]
    fn t003_next_available_uses_counter_and_skips_collisions() {
        let td =
            make_repo("# Roadmap\n\n## Milestones\n\n### [ ] M-1: One\n\n### [x] M-3: Three\n");
        assert_eq!(next_available_milestone_number(td.path(), 1).unwrap(), 2);
        assert_eq!(next_available_milestone_number(td.path(), 3).unwrap(), 4);
    }

    #[test]
    fn t003_highest_milestone_id_returns_none_when_empty() {
        let td = TempDir::new().unwrap();
        assert_eq!(highest_milestone_id(td.path()), None);
        let flow = td.path().join("flow");
        std::fs::create_dir_all(&flow).unwrap();
        std::fs::write(flow.join("roadmap.md"), "# Roadmap\n\n## Milestones\n").unwrap();
        assert_eq!(highest_milestone_id(td.path()), None);
    }

    #[test]
    fn ticks_milestone_once() {
        let td = make_repo("# Roadmap\n\n## Milestones\n\n### [ ] M-1: One\n\nDesc.\n");
        let feat = td.path().join("flow").join("M-1-foo");
        std::fs::create_dir_all(&feat).unwrap();
        std::fs::write(
            feat.join("status.md"),
            "**Change**: M-1-foo\n**Milestone**: M-1\n## History\n\n- 2026-01-01 — done — ok\n",
        )
        .unwrap();
        let missing = tick_milestones(&feat, td.path(), "", "2026-01-02").unwrap();
        assert!(missing.is_empty());
        let contents = std::fs::read_to_string(td.path().join("flow/roadmap.md")).unwrap();
        assert!(contents.contains("### [x] M-1: One"));
        assert!(contents.contains("Desc."));
    }

    #[test]
    fn t004_ticks_heading_milestone_without_losing_body() {
        let td = make_repo("# Roadmap\n\n## Milestones\n\n### [~] M-1: One\n\n#### Description\n\nKeep **all** of this.\n\n### [ ] M-2: Two\n");
        let feat = td.path().join("flow").join("M-1-foo");
        std::fs::create_dir_all(&feat).unwrap();
        std::fs::write(
            feat.join("status.md"),
            "**Change**: M-1-foo\n**Milestone**: M-1\n## History\n\n- 2026-01-01 — done — ok\n",
        )
        .unwrap();
        let missing = tick_milestones(&feat, td.path(), "", "2026-01-02").unwrap();
        assert!(missing.is_empty());
        let contents = std::fs::read_to_string(td.path().join("flow/roadmap.md")).unwrap();
        assert!(contents.contains(
            "### [x] M-1: One\n\n#### Description\n\nKeep **all** of this.\n\n### [ ] M-2: Two"
        ));
    }

    #[test]
    fn t002_tick_milestones_requires_exact_unpadded_id() {
        let td = make_repo("# Roadmap\n\n## Milestones\n\n### [~] M-1: One\n\nBody.\n");
        let feat = td.path().join("flow").join("M-1-foo");
        std::fs::create_dir_all(&feat).unwrap();
        std::fs::write(
            feat.join("status.md"),
            "**Change**: M-1-foo\n**Milestone**: M-1\n## History\n\n- 2026-01-01 — done — ok\n",
        )
        .unwrap();
        tick_milestones(&feat, td.path(), "./archive/M-1-foo/", "2026-01-02").unwrap();
        let contents = std::fs::read_to_string(td.path().join("flow/roadmap.md")).unwrap();
        assert!(contents.contains("### [x] M-1: One"), "{contents}");
    }

    #[test]
    fn refuses_roadmap_update_that_drops_milestones() {
        let before = "# Roadmap\n\n## Milestones\n\n### [ ] M-1: One\n\n### [ ] M-2: Two\n";
        let after = "# Roadmap\n\n## Milestones\n\n### [ ] M-1: One\n";

        let err = ensure_milestone_count_not_decreased(before, after).unwrap_err();

        assert!(err.to_string().contains("milestone count would decrease"));
    }
}
