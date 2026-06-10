# Spec: M-1-cli-prints-the-default-greeting

**Change**: M-1-cli-prints-the-default-greeting
**Created**: 2026-06-10
**Closed**: 2026-06-10

---

## What & Why

The project needs its first piece of executable behavior: running
`python3 hello.py` prints exactly `Hello, world!`. This milestone delivers
the default greeting only — no arguments, no options — and establishes where
code lives and how it is tested.

Per the roadmap's constraints, the greeting text comes from a pure function
that the CLI entry point wraps, so behavior is testable without spawning a
subprocess, and the implementation uses the Python standard library only.
A unit test pins the default greeting so `flow test` has something real to
gate on from the start.

## Requirements

### Functional Requirements

- **FR-001**: Running the CLI with no arguments prints exactly `Hello, world!` followed by a newline.

## Success Criteria

### Measurable Outcomes

- **SC-001**: An automated test asserts the default output is `Hello, world!` and the suite passes.

## Out of Scope

- Name arguments or any other CLI options (deferred to M-2).
- Third-party dependencies.
