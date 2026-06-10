// Workspace-level task anchors for the canonical Flow directory layout milestone.
//
// T-001: current layout checks inspect the artifact layout without moving
// files.
//
// T-002: the unified run layout keeps all current work under `flow/runs/`,
// with child changes under `flow/runs/<run>/changes/<change>/` and Flow docs
// under `flow/docs/`.
//
// T-003: `flow/docs/directory-layout.md` documents the canonical Flow-owned
// directory layout and ownership boundaries.
//
// T-004: `cargo fmt --all --check` and `cargo test --workspace` verify the
// directory rework without behavior-changing source edits.
