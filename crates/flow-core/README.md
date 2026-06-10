# flow-core

Host-agnostic domain logic for [Flow](https://github.com/oharlem/flow), the
spec-driven workflow toolkit for AI coding agents.

This crate contains Flow's parsers, renderers, drift engine, envelope
composer, and typed identifiers (FR/SC/T/M/P/R/D). It has no host
knowledge — host adapters live in separate `flow-host-*` crates.

Most users want the `flow-cli` package in this repository, which ships the
`flow` binary built on top of `flow-core`. Depend on `flow-core` directly only
if you are building tooling that needs to read or generate Flow's Markdown
artifacts.

## License

MIT.
