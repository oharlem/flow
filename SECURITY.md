# Security Policy

## Supported versions

Flow follows SemVer. The latest minor of the current major is supported.

## Reporting a vulnerability

Please report suspected security issues **privately** via GitHub Security
Advisories: <https://github.com/oharlem/flow/security/advisories/new>. Do
**not** open a public GitHub issue for security reports.

You can expect a first reply within 72 hours. Confirmed issues are
triaged and a fix released on a patch version (`x.y.Z`) together with a
SECURITY advisory.

## Scope

Flow has a deliberately narrow threat model:

- Flow is an **offline, local CLI**. It makes no network calls and stores
  no credentials.
- Flow never runs `git push`, `git pull`, `git fetch`, `gh`, or `glab`,
  so it cannot accidentally publish work.
- Flow does not execute arbitrary user-supplied shell unless configured via
  `.flow/config.yaml: test.command`.

In-scope security issues include: path traversal in user-controlled inputs,
unbounded resource consumption by malicious artifacts, privilege escalation
on the build host. Out-of-scope: what AI coding agents do on top of Flow.
