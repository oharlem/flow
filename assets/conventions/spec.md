# Flow Artifact Conventions — Spec

```
Conventions-Version: 1.1
```

Artifact shapes used by `/flow-start` and `/flow-amend`. Core invariants (ID grammar, file rules, forbidden patterns, tolerance, confirmation, status schema) live in `core.md` and are always loaded alongside this shard.

---

## `spec.md`

- `## What & Why` *(mandatory — 2-3 paragraphs describing the change in plain English)*

Optional sections (emit when useful for larger changes):

- `## Requirements` → `### Functional Requirements` (`FR-NNN` bullets)
- `## Success Criteria` → `### Measurable Outcomes` (`SC-NNN` bullets)
- `## Clarifications` → `### Session YYYY-MM-DD`
- `## Edge Cases`
- `## Out of Scope`
- `## Key Entities`

## `docs/principles.md` (optional)

- `## Engineering Principles` *(mandatory if the file exists and is non-empty)*
- Principles listed as bullets, optionally prefixed by `**P-NNN**:`.
- Read **live** by every phase agent when present. Never pinned to a SHA. Never gated.

## 4.1 Functional Requirement

In `spec.md`, under `### Functional Requirements`:

```markdown
- **FR-NNN**: <one-paragraph description>.
```

The ID MUST be the first bold token in the bullet.

## 4.2 Success Criterion

In `spec.md`, under `### Measurable Outcomes`:

```markdown
- **SC-NNN**: <measurable, technology-agnostic outcome>.
```

## 7. The `## Clarifications` block

Located near the top of `spec.md` when present. Structure:

```markdown
## Clarifications

### Session YYYY-MM-DD

- Q: <question> → A: <answer>
```

`/flow-start` and `/flow-amend` append Q&A pairs to the most recent session block (creating it if needed). History is never deleted.
