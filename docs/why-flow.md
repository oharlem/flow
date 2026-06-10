# Why Flow

AI coding agents are good at generating work and bad at leaving evidence of
it. A final diff shows what changed, but not what was asked, what alternatives
were considered, which requirements were intended, what verification actually
ran, or whether the work is complete. For a one-line fix that is tolerable.
For delegated, multi-iteration work it is the bottleneck: the cost of
reviewing agent output grows faster than the cost of producing it.

> Flow's working hypothesis: the more work we hand to agents, 
> the less the next plan matters and the more a trustworthy record of the last one does.

## The Gap Flow Fills

Spec-driven tools structure what goes *into* the agent. Flow structures what
comes *out*.

| Tool | What persists in the repo | What is missing |
|---|---|---|
| Kiro | `requirements.md`, `design.md`, `tasks.md` per feature | Planning artifacts only; no verification or closeout record; bound to one IDE |
| spec-kit | Constitution, spec, plan, tasks per feature | Front-loaded; nothing records implementation or verification; consistency check (`/analyze`) is an LLM prompt, run optionally |
| OpenSpec | Proposals, spec deltas, design, tasks; archived per change | The strongest history of the group — but the archive is plan-side; no record of verification outcomes; no deterministic gate on completion |
| Cursor | Plans saved to `.cursor/plans/` | Plans only, in tool-specific space; no lifecycle state, verification record, or closeout |
| Claude Code | `CLAUDE.md` context; plans are session-scoped | Context *into* the agent, not a record out of it |

All of these artifacts are addressed to the agent, or to the developer at
plan-approval time. None of them produce a structured account addressed to a
reviewer who was never in the session. That reviewer — a teammate, a future
you, an auditor, another agent resuming the work — is who Flow writes for.

## Two Mechanisms

**1. The record covers the whole lifecycle.** A Flow change is a state
machine, not a folder of planning notes:

- `spec.md` records the requested change, requirements (`FR-NNN`), and
  success criteria (`SC-NNN`).
- `plan.md` records the implementation strategy and documentation impact.
- `tasks.md` records the ordered work queue, each task declaring which FRs it
  `Covers:` and which SCs it `Verifies:`.
- `status.md` records lifecycle state (drafting → building → closed) and
  history.
- `flow test` records verification: the project test suite plus Flow's
  consistency checks.
- `flow close` stamps closeout, and roadmap runs accumulate `log.md`,
  `manual.md`, and `release-notes.md` as handoff documents.

**2. Closeout is gated deterministically.** Flow's drift rules (D1–D3) check
the FR/SC/T traceability graph in Rust code with stable exit codes — not by
asking another model. A requirement with no covering task, or a task pointing
at a deleted requirement, blocks `flow close`. The agent cannot self-certify.
See [Drift rules](./drift-rules.md).

The combination is the point. History without gates can be confidently wrong;
gates without history leave nothing to review.

## Design Commitments

| Commitment | Reason |
|---|---|
| Repo-local state | The record travels with the code, works offline, and survives tool changes |
| Plain Markdown | Humans can read, diff, repair, and review artifacts with git alone |
| Host-neutral core | Claude Code, Codex, Cursor, and future hosts share one model and one record format |
| Deterministic verification gate | Completion is checked by code, not asserted by an agent |
| Git safety | Flow never performs remote or destructive git operations |

## Boundaries

Flow is not an AI model, IDE, CI service, policy engine, or release tool, and
it does not compete with hosts on planning ergonomics — Kiro, Cursor, and
Claude Code do interactive planning well. Flow's claim is narrower and
honest: it guarantees the *structural integrity* of the record (IDs resolve,
states are legal, gates ran), not the truthfulness of agent-written prose.
Code review, tests, and human judgment still do their jobs; Flow preserves
the evidence they need to do them after the chat is gone.

For exact files, read [Artifacts](./reference/artifacts.md). For exact
command behavior, read [Commands](./reference/commands.md).

## A Hypothesis, Not A Conclusion

Flow v0.1.0 is an in-development early prototype, and the argument on this page
is a position under test, not a settled one. The diagnosis — that reviewing
agent work is becoming the bottleneck — seems robust. Whether this particular
design is the right response is open: whether evidence gets read or becomes the
bloat it was meant to cure, where the boundary between deterministic checks and
human judgment belongs, and whether a host-neutral record survives as the hosts
themselves add persistence. The root README lists these open questions in full.
Disagreement is welcome; it is the point of publishing.
