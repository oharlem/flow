# Run Log: Hello CLI — product notes

**Run**: 20260610-roadmap-hello-cli-product-notes
**Target**: planned roadmap
**Started**: 2026-06-10T17:39:54Z
**Status**: complete

## Event Log

- 2026-06-10T17:39:54Z — run-started — Created run workspace for planned roadmap.

## Decisions

- 2026-06-10T17:43Z — Keep the demo as a two-milestone roadmap run instead of a one-off change, so the record shows the same `flow roadmap` -> `flow run` path described in the article.
- 2026-06-10T17:44Z — Keep the product deliberately small: M-1 delivers the default `Hello, world!` behavior, and M-2 adds the optional named greeting without changing the default output.
- 2026-06-10T18:07Z — Use a flat Python layout (`hello.py`, `test_hello.py`) and the standard-library `unittest` runner.
- 2026-06-10T18:25Z — Preserve the M-1 default tests unchanged during M-2 so the second milestone proves regression safety as well as new behavior.

## Operations

- 2026-06-10T17:39Z — `flow roadmap docs/prd.md` created the planned roadmap run under `flow/runs/20260610-roadmap-hello-cli-product-notes/`.
- 2026-06-10T17:43Z — The generated roadmap was edited into two source-preserving milestones: M-1 default greeting and M-2 named greeting.
- 2026-06-10T17:44Z — `flow roadmap --finalize` validated and saved the two-milestone roadmap.
- 2026-06-10T17:47Z — `flow run` attached all open milestones, created branch `flow/run-20260610-roadmap-hello-cli-product-notes`, and queued M-1.
- 2026-06-10T17:53Z — M-1 spec finalized with FR-001 and SC-001 for exact default output.
- 2026-06-10T18:07Z — M-1 plan and tasks finalized: implement `hello.py`, then add tests covering the default greeting.
- 2026-06-10T18:13Z — M-1 tests were written first, failed before `hello.py` existed, then passed after implementation.
- 2026-06-10T18:14Z — M-1 closed and ticked `M-1` in the run-local roadmap.
- 2026-06-10T18:15Z — `flow run --checkpoint ... --milestone M-1` created checkpoint commit `b160ea87860312df5e53f575bd18065691d79dea`.
- 2026-06-10T18:21Z — M-2 spec finalized with named greeting behavior plus unchanged M-1 default behavior.
- 2026-06-10T18:25Z — M-2 plan and tasks finalized: extend `greet(name="world")`, pass the first CLI argument, and add named-greeting tests.
- 2026-06-10T18:27Z — M-2 tests were added first and failed against the M-1 implementation, then passed after extending `hello.py`.
- 2026-06-10T18:31Z — Traceability was deliberately broken by removing `Covers:` references from M-2 tasks; Flow refused closeout with D1 errors pointing at `spec.md:23` and `spec.md:24`.
- 2026-06-10T18:33Z — M-2 traceability was repaired, closeout passed, and `M-2` was ticked in the run-local roadmap.
- 2026-06-10T18:47Z — `flow run --checkpoint ... --milestone M-2` created checkpoint commit `0616bf9dac98783859afb99847220bb7d569278e`.

## Stops And Human Interventions

- 2026-06-10T17:47:38Z — run-attached — Attached `all open roadmap milestones (2)` to the open roadmap run.
- 2026-06-10T18:14:24Z — change-closed — Closed flow/runs/20260610-roadmap-hello-cli-product-notes/changes/M-1-cli-prints-the-default-greeting.
- 2026-06-10T18:15:35Z — checkpoint — Preparing local checkpoint commit for M-1. The command output prints the exact SHA after Git creates it.
- 2026-06-10T18:15:35Z — checkpoint-complete — Local checkpoint commit for M-1 created as b160ea87860312df5e53f575bd18065691d79dea.
- 2026-06-10T18:33:34Z — change-closed — Closed flow/runs/20260610-roadmap-hello-cli-product-notes/changes/M-2-cli-greets-a-named-user.
- 2026-06-10T18:47:42Z — checkpoint — Preparing local checkpoint commit for M-2. The command output prints the exact SHA after Git creates it.
- 2026-06-10T18:47:42Z — checkpoint-complete — Local checkpoint commit for M-2 created as 0616bf9dac98783859afb99847220bb7d569278e.
- 2026-06-10T18:48:55Z — run-finalized — Log, owner's manual, and release notes completed.
- 2026-06-10T18:48:55Z — run-finalize-commit — Closing commit for the finalized run state follows. The command output prints the exact SHA after Git creates it.
