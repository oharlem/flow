# 0016 — Unified Run Workspace

**Status**: Accepted
**Date**: 2026-05-19

## Context

Flow needs one canonical workspace shape for one-off changes and roadmap
delivery runs so status, resume, closeout, checkpoint, and verification logic
all read the same state model.

## Decision

Flow stores work under `flow/runs/<run-id>/`.

Each run has:

- `run.md` for machine-critical run state.
- `log.md` for audit history.
- `manual.md`, `release-notes.md`, and `roadmap.md` for roadmap runs.
- `changes/<change>/` for child change artifacts: `spec.md`, `plan.md`,
  `tasks.md`, and `status.md`.

One-off work creates one run with one child change. Roadmap automation creates
or attaches to one roadmap run and adds one child change per milestone.

`flow close` closes the child change in place, ticks linked roadmap milestones,
and updates `run.md`. `flow run --finalize` finishes run-level handoff
artifacts for completed roadmap runs.

Public context variables are `FLOW_RUN_DIR` for the parent run and
`FLOW_CHANGE_DIR` for an explicit child change.

## Consequences

Resume, closeout, checkpoint, and status resolution have one canonical state
source: `run.md` plus the selected child change directory.
