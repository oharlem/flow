# Flow Artifact Conventions — Core

**Versioning**: grammar version declared below; breaking changes bump major, additive changes bump minor.

```
Conventions-Version: 1.1
```

Core invariants loaded on every Flow phase envelope. Phase-specific artifact shapes live in per-phase shards under `.flow/conventions/`.

---

## 1. File-level rules

- All Flow-produced artifacts are **plain GitHub-flavored Markdown**. No YAML frontmatter for structured metadata. The version line above is informational.
- Files use UTF-8, LF line endings, and end with a single trailing newline.
- Headings use ATX (`#`, `##`, …). Setext underlines are not used.
- Section identity is the heading **text** (case-sensitive, trimmed).

## 2. `status.md`

Current schema — five mandatory key-value lines plus a mandatory History section:

```
# Status: <change-name>

**Change**: <change-name>
**Started**: <YYYY-MM-DD>
**Updated**: <ISO datetime>
**State**: drafting | building | closed
**Branch**: <branch-name>

## History

- <ISO> — <action> — <short summary>
```

`State` is a three-value enum. All prior v1 fields (`Current-Phase`, `Last-Action`, `Last-Action-At`, `Principles-Pinned-At`, `Gates Passed`, `Pending Blockers`, `Next Recommended Action`) are removed. `status.md` is the single source of truth for state; `/flow-status` computes everything else live.

## 3. ID grammar

| Prefix | Used for | Grammar | Example |
|---|---|---|---|
| `FR-` | Functional Requirement | `FR-` + optional group letter + `\d{1,4}` | `FR-001`, `FR-V12` |
| `SC-` | Success Criterion | `SC-` + optional group letter + `\d{1,4}` + optional sub-letter | `SC-001`, `SC-007a` |
| `T-` | Task | `T-\d{1,4}` | `T-001`, `T-V07` |
| `M-` | Milestone (in `flow/runs/<run>/roadmap.md`) | `M-[1-9]\d*` | `M-1`, `M-42` |
| `P-` | Engineering Principle | `P-\d{1,4}` | `P-005` |
| `R-` | Research decision | `R-\d{1,4}` | `R-V1` |
| `D[1-9]\d?` | Drift type in reports | — | `D1`, `D3` |

Rules:

- IDs MUST be unique within their owning file.
- IDs are case-sensitive.
- First occurrence in a file is the **definition**; later occurrences are **references**.
- References are bare IDs (backticks tolerated, not required).

## 6. Phase-action atomicity

`status.md` is the single source of truth for `State`. Parsers MUST NOT infer state from the presence of an artifact file alone; only `status.md`'s `State` field and most-recent History entry declare completion.

Writes to `status.md` are atomic: driver writes `<file>.tmp.$PID` then `mv`.

## 9. Forbidden patterns

- No YAML frontmatter for structured metadata (only the informational `Conventions-Version` line).
- No JSON sidecars (no `spec.json`, no `tasks.json`).
- No HTML in artifacts (beyond what GFM permits inline).
- No tabs (use 2-space indentation for nested bullets).
- No `~~strikethrough~~` for removed items — history lives in git.

## 10. Parser Tolerance

- Parsers MUST tolerate the presence of additional unknown sections at any level and MUST NOT fail on them.
- Parsers MUST tolerate additional unknown ID prefixes and MUST NOT fail on them.
- Parsers MUST NOT silently drop content during round-trips; Flow only writes whole files.

## 11. Confirmation behavior

Phase agents read two lines from the `# Runtime Context` block:

- `**Confirmation**: required | disabled` — whether the project requires explicit user confirmation before the finalize step.
- `**Destructive action**: <reason>` — present when a phase needs the confirmation prompt to name a higher-risk local action.

Project setting: `confirmation: yes | no` in `.flow/config.yaml`. Default: `no` (disabled).

Rules:
- When `**Confirmation**: disabled`, agents run the finalize command directly after writing the artifact without asking for `yes` or `y`.
- When `**Confirmation**: required`, agents always ask `Reply yes or y to save, or tell me what to change.`
- When `**Destructive action**: <reason>` is present and `**Confirmation**: required`, agents mention that reason in the confirmation prompt. It never overrides `**Confirmation**: disabled`.
- `flow close` honors the project confirmation setting and does not include a destructive-action override.
