# 0002 — Shell out to `git` instead of linking `libgit2`

- **Status:** Accepted
- **Date:** 2026-05-06

## Context

Flow needs a tiny subset of git: `rev-parse`, `status`, `switch`, `branch`,
`show-ref`, `worktree add`. Flow never pushes, pulls, or fetches.

## Decision

Shell out to the `git` CLI via `std::process::Command` instead of linking
`libgit2-sys`.

## Consequences

- No build-time dependency on OpenSSL/zlib/libgit2-sys.
- Cross-compile to `aarch64-unknown-linux-musl` "just works".
- Flow delegates auth, credential handling, and protocol details to the
  user's `git` install, which is already required for Flow.
- Per-invocation cost: ~5-10 ms overhead per `git` call; acceptable given
  Flow runs only a handful of git calls per command.
