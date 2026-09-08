#!/usr/bin/env bash
#
# Report how far the fork has drifted behind upstream.
#
# Usage:
#   scripts/upstream-drift.sh
#
# Replaces the weekly `Sync upstream` cron, which was removed after failing
# 5/5 runs since 2026-08-10. Only the PR-opening half of that job was broken --
# the detection half worked, and removing it left nothing watching upstream.
# This is the detection half, on demand.
#
# Exit status:
#   0  the fork is level with upstream
#   1  the fork is behind (the count and the newest upstream tag are printed)
#   2  the script could not answer (missing remote, fetch failure)

set -euo pipefail

UPSTREAM_REMOTE="${UPSTREAM_REMOTE:-origin}"
FORK_REMOTE="${FORK_REMOTE:-fork}"

die() {
  printf 'upstream-drift: %s\n' "$1" >&2
  exit 2
}

for r in "$UPSTREAM_REMOTE" "$FORK_REMOTE"; do
  git remote get-url "$r" >/dev/null 2>&1 \
    || die "no remote named '$r' (override with UPSTREAM_REMOTE= / FORK_REMOTE=)"
done

git fetch --quiet "$UPSTREAM_REMOTE" master || die "could not fetch $UPSTREAM_REMOTE"
git fetch --quiet --no-tags "$FORK_REMOTE" master || die "could not fetch $FORK_REMOTE"

behind="$(git rev-list --count "$FORK_REMOTE/master..$UPSTREAM_REMOTE/master")"
ahead="$(git rev-list --count "$UPSTREAM_REMOTE/master..$FORK_REMOTE/master")"

upstream_tag="$(git tag --list 'v*' --sort=-v:refname \
  --merged "$UPSTREAM_REMOTE/master" | grep -v -- '-palette\.' | head -1)"
[ -n "$upstream_tag" ] || upstream_tag="(none found -- run: git fetch $UPSTREAM_REMOTE --tags)"

printf 'fork ahead:   %s commits\n' "$ahead"
printf 'fork behind:  %s commits\n' "$behind"
printf 'newest upstream tag: %s\n' "$upstream_tag"

if [ "$behind" -eq 0 ]; then
  printf '\nPASS  level with %s/master\n' "$UPSTREAM_REMOTE"
  exit 0
fi

printf '\nBEHIND  %s commits. Sync with:\n' "$behind"
printf '        gh workflow run "Sync upstream" --repo cameronsjo/herdr\n'
printf '        or merge %s/master into a sync branch by hand.\n' "$UPSTREAM_REMOTE"
exit 1
