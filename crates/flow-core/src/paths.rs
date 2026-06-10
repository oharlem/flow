//! Path resolution for Flow repositories.

use crate::config::Config;
use crate::error::{Error, Result};
use std::path::{Path, PathBuf};
use std::process::Command;

/// Default visible Flow artifact root and branch namespace.
pub const DEFAULT_PREFIX: &str = "flow";

/// Return the absolute path of the git repository containing `start`, or the
/// current working directory when `start` is `None`.
pub fn repo_root(start: Option<&Path>) -> Result<PathBuf> {
    let mut cmd = Command::new("git");
    cmd.args(["rev-parse", "--show-toplevel"]);
    if let Some(dir) = start {
        cmd.current_dir(dir);
    }
    let out = cmd.output().map_err(|_| Error::NotAGitRepository)?;
    if !out.status.success() {
        return Err(Error::NotAGitRepository);
    }
    let line = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if line.is_empty() {
        return Err(Error::NotAGitRepository);
    }
    Ok(PathBuf::from(line))
}

/// Return the `.flow/` directory for a repo, creating nothing.
#[must_use]
pub fn flow_dir(repo: &Path) -> PathBuf {
    repo.join(".flow")
}

/// Return the configured visible Flow artifact root for a repo.
#[must_use]
pub fn work_dir(repo: &Path) -> PathBuf {
    repo.join(prefix(repo))
}

/// Return `<prefix>/roadmap.md`.
#[must_use]
pub fn roadmap_path(repo: &Path) -> PathBuf {
    layout(repo).roadmap_path
}

/// Return the durable run artifact root. (T-014)
#[must_use]
pub fn runs_dir(repo: &Path) -> PathBuf {
    layout(repo).runs_dir
}

/// Return the default Flow-maintained documentation directory. (T-006)
#[must_use]
pub fn documentation_dir(repo: &Path) -> PathBuf {
    layout(repo).documentation_dir
}

/// Return the preferred on-disk conventions shard directory.
///
/// The canonical location is `.flow/conventions/`.
#[must_use]
pub fn conventions_dir(repo: &Path) -> PathBuf {
    flow_dir(repo).join("conventions")
}

/// Return the preferred on-disk path list for a single conventions shard
/// (`"core"`, `"spec"`, `"plan"`, `"build"`, `"test"`, `"close"`, `"run"`).
///
/// Currently the list contains a single path (`<repo>/.flow/conventions/<name>.md`).
/// The `Vec` shape is kept so the composer can fall back to additional
/// locations in the future without changing the public API.
#[must_use]
pub fn conventions_shard_paths(repo: &Path, shard_name: &str) -> Vec<PathBuf> {
    vec![conventions_dir(repo).join(format!("{shard_name}.md"))]
}

/// Resolved installed artifact layout for a repository. (T-001)
#[derive(Clone, Debug)]
pub struct LayoutPaths {
    pub workspace_dir: PathBuf,
    pub runs_dir: PathBuf,
    pub roadmap_path: PathBuf,
    pub documentation_dir: PathBuf,
    pub conventions_dir: PathBuf,
}

/// Resolve Flow's installed artifact layout from `.flow/config.yaml`. (T-001)
#[must_use]
pub fn layout(repo: &Path) -> LayoutPaths {
    let cfg = Config::load_for_repo(repo).unwrap_or_default();
    let prefix = cfg.prefix;
    let workspace = repo.join(&prefix);
    let conventions_dir = repo.join(".flow").join("conventions");
    LayoutPaths {
        workspace_dir: workspace.clone(),
        runs_dir: workspace.join("runs"),
        roadmap_path: workspace.join("roadmap.md"),
        documentation_dir: cfg
            .docs
            .documentation_path
            .map_or_else(|| workspace.join("docs"), |p| repo.join(p)),
        conventions_dir,
    }
}

/// Return the configured visible Flow prefix.
#[must_use]
pub fn prefix(repo: &Path) -> String {
    Config::load_for_repo(repo)
        .map(|config| config.prefix)
        .unwrap_or_else(|_| DEFAULT_PREFIX.to_string())
}

/// Validate a user-supplied Flow prefix.
pub fn validate_prefix(input: &str) -> Result<String> {
    let prefix = input.trim().trim_matches('"').trim_matches('\'');
    if prefix.is_empty() {
        return Err(Error::User(
            "prefix must be a non-empty repo-root directory name".into(),
        ));
    }
    if prefix.starts_with('.') {
        return Err(Error::User(
            "prefix must be visible; hidden directories such as .flow are reserved for Flow internals"
                .into(),
        ));
    }
    if prefix.contains('/')
        || prefix.contains('\\')
        || prefix == ".."
        || prefix == "."
        || Path::new(prefix).is_absolute()
    {
        return Err(Error::User(
            "prefix must be a single repo-root directory name, e.g. prefix=flow".into(),
        ));
    }
    if matches!(prefix, "docs" | "src" | "tests" | "target" | "node_modules") {
        return Err(Error::User(format!(
            "prefix {prefix:?} is reserved for common project content; choose a Flow-specific name"
        )));
    }
    Ok(prefix.to_string())
}

/// Return the branch name for a Flow feature.
#[must_use]
pub fn branch_name(repo: &Path, feature_name: &str) -> String {
    format!("{}/{}", prefix(repo), feature_name)
}

/// Slugify a free-form change description into a kebab-case token
/// (max 60 chars; trimmed at word boundary).
#[must_use]
pub fn slugify(desc: &str) -> String {
    let lower = desc.to_lowercase();
    let mut out = String::with_capacity(lower.len());
    let mut last_hyphen = true;
    for c in lower.chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c);
            last_hyphen = false;
        } else if !last_hyphen {
            out.push('-');
            last_hyphen = true;
        }
    }
    while out.ends_with('-') {
        out.pop();
    }
    while out.starts_with('-') {
        out.remove(0);
    }
    if out.len() > 60 {
        let mut cut = 60;
        if let Some(idx) = out[..60].rfind('-') {
            cut = idx;
        }
        out.truncate(cut);
    }
    if out.is_empty() {
        "feature".to_string()
    } else {
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slugify_basic() {
        assert_eq!(slugify("Hello World!"), "hello-world");
        assert_eq!(slugify("add  login  form"), "add-login-form");
    }

    #[test]
    fn slugify_length_cap() {
        let long = "a".repeat(100);
        let s = slugify(&long);
        assert!(s.len() <= 60);
    }

    // T-017: missing layout config still resolves canonical v2 paths.
    #[test]
    fn t017_default_layout_paths_are_canonical_v2() {
        let td = tempfile::TempDir::new().unwrap();
        let repo = td.path();
        let l = layout(repo);
        assert_eq!(l.runs_dir, repo.join("flow").join("runs"));
        assert_eq!(l.roadmap_path, repo.join("flow").join("roadmap.md"));
        assert_eq!(l.documentation_dir, repo.join("flow").join("docs"));
    }

    // T-001/T-006: layout v2 resolves changes and documentation.
    #[test]
    fn t001_v2_layout_paths() {
        let td = tempfile::TempDir::new().unwrap();
        let repo = td.path();
        std::fs::create_dir_all(repo.join(".flow")).unwrap();
        std::fs::write(
            repo.join(".flow").join("config.yaml"),
            "prefix: product-flow\nlayout:\n  version: 2\n",
        )
        .unwrap();
        let l = layout(repo);
        assert_eq!(l.workspace_dir, repo.join("product-flow"));
        assert_eq!(l.runs_dir, repo.join("product-flow").join("runs"));
        assert_eq!(l.roadmap_path, repo.join("product-flow").join("roadmap.md"));
        assert_eq!(l.documentation_dir, repo.join("product-flow").join("docs"));
    }

    // T-006: docs.documentation_path overrides the default documentation dir.
    #[test]
    fn t006_documentation_path_override() {
        let td = tempfile::TempDir::new().unwrap();
        let repo = td.path();
        std::fs::create_dir_all(repo.join(".flow")).unwrap();
        std::fs::write(
            repo.join(".flow").join("config.yaml"),
            "layout:\n  version: 2\ndocs:\n  documentation_path: docs/flow\n",
        )
        .unwrap();
        assert_eq!(documentation_dir(repo), repo.join("docs").join("flow"));
    }

    // T-010: conventions shard dir resolves to hidden control plane on both layouts.
    #[test]
    fn t010_conventions_dir_is_hidden_flow_subdir() {
        let td = tempfile::TempDir::new().unwrap();
        let repo = td.path();
        assert_eq!(
            conventions_dir(repo),
            repo.join(".flow").join("conventions")
        );
    }

    // T-010: per-shard path resolution points into the shard dir.
    #[test]
    fn t010_conventions_shard_paths_list_single_path() {
        let td = tempfile::TempDir::new().unwrap();
        let repo = td.path();
        let list = conventions_shard_paths(repo, "core");
        assert_eq!(list.len(), 1);
        assert_eq!(
            list[0],
            repo.join(".flow").join("conventions").join("core.md")
        );
    }

    // T-010: layout exposes conventions_dir (no more visible_conventions_path).
    #[test]
    fn t010_layout_exposes_conventions_dir() {
        let td = tempfile::TempDir::new().unwrap();
        let repo = td.path();
        std::fs::create_dir_all(repo.join(".flow")).unwrap();
        std::fs::write(
            repo.join(".flow").join("config.yaml"),
            "layout:\n  version: 2\n",
        )
        .unwrap();
        let l = layout(repo);
        assert_eq!(l.conventions_dir, repo.join(".flow").join("conventions"));
    }
}
