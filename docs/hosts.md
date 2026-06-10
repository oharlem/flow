# Host Adapters

Flow ships host adapters for AI coding environments. Each adapter installs the
host-specific files needed to expose the same Flow commands; the workflow,
artifacts, and record format are identical across hosts.

| Host | Invocation | Installed files | Crate |
|---|---|---|---|
| Claude Code | `/flow-<name>` | `.claude/skills/flow-*/SKILL.md`, `.claude/settings.json` | `flow-host-claude-code` |
| Codex | `$flow-<name>` | `.agents/skills/flow-*/SKILL.md` | `flow-host-codex` |
| Cursor (preview) | `/flow-<name>` via rule guidance | `.cursor/rules/flow.mdc` | `flow-host-cursor` |

Use `flow setup --host <name>` to add an adapter. Cursor support is preview
because Cursor rules are a lighter integration than host-native skills or
slash commands.

## Shared Contract

- Flow drivers print an envelope to stdout for the host to consume.
- Installed host files set `FLOW_HOST=<host>` so next-command recommendations
  render in the right syntax (`/flow-plan`, `$flow-plan`, and so on).
- Host run commands keep one shape: `FLOW_HOST=<host> flow run` with an
  optional `M-N` or `all` target.
- Host instructions tell agents not to edit `status.md` by hand, not to invent
  `/flow-finalize`, and not to run remote or destructive git commands.
- Each adapter installs the same skill set: `flow-setup`, `flow-doctor`,
  `flow-start`, `flow-amend`, `flow-plan`, `flow-run`, `flow-build`,
  `flow-build-task`, `flow-test`, `flow-close`, `flow-status`.
- The root `AGENTS.md` gets a Flow-owned, marker-bounded notes section per
  host, refreshed by `flow update` without touching user-owned content.

## Installed Binary

Generated host assets invoke the installed `flow` binary with
`FLOW_HOST=<host>`. The executable lives in Cargo's bin directory (typically
`~/.cargo/bin/flow`) after
`cargo install --git https://github.com/oharlem/flow --locked flow-cli`. If
`flow` is not on `PATH`, install the Flow CLI, then retry the host command.
