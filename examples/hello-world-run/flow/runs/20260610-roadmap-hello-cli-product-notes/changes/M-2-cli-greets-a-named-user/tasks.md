# Tasks: M-2-cli-greets-a-named-user

**Input**: Design documents from `/changes/M-2-cli-greets-a-named-user/`
**Prerequisites**: plan.md (✓), spec.md (✓)

## Format: `[ID] [P?] Description`

- **[P]**: Can run in parallel (different files, no dependencies on incomplete tasks)
- All file paths are relative to the repo root

---

## Tasks

- [x] **T-001**: Extend `greet()` with an optional `name="world"` parameter and pass the CLI's first argument through.
  - Covers: FR-001, FR-002
  - Depends-On: (none)
- [x] **T-002**: Add named-greeting tests (function and CLI) and keep the M-1 default tests passing unchanged.
  - Verifies: SC-001, SC-002
  - Depends-On: T-001
