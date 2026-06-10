# D1-D3 Drift Rules

Flow's drift engine compares `spec.md` and `tasks.md` so closeout does not
silently lose requirements or success criteria.

## Severity Model

| Rule | `flow status` | `flow test` | `flow plan --finalize` | `flow close` |
|---|---|---|---|---|
| D1 | warn | error | warn | error |
| D2 | warn | error | error | error |
| D3 | warn | error | error | error |

## Rules

### D1 - Requirement Has No Task

Fires when an `FR-NNN` defined in `spec.md` is never listed under any task's
`Covers:`.

Fix it by adding a task whose `Covers:` includes the FR, or by removing the FR
from `spec.md` if it is no longer required.

### D2 - Task Points To A Missing Requirement

Fires when a task's `Covers:` references an FR that `spec.md` does not define.

Fix it by correcting the typo, adding the FR to `spec.md`, or removing the stale
reference from the task.

### D3 - Task Points To A Missing Success Criterion

Fires when a task's `Verifies:` references an SC that `spec.md` does not define.

Fix it by correcting the typo, adding the SC to `spec.md`, or removing the stale
reference from the task.

## Missing Tasks File

When `tasks.md` is absent, Flow reports that planning is incomplete and points
back to `flow plan`.
