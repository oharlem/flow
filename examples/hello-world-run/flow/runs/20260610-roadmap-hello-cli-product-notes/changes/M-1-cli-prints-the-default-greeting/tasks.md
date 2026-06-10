# Tasks: M-1-cli-prints-the-default-greeting

**Input**: Design documents from `/changes/M-1-cli-prints-the-default-greeting/`
**Prerequisites**: plan.md (✓), spec.md (✓)

## Format: `[ID] [P?] Description`

- **[P]**: Can run in parallel (different files, no dependencies on incomplete tasks)
- All file paths are relative to the repo root

---

## Tasks

- [x] **T-001**: Implement `hello.py` with a pure `greet()` function and a CLI entry point that prints the default greeting.
  - Covers: FR-001
  - Depends-On: (none)
- [x] **T-002**: Add `test_hello.py` pinning `greet()`'s return value and the CLI's exact stdout.
  - Verifies: SC-001
  - Depends-On: T-001
