# 0010 — Close command and local history

**Status**: Accepted
**Date**: 2026-05-10

## Context

Flow needs a final workflow step that marks a completed change as historical
without assuming anything about the host application's package manager,
versioning scheme, or Git release process. GitHub releases are tag-based
distribution events, while package versions belong to project-native tooling.
Flow's closeout should remain workflow-focused.

## Decision

Flow exposes `flow close` as the only public closeout command. Host adapters
expose the same intent as `/flow-close` or `$flow-close`.

The default branch prefix remains `flow/`. The prefix names ownership by the
Flow workflow, not just the drafting spec, and it continues to cover the full
start-plan-build-test-close lifecycle. Projects that prefer another branch
namespace is still configurable through Flow's git settings in `.flow/config.yaml`.

`flow close` updates Flow artifacts only: it stamps the child change closed,
verifies central documentation evidence, ticks linked milestones, and updates
the parent run state. Flow does not bump application
versions, edit package manifests, create commits, create tags, merge branches,
push, pull, fetch, or call GitHub/GitLab CLIs.

## Consequences

Users get one clear final workflow command. Project-native release tooling
remains responsible for version bumps, tags, distribution, and publishing after
the closed Flow changes have been reviewed and merged through the normal path.
The "Flow never creates repository history" rule has no closeout exception.
