# 0018 — Run-finalize closing commit

- **Status:** Accepted
- **Date:** 2026-06-10

## Context

A milestone checkpoint can only record its own commit SHA after Git creates
the commit, so every `flow run --checkpoint` leaves `run.md` and `log.md`
modified. On intermediate milestones the next checkpoint sweeps that
bookkeeping up, but the final checkpoint has no successor, and
`flow run --finalize` mutated the run workspace without committing. Every
completed roadmap run therefore ended with a dirty worktree and no
explanation, which reads as a bug and gives the user no closing handoff.

## Decision

When checkpoint commits are enabled (`git.run_checkpoint_commits: true`),
`flow run --finalize` ends the run with one closing commit:

- It stages and commits only the run directory
  (`git add -A -- <run-dir>` then `git commit -- <run-dir>`), so unrelated
  user changes — staged or not — are never swallowed.
- The message is `flow run finalize: <run-name>`, parallel to
  `flow run checkpoint: M-N <title>`.
- The commit requires being on the run branch, the same precondition as
  checkpoints. No SHA is written back afterwards, so finalize introduces no
  new post-commit bookkeeping.

Finalize also prints a **Verify this run** summary (release notes, manual,
log, run state, roadmap, checkpoint history) and a next step: review the
files, then merge the run branch manually. With checkpoint commits disabled,
finalize commits nothing and states explicitly that the run artifacts are
intentionally left uncommitted.

## Consequences

- A completed branch-backed roadmap run leaves a clean worktree:
  one checkpoint commit per milestone plus one closing commit.
- The documented commit exception widens from "checkpoint commits only" to
  "checkpoint commits plus the run-closing finalize commit"; AGENTS guidance,
  conventions, host adapter assets, and security docs say so together.
- Rerunning a finalize that failed after mutating artifacts retries the
  commit; finalize edits are idempotent stamps, so the retry stays safe.
- Runs without checkpoint commits keep their fully user-managed history.
