# 0009 — Documentation architecture (three planes, one owner per fact)

**Status**: Accepted
**Date**: 2026-05-09

## Decision

Flow documentation is organized into three planes: INTENT for task-oriented
guides, RECORD for durable facts and decisions, and AGENT CONTEXT for the
host-specific instructions agents load while working. Each durable fact has one
owner, and other documents link to that owner instead of duplicating it.
Flow-owned current-state docs live under `flow/docs/` by default and are kept
up to date as part of Flow change work.

## Consequences

Review markers and touch-map advisories help identify stale INTENT pages, and
agent context stays thin by linking back to RECORD pages instead of restating
facts.
