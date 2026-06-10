# Example: a complete roadmap run

This directory is a captured snapshot of a real Flow roadmap run, executed on
2026-06-10 in a fresh scratch repository. The demo is intentionally tiny: a
Python CLI first prints `Hello, world!`, then grows one optional name argument.
The point is the record Flow leaves behind after the chat is gone.

The run followed the article path:

```sh
flow roadmap docs/prd.md
flow run
```

In Codex, the public commands are `$flow-roadmap docs/prd.md` and `$flow-run`.

## Reading Order

| File | What it records |
|---|---|
| [docs/prd.md](docs/prd.md) | The source notes Flow decomposed into milestones |
| [roadmap.md](flow/runs/20260610-roadmap-hello-cli-product-notes/roadmap.md) | The run-local roadmap: M-1 default greeting, M-2 named greeting |
| [M-1 spec](flow/runs/20260610-roadmap-hello-cli-product-notes/changes/M-1-cli-prints-the-default-greeting/spec.md) | What the first milestone promised |
| [M-1 tasks](flow/runs/20260610-roadmap-hello-cli-product-notes/changes/M-1-cli-prints-the-default-greeting/tasks.md) | Work and traceability for the default greeting |
| [M-1 status](flow/runs/20260610-roadmap-hello-cli-product-notes/changes/M-1-cli-prints-the-default-greeting/status.md) | Flow-owned lifecycle state for the first closed change |
| [M-2 spec](flow/runs/20260610-roadmap-hello-cli-product-notes/changes/M-2-cli-greets-a-named-user/spec.md) | What the second milestone promised |
| [M-2 tasks](flow/runs/20260610-roadmap-hello-cli-product-notes/changes/M-2-cli-greets-a-named-user/tasks.md) | Work and traceability for the named greeting |
| [M-2 status](flow/runs/20260610-roadmap-hello-cli-product-notes/changes/M-2-cli-greets-a-named-user/status.md) | Flow-owned lifecycle state for the second closed change |
| [run.md](flow/runs/20260610-roadmap-hello-cli-product-notes/run.md) | Run-level state: roadmap run, scope `all`, status `complete` |
| [log.md](flow/runs/20260610-roadmap-hello-cli-product-notes/log.md) | Decisions, operations, checkpoints, and intervention points |
| [manual.md](flow/runs/20260610-roadmap-hello-cli-product-notes/manual.md) | How to operate and verify the result |
| [release-notes.md](flow/runs/20260610-roadmap-hello-cli-product-notes/release-notes.md) | What changed and what was verified |
| [hello.py](hello.py), [test_hello.py](test_hello.py) | The code the run produced |

## What This Shows

- **Milestones close as separate changes.** M-1 and M-2 each have their own
  spec, plan, tasks, status history, close stamp, and roadmap checkbox.
- **The run accumulates handoff docs.** `log.md`, `manual.md`, and
  `release-notes.md` sit at the run root and describe the whole delivery.
- **Checkpoint commits are visible.** The scratch repo history was:

```text
646b503 flow run finalize: 20260610-roadmap-hello-cli-product-notes
0616bf9 flow run checkpoint: M-2 CLI greets a named user
b160ea8 flow run checkpoint: M-1 CLI prints the default greeting
c94b690 chore: wire Flow (init + roadmap: Hello CLI)
659f616 init: project notes
```

## Gate Refusal

The run also captured a deliberate traceability break. After M-2 was built,
the `Covers:` references were temporarily removed from `tasks.md`. Flow refused
closeout with exit code `2`:

```text
Flow stopped before closing.

### Why Flow stopped

Your spec and task list do not match. Closing now could archive work that misses a requirement or points to stale tasks.

### What to fix

1. **Requirement has no task** (must fix; developer detail: D1)
   - Why: FR-001 is in spec.md, but no task in tasks.md lists it under Covers.
   - Where: `spec.md:23`
   - Fix: Add a task in tasks.md with Covers: FR-001.; or Remove FR-001 from spec.md if it is no longer needed.
2. **Requirement has no task** (must fix; developer detail: D1)
   - Why: FR-002 is in spec.md, but no task in tasks.md lists it under Covers.
   - Where: `spec.md:24`
   - Fix: Add a task in tasks.md with Covers: FR-002.; or Remove FR-002 from spec.md if it is no longer needed.

[flow] ERROR: consistency check failed: 2 error(s), 0 warning(s)
```

After the `Covers:` lines were restored, the same closeout path passed, M-2
was checked off in `roadmap.md`, and Flow printed the M-2 checkpoint command.

## Try It Yourself

This snapshot is meant to be read, not replayed through a canned script. To
try the workflow, use Flow in a real repository with a small PRD or notes file:

```sh
$flow-roadmap path/to/notes.md
$flow-run
```

Use `/flow-roadmap` and `/flow-run` in slash-command hosts such as Claude Code
or Cursor.

## Honest Boundary

Flow guarantees the structural integrity of this record: IDs resolve, states
are legal, checkpoints came from Flow, and gates ran. It does not guarantee
the prose is true or the code is semantically correct; tests, review, and
human judgment still do their jobs.
