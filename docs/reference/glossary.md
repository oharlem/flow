# Glossary

Plain-language definitions for Flow terms. If a term appears in the docs but is
missing here, treat that as a documentation bug.

## Workflow Terms

- **Phase.** One named step of Flow work, such as `start`, `plan`, `build`,
  `test`, or `close`.
- **State.** The lifecycle of one change in `status.md`: `drafting`,
  `building`, or `closed`.
- **Change.** One package of Flow work under `flow/runs/<run>/changes/<change>/`.
- **Milestone.** A roadmap heading such as `### [ ] M-1: Add login`. A change
  can link to one milestone.
- **Run.** The canonical work container under `flow/runs/<run>/`. A one-off run
  has one child change; a roadmap run has one child change per milestone.
- **Finalize / finalization.** The internal state-save step a phase agent runs
  after artifacts are ready. There is no public `/flow-finalize` command.
- **Flow docs.** Current-state Flow documentation under `flow/docs/` by
  default. Flow checks these before closeout.

## Architecture Terms

- **Host.** The AI coding environment that consumes Flow commands, such as
  Claude Code, Codex, or Cursor.
- **Host adapter.** A Rust crate that installs Flow skills, slash commands, or
  rules for one host.
- **Envelope.** The context block Flow prints for the host: conventions,
  phase prompt, runtime state, and next actions.

## Artifact And ID Terms

- **ADR.** Architecture Decision Record under `docs/decisions/`.
- **FR.** Functional Requirement in `spec.md`, written as `FR-NNN`.
- **SC.** Success Criterion in `spec.md`, written as `SC-NNN`.
- **T.** Task in `tasks.md`, written as `T-NNN`.
- **M.** Milestone in `flow/runs/<run>/roadmap.md`, written as `M-N`.
- **P.** Engineering Principle in optional `docs/principles.md`, written as
  `P-NNN`.
- **R.** Research decision in `plan.md`, written as `R-NNN`.

## Drift Terms

- **Drift.** An inconsistency between artifacts, such as a task referencing an
  FR that no longer exists.
- **D-rule.** One numbered drift check. The active rules are D1, D2, and D3
  (see [Drift rules](../drift-rules.md)).
