# AGENTS.md

- When proposing  multiple options, outline the recommended option as round brackets, for example "(A)".

Universal guidance for coding agents working in this repository.

This file is host-neutral. Apply it regardless of host or tool (Codex, Claude
Code, Cursor, Aider, Windsurf, …). Explicit user
instructions override this file. Host-specific notes live in each adapter
crate's `assets/AGENTS.md.fragment`.

## Project Overview

Flow is a spec-driven workflow toolkit for AI-assisted coding. It is written
in Rust and ships as a single static binary (`flow`). The source tree is a
Cargo workspace.

- **Runtime surface:** `crates/flow-cli/` builds the `flow` binary.
- **Host-agnostic core:** `crates/flow-core/` — parsers, renderers, drift
  engine, envelope composer. No host knowledge.
- **Host adapters:** `crates/flow-host-{claude-code,codex,cursor}/` —
  each crate embeds its own skill/command assets via `include_str!`.
- **Embedded assets:** `assets/templates/`, `assets/agents/`,
  `assets/conventions/`, `assets/gitignore.d/` — single source of truth.

Flow must work offline and stay dependency-light. Do not add runtime
third-party dependencies unless a task explicitly requires one.

Flow never performs remote or destructive git actions: no `git push`,
`git pull`, `git fetch`, `gh`, `glab`, `git reset --hard`, `git clean -fd`,
or force operations. Flow commands do not create tags. The only commit
exceptions are local commits created by branch-backed roadmap runs when
`git.run_checkpoint_commits: true`: checkpoint commits after closed
milestones, and the closing commit of the run workspace created by
`flow run --finalize`.

`run.md` is the source of truth for Flow run state.
`status.md` is the source of truth for Flow child-change state.
`flow/roadmap.md` is user-authored. Never overwrite or auto-generate it;
Flow mutates it at closeout only to tick linked milestone checkboxes.

## Source Map

- `crates/flow-core/src/ids.rs` — typed FR/SC/T/M/P/R/D identifiers.
- `crates/flow-core/src/parse/` — per-artifact parsers.
- `crates/flow-core/src/render.rs` — template rendering + status stamping.
- `crates/flow-core/src/envelope.rs` — `/flow-<cmd>` envelope composer.
- `crates/flow-core/src/drift.rs` + `drift/render.rs` — D1/D2/D3 engine.
- `crates/flow-core/src/roadmap.rs` — roadmap milestone tick.
- `crates/flow-cli/src/cmd/` — one module per `/flow-<cmd>`.
- `crates/flow-host-*/src/install.rs` — per-host installer + embedded assets.

## Change Rules

- If artifact formats change, update conventions, templates, parsers, agent
  prompts, and tests **together**.
- If command behavior changes, update the relevant driver plus every affected
  host adapter.
- Keep generated project assets aligned with the current release behavior;
  document breaking workflow changes in `docs/decisions/` as an ADR.

## Verification

- Full suite: `cargo test --workspace`
- Format: `cargo fmt --all --check`
- Lint: `cargo clippy --workspace --all-targets -- -D warnings`
- Release build: `cargo build --release`

When in doubt, start from `cargo test --workspace`.

## Git Safety

- NEVER run destructive git operations (`push`, `pull`, `fetch`, `reset --hard`,
  `clean -fd`, `--force`).
- NEVER invoke `gh` or `glab`.
- NEVER create commits, tags, or repository history as part of a Flow run,
  except through Flow's printed `flow run --checkpoint <run-dir> --milestone M-N`
  command and the run-closing commit created by `flow run --finalize`, both
  during `flow run` when checkpoint commits are enabled.
- On `main`/`master`/`trunk`/`develop`/`release/*` warn and prompt before
  creating a Flow branch.
## Cursor Notes

Flow is wired for **Cursor** (preview). Flow rules live under `.cursor/rules/flow.mdc` and describe the spec-driven workflow. Invoke Flow with the canonical shape `FLOW_HOST=cursor flow <command>` from Cursor's chat. If `flow` is not on `PATH`, install the Flow CLI, then retry.

The `review.before_finalize` config setting (default `false`) uses collapsed review on the green path: the printed finalize footer is suppressed and the agent runs the command shown as `**Save state with**` in the envelope (`flow <cmd> --finalize`, with the task ID for `build-task` and `FLOW_RUN_DIR` set for `roadmap`) in the same session when artifacts are ready. Set it to `true` in `.flow/config.yaml` to keep the two-stage protocol (prepare → footer checkpoint → finalize). Per-command overrides via `review.per_command.<cmd>: true|false` are supported. Internal drivers may chain phases (e.g. `build` → `test`) without a second prepare envelope.

## Codex Notes

Flow is wired for **Codex**. Flow commands are registered as Codex skills under `.agents/skills/flow-*/`. Skills invoke Flow with the canonical shape `FLOW_HOST=codex flow <sub>`.

Invoke them in Codex with skill mentions:

- `$flow-setup` — install or upgrade Flow for Codex.
- `$flow-doctor` — sanity-check the local Flow installation.
- `$flow-start <description>` — draft a new `spec.md`.
- `$flow-amend <change request>` — update the active change `spec.md`.
- `$flow-plan` — turn an approved spec into a plan and task list.
- `$flow-build` — implement all remaining tasks test-first.
- `$flow-build-task [T-NNN]` — implement one next or selected task test-first.
- `$flow-test` — verification gate: tests + consistency checks.
- `$flow-close` — close a completed change in place.
- `$flow-status` — read-only report.

The `review.before_finalize` config setting (default `false`) suppresses the printed finalize footer on the green path; the agent runs the command shown as `**Save state with**` in the envelope when artifacts are ready. Set to `true` to keep the two-stage footer checkpoint.

<!-- FLOW:CLAUDE-CODE-NOTES:START -->
## Claude Code Notes (Flow-owned)

Flow owns this section. `flow setup --host claude-code` and `flow update` may refresh it.

Flow is wired for **Claude Code**. Flow commands are registered as Claude Code skills under `.claude/skills/flow-*/`. Those skills run the installed `flow` binary. Invoke them with `/flow-<name>` in any Claude Code session:

- `/flow-roadmap`, `/flow-run [M-N]`, `/flow-doctor`, `/flow-start`, `/flow-amend`, `/flow-plan`, `/flow-build`, `/flow-build-task`, `/flow-test`, `/flow-close`, `/flow-status` (+ auto-run `/flow-setup`).

### Permissions

Flow's read-only git operations are allowlisted in `.claude/settings.json`. Flow never runs `git push`, `git pull`, `git fetch`, or `gh`/`glab`; the only history exception is local roadmap-run checkpoint commits and the run-closing finalize commit when enabled.

### Protected branches

Flow warns (doesn't refuse) when you run it on `main`, `master`, `trunk`, `develop`, or `release/*`. Set `FLOW_FORCE_ON_PROTECTED=1` to skip the prompt.
<!-- FLOW:CLAUDE-CODE-NOTES:END -->
