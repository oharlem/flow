# Architecture Decision Records

Flow uses [MADR 3.0](https://adr.github.io/madr/) for architectural decisions.
Each decision lives in a numbered file in this directory. Records are kept
concise and current for the v0.1.0 in-development prototype.

| # | Status | Title |
|---|---|---|
| 0001 | Accepted | [Choose Rust for the Flow CLI](./0001-choose-rust.md) |
| 0002 | Accepted | [Shell out to `git` instead of linking `libgit2`](./0002-git-shellout-vs-libgit2.md) |
| 0009 | Accepted | [Documentation architecture (three planes, one owner per fact)](./0009-documentation-architecture.md) |
| 0010 | Accepted | [Close command and local history](./0010-release-commands-and-local-history.md) |
| 0012 | Accepted | [Sharded conventions layout under `.flow/conventions/`](./0012-conventions-shards.md) |
| 0016 | Accepted | [Unified run workspace](./0016-unified-run-workspace.md) |
| 0017 | Accepted | [Cargo Git install](./0017-cargo-only-install.md) |
| 0018 | Accepted | [Run-finalize closing commit](./0018-run-finalize-closing-commit.md) |
