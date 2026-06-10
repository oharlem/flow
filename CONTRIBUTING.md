# Contributing to Flow

## Development environment

1. Install Rust via [rustup](https://rustup.rs/). The toolchain version is
   pinned in `rust-toolchain.toml`.
2. Clone the repo and run `cargo test --workspace`.

Everything else — formatting, linting, building — is standard Cargo.

## Updating a local Flow build in consumer repos

When you build a new local Flow version from source, reinstall the CLI binary
and refresh any repository where you use Flow.

```sh
# In this repo:
cd ~/flow
make up

# In a consumer repo (example):
cd ~/e/productops
flow doctor
flow update
```

`flow update` refreshes core Flow files plus generated host assets that are
already installed in that repo. Run `flow setup --host <name>` only when you
need to add or repair a host adapter.

## Running tests

```sh
cargo test --workspace            # all tests
cargo test -p flow-core           # only the core library
cargo test --test integration     # end-to-end CLI tests
```

Integration tests spawn the compiled `flow` binary against a scratch git repo
so `git` ≥ 2.30 must be on `$PATH`.

## Style

- `cargo fmt --all` before pushing.
- `cargo clippy --workspace --all-targets -- -D warnings` must be green.
- No `unwrap()` / `expect()` outside tests and `main.rs` setup; return
  `flow_core::Result` instead.
- Prefer `&str` over `String` in signatures; prefer `&Path` over `&PathBuf`.

## Commit style

Conventional commits: `feat:`, `fix:`, `docs:`, `refactor:`, `test:`, `chore:`.
Keep subject lines ≤ 72 chars.

## ADRs

Architectural decisions live under `docs/decisions/NNNN-kebab-title.md`, in
[MADR 3.0](https://adr.github.io/madr/) format. Number sequentially.

## Adding a host adapter

1. `cargo new --lib crates/flow-host-<name>`
2. Depend on `flow-core`. Embed assets via `include_str!`.
3. Register the crate in `flow-cli/Cargo.toml` + `flow-cli/src/cmd/init.rs`'s
   `install_host` function.
4. Add an integration test in `crates/flow-cli/tests/integration.rs` that
   runs `flow init --host <name>` and asserts the expected file tree.

## Pull requests

- One topic per PR.
- Update docs + tests in the same PR as the code change.
- Add a `CHANGELOG.md` entry.
