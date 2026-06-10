# CLI reference

<!-- Flow-Managed: cli-reference v1 -->

*Generated from `cargo run -p flow-cli --example generate_cli_reference`. Do not edit by hand.*

## `flow`

```
Spec-driven workflow toolkit for AI coding agents.

Usage: flow [OPTIONS] <COMMAND>

Commands:
  init           Install Flow into the current repository
  update         Update Flow templates and host assets in the current repository
  doctor         Check the local Flow installation
  export-assets  Export embedded default conventions and base prompts
  start          Draft a new change spec
  amend          Update the active change spec
  plan           Draft the implementation plan and task list
  build          Implement all remaining tasks
  build-task     Implement one task
  test           Run verification: tests and consistency checks
  close          Close a completed change in place
  status         Show current status, consistency findings, and next action
  set            Store a project setting
  settings       Show current project settings
  setup          Install or upgrade Flow host assets
  roadmap        Decompose a PRD or notes file into a planned roadmap run
  run            Start or continue a planned roadmap run
  help           Print this message or the help of the given subcommand(s)

Options:
      --json
          Emit machine-readable JSON where supported

  -h, --help
          Print help

  -V, --version
          Print version

```

## `flow amend`

```
Update the active change spec

Usage: flow amend [OPTIONS] [CHANGE]...

Arguments:
  [CHANGE]...
          Change request text

Options:
      --ask <QUESTION>
          Append a Q/A pair to `## Clarifications` in `spec.md`.
          
          Must be used with `--answer`.

      --json
          Emit machine-readable JSON where supported

      --answer <ANSWER>
          Answer text that pairs with `--ask`

      --finalize
          Post-model finalize step. The change directory is inferred from `FLOW_CHANGE_DIR` /
          run-state / the current branch

  -h, --help
          Print help (see a summary with '-h')

```

## `flow build`

```
Implement all remaining tasks

Usage: flow build [OPTIONS]

Options:
      --finalize
          Post-model finalize step. The change directory is inferred from `FLOW_CHANGE_DIR` /
          run-state / the current branch

      --json
          Emit machine-readable JSON where supported

  -h, --help
          Print help

```

## `flow build-task`

```
Implement one task

Usage: flow build-task [OPTIONS] [T-NNN]

Arguments:
  [T-NNN]
          Optional task selector (e.g. `T-001`)

Options:
      --finalize
          Post-model finalize step. The change directory is inferred from `FLOW_CHANGE_DIR` /
          run-state / the current branch

      --json
          Emit machine-readable JSON where supported

  -h, --help
          Print help

```

## `flow close`

```
Close a completed change in place

Usage: flow close [OPTIONS]

Options:
      --json
          Emit machine-readable JSON where supported

  -h, --help
          Print help

```

## `flow doctor`

```
Check the local Flow installation

Usage: flow doctor [OPTIONS]

Options:
      --json
          Emit machine-readable JSON where supported

  -h, --help
          Print help

```

## `flow export-assets`

```
Export embedded default conventions and base prompts

Usage: flow export-assets [OPTIONS] --dir <DIR>

Options:
      --dir <DIR>
          Directory where embedded conventions and base prompts should be written

      --json
          Emit machine-readable JSON where supported

  -h, --help
          Print help

```

## `flow init`

```
Install Flow into the current repository

Usage: flow init [OPTIONS]

Options:
      --host <HOSTS>
          Comma-separated hosts to wire up (`claude-code,codex,cursor`)

      --json
          Emit machine-readable JSON where supported

  -h, --help
          Print help

```

## `flow plan`

```
Draft the implementation plan and task list

Usage: flow plan [OPTIONS]

Options:
      --finalize
          Post-model finalize step. The change directory is inferred from `FLOW_CHANGE_DIR` /
          run-state / the current branch

      --json
          Emit machine-readable JSON where supported

  -h, --help
          Print help

```

## `flow roadmap`

```
Decompose a PRD or notes file into a planned roadmap run

Usage: flow roadmap [OPTIONS] [SOURCE]...

Arguments:
  [SOURCE]...
          Source: path to a PRD/notes file, or inline text. Reads stdin when empty and not a TTY

Options:
      --append
          Always append new milestones (never prompt)

      --json
          Emit machine-readable JSON where supported

      --replace
          Replace existing milestones (always prompts when confirmation=required)

      --finalize
          Post-model finalize step for a planned roadmap run

  -h, --help
          Print help

```

## `flow run`

```
Start or continue a planned roadmap run

Usage: flow run [OPTIONS] [TARGET]

Arguments:
  [TARGET]
          Optional milestone to target inside the active roadmap run (e.g. `M-1`). Omit to start or
          continue the run across every open milestone

Options:
      --json
          Emit machine-readable JSON where supported

      --resume [<RUN_DIR>]
          Resume guidance for an interrupted run

      --rescan [<RUN_DIR>]
          Refresh run roadmap fingerprint and milestone snapshot from the run-local roadmap

  -h, --help
          Print help

```

## `flow set`

```
Store a project setting

Usage: flow set [OPTIONS] <ASSIGNMENT>

Arguments:
  <ASSIGNMENT>
          Setting assignment, such as `prefix=flow` or `confirmation=no`

Options:
      --json
          Emit machine-readable JSON where supported

  -h, --help
          Print help

```

## `flow settings`

```
Show current project settings

Usage: flow settings [OPTIONS]

Options:
      --json
          Emit machine-readable JSON where supported

  -h, --help
          Print help

```

## `flow setup`

```
Install or upgrade Flow host assets

Usage: flow setup [OPTIONS]

Options:
      --host <HOSTS>
          Comma-separated hosts to target (`claude-code,codex,cursor`)

      --json
          Emit machine-readable JSON where supported

  -h, --help
          Print help

```

## `flow start`

```
Draft a new change spec

Usage: flow start [OPTIONS] [DESCRIPTION]...

Arguments:
  [DESCRIPTION]...
          Free-form change description. May include a single positional `M-N` token to link a
          roadmap milestone

Options:
      --finalize
          Post-model finalize step. The change directory is inferred from `FLOW_CHANGE_DIR` /
          run-state / the current branch

      --json
          Emit machine-readable JSON where supported

  -h, --help
          Print help

```

## `flow status`

```
Show current status, consistency findings, and next action

Usage: flow status [OPTIONS]

Options:
      --change-dir <CHANGE_DIR>
          Explicit change directory (default: resolve from branch or run state)

      --json
          Emit machine-readable JSON where supported

  -h, --help
          Print help

```

## `flow test`

```
Run verification: tests and consistency checks

Usage: flow test [OPTIONS]

Options:
      --finalize
          Post-model finalize step. The change directory is inferred from `FLOW_CHANGE_DIR` /
          run-state / the current branch

      --json
          Emit machine-readable JSON where supported

  -h, --help
          Print help

```

## `flow update`

```
Update Flow templates and host assets in the current repository

Usage: flow update [OPTIONS]

Options:
      --force
          Drop divergent generated default-asset copies under `.flow/conventions/` and
          `.flow/agents/*.base.md`, accepting the embedded defaults. Local prompt overrides under
          `.flow/agents/*.local.md` are preserved. Also allows the update to proceed when the
          running `flow` binary is older than the version recorded in `.flow/version`

      --json
          Emit machine-readable JSON where supported

  -h, --help
          Print help

```
