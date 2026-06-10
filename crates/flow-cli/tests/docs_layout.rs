//! Working-tree guard tests for docs layout and stale doc-path references.
//!
//! Walks every Markdown file in the repo and fails on references to retired
//! doc paths (`docs/cli-reference.md`, `docs/commands/`, `docs/artifacts/`).
//! Also asserts the required reference and ADR files exist and that every
//! `docs/**/*.md` file is referenced from `docs/SUMMARY.md`.

use std::path::{Path, PathBuf};
use walkdir::WalkDir;

const FORBIDDEN_SUBSTRINGS: &[&str] = &[
    "docs/cli-reference.md",
    "docs/commands/",
    "docs/artifacts/",
    "./commands/",
    "./artifacts/",
    "flow/runs/<date-or-timestamp-title>",
    "flow/runs/<timestamp",
    "docs/_record.md",
    "docs/reviewer-brief.md",
    "docs/why-this-matters-for-engineering-orgs.md",
    "docs/workflow-model.md",
    "docs/tradeoffs.md",
    "./_record.md",
    "./reviewer-brief.md",
    "./why-this-matters-for-engineering-orgs.md",
    "./workflow-model.md",
    "./tradeoffs.md",
];

const RETIRED_PUBLIC_TERMS: &[&str] = &[
    "OpenCode",
    "opencode",
    "flow learn",
    "flow next",
    "flow-learn",
    "flow-next",
    "learnings.md",
    "qa.md",
    "manual-qa",
    "manual QA",
    "QA-",
    "D7",
    "D8",
];

fn repo_root() -> PathBuf {
    // The workspace root is the parent of CARGO_MANIFEST_DIR's grand-parent
    // (crates/flow-cli/Cargo.toml → workspace root).
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest_dir
        .parent()
        .and_then(|p| p.parent())
        .map(Path::to_path_buf)
        .expect("expected workspace root above crates/flow-cli")
}

fn is_exempt(path: &Path, root: &Path) -> bool {
    let rel = match path.strip_prefix(root) {
        Ok(r) => r,
        Err(_) => return true,
    };
    let rel_str = rel.to_string_lossy().replace('\\', "/");
    if rel_str.starts_with("flow/") {
        return true;
    }
    if rel_str.starts_with("target/") || rel_str.starts_with(".git/") {
        return true;
    }
    false
}

#[test]
fn t011_no_links_to_deleted_doc_paths() {
    let root = repo_root();
    let mut violations: Vec<(PathBuf, String)> = Vec::new();

    for entry in WalkDir::new(&root)
        .into_iter()
        .filter_entry(|e| {
            let name = e.file_name().to_string_lossy();
            if e.depth() == 0 {
                return true;
            }
            !(name == "target" || name == ".git" || name == "node_modules")
        })
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .filter(|e| e.path().extension().map(|x| x == "md").unwrap_or(false))
    {
        let path = entry.path();
        if is_exempt(path, &root) {
            continue;
        }
        let text = match std::fs::read_to_string(path) {
            Ok(t) => t,
            Err(_) => continue,
        };
        for needle in FORBIDDEN_SUBSTRINGS {
            if text.contains(needle) {
                violations.push((path.to_path_buf(), (*needle).to_string()));
            }
        }
    }

    assert!(
        violations.is_empty(),
        "found references to deleted Phase B doc paths:\n{}",
        violations
            .iter()
            .map(|(p, n)| format!("  {} → {}", p.display(), n))
            .collect::<Vec<_>>()
            .join("\n")
    );
}

#[test]
fn m30_retired_public_surface_terms_are_gone() {
    let root = repo_root();
    let mut violations: Vec<(PathBuf, String)> = Vec::new();

    for entry in WalkDir::new(&root)
        .into_iter()
        .filter_entry(|e| {
            let name = e.file_name().to_string_lossy();
            if e.depth() == 0 {
                return true;
            }
            !(name == "target" || name == ".git" || name == "node_modules" || name == "book")
        })
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .filter(|e| {
            matches!(
                e.path().extension().and_then(|x| x.to_str()),
                Some("md" | "rs" | "tmpl" | "snap" | "yaml" | "toml")
            )
        })
    {
        let path = entry.path();
        let rel = match path.strip_prefix(&root) {
            Ok(rel) => rel.to_string_lossy().replace('\\', "/"),
            Err(_) => continue,
        };
        if rel.starts_with("target/") || rel.starts_with(".git/") {
            continue;
        }
        if rel == "crates/flow-cli/tests/docs_layout.rs" {
            continue;
        }
        let text = match std::fs::read_to_string(path) {
            Ok(t) => t,
            Err(_) => continue,
        };
        for needle in RETIRED_PUBLIC_TERMS {
            if text.contains(needle) {
                violations.push((path.to_path_buf(), (*needle).to_string()));
            }
        }
    }

    assert!(
        violations.is_empty(),
        "found references to retired Flow public surface:\n{}",
        violations
            .iter()
            .map(|(p, n)| format!("  {} -> {}", p.display(), n))
            .collect::<Vec<_>>()
            .join("\n")
    );
}

#[test]
fn t004_adr_0009_exists() {
    let root = repo_root();
    let path = root.join("docs/decisions/0009-documentation-architecture.md");
    let content = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("expected ADR 0009 at {}: {e}", path.display()));
    assert!(content.contains("**Status**: Accepted"));
}

#[test]
fn t004_top_level_specs_directory_is_retired() {
    // Covers: T-004.
    let root = repo_root();
    assert!(
        !root.join("specs").exists(),
        "top-level specs/ is retired; current Flow artifacts live under flow/"
    );
}

#[test]
fn t011_new_reference_files_exist() {
    // Covers task IDs:
    //   T-002 — `docs/reference/cli.md` is the new generated CLI reference.
    //   T-003 — `docs/reference/commands.md` owns command intent.
    //   T-004 — `docs/reference/artifacts.md` is the artifact orientation page.
    let root = repo_root();
    for required in [
        "docs/reference/cli.md",
        "docs/reference/commands.md",
        "docs/reference/artifacts.md",
    ] {
        let p = root.join(required);
        assert!(
            p.is_file(),
            "expected Phase B reference file to exist: {}",
            p.display()
        );
    }
}

#[test]
fn t011_old_doc_paths_are_gone() {
    // Covers task IDs:
    //   T-005 — Phase B deletes `docs/commands/` and `docs/artifacts/`.
    //   T-002 — Phase B deletes `docs/cli-reference.md`.
    let root = repo_root();
    for gone in [
        "docs/cli-reference.md",
        "docs/commands",
        "docs/artifacts",
        "docs/_record.md",
        "docs/reviewer-brief.md",
        "docs/why-this-matters-for-engineering-orgs.md",
        "docs/workflow-model.md",
        "docs/tradeoffs.md",
    ] {
        let p = root.join(gone);
        assert!(
            !p.exists(),
            "expected Phase B to have removed: {}",
            p.display()
        );
    }
}

#[test]
fn workflow_diagram_path_is_current() {
    let root = repo_root();
    let current = root.join("docs/flow-main-workflow-v0.1.0.png");
    assert!(
        current.is_file(),
        "expected workflow diagram to exist: {}",
        current.display()
    );

    let mut stale_refs: Vec<(PathBuf, String)> = Vec::new();
    for entry in WalkDir::new(&root)
        .into_iter()
        .filter_entry(|e| {
            let name = e.file_name().to_string_lossy();
            if e.depth() == 0 {
                return true;
            }
            !(name == "target" || name == ".git" || name == "node_modules")
        })
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .filter(|e| e.path().extension().map(|x| x == "md").unwrap_or(false))
    {
        let path = entry.path();
        if is_exempt(path, &root) {
            continue;
        }
        let text = match std::fs::read_to_string(path) {
            Ok(t) => t,
            Err(_) => continue,
        };
        for stale in [
            "docs/workflow.png",
            "./docs/workflow.png",
            "../workflow.png",
        ] {
            if text.contains(stale) {
                stale_refs.push((path.to_path_buf(), stale.to_string()));
            }
        }
    }

    assert!(
        stale_refs.is_empty(),
        "found references to the retired workflow diagram path:\n{}",
        stale_refs
            .iter()
            .map(|(p, n)| format!("  {} -> {}", p.display(), n))
            .collect::<Vec<_>>()
            .join("\n")
    );
}

#[test]
fn mermaid_workflow_diagrams_match_command_model() {
    let root = repo_root();
    let readme = std::fs::read_to_string(root.join("README.md")).expect("README.md");
    let commands =
        std::fs::read_to_string(root.join("docs/reference/commands.md")).expect("commands.md");
    let architecture =
        std::fs::read_to_string(root.join("docs/architecture.md")).expect("architecture.md");

    assert!(
        readme.contains("flow run --finalize<br/>complete run handoff"),
        "README workflow diagram must show run finalization after change closeout"
    );
    assert!(
        readme.contains("roadmap: next milestone"),
        "README workflow diagram must show roadmap runs can loop after closeout"
    );
    assert!(
        commands.contains("runfinal[\"run --finalize<br/>complete run handoff\"]"),
        "command map must show the terminal run finalize command"
    );
    assert!(
        architecture.contains("flow build / build-task<br/>code changes + task acceptance"),
        "artifact flow diagram must include build before verification"
    );
    assert!(
        architecture.contains("Checkpoint commits enabled?"),
        "run automation diagram must route optional checkpoints before resuming/finalizing"
    );
}

/// Anchor test that names every Phase B task this file's tests cover, so the
/// drift scanner finds the IDs without the assertions duplicating what the
/// other tests already check.
#[test]
fn phase_b_d8_anchor() {
    // T-002 — `docs/reference/cli.md` exists and `docs/cli-reference.md` is gone.
    // T-003 — `docs/reference/commands.md` exists.
    // T-004 — `docs/reference/artifacts.md` exists.
    // T-005 — `docs/commands/` and `docs/artifacts/` directories are gone.
    // T-006 — no inbound Markdown link references the deleted paths.
    // T-007 — retired narrative docs are swept by the link-guard.
    // T-008 — `docs/SUMMARY.md` is swept by the link-guard.
    // T-011 — this file is the workspace-level regression test.
    // T-012 — this file participates in the workspace green-bar.
    let root = repo_root();
    assert!(root.join("docs").join("reference").is_dir());
}

type StructuralFact<'a> = (&'a str, &'a [&'a [&'a str]]);

macro_rules! fact {
    ($label:expr, [$([$($token:expr),+ $(,)?]),+ $(,)?]) => {
        ($label, &[$(&[$($token),+][..]),+][..])
    };
}

fn assert_structural_facts(rel: &str, text: &str, facts: &[StructuralFact<'_>]) {
    for (label, alternatives) in facts {
        let matched = alternatives
            .iter()
            .any(|tokens| tokens.iter().all(|token| text.contains(token)));
        assert!(
            matched,
            "expected {rel} to document {label}; accepted token groups:\n{}",
            alternatives
                .iter()
                .map(|tokens| format!("  - {}", tokens.join(" + ")))
                .collect::<Vec<_>>()
                .join("\n")
        );
    }
}

fn assert_doc_facts(root: &Path, rel: &str, facts: &[StructuralFact<'_>]) {
    let path = root.join(rel);
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("expected {}: {e}", path.display()));
    assert_structural_facts(rel, &text, facts);
}

#[test]
fn m9_structural_fact_matcher_accepts_paraphrased_install_and_export_commands() {
    // T-001 / T-003 / T-004: structural docs facts should tolerate prose
    // rewrites while still requiring the underlying documented evidence.
    let text = "\
Generated host assets invoke the installed `flow` binary with `FLOW_HOST`.
Use `flow export-assets --dir ./tmp/defaults` to inspect embedded defaults.
";

    assert_structural_facts(
        "synthetic.md",
        text,
        &[
            fact!(
                "installed binary is used by host assets",
                [["installed", "flow", "FLOW_HOST"]]
            ),
            fact!(
                "export-assets command exposes a caller-selected directory",
                [
                    ["flow export-assets --dir <DIR>"],
                    ["flow", "export-assets", "--dir"],
                ]
            ),
        ],
    );
}

#[test]
#[should_panic(expected = "export-assets command exposes a caller-selected directory")]
fn m9_structural_fact_matcher_rejects_missing_required_fact() {
    // T-001 / T-003: structural matching must still fail when a required
    // documentation fact disappears rather than merely being paraphrased.
    assert_structural_facts(
        "synthetic.md",
        "Generated host assets invoke installed flow.",
        &[fact!(
            "export-assets command exposes a caller-selected directory",
            [
                ["flow export-assets --dir <DIR>"],
                ["flow", "export-assets", "--dir"],
            ]
        )],
    );
}

#[test]
fn m2_preserved_artifacts_are_discoverable() {
    // Covers M-2 task IDs:
    //   T-001 — the artifact index orients readers to current records.
    //   T-002 — these assertions check representative current artifact locations.
    //   T-003 — this test runs in the workspace verification gate.
    let root = repo_root();

    for required_dir in ["flow/docs", "flow/runs"] {
        let path = root.join(required_dir);
        assert!(
            path.is_dir(),
            "expected current artifact directory to exist: {}",
            path.display()
        );
    }

    for required_file in [
        "assets/conventions/core.md",
        "assets/conventions/spec.md",
        "assets/conventions/build.md",
        "assets/conventions/close.md",
        "assets/conventions/roadmap.md",
        "flow/docs/artifact-index.md",
        "flow/runs/.gitkeep",
    ] {
        let path = root.join(required_file);
        assert!(
            path.is_file(),
            "expected current artifact file to exist: {}",
            path.display()
        );
    }
}

#[test]
fn m3_structure_guidance_links_related_pages() {
    // Covers M-3 task IDs and M-9 T-002:
    //   T-001 — directory layout docs include concrete placement guidance.
    //   T-002 — Flow guidance links related Flow and reference pages.
    //   T-003 — this assertion verifies the guidance link contract.
    //   T-004 — this test runs in the workspace verification gate.
    let root = repo_root();
    let layout_path = root.join("flow/docs/directory-layout.md");
    let index_path = root.join("flow/docs/artifact-index.md");
    let layout = std::fs::read_to_string(&layout_path)
        .unwrap_or_else(|e| panic!("expected {}: {e}", layout_path.display()));
    let index = std::fs::read_to_string(&index_path)
        .unwrap_or_else(|e| panic!("expected {}: {e}", index_path.display()));
    for forbidden in ["date-or-timestamp-title", "flow/runs/<timestamp"] {
        assert!(
            !layout.contains(forbidden),
            "flow/docs/directory-layout.md should use date-plus-slug run paths, not {forbidden}"
        );
        assert!(
            !index.contains(forbidden),
            "flow/docs/artifact-index.md should use date-plus-slug run paths, not {forbidden}"
        );
    }

    assert_structural_facts(
        "flow/docs/directory-layout.md",
        &layout,
        &[
            fact!("placement guide section", [["## Placement Guide"]]),
            fact!(
                "child change location",
                [["flow/runs/<run>/changes/<change>/"]]
            ),
            fact!(
                "run handoff location",
                [["flow/runs/YYYYMMDD-roadmap-<run-slug>/"]]
            ),
            fact!(
                "Flow docs and application docs roots",
                [["flow/docs/", "docs/"]]
            ),
            fact!(
                "structure-only boundary for run records",
                [
                    ["Do not duplicate Flow artifacts", "command logic"],
                    ["flow/runs/", "run history"],
                ]
            ),
            fact!("artifact index link", [["artifact-index.md"]]),
            fact!(
                "application artifact reference link",
                [["../../docs/reference/artifacts.md"]]
            ),
            fact!(
                "application command reference link",
                [["../../docs/reference/commands.md"]]
            ),
        ],
    );

    assert_structural_facts(
        "flow/docs/artifact-index.md",
        &index,
        &[
            fact!("directory layout link", [["directory-layout.md"]]),
            fact!(
                "application artifact reference link",
                [["../../docs/reference/artifacts.md"]]
            ),
        ],
    );
}

#[test]
fn m4_non_logic_verification_gate_is_documented() {
    // Covers M-4 task IDs and M-9 T-002:
    //   T-001 — this assertion protects the verification gate contract.
    //   T-002 — the Flow docs page must describe and link the gate.
    //   T-003 — the documented commands are the final verification suite.
    //   T-004 — the docs preserve the run evidence a future maintainer needs.
    let root = repo_root();
    let gate_path = root.join("flow/docs/non-logic-verification.md");
    let index_path = root.join("flow/docs/artifact-index.md");
    assert!(
        gate_path.is_file(),
        "expected non-logic verification guide to exist: {}",
        gate_path.display()
    );

    let gate = std::fs::read_to_string(&gate_path)
        .unwrap_or_else(|e| panic!("expected {}: {e}", gate_path.display()));
    let index = std::fs::read_to_string(&index_path)
        .unwrap_or_else(|e| panic!("expected {}: {e}", index_path.display()));

    assert_structural_facts(
        "flow/docs/non-logic-verification.md",
        &gate,
        &[
            fact!(
                "format gate command",
                [["cargo", "fmt", "--all", "--check"]]
            ),
            fact!(
                "workspace test gate command",
                [["cargo", "test", "--workspace"]]
            ),
            fact!(
                "clippy gate command",
                [[
                    "cargo",
                    "clippy",
                    "--workspace",
                    "--all-targets",
                    "-D warnings"
                ]]
            ),
            fact!(
                "runtime diff-scope command",
                [[
                    "git diff --name-only",
                    "Cargo.toml",
                    "Cargo.lock",
                    "crates/flow-cli/src",
                    "crates/flow-core/src",
                    "crates/flow-host-claude-code/src",
                    "assets",
                ]]
            ),
            fact!(
                "directory-only exclusions",
                [[
                    "runtime source",
                    "package manifest",
                    "lockfile",
                    "embedded asset"
                ]]
            ),
            fact!(
                "git safety boundary",
                [[
                    "commits",
                    "tags",
                    "remote git operations",
                    "destructive git commands"
                ]]
            ),
            fact!("latest run evidence", [["2026-05-10"]]),
        ],
    );

    assert_structural_facts(
        "flow/docs/artifact-index.md",
        &index,
        &[fact!(
            "non-logic verification guide link",
            [["non-logic-verification.md"]]
        )],
    );
}

#[test]
fn m5_install_model_docs_are_aligned() {
    // Covers M-5 task IDs and M-9 T-003:
    //   T-001 — deterministic documentation guard for the install model.
    //   T-003 — host and safety guardrail documentation remains discoverable.
    //   T-004 — stale release and install docs are caught here.
    let root = repo_root();
    assert_doc_facts(
        &root,
        "README.md",
        &[
            fact!(
                "Cargo Git install command",
                [["cargo install --git https://github.com/oharlem/flow --locked flow-cli"]]
            ),
            fact!("early prototype status", [["early prototype"]]),
            fact!("rustup prerequisite", [["rustup"]]),
            fact!(
                "embedded asset export command",
                [
                    ["flow export-assets --dir <DIR>"],
                    ["flow", "export-assets", "--dir"]
                ]
            ),
        ],
    );

    assert_doc_facts(
        &root,
        "docs/security.md",
        &[
            fact!(
                "cargo bin directory hosts the executable",
                [["Cargo's bin directory"]]
            ),
            fact!(
                "embedded asset export command",
                [
                    ["flow export-assets --dir <DIR>"],
                    ["flow", "export-assets", "--dir"]
                ]
            ),
            fact!("host adapter install link", [["Host adapters"]]),
            fact!(
                "release safety forbids tags",
                [["Never", "tag"], ["tag creation"]]
            ),
        ],
    );

    assert_doc_facts(
        &root,
        "docs/reference/commands.md",
        &[
            fact!(
                "real executable lives in cargo-managed bin directory",
                [
                    ["cargo install --git https://github.com/oharlem/flow --locked flow-cli"],
                    ["Host adapters"]
                ]
            ),
            fact!(
                "embedded asset export command",
                [
                    ["flow export-assets --dir <DIR>"],
                    ["flow", "export-assets", "--dir"]
                ]
            ),
            fact!(
                "embedded conventions and prompts",
                [["embedded", "conventions", "base phase prompts"]]
            ),
        ],
    );

    assert_doc_facts(
        &root,
        "docs/hosts.md",
        &[fact!(
            "host assets invoke installed Flow",
            [["Generated host assets", "installed", "flow", "FLOW_HOST"]]
        )],
    );

    assert_doc_facts(
        &root,
        "flow/docs/README.md",
        &[
            fact!(
                "Cargo Git install command",
                [["cargo install --git https://github.com/oharlem/flow --locked flow-cli"]]
            ),
            fact!(
                "cargo-managed bin location",
                [["Cargo", "bin directory"], ["~/.cargo/bin/flow"],]
            ),
            fact!(
                "host assets invoke installed Flow",
                [["Generated host assets", "installed", "FLOW_HOST"]]
            ),
            fact!(
                "embedded asset export command",
                [
                    ["flow export-assets --dir <DIR>"],
                    ["flow", "export-assets", "--dir"]
                ]
            ),
        ],
    );

    assert_doc_facts(
        &root,
        "flow/docs/directory-layout.md",
        &[
            fact!(
                "host assets invoke installed Flow",
                [["Generated host assets", "installed", "flow", "FLOW_HOST"]]
            ),
            fact!(
                "embedded asset export command",
                [
                    ["flow export-assets --dir <DIR>"],
                    ["flow", "export-assets", "--dir"]
                ]
            ),
            fact!("repo-local state", [["repo-local", "state"]]),
        ],
    );

    let docs_readme = std::fs::read_to_string(root.join("docs/README.md")).unwrap();
    assert!(
        !docs_readme.contains("[`flow/conventions.md`](../flow/conventions.md)"),
        "docs/README.md must not point readers at the removed generated conventions file"
    );
}

#[test]
fn m7_installed_binary_docs_are_aligned() {
    // T-001: public host docs require installed `flow` and generated host
    // assets do not mention a project-local launcher.
    let root = repo_root();

    assert_doc_facts(
        &root,
        "docs/hosts.md",
        &[
            fact!(
                "host assets run installed Flow",
                [
                    ["host assets", "FLOW_HOST", "flow run"],
                    ["Generated host assets", "installed", "flow"],
                    ["Generated host assets", "FLOW_HOST=<host>"],
                ]
            ),
            fact!(
                "Cargo Git install command",
                [["cargo install --git https://github.com/oharlem/flow --locked flow-cli"]]
            ),
        ],
    );

    for rel in [
        "README.md",
        "docs/security.md",
        "docs/reference/commands.md",
        "docs/hosts.md",
        "flow/docs/README.md",
        "flow/docs/directory-layout.md",
    ] {
        let text = std::fs::read_to_string(root.join(rel))
            .unwrap_or_else(|e| panic!("expected {rel}: {e}"));
        for forbidden in [
            ".flow/bin/flow",
            "compatibility launcher",
            "not a copied Flow binary",
            "execs `flow` from `PATH`",
            "fallback",
        ] {
            assert!(
                !text.contains(forbidden),
                "{rel} should not describe removed launcher behavior `{forbidden}`"
            );
        }
    }
}

#[test]
fn m8_run_all_child_command_docs_are_aligned() {
    // T-002: roadmap-scoped run docs describe direct FLOW_RUN_DIR propagation
    // through structural facts rather than exact sentence matches.
    let root = repo_root();

    for rel in ["flow/docs/README.md", "flow/docs/directory-layout.md"] {
        assert_doc_facts(
            &root,
            rel,
            &[
                fact!(
                    "roadmap-scoped run workspace",
                    [
                        ["flow run", "flow/runs/"],
                        ["roadmap", "run directory"],
                        ["roadmap", "run branch"],
                    ]
                ),
                fact!(
                    "child commands carry FLOW_RUN_DIR directly",
                    [
                        ["FLOW_RUN_DIR=<run-dir>", "directly"],
                        ["FLOW_RUN_DIR=<run-dir>", "child commands"],
                        ["FLOW_RUN_DIR", "same command"],
                    ]
                ),
                fact!(
                    "first child start shape",
                    [
                        ["FLOW_RUN_DIR=\"<run-dir>\" flow start <M-N>"],
                        ["FLOW_RUN_DIR", "flow start <M-N>"],
                        ["FLOW_RUN_DIR", "first child start"],
                    ]
                ),
                fact!(
                    "child commands stay on run branch",
                    [
                        ["run branch", "first attempt"],
                        ["stays on the run branch"],
                        ["reuse", "run branch"],
                    ]
                ),
            ],
        );
    }
}

#[test]
fn m29_run_model_docs_and_guidance_are_aligned() {
    // T-004 / T-005 / T-006: the current docs and Flow-owned guidance describe
    // roadmap-scoped runs instead of full-roadmap-only checkpoints or a
    // run-level Auto-finalize setting.
    let root = repo_root();

    assert_doc_facts(
        &root,
        "assets/agents/run.base.md",
        &[
            fact!(
                "unified run workflow heading",
                [["Roadmap-Scoped Run Workflow"]]
            ),
            fact!(
                "invocation-scoped milestone loop",
                [["milestone or milestones requested by `Invocation`"]]
            ),
            fact!(
                "run artifacts refreshed after close",
                [["refresh `log.md`, `manual.md`, and `release-notes.md` after each close"]]
            ),
        ],
    );

    assert_doc_facts(
        &root,
        "docs/reference/commands.md",
        &[
            fact!(
                "run commands attach to roadmap scoped runs",
                [[
                    "flow run",
                    "flow run M-N",
                    "active planned/running roadmap run"
                ]]
            ),
            fact!(
                "run finalization uses review config",
                [[
                    "flow run --finalize",
                    "review.before_finalize",
                    "review.per_command.run"
                ]]
            ),
            fact!(
                "new checkpoint config key documented",
                [["git.run_checkpoint_commits"]]
            ),
        ],
    );

    for rel in [
        "README.md",
        "AGENTS.md",
        "assets/templates/AGENTS.md.tmpl",
        "docs/README.md",
        "docs/architecture.md",
        "docs/reference/artifacts.md",
        "docs/start-here/01-your-first-change.md",
        "docs/security.md",
        "flow/docs/README.md",
        "flow/docs/directory-layout.md",
        "flow/docs/artifact-index.md",
    ] {
        let text = std::fs::read_to_string(root.join(rel))
            .unwrap_or_else(|e| panic!("expected {rel}: {e}"));
        assert!(
            !text.contains("full-roadmap run"),
            "T-006: {rel} should not describe current runs as full-roadmap-only"
        );
        assert!(
            !text.contains("flow run all` checkpoint") && !text.contains("flow run all checkpoint"),
            "T-006: {rel} should not make checkpoint commits full-roadmap-only"
        );
        assert!(
            !text.contains("git.run_all_checkpoint_commits"),
            "T-006: {rel} should prefer git.run_checkpoint_commits in current guidance"
        );
    }
}

#[test]
fn m10_no_docs_impact_contract_is_documented() {
    // T-003: the parser/close-gate contract for docs-neutral work is
    // structurally documented without pinning exact prose.
    let root = repo_root();

    assert_doc_facts(
        &root,
        "assets/conventions/plan.md",
        &[
            fact!("no-docs opt-out marker", [["Impact: none"]]),
            fact!(
                "required docs-current rationale",
                [["Docs already current because <rationale>"]]
            ),
            fact!(
                "rationale alone is insufficient",
                [
                    [
                        "docs-current rationale",
                        "does not satisfy closeout evidence"
                    ],
                    ["without that line", "does not satisfy"],
                ]
            ),
        ],
    );
    assert_doc_facts(
        &root,
        "assets/conventions/close.md",
        &[
            fact!("no-docs opt-out marker", [["Impact: none"]]),
            fact!("docs-current rationale", [["docs-current rationale"]]),
            fact!("current Flow docs evidence", [["Current Flow docs"]]),
        ],
    );
    assert_doc_facts(
        &root,
        "assets/agents/plan.base.md",
        &[
            fact!("no-docs opt-out marker", [["Impact: none"]]),
            fact!(
                "required docs-current rationale",
                [["Docs already current because <rationale>"]]
            ),
            fact!("opt-in no-docs path", [["no-docs path", "opt-in"]]),
        ],
    );
    assert_doc_facts(
        &root,
        "docs/reference/commands.md",
        &[
            fact!("no-docs opt-out marker", [["Impact: none"]]),
            fact!(
                "configured Flow docs path evidence",
                [["changed files", "configured Flow docs path"]]
            ),
            fact!("docs-current rationale", [["Docs already current"]]),
        ],
    );
    assert_doc_facts(
        &root,
        "docs/reference/artifacts.md",
        &[
            fact!("no-docs opt-out marker", [["Impact: none"]]),
            fact!(
                "required docs-current rationale",
                [["Docs already current because <rationale>"]]
            ),
            fact!(
                "configured Flow docs path evidence",
                [["configured Flow docs path"]]
            ),
        ],
    );
    assert_doc_facts(
        &root,
        "flow/docs/README.md",
        &[
            fact!(
                "documentation impact section",
                [["## Documentation Impact"]]
            ),
            fact!("no-docs opt-out marker", [["Impact: none"]]),
            fact!(
                "rationale alone is insufficient",
                [
                    ["docs-current rationale", "Impact: none", "does not"],
                    ["rationale", "without", "Impact: none", "does not"],
                ]
            ),
        ],
    );
}

/// M-19: every `docs/**/*.md` file outside `docs/archive/` must be referenced
/// from `docs/SUMMARY.md`. Guards against future orphan accretion that would
/// recreate the dead-docs surface the 2026-05-15 audit flagged.
///
/// Tasks covered:
/// - T-1 — this test enumerates the docs tree and asserts SUMMARY coverage.
/// - T-2 — the seven `docs/features/*.md` orphan files are removed.
/// - T-3 — deleted docs paths stay absent from current Flow docs.
/// - T-4 — `docs/proposals/doc-revision.md` records the decision to keep
///   `docs/archive/prompts/doc-rearch.md` as an archived prompt.
/// - T-5 — workspace green-bar runs this test alongside the rest.
#[test]
fn m19_every_published_doc_is_in_summary() {
    let root = repo_root();
    let summary_path = root.join("docs/SUMMARY.md");
    let summary = std::fs::read_to_string(&summary_path)
        .unwrap_or_else(|e| panic!("expected {}: {e}", summary_path.display()));
    let mut orphans: Vec<String> = Vec::new();
    for entry in WalkDir::new(root.join("docs"))
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .filter(|e| e.path().extension().map(|x| x == "md").unwrap_or(false))
    {
        let path = entry.path();
        let rel = path
            .strip_prefix(&root)
            .expect("entry under repo root")
            .to_string_lossy()
            .replace('\\', "/");
        if rel == "docs/SUMMARY.md" {
            continue;
        }
        if rel.starts_with("docs/archive/") {
            continue;
        }
        let needle = rel
            .strip_prefix("docs/")
            .expect("docs/ prefix on docs entries");
        if !summary.contains(needle) {
            orphans.push(rel);
        }
    }
    assert!(
        orphans.is_empty(),
        "found docs orphans not referenced by docs/SUMMARY.md:\n{}",
        orphans
            .iter()
            .map(|p| format!("  {p}"))
            .collect::<Vec<_>>()
            .join("\n")
    );
}

/// M-19 anchor: the `docs/features/` tree stays removed.
#[test]
fn m19_docs_features_removed() {
    // T-2 — docs/features/ directory is gone.
    // T-5 — this file participates in cargo test --workspace.
    let root = repo_root();
    assert!(
        !root.join("docs/features").exists(),
        "expected docs/features/ to be removed"
    );
}
