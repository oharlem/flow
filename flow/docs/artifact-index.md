# Artifact Index

<!-- Flow-Managed: docs-page v1 -->

This page lists preserved Flow artifact classes for this repository. It
complements [Directory Layout](directory-layout.md), which explains ownership
boundaries and placement rules.

For exact artifact grammar, read the embedded conventions bundled in the
running binary or export inspectable copies with
`flow export-assets --dir <DIR>`. For the public artifact reference, see
[`docs/reference/artifacts.md`](../../docs/reference/artifacts.md).

## Preserved Artifact Classes

| Artifact class | Canonical location | Notes |
|---|---|---|
| Runs | `flow/runs/<YYYYMMDD-slug>/` | Run state, audit log, owner manual, release notes, child changes, and run-local roadmap state when applicable. |
| Child changes | `flow/runs/<run>/changes/<change>/` | Current and closed specs, plans, tasks, and status files. |
| Run-local roadmap | `flow/runs/<run>/roadmap.md` | Forward-looking milestone list for one planned roadmap run. |
| Artifact grammar | embedded binary assets; optional `flow export-assets --dir <DIR>` copies | Sharded Markdown schema for Flow artifacts. |
| Current Flow docs | `flow/docs/` | Current-state workflow guidance maintained during Flow change work. |
| Runtime control plane | `.flow/` | Repo-local config, state, version marker, and prompt overrides. |

## Placement Rules

- Put all work in `flow/runs/<run>/`.
- Put child change work in `flow/runs/<run>/changes/<change>/`.
- Let `flow close` mark child changes closed in place and update `run.md`.
- Keep current workflow guidance in `flow/docs/`.
- Do not add closed-change summary pages to `flow/docs/`; `flow/runs/` owns
  that history.
- Keep application product, architecture, and public reference docs under
  `docs/`.

## Related Guides

- [Directory Layout](directory-layout.md)
- [Non-Logic Verification](non-logic-verification.md)
- [`docs/reference/artifacts.md`](../../docs/reference/artifacts.md)
- [`docs/reference/commands.md`](../../docs/reference/commands.md)

## Verification

The workspace test `m2_preserved_artifacts_are_discoverable` in
`crates/flow-cli/tests/docs_layout.rs` checks representative directories and
files from this index.

The workspace test `m4_non_logic_verification_gate_is_documented` checks that
the non-logic verification guide stays discoverable from this index.
