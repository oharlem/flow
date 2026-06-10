# Roadmap: Hello CLI

## Milestones

### [x] M-1: CLI prints the default greeting

Source: `docs/prd.md`, sections "Goals" item 1 and "Constraints".

Outcome: A user can run `python3 hello.py` and see exactly `Hello, world!`,
backed by the project's first unit tests.

Must preserve:
- Output is exactly the string `Hello, world!` followed by a newline.
- Greeting logic lives in a pure function so it is testable without subprocesses.
- Python standard library only; no third-party dependencies.

Done when:
- `python3 hello.py` prints `Hello, world!`.
- A unit test asserts the default greeting and the suite passes.

Do not include:
- Name arguments or any other CLI options (deferred to M-2).

### [x] M-2: CLI greets a named user

Source: `docs/prd.md`, sections "Goals" item 2 and "Constraints".

Outcome: A user can run `python3 hello.py <name>` and see `Hello, <name>!`
with the name's capitalization preserved.

Must preserve:
- Output is exactly `Hello, <name>!` followed by a newline (e.g. `Hello, Ada!`).
- The name's capitalization is preserved verbatim.
- One optional positional argument only; the M-1 default behavior is unchanged.

Done when:
- `python3 hello.py Ada` prints `Hello, Ada!` and `python3 hello.py` still prints `Hello, world!`.
- Unit tests assert the named greeting and the unchanged default, and the suite passes.

Do not include:
- Localization, flags, or multiple-name support.
