# 0001 — Choose Rust for the Flow CLI

- **Status:** Accepted
- **Date:** 2026-05-06

## Context

Flow needs deterministic parsing, fast startup, offline operation, and a
single command-line binary that host adapters can call from any supported
coding environment.

## Decision

Build Flow as a Rust workspace that produces one `flow` CLI binary. Keep the
host-neutral parser, renderer, drift, roadmap, and envelope logic in
`flow-core`; keep the executable surface in `flow-cli`; keep host asset
installers in per-host adapter crates.

## Consequences

- One language and one binary for the runtime surface.
- Strong typing for artifact IDs and parser outputs.
- No interpreter dependency on user machines.
- Host adapters share the same core behavior and differ only in installed
  host files.
