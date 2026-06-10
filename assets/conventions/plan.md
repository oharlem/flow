# Flow Artifact Conventions — Plan

```
Conventions-Version: 1.1
```

Artifact shapes used by `/flow-plan`. Core invariants live in `core.md` and are always loaded alongside this shard.

---

## `plan.md`

- `## Summary` *(mandatory)*
- `## Technical Context` *(mandatory)*
- `## Documentation Impact` *(mandatory)*

`## Documentation Impact` names current Flow docs that must be updated, or explicitly opts out when no central documentation is affected:

```markdown
Impact: none

Docs already current because <rationale>.
```

`Impact: none` is the only close-gate opt-out. A docs-current rationale without that line does not satisfy closeout evidence. When documentation impact exists, name the `flow/docs/**` page(s) to update instead of declaring `none`.

Optional sections:

- `## Project Structure`
- `## Research` (R-NNN decisions captured in the plan)
- `## Principles Check` (only emitted when `docs/principles.md` exists and is non-empty)
- `## Complexity Tracking`

## `tasks.md`

- `## Tasks` *(mandatory)*, MAY contain `### User Story N - <title>` sub-groupings. Always emitted by `/flow-plan`.

## 4.3 Task (shape)

In `tasks.md`, under `## Tasks` (or a `### User Story N - …` sub-section):

```markdown
- [ ] **T-NNN**: <imperative one-line summary>.
  - Covers: FR-001, FR-002
  - Verifies: SC-001
  - Depends-On: T-014
  - Requires: docker
```

The ID MUST be the first bold token in the bullet. `/flow-plan` emits new
tasks in the `[ ]` state. Checkbox state semantics (`[ ]`, `[~]`, `[x]`), the
state transition rules, and the preflight `Requires:` catalog are detailed
in the `build` shard and not reproduced here.

## 4.6 Engineering Principle

In `docs/principles.md`, under `## Engineering Principles`:

```markdown
- **P-NNN**: <directive>. Rationale: <one sentence>.
```

`P-NNN` IDs are recommended but optional.

## 5. Cross-references

- Tasks reference FRs via `Covers: FR-NNN[, FR-NNN]`.
- Tasks reference SCs via `Verifies: SC-NNN[, SC-NNN]`.
- Drift reports reference items by bare ID.
- `status.md` references items by bare ID where applicable.
