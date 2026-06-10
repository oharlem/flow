# Backlog

`flow/backlog/` stores ideas that are not yet committed to implementation.

Use it for rough notes, PRDs, snippets, spikes, and deferred ideas that may
never reach a planned roadmap run. Use `flow roadmap <source>` when work is
ready to decompose into executable milestones.

## Closeout: May-13 refactoring backlog (2026-05-16, M-20)

The earlier `may-13-refactoring.md` entry has been retired. Its candidate
milestones landed through roadmap milestones M-1..M-18 plus a same-day
2026-05-14 ad-hoc fix wave. Historical Flow records from that pre-unified-run
period were removed from the worktree; git history is the preservation record.
The S2 follow-up question about layout-shape accessors is verified resolved:
in `crates/flow-core/src/paths.rs`, every current accessor (`layout`, `prefix`,
`work_dir`, `runs_dir`, `roadmap_path`, `documentation_dir`) calls
`Config::load_for_repo` at most once per invocation, so nothing parses config
twice in a single call.

## Workflow

1. Capture one idea per Markdown file.
2. Refine the file freely until the idea is clear enough to evaluate.
3. Promote it to a roadmap run only when you want Flow to turn it into milestones.
4. Mark the backlog item as `promoted`, `parked`, or `dropped` so old ideas stay
   understandable.

## File Names

Use a simple idea ID plus a short slug:

```text
I-1-backlog-before-roadmap.md
I-2-agent-orchestration-research.md
```

Use `I-N` for backlog ideas. Do not use `M-N`; milestone IDs belong to
run-local roadmaps under `flow/runs/<run>/roadmap.md`.

## Item Template

```markdown
# Idea: <short title>

**ID**: I-N
**Status**: inbox
**Created**: YYYY-MM-DD
**Source**: inline | file | link
**Tags**: comma, separated
**Promoted**: none

## Raw Input

Paste the original idea, PRD, notes, or links here. Preserve source detail.

## Notes

Refinement, constraints, non-goals, related files, and useful context.

## Open Questions

- <question that affects whether or how this should be promoted>

## Promotion Notes

Record generated roadmap milestones here after promotion.
```

Allowed statuses: `inbox`, `shaping`, `parked`, `promoted`, `dropped`.

## Promote To Roadmap Run

When an idea is ready, run the roadmap phase with the backlog file as source:

```sh
$flow-roadmap flow/backlog/I-N-short-title.md
```

After the roadmap is saved, update the backlog header:

```markdown
**Status**: promoted
**Promoted**: M-12, M-13
```

## Rules

- Preserve raw input; summarize in `Notes`, not by deleting source detail.
- Do not write implementation tasks here. Tasks belong in `tasks.md` after
  `flow plan`.
- Do not put executable milestone checkboxes here. Milestones belong in
  run-local roadmaps under `flow/runs/<run>/roadmap.md`.
- No Flow command owns this directory yet; users and agents may edit it directly.
