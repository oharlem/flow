# Changelog

All notable public changes to this project are documented here. The format is
based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [0.1.0] - In development

First public early prototype release line.

### Added

- Local `flow` CLI installable from GitHub with
  `cargo install --git https://github.com/oharlem/flow --locked flow-cli`.
- Spec-driven change workflow: `start`, `amend`, `plan`, `build`,
  `build-task`, `test`, `close`, and `status`.
- Roadmap workflow: `roadmap` and `run` for planned milestone work.
- Host adapters for Claude Code, Codex, and Cursor.
- Repo-local Markdown artifacts under `flow/runs/<run>/`.
- D1-D3 drift checks between `spec.md` and `tasks.md`.
- Local-only git safety model: no push, pull, fetch, tags, force operations,
  `gh`, or `glab`.
- Documentation for install, commands, artifacts, security, release, host
  adapters, and architecture.

### Changed

- Public versioning starts with a deliberately narrow release surface.
- Installation is Cargo-only and GitHub-first for v0.1.0 development;
  crates.io publishing, prebuilt binaries, Homebrew, shell installers,
  PowerShell installers, and release archives are out of scope.

### Removed

- Internal distribution machinery and release workflow files outside the
  current Cargo publish path.
- Host support outside Claude Code, Codex, and Cursor.
- Auxiliary commands and artifact types that are not part of the early
  prototype release surface.
