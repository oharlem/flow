# Flow Documentation

Flow is a local Rust CLI that records AI-assisted coding work as a reviewable
evidence trail — spec, plan, tasks, status, verification, closeout — stored as
plain Markdown in the repository.

## Start Here

```sh
cd your-repo
flow init --host claude-code       # or codex | cursor
flow start "add login form"        # draft spec.md
flow plan                          # draft plan.md and tasks.md
flow build-task                    # implement the next task
flow test                          # tests and consistency checks
flow close                         # close the active change
```

For multi-milestone work:

```sh
flow roadmap docs/prd.md
flow run
```

## Reading Path

| Need | Read |
|---|---|
| Understand the idea | [Why Flow](./why-flow.md) |
| Try the workflow | [Your first change](./start-here/01-your-first-change.md), [Your first roadmap run](./start-here/02-your-first-roadmap-run.md) |
| See command behavior | [Commands](./reference/commands.md), [CLI reference](./reference/cli.md) |
| Inspect written files | [Artifacts](./reference/artifacts.md), [Drift rules](./drift-rules.md), [Glossary](./reference/glossary.md) |
| Evaluate implementation | [Architecture](./architecture.md), [Security](./security.md) |
| Integrate a host | [Host adapters](./hosts.md) |
| Understand key decisions | [ADRs](./decisions/README.md) |

## Ownership Model

- Flow owns workflow artifacts under `flow/runs/` and generated host assets.
- Users own source code, product decisions, releases, publishing, and repo
  policy.
- Flow never pushes, pulls, fetches, creates tags, force-resets, or calls
  GitHub/GitLab CLIs. Branch-backed `flow run` lifecycles can create local
  checkpoint commits and a run-closing finalize commit when
  `git.run_checkpoint_commits: true`; Flow never publishes those commits.
