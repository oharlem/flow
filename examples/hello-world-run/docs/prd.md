# Hello CLI — product notes

A tiny command-line greeter for this project.

Goals, in delivery order:

1. Running `python3 hello.py` prints exactly `Hello, world!`.
2. Running `python3 hello.py <name>` prints `Hello, <name>!`, preserving
   the name's capitalization.

Constraints: Python standard library only; logic in a pure function so it
is testable without subprocesses; every behavior pinned by unit tests.
