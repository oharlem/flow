# Your First Flow Change

**Reviewed**: 2026-06-09

This is a ten-minute walkthrough. By the end you will have used Flow to draft,
plan, build, test, and close one tiny change: adding a one-line note to a
project's README. No prior Flow knowledge is assumed. Unfamiliar terms such as
envelope, FR, and drift are defined in
[`reference/glossary.md`](../reference/glossary.md).

> **You will need:** a project that Flow has been installed in (run `flow init`
> if you have not), and a host that speaks Flow: Claude Code, Codex, or
> Cursor. The commands below use Claude Code's `/flow-<name>` syntax. Use
> `$flow-<name>` in Codex, `/flow-<name>` in Cursor, or
> `flow <name>` in a shell.

## 0. Set up a scratch project (optional)

If you do not have a project handy, create a throwaway one:

```sh
mkdir flow-tutorial && cd flow-tutorial
git init -q
echo "# Flow tutorial" > README.md
git add README.md && git commit -q -m "init"
flow init
```

`flow init` is the one-time installer for a Flow-enabled project. After it runs, you will see a new `.flow/` directory and (depending on your host) a `.claude/`, `.agents/`, or `.cursor/` directory containing the command registrations.

> **Sanity check.** Run `flow doctor`. If it prints "Flow is healthy", you are ready.

## 1. Draft the spec — `/flow-start`

A **spec** is a short Markdown file that says what you want to build and why, in plain English. You write it together with the start agent, by conversation.

```text
/flow-start add a one-line note to the README pointing at the contributing guide
```

The agent will greet you, rephrase what it heard, and may ask up to five short clarifying questions. For a tiny change like this it will probably ask zero or one. When it has enough, it will write a `spec.md` with at minimum a `## What & Why` paragraph.

By default, Flow projects use `confirmation: no`, so the agent saves state after
writing the approved draft. If your project has `confirmation: yes`, reply
`yes` or `y` when the draft looks right. The agent then runs an internal save
step and prints the next command in your host's syntax, such as
`Next command: /flow-plan` in slash-command hosts or
`Next command: $flow-plan` in Codex.

> **What just happened?** Flow created a run under `flow/runs/<run>/` and a child change under `flow/runs/<run>/changes/<change>/` containing a fresh `spec.md` and a `status.md` that says **State**: drafting. **Spec** = the intent. **Status** = the lifecycle marker. You did not have to write either by hand.

## 2. Plan it — `/flow-plan`

```text
/flow-plan
```

The plan agent reads your spec and produces two artifacts:

- **`plan.md`** — a one-page implementation strategy with technical context (language, dependencies, testing approach).
- **`tasks.md`** — a dependency-ordered list of small tasks of the form `- [ ] **T-001**: …`. Each task says which `FR-NNN` requirements it covers and which `SC-NNN` success criteria it verifies.

For our example the plan will likely have one or two tasks: edit `README.md`,
then verify it. If confirmations are enabled, reply `yes` to save the plan
state. `flow plan --finalize` validates, stamps `plan-complete`, sets
`status.md` **State** to **building**, then the next command is `flow-build`,
rendered for your active host.

> **The IDs.** `T-NNN` is a Task. `FR-NNN` is a Functional Requirement (from the spec). `SC-NNN` is a Success Criterion. They are how the planner shows that every part of the spec gets implemented and verified — and how Flow's drift checks notice when something has been left behind.

## 3. Build it — `/flow-build` or `/flow-build-task`

```text
/flow-build
```

`/flow-build` works through every remaining task. Use `/flow-build-task` to do exactly one task at a time. The build agent works **test-first** for code tasks (write a failing test, watch it fail for the right reason, implement, watch it pass).

For our README change, the agent will:

1. Edit `README.md` to add the line.
2. Show you the diff.
3. Mark the task `[~]` (implemented, awaiting your acceptance).
4. Save Flow state directly, or ask for `yes` first when confirmations are
   enabled.

After the state-save step, the driver flips accepted task checkboxes to `[x]`
and appends build history. When `/flow-build` accepts the final task, Flow
runs the verification gate immediately. If tests and consistency checks pass,
Flow stamps `build-complete` and routes to `/flow-close`.

## 4. Verify — `/flow-test` when needed

```text
/flow-test
```

This is the explicit rerun gate before closing. If `/flow-build` already accepted the final task and verification passed, you can follow its `/flow-close` footer. Run `/flow-test` after a failed final build verification, after stepping through tasks with `/flow-build-task`, or any time `status.md` has no `build-complete` entry yet.

The test agent runs the full automated suite (`cargo test --workspace` for Flow's own repo; whatever your project uses elsewhere), runs Flow's drift checks (`D1`, `D2`, `D3`), and reports anything that is inconsistent.

If confirmations are enabled, reply `yes` to save the test state.

## 5. Close — `/flow-close`

```text
/flow-close
```

Closing is the moment Flow declares the change done.

The close agent will:

- Add `**Closed**: <today's date>` to your `spec.md`.
- Leave the change directory in place under `flow/runs/<run>/changes/<change>/`.
- Update `run.md` so the run records the closed change and next command.
- Tick any `M-N` milestone in the run-local roadmap that the change was linked to.
- Verify that current Flow documentation under `flow/docs/` was updated, or that the plan declares `Impact: none` and explains why docs were already current.

If confirmations are enabled, reply `yes` to confirm. `status.md` flips to
**State**: closed.

> **Close is not publish.** Flow never runs `git push`, `git pull`, `git fetch`, or `gh`, and it never creates tags. Standalone change closeout does not create commits; branch-backed roadmap runs can create local checkpoint commits when `git.run_checkpoint_commits: true`. Application releases stay in your normal project-native release process.

## What now?

You have just produced:

- A complete spec, plan, and task list for one change under `flow/runs/<run>/changes/<change>/`.
- An updated README in your project.
- Updated current documentation under `flow/docs/`, or an explicit `Impact: none` plan rationale that no Flow docs changed.

Try it again with a slightly bigger change. The same five public commands
handle a one-line README edit and a multi-week subsystem rewrite; only the
middle conversations during `/flow-start`, `/flow-plan`, and `/flow-build` get
longer. If you get stuck, run `/flow-status` for a read-only report.

Welcome to spec-driven coding.
