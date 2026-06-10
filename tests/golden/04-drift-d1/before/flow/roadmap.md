# Roadmap

Forward-looking milestones for this project. Each `M-NNNN` is one planned chunk
of work. Flow ticks `[x]` automatically when `/flow-close`
finalizes a change whose `status.md` references the milestone.

> **Editing rules.** This file is yours to edit. Add, reorder, and rephrase
> milestones freely. Flow only ever flips `[ ]` → `[x]` on `M-NNNN` lines and
> appends a `_Done <date> → [change](./change/)_` annotation; it never
> rewrites the rest of the file.

## How it works

1. Add a milestone:

   ```markdown
   - [ ] **M-NNNN**: <short title> — <one-line description>.
   ```

2. Start a change for it: `/flow-start M-NNNN` (the milestone title and
   description seed `spec.md`'s `## What & Why`, and `**Milestone**: M-NNNN`
   is recorded in `status.md`).

3. Close the change with `/flow-close`. Flow flips the checkbox and appends
   the date and change link in place.

You may group milestones with optional `### Section` headings (e.g. `### Now`,
`### Next`, `### Later`, `### Backlog`). Flow ignores headings and only
reacts to `M-NNNN` bullets, so use whatever grouping fits the team.

## Milestones

<!-- Add milestones below. Pattern (delete this comment when ready):

  - [ ] **M-NNNN**: <short title> — <one-line description>.

Replace NNN with sequential numbers (001, 002, 003, …).
-->
