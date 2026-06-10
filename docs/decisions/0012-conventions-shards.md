# 0012 — Embedded Convention Shards

**Status**: Accepted
**Date**: 2026-05-11

## Context

Every Flow phase needs artifact grammar instructions, but each phase needs a
different subset. The CLI must also work offline and avoid creating
project-local generated files that users could mistake for editable workflow
state.

## Decision

Store the canonical convention shards in `assets/conventions/*.md` and embed
them in `flow-core` with `include_str!`. Envelope composition loads `core.md`
for every phase and one phase shard when needed:

- `start` and `amend` load `spec.md`.
- `plan` loads `plan.md`.
- `build` and `build-task` load `build.md`.
- `test` loads `test.md`.
- `close` loads `close.md`.
- `run` loads `run.md`.

`flow export-assets --dir <DIR>` writes inspectable copies when a user wants to
review the embedded defaults.

## Consequences

- Phase envelopes stay smaller because they include only the relevant shards.
- The binary is self-contained and works offline.
- `.flow/` keeps configuration, state, version markers, and local prompt
  overrides, not generated copies of the default grammar.
- Users inspect defaults explicitly through `flow export-assets`.
