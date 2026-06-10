# Owner's Manual: Hello CLI — product notes

**Run**: 20260610-roadmap-hello-cli-product-notes
**Target**: planned roadmap
**Started**: 2026-06-10T17:39:54Z
**Status**: complete

## Quickstart

Run the CLI from the demo repository root:

```sh
python3 hello.py
python3 hello.py Ada
```

Expected output:

```text
Hello, world!
Hello, Ada!
```

## Resulting State

The run delivers a tiny Python CLI and its tests:

- `hello.py` exposes `greet(name="world")` and prints the default or named greeting.
- `test_hello.py` verifies both the pure function and subprocess CLI output.
- `flow/runs/20260610-roadmap-hello-cli-product-notes/roadmap.md` is fully checked off.
- Each milestone has its own closed change record under `changes/`.

## Configuration

The demo uses the standard-library Python test runner:

```sh
python3 -m unittest discover -s . -p 'test_*.py' -v
```

No third-party Python packages are required.

## Operating Guide

Use `greet()` directly when code needs the greeting string without spawning a process:

```python
from hello import greet

greet()
greet("Ada")
```

Use the script entry point for the CLI surface:

```sh
python3 hello.py
python3 hello.py Ada
```

Only one optional positional argument is supported. Flags, localization, packaging, and multiple-name formatting were outside this demo run.

## Troubleshooting

- If `python3 hello.py` cannot be found, run commands from the demo repository root.
- If tests fail with an import error for `hello`, verify `hello.py` is still at the repository root beside `test_hello.py`.
- If the CLI output changes, update the implementation and tests together; the Flow records intentionally pin exact stdout including the trailing newline.
