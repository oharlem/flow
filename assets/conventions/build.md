# Flow Artifact Conventions — Build

```
Conventions-Version: 1.1
```

Artifact shapes used by `/flow-build` and `/flow-build-task`. Core invariants live in `core.md` and are always loaded alongside this shard.

---

## `tasks.md`

- `## Tasks` *(mandatory)*, MAY contain `### User Story N - <title>` sub-groupings.

## 4.3 Task

In `tasks.md`, under `## Tasks` (or a `### User Story N - …` sub-section):

```markdown
- [ ] **T-NNN**: <imperative one-line summary>.
  - Covers: FR-001, FR-002
  - Verifies: SC-001
  - Depends-On: T-014
  - Requires: docker
```

Task checkbox states are Flow-owned:

- `[ ]` — not implemented or not review-ready.
- `[~]` — implemented locally and awaiting user acceptance.
- `[x]` — accepted and saved into Flow state.

State transition: `[ ]` → `[~]` is performed by the build agent after tests pass
and before asking the user to accept the work. `[~]` → `[x]` is performed by
Flow's build driver only after the user accepts the implemented task. The same
state-save step updates `status.md` with the accepted task ID. This state save
does not create repository history and does not require pending git changes.
`[~]` does not satisfy `Depends-On` and does not count as complete for
`/flow-test` or release.

`Requires:` is optional and lists task-scoped preflight requirement IDs, such
as `docker`, that must be available before implementation starts. Multiple
requirements are comma-separated. Omit `Requires:` when the task can be
implemented and verified without extra local services. Flow validates every
listed requirement against the built-in catalog plus `.flow/config.yaml:
preflight.requirements`.

## 5. Cross-references

- Tasks reference FRs via `Covers: FR-NNN[, FR-NNN]`.
- Tasks reference SCs via `Verifies: SC-NNN[, SC-NNN]`.
- Drift reports reference items by bare ID.
- `status.md` references items by bare ID where applicable.
