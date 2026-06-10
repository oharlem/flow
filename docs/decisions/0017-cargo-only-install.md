# 0017 — Cargo Git install

- **Status:** Accepted
- **Date:** 2026-06-07

## Context

Flow v0.1.0 is an in-development early prototype. It needs one reliable
installation path that can be documented, tested, and maintained without a
binary distribution matrix or crates.io release ceremony.

## Decision

Distribute Flow v0.1.0 from GitHub with Cargo:

```sh
cargo install --git https://github.com/oharlem/flow --locked flow-cli
```

For an immutable public tag, use the same model with a tag:

```sh
cargo install --git https://github.com/oharlem/flow --tag v0.1.0 --locked flow-cli
```

Publishing to crates.io is deferred until Flow needs crates.io discoverability
or downstream crate consumers. It is not required for the first release line.

## Consequences

- Users need a Rust toolchain.
- The release process does not require binary signing, platform archives, or
  installer checksum verification.
- The install docs have one primary command and one expected executable
  location: Cargo's bin directory.
- Trust derives from the GitHub repository revision selected by Cargo and the
  locked manifest.
