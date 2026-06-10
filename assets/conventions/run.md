# Flow Artifact Conventions — Run

```
Conventions-Version: 1.1
```

Artifact shapes and run contract used by `/flow-run`. Core invariants live in `core.md` and are always loaded alongside this shard.

---

## `flow/runs/YYYYMMDD-<run-slug>/`

Created by `/flow-start` and `/flow-run`. Run artifacts are
tracked Markdown state and handoff documents:

- `run.md` records run-level machine state, the current child change, next
  command, checkpoint setting, roadmap fingerprint, and change index.
- `roadmap.md` records this run's milestone backlog for roadmap runs.
- `log.md` records timestamps, operations, decisions with reasoning, actual
  actions, files affected, test outcomes, stops, and human interventions.
  It intentionally omits code blocks, diffs, and patches.
- `manual.md` is the owner's manual for a new user: quickstart,
  resulting state, configuration, workflows, operating notes, and
  troubleshooting.
- `release-notes.md` is the delta-oriented user/operator summary of what
  actually changed in the run. It is derived from completed work, not from
  roadmap intent alone, and it is separate from the root versioned
  `CHANGELOG.md`.

Child changes live under `changes/<change>/` and contain `spec.md`, `plan.md`,
`tasks.md`, and `status.md`.

## Run State schema

`run.md` includes these exact field names:

- `Run name`
- `Run type`
- `Status`
- `Run branch`
- `Roadmap fingerprint`
- `Checkpoint commits`
- `Current milestone`
- `Current change`
- `Current phase`
- `Last saved Flow action`
- `Next command`
- `Last checkpoint`

The roadmap fingerprint is `sha256:<short>`, where `<short>` is the first 12
lowercase hex characters of the SHA-256 digest of the exact
`flow/runs/<run>/roadmap.md` text. Run-state readers require the current field
set and do not infer missing lifecycle identity.

`run.md` also includes `## Changes` and `## Milestones` indexes used for run
handoff and recovery. `log.md` is audit-only and is not machine-critical state.

## Run behavior

`/flow-roadmap <source>` creates a planned roadmap run. `/flow-run` starts or
continues the active planned/running roadmap run, and `/flow-run M-N` targets a
milestone inside that run's `roadmap.md`. The run drives the normal start →
plan → build → test → close workflow for requested milestones. Run directory
names add `-2`, `-3`, and so on for same-day conflicts. With checkpoint commits
enabled, Flow creates local commits only through the printed `flow run
--checkpoint <run-dir> --milestone M-N` command and the closing commit of the
run workspace that `flow run --finalize` makes itself; Flow never pushes,
pulls, fetches, creates tags, or invokes GitHub/GitLab CLIs.

Run automation may save routine Flow phase state without asking for per-phase
confirmation, but it must stop for critical discoveries, missing critical
specs, explicit user-requested review, manual verification, failing verification, missing
credentials, unavailable local services, and forbidden operations. Run
finalization follows the same `review.before_finalize` and
`review.per_command.run` gate used by other Flow subcommands. A run is not complete
until `release-notes.md` is complete and `flow run --finalize` succeeds. With
checkpoint commits enabled, finalize ends the run with a closing commit of the
run workspace so the post-checkpoint bookkeeping in `run.md` and `log.md` does
not linger uncommitted; with them disabled, run artifacts are intentionally
left uncommitted for the user to commit.

Run handoff is tiered by `Run scope`, captured in `run.md` at attach time:

- `Run scope: all` (set by `flow run all`, or escalated when distinct
  single-milestone targets are attached to one run) requires complete
  `log.md`, `manual.md`, and `release-notes.md`.
- `Run scope: single` (set by `flow run M-N` and never escalated when re-attaching
  the same milestone) and one-off runs require only `release-notes.md`;
  `log.md` and `manual.md` are optional.
