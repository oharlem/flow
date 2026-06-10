# Implementation Plan: M-1-cli-prints-the-default-greeting

**Branch**: `flow/run-20260610-roadmap-hello-cli-product-notes` | **Date**: 2026-06-10 | **Spec**: [spec.md](./spec.md)

## Summary

Add `hello.py` at the repo root: a pure `greet()` function returning
`"Hello, world!"` and a `__main__` block that prints it. Add
`test_hello.py` with a unittest case for the function's return value and a
subprocess case pinning the CLI's exact stdout. Tests are written first, per
the build protocol; the named-greeting behavior is explicitly deferred to
M-2, so `greet()` takes no arguments yet.

## Technical Context

**Language/Version**: Python 3.9 (system `python3`)
**Primary Dependencies**: None — standard library only
**Storage**: None
**Testing**: `unittest`, discovered via `python3 -m unittest discover -s . -p 'test_*.py' -v`
**Target Platform**: Any OS with Python 3 on PATH
**Project Type**: CLI
**Performance Goals**: Not applicable for a print-and-exit script
**Constraints**: No third-party packages; no CLI arguments in this milestone
**Scale/Scope**: Two new files (`hello.py`, `test_hello.py`), ~30 lines

## Documentation Impact

Impact: none

Docs already current because this milestone introduces the project's first
behavior and no `flow/docs/**` page describes the greeter yet.
