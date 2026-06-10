# Directory Layout

<!-- Flow-Managed: docs-page v1 -->

Flow uses two repository-local roots for repo-local state:

- `.flow/` is the hidden control plane for config, state, install/version
  bookkeeping, and local prompt overrides.
- `flow/` is the visible workspace for Flow artifacts that humans and agents
  read while planning, building, testing, and closing work.

This repository uses layout v2, recorded in `.flow/config.yaml` as
`layout.version: 2`.

## Canonical Directories

```text
.flow/
  agents/
  config.yaml
  state.yaml
  version
flow/
  docs/
  runs/
    <YYYYMMDD-slug>/
      run.md
      log.md
      manual.md
      release-notes.md
      roadmap.md
      changes/
        <change-slug>/
```

Default artifact-grammar shards and base phase prompts are embedded in the
`flow` binary. `core.md` loads on every envelope, plus one phase shard for the
active phase. Use `flow export-assets --dir <DIR>` to write inspectable copies
when needed.

`.flow/config.yaml` is the human-editable project policy file. It stores
layout, prefix, hosts, confirmation, git, test, preflight, phase, UI, and docs
settings.

`.flow/state.yaml` is Flow-owned mutable state, such as the next roadmap
milestone counter.

Generated host assets invoke the installed `flow` executable with
`FLOW_HOST=<host>`. If `flow` is not on `PATH`, install the Flow CLI before
running host commands.

## Ownership

Flow owns `.flow/` runtime files and the human-facing artifacts under `flow/`.
Agents may edit child change artifacts in `flow/runs/<run>/changes/<change>/`,
run handoffs in `flow/runs/YYYYMMDD-<run-slug>/`, and current-state Flow docs
in `flow/docs/` when the active plan calls for it.

Application-owned source code remains outside the Flow workspace. In this repo,
that mainly means `crates/`, `assets/`, `docs/`, and `tests/`.

Application docs under `docs/` describe the product, architecture, public
reference, release, and security posture. Flow docs under
`flow/docs/` describe current workflow behavior that closeout checks expect to
stay fresh.

## Placement Guide

Use `flow/runs/<run>/changes/<change>/` for the active spec, plan, tasks,
status for one Flow change. A one-off run has one child
change; a roadmap run has one child change per milestone.

Use `flow/runs/YYYYMMDD-roadmap-<run-slug>/` for roadmap delivery logs, owner
manuals, release notes, and the run-local roadmap created by `flow roadmap` and
continued by `flow run`. `run.md` records `Run name`, `Run type`, `Roadmap
fingerprint`, run branch/checkpoint state, current milestone/change/phase, next
command, last checkpoint, and change plus milestone indexes. Use `flow run
--rescan` after intentional run-local roadmap edits to refresh the saved
fingerprint and milestone snapshot for an open run.

Roadmap delivery child commands carry `FLOW_RUN_DIR=<run-dir>` directly so each
milestone start, plan, build, test, and close phase stays on the run branch.
The first child start is printed as:

```sh
FLOW_RUN_DIR="<run-dir>" flow start <M-N>
```

When a roadmap delivery run completes, `flow run --finalize` validates the run
handoff documents and leaves the final checked roadmap at
`flow/runs/YYYYMMDD-roadmap-<run-slug>/roadmap.md`. There is no root roadmap
reset. That run finalization follows `review.before_finalize` and
`review.per_command.run`; there is no run-level auto-finalize setting.

Use `flow/docs/` for current Flow workflow guidance, such as this layout guide
and the [Artifact Index](artifact-index.md). Do not put closed-change summary
pages here; `flow/runs/` owns run history.

Use `docs/` for application-owned product, architecture, public reference, and
release documentation.

Use `flow roadmap <source>` for future milestones; the generated milestones
live under that run's `roadmap.md`.

For broader context, read the public artifact map in
[`docs/reference/artifacts.md`](../../docs/reference/artifacts.md) and the
command reference in [`docs/reference/commands.md`](../../docs/reference/commands.md).

Do not duplicate Flow artifacts in multiple locations or change command logic
as part of documentation-only structure work.
