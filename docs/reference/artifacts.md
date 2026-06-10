# Artifacts

This page explains every file Flow reads or writes. It is the readable map, not
the grammar contract. The exact artifact grammar lives in the embedded
convention shards from [`assets/conventions/`](../../assets/conventions/). Use
`flow export-assets --dir <DIR>` to inspect the copies bundled in the running
binary.

## Map

| Artifact | Location | Owner | Purpose |
|---|---|---|---|
| `run.md` | `flow/runs/<run>/` | Flow driver | Run-level state, current change, next command, and change index |
| `spec.md` | `flow/runs/<run>/changes/<change>/` | Host agent through `flow start` and `flow amend` | What to build and why |
| `plan.md` | `flow/runs/<run>/changes/<change>/` | Host agent through `flow plan` | How to build it and what docs change |
| `tasks.md` | `flow/runs/<run>/changes/<change>/` | Host agent through `flow plan` and build phases | Dependency-ordered work queue |
| `status.md` | `flow/runs/<run>/changes/<change>/` | Flow driver | Source of truth for child-change state |
| `roadmap.md` | `flow/runs/<run>/` | Roadmap agent/user | Run-local milestone list |
| `log.md`, `manual.md`, `release-notes.md` | `flow/runs/YYYYMMDD-<run-slug>/` | Run agent | Durable automation handoff |
| `flow/docs/**` | `flow/docs/` | Team/agent during Flow change work | Current-state Flow documentation required before closeout |
| `docs/principles.md` | `docs/` | Team, optional | Engineering principles loaded into envelopes |

## `spec.md`

The spec states what the change is, who it serves, and why it matters.

Required:

- `## What & Why`

Common optional sections:

- `## Requirements` with `FR-NNN` bullets
- `## Success Criteria` with `SC-NNN` bullets
- `## Clarifications`
- `## Edge Cases`
- `## Out of Scope`
- `## Key Entities`

## `plan.md`

The plan turns the approved spec into an implementation strategy.

Required:

- `## Summary`
- `## Technical Context`
- `## Documentation Impact`

`## Documentation Impact` names current Flow docs to update, or declares:

```markdown
Impact: none

Docs already current because <rationale>.
```

This is not release-note content. It is closeout evidence that current docs are
still accurate.

## `tasks.md`

Tasks are the small units the build agent works through. Each task references
the spec IDs it covers and verifies.

Task checkbox states are Flow-owned:

- `[ ]` - not implemented
- `[~]` - implemented and awaiting acceptance
- `[x]` - accepted and saved by Flow

`[~]` does not satisfy a dependency and does not count as complete for
`flow test` or `flow close`.

## `status.md`

`status.md` is Flow's source of truth for **child-change** state — one per
change, under `flow/runs/<run>/changes/<change>/`. It is distinct from `run.md`,
which records **run-level** state for the parent run. Flow writes `status.md`
atomically. Do not edit it by hand.

It records:

- `Change`
- `Started`
- `Updated`
- `State`
- `Branch`
- optional `Milestone`
- `## History`

Valid current states are `drafting`, `building`, and `closed`.

## `flow/runs/<run>/roadmap.md`

The roadmap is the run-local list of future milestones for a planned roadmap
run. `flow roadmap <source>` creates the run directory and writes this file
from the start.

Milestones use heading checkboxes:

```markdown
### [ ] M-1: Short title

Optional description.
```

`[ ]` is open, `[~]` is intentionally in progress, and `[x]` is closed. Flow
only flips a linked milestone to `[x]` during `flow close`.

Use `flow roadmap` to derive milestones from a PRD or notes file. Use
`FLOW_RUN_DIR=flow/runs/<run> flow start M-1` to link a child change to an
existing run-local milestone. Milestone IDs are variable-width and unpadded.
New roadmap generation scans existing run-local roadmaps before allocating
IDs.

When a roadmap run completes, Flow leaves the final checked roadmap in that run
directory. No root roadmap is reset.

## `flow/runs/**`

Run directories are durable handoffs for `flow roadmap`, `flow run`, and
`flow run M-1`.

They contain:

- `log.md` - trace of actions, decisions, tests, stops, and handoffs
- `manual.md` - owner manual for operating the result
- `release-notes.md` - user/operator summary of actual delivered changes
- `roadmap.md` - run-local milestone list and final checked roadmap state

`flow run --resume <run-dir>` reads `run.md` to print the next safe command
after an interruption. `FLOW_RUN_DIR=<run-dir> flow run --finalize` validates
that the run handoff documents are complete.

## `flow/docs/**`

These are Flow-maintained current-state docs. They describe how Flow works now;
they are not closed-change summaries.

Before closeout, Flow requires either changed files under the configured Flow docs path
or a `plan.md` `Impact: none` rationale that says docs are already
current. The default path is `flow/docs/`; it can be changed with
`.flow/config.yaml: docs.documentation_path`.

## `docs/principles.md`

This optional file contains project-wide engineering principles. When present,
Flow loads it into every phase envelope. Use `P-NNN` IDs for principles that a
plan should explicitly check.

## When In Doubt

- Exact artifact grammar: export embedded convention shards with
  `flow export-assets --dir <DIR>`.
- Command behavior: read [Commands](./commands.md).
- Terms such as envelope, drift, FR, and SC: read [Glossary](./glossary.md).
