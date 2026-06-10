# Spec: M-2-cli-greets-a-named-user

**Change**: M-2-cli-greets-a-named-user
**Created**: 2026-06-10
**Closed**: 2026-06-10

---

## What & Why

With the default greeting shipped in M-1, the CLI should now greet a named
user: `python3 hello.py Ada` prints exactly `Hello, Ada!`, preserving the
name's capitalization verbatim. The default behavior from M-1 must not
change — `python3 hello.py` still prints `Hello, world!`.

The roadmap constrains this to one optional positional argument, standard
library only, with the logic staying in the pure `greet` function so both
behaviors remain testable without subprocesses.

## Requirements

### Functional Requirements

- **FR-001**: Running the CLI with a single name argument prints exactly `Hello, <name>!` followed by a newline, preserving the name's capitalization.
- **FR-002**: Running the CLI with no arguments still prints exactly `Hello, world!` followed by a newline (M-1 behavior unchanged).

## Success Criteria

### Measurable Outcomes

- **SC-001**: An automated test asserts that greeting a name (e.g. `Ada`) yields `Hello, Ada!` and the suite passes.
- **SC-002**: The M-1 default-greeting tests still pass unchanged.

## Out of Scope

- Localization, flags, or multiple-name support.
