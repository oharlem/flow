# Flow Workspace

`flow/` is the visible Flow workspace. `.flow/` is the hidden runtime control
plane.

Key paths:

- `runs/<run>/roadmap.md` for roadmap-run milestones
- `runs/<run>/` for run state, audit logs, handoffs, and child changes
- `runs/<run>/changes/<change>/` for change artifacts
- `docs/` for current Flow workflow guidance

Do not edit generated host assets or Flow-owned `status.md` files by hand.
