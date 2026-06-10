# Security

Flow is a local, offline CLI. It stores workflow state in the repository and
does not contact external services. For vulnerability reporting, see the root
[SECURITY.md](../SECURITY.md).

## At A Glance

| Concern | Flow behavior |
|---|---|
| Network calls | None |
| Telemetry | None |
| Credential storage | None; Flow never reads `~/.ssh`, `~/.aws`, `~/.netrc`, git credential helpers, or GitHub CLI config |
| Remote git | Never runs `git push`, `git pull`, `git fetch`, `gh`, or `glab` |
| Destructive git | Never runs `git reset --hard`, `git clean -fd`, force operations, or tag creation |
| Local commits | Only roadmap-run checkpoint commits and the run-closing `flow run --finalize` commit when `git.run_checkpoint_commits: true` and the run has a branch |
| Test execution | Runs configured or auto-detected local test commands |
| Preflight checks | Runs only task-declared `Requires:` checks |

## What Flow Reads And Writes

Flow reads project policy and state under `.flow/` (`config.yaml`,
`state.yaml`, `version`, optional prompt overrides under `agents/`), workflow
artifacts under `flow/runs/<run>/`, current-state docs under `flow/docs/`, and
the optional `docs/principles.md`. It honors the environment variables
`FLOW_FORCE_ON_PROTECTED`, `FLOW_LOG`, `FLOW_CHANGE_DIR`, and `FLOW_RUN_DIR`.

Flow writes only inside the repository: `.flow/` bookkeeping, `flow/runs/`
artifacts, generated host assets under `.claude/`, `.agents/`, or `.cursor/`,
and a marker-bounded notes block in `AGENTS.md`. The one exception is
`flow export-assets --dir <DIR>`, which writes embedded defaults to a
caller-selected directory. `flow update` refreshes generated default assets
only when they exactly match the embedded copy; divergent copies and local
overrides are preserved.

The real `flow` executable lives in Cargo's bin directory (typically
`~/.cargo/bin/flow`). During v0.1.0 development, install it from GitHub with
`cargo install --git https://github.com/oharlem/flow --locked flow-cli`.
Generated host assets invoke that installed binary with `FLOW_HOST=<host>`;
see [Host adapters](./hosts.md).

## What Flow Never Does

- Make outbound network calls.
- Read credential stores of any kind.
- Write outside the repository root (except explicit `export-assets`).
- Spawn cloud SDKs or GitHub/GitLab CLIs.
- Push, pull, fetch, force, reset, clean, or tag.

If Flow does any of these things, report it as a bug.

## Supply Chain

Runtime dependencies are declared in the workspace `Cargo.toml`. `cargo deny
check` enforces license and advisory policy in CI, which runs workspace tests
on `ubuntu-latest`, `macos-14`, and `macos-13` for every pull request and push
to `main`. The v0.1.0 install path uses `cargo install --git`, so Cargo builds
locally from the selected GitHub revision and the locked manifest. Publishing
to crates.io is optional later, not a prerequisite for this first release line.

## Non-Goals

- Flow does not sandbox untrusted repositories. A repo's configured or
  auto-detected test command, and task-declared preflight commands, can
  execute repository code.
- Flow does not run the AI agent; it prints the envelope for the host.
- Flow does not provide binary signature verification.
