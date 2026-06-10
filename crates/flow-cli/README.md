# flow-cli

The `flow` binary — a spec-driven workflow toolkit for AI coding agents.

Flow is a local Rust CLI that helps an AI coding host (Claude Code, Codex,
Cursor) turn an idea into a reviewable spec, plan, task list,
implementation, verification record, and run record. It stores the workflow
as plain Markdown in the repository so reviewers can inspect the same source
of truth.

Flow does not call an LLM itself. Each `flow` command composes an envelope
that the AI coding host consumes.

Flow v0.1.0 is an in-development early prototype.

## Install

```sh
cargo install --git https://github.com/oharlem/flow --locked flow-cli
```

Requires a Rust toolchain — install via [rustup](https://rustup.rs/).

## Quickstart

```sh
cd your-repo
flow init --host claude-code      # or codex | cursor
flow start "add login form"
flow plan
flow build-task
flow test
flow close
```

Run these commands inside a host session so the host can implement the
change. Standalone they update Flow state and emit the envelope text.

## Documentation

- Project home: <https://github.com/oharlem/flow>
- Documentation: <https://github.com/oharlem/flow/blob/main/docs/README.md>
- First-change tutorial: <https://github.com/oharlem/flow/blob/main/docs/start-here/01-your-first-change.md>

## License

MIT.
