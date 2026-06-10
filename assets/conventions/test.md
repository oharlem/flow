# Flow Artifact Conventions — Test

```
Conventions-Version: 1.1
```

Artifact shapes used by `/flow-test`. Core invariants live in `core.md` and are always loaded alongside this shard.

`/flow-test` runs the configured or auto-detected test runner and then runs the consistency checks (D1/D2/D3). It does not create or update any additional artifacts of its own.
