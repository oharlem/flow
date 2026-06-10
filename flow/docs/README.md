# Flow Documentation

This directory contains current Flow workflow guidance for this repository.
Run and change history belongs in `flow/runs/`; do not duplicate closed-change
summaries here.

## Current Pages

- [Directory Layout](directory-layout.md) explains the `.flow/` control plane
  and the visible `flow/` workspace.
- [Artifact Index](artifact-index.md) lists preserved artifact classes and
  where they live.
- [Non-Logic Verification](non-logic-verification.md) defines the verification
  gate for structure-only work.

## Installation And Launching

The real `flow` executable lives outside project repositories. During v0.1.0
development, install it directly from GitHub with
`cargo install --git https://github.com/oharlem/flow --locked flow-cli`.
Cargo places the executable in its configured bin directory (typically
`~/.cargo/bin/flow`). Generated host assets invoke that installed binary with
`FLOW_HOST=<host>`.

## Repository State

Project-specific Flow state stays under `.flow/`, including
`.flow/config.yaml`, `.flow/state.yaml`, `.flow/version`, and
`.flow/agents/*.local.md` prompt overrides.

Flow docs live under `flow/docs/`. Application-owned product, architecture, and
reference docs live under `docs/`. See [Directory Layout](directory-layout.md)
for placement details.

## Roadmap-Scoped Runs

`flow roadmap <source>` creates a planned date-based `roadmap-<slug>` run
directory under `flow/runs/` with `run.md`, `roadmap.md`, a log, owner's
manual, release notes, child changes, and roadmap delivery state. `run.md`
records a `Roadmap fingerprint` so attach and resume workflows can tell whether
the run-local roadmap still matches the run state.

Run state is stored in `run.md` as fields such as `Run name`, `Run type`, `Roadmap fingerprint`,
`Run branch`, `Current milestone`, `Current change`, `Current phase`,
`Next command`, and `Last checkpoint`, followed by change and milestone indexes.
When a user intentionally edits `flow/runs/<run>/roadmap.md` during an open run,
`flow run --rescan` refreshes the saved fingerprint and milestone snapshot;
without that explicit rescan, fingerprint mismatches remain a stop.

Child commands carry `FLOW_RUN_DIR=<run-dir>` directly in the same command so
they reuse the run context on the first attempt and stay on the run branch. The
first child start for a roadmap delivery run is printed as:

```sh
FLOW_RUN_DIR="<run-dir>" flow start <M-N>
```

When every roadmap milestone is complete and the run manual and release notes
are no longer placeholders, `flow run --finalize` validates the final checked
`flow/runs/<run-id>/roadmap.md` and marks the run complete. There is no root
roadmap reset. This finalization follows `review.before_finalize` and
`review.per_command.run`; it does not use a run-level auto-finalize setting.
`release-notes.md` records the actual delivered changes for users and
operators; root `CHANGELOG.md` remains separate versioned release history.

## Documentation Impact

Every `plan.md` has `## Documentation Impact`. If current Flow guidance
changes, that section names the affected `flow/docs/**` pages. If central docs
are already current, the plan must declare:

```markdown
Impact: none

Docs already current because <rationale>.
```

A docs-current rationale without `Impact: none` does not bypass the closeout
documentation evidence gate.

## Embedded Default Assets

Default conventions and base phase prompts are embedded in the `flow` binary.
Use `flow export-assets --dir <DIR>` to inspect embedded defaults. Keep local
prompt customizations in `.flow/agents/*.local.md`.
