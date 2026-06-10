# Implementation Plan: M-2-cli-greets-a-named-user

**Branch**: `flow/run-20260610-roadmap-hello-cli-product-notes` | **Date**: 2026-06-10 | **Spec**: [spec.md](./spec.md)

## Summary

Extend `greet()` to accept an optional `name` parameter defaulting to
`"world"`, and extend the CLI entry point to pass `sys.argv[1]` when present.
Add tests first: a function-level case for the named greeting and a
subprocess case pinning the CLI's exact stdout for a named argument. The
existing M-1 tests stay untouched and must keep passing, which verifies the
default behavior is unchanged.

## Technical Context

**Language/Version**: Python 3.9 (system `python3`)
**Primary Dependencies**: None — standard library only
**Storage**: None
**Testing**: `unittest`, discovered via `python3 -m unittest discover -s . -p 'test_*.py' -v`
**Target Platform**: Any OS with Python 3 on PATH
**Project Type**: CLI
**Performance Goals**: Not applicable for a print-and-exit script
**Constraints**: One optional positional argument only; M-1 default output byte-identical
**Scale/Scope**: Two files touched (`hello.py`, `test_hello.py`), ~15 lines changed

## Documentation Impact

Impact: none

Docs already current because no `flow/docs/**` page documents the greeter;
usage is covered by the run-level owner's manual at finalize.
