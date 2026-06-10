# Flow Artifact Conventions — Roadmap

```
Conventions-Version: 1.1
```

Artifact shape used by `/flow-roadmap`. Core invariants live in `core.md` and are always loaded alongside this shard. This shard documents the **source-preserving milestone shape** that `/flow-roadmap` emits into a run-local roadmap and that downstream phases (`/flow-start`, `/flow-plan`, `/flow-build`, `/flow-run`) rely on.

---

## `flow/runs/<run>/roadmap.md` — milestone shape

Each new milestone uses a structured five-field body so downstream phases can reconstruct intent without the original source document.

```markdown
### [ ] M-N: Short outcome title

Source: `<source file or prompt>`, sections <section numbers/headings>.

Outcome: <one or two sentences describing delivered user value>.

Must preserve:
- <specific requirement, constraint, copy string, token rule, or behavior>
- <specific acceptance criterion with exact anchor>
- <specific edge case or verification concern>

Done when:
- <observable acceptance outcome>
- <test or verification evidence expected>

Do not include:
- <explicit non-goal or deferred item relevant to this milestone>
```

### Field semantics

- **Heading** — `### [ ] M-N: <title>`. The title is 3–7 words and describes the user-facing outcome, not the implementation. Never emit `[~]` or `[x]` state during roadmap generation; `/flow-roadmap` only writes `[ ]`.
- **`Source:`** — cites the source file (or inline prompt) plus section numbers or headings so downstream phases can open the reference directly.
- **`Outcome:`** — one or two sentences of delivered user value. Outcome focus is mandatory: a milestone is what the user gets, not how it is built.
- **`Must preserve:`** — bullets that carry concrete literals the source provides: copy strings, option names, status labels, token rules, required control behavior, and implementation constraints that are part of the requirement (e.g., "no raw Tailwind status colors," "no SEO column," "minimum 14px text").
- **`Done when:`** — bullets with observable acceptance outcomes and the test or verification evidence expected. Manual verification groups from the source land here.
- **`Do not include:`** — bullets capturing explicit non-goals from the source and deferred work relevant to this milestone.

## Parser Tolerance

The roadmap parser in `crates/flow-core/src/parse/roadmap.rs` identifies
milestones by their `### [ ] M-N:` heading and treats every line after the
heading as the milestone's description, until the next milestone heading. The
five-field body shape is a writing convention the agent follows when composing
new milestones.

Do not hand-author `[~]` or `[x]` state unless you are explicitly marking milestones as in-progress or done. `/flow-close` flips `[ ]` → `[x]` only in `FLOW_RUN_DIR/roadmap.md`, and `[~]` is reserved for users.

## Relationship to other shards

- The `close` shard documents how `/flow-close` ticks milestones and the optional `**Milestone**:` line on `status.md`. The roadmap shard does not duplicate tick rules.
- The `run` shard documents how `/flow-run` consumes milestones. The roadmap shard does not duplicate run behavior.
- `M-N` ID grammar lives in `core.md` (`M-[1-9]\d*`). This shard does not redefine it.
