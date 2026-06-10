# Flow Artifact Conventions — Close

```
Conventions-Version: 1.1
```

Artifact shapes and closeout contract used by `/flow-close`. Core invariants live in `core.md` and are always loaded alongside this shard.

---

## `flow/runs/<run>/roadmap.md`

- **Run-local** flat list of forward-looking milestones. `flow roadmap
  <source>` creates it with the planned run. If a run-local roadmap is missing
  or contains no `M-N` entries, the milestone hierarchy is inactive for that
  run and Flow makes no attempt to use it.
- One milestone per heading, using GitHub-flavored checkbox syntax:

  ```markdown
  ### [ ] M-N: <short title>

  Optional description text.
  ```

- The milestone description is every line after the milestone heading up to,
  but not including, the next milestone heading.
- Use `/flow-roadmap` to create the milestone list. `/flow-start` does not
  create milestones automatically; pass `M-N` positionally (`flow start M-3`)
  with `FLOW_RUN_DIR=<run-dir>` to link a change to an existing run-local
  milestone.
- The next generated milestone number is allocated by scanning existing
  run-local roadmaps, then reflected in `.flow/state.yaml: counter`.
  Milestone IDs are variable-width and unpadded, such as `M-1` and `M-12`.
- Flow mutates run-local roadmap files narrowly:
  - `flow close` flips `[~]` → `[x]` on the milestone(s) referenced by the
    closed change's `status.md`.
- Live in-flight change state is rendered on demand by `/flow-status`; the
  per-change `status.md` remains the single source of truth. Current projects
  should use `M-N` milestones; dashboard cards are not maintained by closeout.

### Optional `Milestone` field on `status.md`

When a change is associated with one or more roadmap milestones, `status.md`
gains an optional key-value line directly below `**Branch**:`:

```
**Milestone**: M-N[, M-N…]
```

Set by `/flow-start` when the user passes a positional `M-N` token
(`flow start M-3`). Absent for changes that did not link a milestone.
Missing, unreadable, or unknown milestone IDs block closeout.

## 4.5 Milestone

In `flow/runs/<run>/roadmap.md`, anywhere in the file:

```markdown
### [ ] M-N: <short title>

Optional description text.
```

State transition: `[ ]` → `[x]` on close for the first change in the same run
whose `status.md` references this milestone. Subsequent closes that reference
the same milestone do not modify the line further. The `[~]` state is
user-authored (indicating the milestone is actively being worked) and is not set
automatically by `/flow-start`.

## 8. Close behavior

`/flow-close` adds `**Closed**: <YYYY-MM-DD>` below the spec header, requires evidence that Current Flow docs under `flow/docs/**` were updated or that `plan.md` declares `Impact: none` under `## Documentation Impact` with a docs-current rationale, stamps the child change closed in place, updates `run.md`, and ticks linked roadmap milestones. Standalone closeout never bumps application versions, creates commits, creates tags, merges branches, pushes, pulls, fetches, or calls GitHub/GitLab CLIs.
