# Release Notes: Hello CLI — product notes

**Run**: 20260610-roadmap-hello-cli-product-notes
**Target**: planned roadmap
**Started**: 2026-06-10T17:39:54Z
**Status**: complete

## Delivered Changes

### M-1: CLI prints the default greeting

- Added `hello.py` with a pure `greet()` function and a CLI entry point.
- Added tests for the function return value and exact `python3 hello.py` stdout.

### M-2: CLI greets a named user

- Extended `greet()` to accept an optional `name="world"` parameter.
- Extended the CLI to pass one optional positional argument.
- Added tests for `greet("Ada")`, `python3 hello.py Ada`, and unchanged default behavior.

## User Impact

Users can now run:

```sh
python3 hello.py
python3 hello.py Ada
```

The first command prints `Hello, world!`; the second prints `Hello, Ada!`.

## Upgrade Notes

No migration, dependency installation, or packaging step is required.

## Verification Summary

- M-1 tests failed before implementation because `hello.py` did not exist, then passed after the default greeter was implemented.
- M-2 named-greeting tests failed against the M-1 implementation, then passed after `greet(name="world")` and CLI argument handling were added.
- Final verification command: `python3 -m unittest discover -s . -p 'test_*.py' -v`.
- Flow closeout refused a deliberately broken traceability record with D1 errors, then passed after `Covers:` references were restored.
- Flow checkpoint commits were created for M-1 and M-2.

## Source Milestones

- M-1: CLI prints the default greeting.
- M-2: CLI greets a named user.
