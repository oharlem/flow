#!/usr/bin/env bash
set -euo pipefail

die() {
  printf 'error: %s\n' "$*" >&2
  exit 1
}

repo_root="$(git rev-parse --show-toplevel)"
cd "$repo_root"

_script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=/dev/null
source "$_script_dir/release-preflight.sh"
release_preflight

current_version="$(
  awk '
    /^\[workspace.package\]$/ { in_package = 1; next }
    /^\[/ { in_package = 0 }
    in_package && /^version = / {
      gsub(/"/, "", $3)
      print $3
      exit
    }
  ' Cargo.toml
)"

if [[ ! "$current_version" =~ ^([0-9]+)\.([0-9]+)\.([0-9]+)$ ]]; then
  die "workspace version must be plain SemVer X.Y.Z, got '$current_version'"
fi

major="${BASH_REMATCH[1]}"
minor="${BASH_REMATCH[2]}"
patch="${BASH_REMATCH[3]}"
new_version="${major}.${minor}.$((patch + 1))"
today="$(date -u +%F)"
changelog_note="${CHANGELOG_NOTE:-Patch release.}"

if [[ "$changelog_note" == *$'\n'* ]]; then
  die "CHANGELOG_NOTE must be a single line"
fi

if grep -Fq "## [$new_version]" CHANGELOG.md; then
  die "CHANGELOG.md already contains an entry for $new_version"
fi

if git rev-parse --verify --quiet "refs/tags/v${new_version}" >/dev/null; then
  die "git tag v${new_version} already exists"
fi

export FLOW_OLD_VERSION="$current_version"
export FLOW_NEW_VERSION="$new_version"
export FLOW_CHANGELOG_DATE="$today"
export FLOW_CHANGELOG_NOTE="$changelog_note"

LC_ALL=C perl -0pi -e 's/version = "\Q$ENV{FLOW_OLD_VERSION}\E"/version = "$ENV{FLOW_NEW_VERSION}"/g' Cargo.toml

tmp_changelog="$(mktemp)"
awk '
  BEGIN { inserted = 0 }
  /^## \[/ && inserted == 0 {
    print "## [" ENVIRON["FLOW_NEW_VERSION"] "] — " ENVIRON["FLOW_CHANGELOG_DATE"]
    print ""
    print "### Changed"
    print ""
    print "- " ENVIRON["FLOW_CHANGELOG_NOTE"]
    print ""
    inserted = 1
  }
  { print }
  END {
    if (inserted == 0) {
      exit 1
    }
  }
' CHANGELOG.md > "$tmp_changelog" || {
  rm -f "$tmp_changelog"
  die "could not insert CHANGELOG.md entry"
}
mv "$tmp_changelog" CHANGELOG.md

cargo update --workspace --offline
"${MAKE:-make}" up

git add Cargo.toml Cargo.lock CHANGELOG.md
if git diff --cached --quiet; then
  die "version bump produced no staged changes"
fi

# Ad hoc: include all tracked modifications in the same release commit (dogfooding / WIP on branch).
git add -u "$repo_root"
if git diff --cached --quiet; then
  die "no staged changes after git add -u"
fi

git commit -m "chore: release v${new_version}"
git tag "v${new_version}"
printf 'Created local commit and tag v%s. To publish: git push origin v%s\n' "$new_version" "$new_version"
