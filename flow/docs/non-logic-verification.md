# Non-Logic Verification

<!-- Flow-Managed: docs-page v1 -->

Use this gate after directory-only Flow work to prove the change stayed
structural. The expected result is that Flow artifacts, documentation,
configuration, and focused tests may change, while runtime code, package
metadata, lockfiles, and embedded assets remain untouched.

## Expected Scope

Expected for structure-only work:

- `flow/runs/**` and `flow/docs/**`
- `.flow/config.yaml` and `.flow/state.yaml` when Flow records local workflow
  settings
- focused tests that protect structure and discoverability

Unexpected for structure-only work:

- `Cargo.toml` or `Cargo.lock`
- runtime source under `crates/*/src/`
- embedded host, template, or convention assets under `assets/`
- command behavior, parser behavior, renderer behavior, or host adapter logic

## Gate Commands

Run from the repository root:

```sh
cargo fmt --all --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
git diff --name-only -- Cargo.toml Cargo.lock crates/flow-cli/src crates/flow-core/src crates/flow-host-claude-code/src crates/flow-host-codex/src crates/flow-host-cursor/src assets
```

The final `git diff --name-only` command is read-only. It should print no file
names for directory-only work. If it prints a runtime source, package manifest,
lockfile, or embedded asset path, either move that work into a separate change
or document why the change is no longer directory-only.

## Safety Boundaries

Do not run commits, tags, remote git operations, or destructive git commands as
part of this gate. That means no `git push`, `git pull`, `git fetch`,
`git reset --hard`, `git clean -fd`, `gh`, or `glab`.

Do not use this gate to invent alternate Flow artifact locations. Run and
change records stay under `flow/runs/`.

## Evidence To Record

Latest baseline evidence came from the 2026-05-10 directory-layout cleanup run.
Future structure-only work should record the same kind of evidence.

When using this gate, record:

- commands run
- pass/fail outcome
- any runtime-scope diff and why it is acceptable
- link to the run record that contains the final evidence
