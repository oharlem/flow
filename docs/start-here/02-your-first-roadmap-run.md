# Your First Roadmap Run

**Reviewed**: 2026-06-09

This walkthrough covers the **automated multi-milestone path**: turning a PRD
or rough notes into a roadmap, then letting Flow drive the host agent through
each milestone end to end. The single-change path is covered in
[`01-your-first-change.md`](./01-your-first-change.md); read that first if you
have not.

The canonical picture of this workflow is at
[`docs/flow-main-workflow-v0.1.0.png`](../flow-main-workflow-v0.1.0.png).
This page narrates the same diagram, step by step.

> **You will need:** a Flow-initialized project, a host that speaks Flow
> (Claude Code, Codex, or Cursor), and a short PRD or notes file describing
> the outcomes you want. The commands below use Claude Code's `/flow-<name>`
> syntax. Use `$flow-<name>` in Codex, `/flow-<name>` in Cursor, or
> `flow <name>` in a shell.

## 1. Create a roadmap — `/flow-roadmap`

A **roadmap** is a run-local list of milestones — outcomes, not tasks. Each
milestone becomes one closed change later.

```text
/flow-roadmap path/to/prd.md
```

Inline text also works:

```text
/flow-roadmap ship a login page, add email magic links, and rate-limit attempts
```

Flow creates `flow/runs/<run>/` and writes:

- `roadmap.md` — the milestone backlog
- `run.md` — run-level state (status starts as `planned`)
- `log.md`, `manual.md`, `release-notes.md` — placeholder hand-off files
- `changes/` — empty placeholder; per-milestone change directories appear later

> **Milestones are outcomes, not tasks.** "Users can log in with a magic link"
> is a milestone. "Add `POST /auth/magic-link` endpoint" is a task — it belongs
> in `tasks.md` later, not in `roadmap.md`.

## 2. Review and edit `roadmap.md`

Open `flow/runs/<run>/roadmap.md`. Reorder, rename, split, or merge milestones
until the list reads as the outcomes you want delivered, in the order you want
them delivered. This is the only step on the diagram drawn with a dashed
border, because it is the only step that expects manual editing.

## 3. Attach the automation run — `/flow-run`

```text
/flow-run
```

This attaches the host agent to the roadmap and processes **all** open
milestones in order. To work one milestone at a time, target it explicitly:

```text
/flow-run M-1
```

On the first attachment, Flow transitions the run from `planned` to `running`.
If `git.run_branch: true` is set, Flow also creates one **run-level** git
branch named `<prefix>/run-<date>-<run-slug>` — used for every milestone in the
run, not one branch per milestone.

## 4. Per-milestone loop

For each requested milestone, the diagram repeats the same shape. Flow does
not loop internally — the host agent reads `run.md` after each phase and
re-invokes the next command. The shape per milestone:

1. **Milestone init** — Flow creates `flow/runs/<run>/changes/M-<id>/` with
   `spec.md`, `plan.md`, `tasks.md`, and `status.md` scaffolding.
2. **Generate spec** — the start agent (and `flow amend` if you need to
   revise) writes `spec.md`.
3. **Plan work** — `flow plan` writes `plan.md` and a dependency-ordered
   `tasks.md`.
4. **Build tasks** — `flow build` (whole queue) or `flow build-task` (one
   at a time) implements each task test-first.
5. **Verification** — `flow test` runs the project test suite and Flow's
   `D1`–`D3` drift checks.
6. **Milestone hand-off** — `flow close` stamps the change closed, ticks the
   matching `[ ] M-<id>` checkbox in `roadmap.md`, and (when
   `git.run_checkpoint_commits: true`) prints a `flow run --checkpoint`
   command that creates a local checkpoint commit on the run branch.

After hand-off, Flow checks whether all targeted milestones are complete.
If not, the host re-attaches and the loop continues at the next milestone.

> **Where state lives.** `status.md` is per-change (drafting → building →
> closed). `run.md` is per-run (planned → running → complete). The loop is
> driven by the host agent reading `run.md`, not by Flow itself.

## 5. Run finalize — `/flow-run --finalize`

When every targeted milestone is closed, the diagram's last node is
**Run finalize**. This step does **not** generate new artifacts: the
hand-off files were created in step 1 and have been appended throughout the
run. Finalize validates that the required documents are complete and stamps
the run status `running → complete`:

- `Run scope: all` requires complete `log.md`, `manual.md`, and
  `release-notes.md`.
- `Run scope: single` (from `/flow-run M-N`) requires only
  `release-notes.md`; `log.md` and `manual.md` are optional.

With `git.run_checkpoint_commits: true`, finalize also creates one closing
commit of the run workspace (`flow run finalize: <run-name>`), so a completed
run leaves a clean worktree. It then prints a **Verify this run** summary
pointing at `release-notes.md`, `manual.md`, `log.md`, `run.md`, and the
roadmap — review those, then merge the run branch yourself when satisfied;
Flow never pushes or merges.

The `review.before_finalize` config gate (and per-command
`review.per_command.run`) controls whether finalize runs in one go or pauses
for a checkpoint footer. See [`reference/commands.md`](../reference/commands.md).

## What now?

You have produced one `flow/runs/<run>/` directory containing:

- one closed change per milestone, under `changes/M-<id>/`
- a fully checked `roadmap.md`
- complete hand-off documents at the run root
- a `run.md` stamped `complete`

The same five public commands (`/flow-start`, `/flow-plan`, `/flow-build`,
`/flow-test`, `/flow-close`) that handled a one-line README change handle a
multi-milestone run — `/flow-run` simply chains them milestone after milestone
on your behalf. Drop back into individual commands at any time for nuanced
control or debugging; the diagram path is the automated default, not the only
way to use Flow.

If you get stuck, run `/flow-status` for a read-only report or
`/flow-doctor` to check the local install.
