#!/usr/bin/env bash
# shellcheck shell=bash
# Sourced by bump-patch.sh / bump-minor.sh after `die` is defined and cwd is repo root.

release_preflight() {
  local porcelain
  porcelain="$(git status --porcelain)"
  if [[ "${FLOW_RELEASE_REQUIRE_CLEAN:-}" == "1" ]]; then
    if [[ -n "$porcelain" ]]; then
      die "FLOW_RELEASE_REQUIRE_CLEAN=1 requires a clean worktree (commit, stash, or remove changes)"
    fi
  else
    if printf '%s\n' "$porcelain" | grep -q '^??'; then
      die "untracked files present; stage intended paths with git add or remove them. Set FLOW_RELEASE_REQUIRE_CLEAN=1 to require a clean worktree."
    fi
  fi
}
