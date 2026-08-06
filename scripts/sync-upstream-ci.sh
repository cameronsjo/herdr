#!/usr/bin/env bash
# Merges herdrdev/herdr master into a fresh branch and prints what happened.
# The CI counterpart to scripts/sync-upstream.sh, which drives worktrees on a
# developer machine and cannot run on a single-remote CI checkout.
#
# Auto-resolves conflicts only in files this fork deliberately owns and never
# wants upstream's version of (FORK_OWNED below). Anything else aborts the merge
# and fails, because a sync that silently picks a side is how a fork loses its
# own changes.
#
# Usage: scripts/sync-upstream-ci.sh <upstream-url> <branch-name>
# Outputs (to $GITHUB_OUTPUT when set): synced=true|false, base=<sha>
set -euo pipefail

# Files the fork owns outright. Adding one here says "upstream's version of this
# file is never what we want" — not "conflicts here are inconvenient".
FORK_OWNED=(
  # Replaced with a fork notice (07821baa).
  "README.md"
  # Fork-local release pipeline; upstream's gates on a changelog announcement,
  # docs/next, and a website build that this fork does not maintain.
  ".github/workflows/release.yml"
)

UPSTREAM_URL=${1:?usage: sync-upstream-ci.sh <upstream-url> <branch-name>}
BRANCH=${2:?usage: sync-upstream-ci.sh <upstream-url> <branch-name>}

emit() {
  echo "$1"
  [[ -n ${GITHUB_OUTPUT:-} ]] && echo "$1" >>"$GITHUB_OUTPUT"
  return 0
}

echo "==> fetching upstream $UPSTREAM_URL"
git remote add upstream "$UPSTREAM_URL" 2>/dev/null || git remote set-url upstream "$UPSTREAM_URL"
git fetch --quiet upstream master

behind=$(git rev-list --count HEAD..upstream/master)
if [[ $behind -eq 0 ]]; then
  echo "==> already current with upstream; nothing to sync"
  emit "synced=false"
  exit 0
fi

upstream_sha=$(git rev-parse --short upstream/master)
echo "==> $behind commit(s) behind upstream (through $upstream_sha)"

git switch --quiet --create "$BRANCH"

# --no-commit so the fork-owned rules can apply before the merge commit is
# written. Only exit 1 means "conflicts"; anything else (unrelated histories, a
# bad ref, a dirty tree) is a real failure and must not fall through to a commit.
merge_status=0
git merge --no-commit --no-ff upstream/master || merge_status=$?
if [[ $merge_status -gt 1 ]]; then
  echo "error: git merge failed for a reason other than conflicts (exit $merge_status)" >&2
  git merge --abort 2>/dev/null || true
  exit "$merge_status"
fi

# Restore fork-owned files unconditionally, not just when they conflict. A
# conflict-only rule leaks: if upstream edits a region the fork left alone, git
# merges it cleanly and upstream content lands in a file this script claims the
# fork owns outright. Restoring from HEAD (still the pre-merge fork tip during a
# --no-commit merge) makes "owned" mean owned.
for owned in "${FORK_OWNED[@]}"; do
  if ! git cat-file -e "HEAD:$owned" 2>/dev/null; then
    echo "==> $owned is not in the fork tree; leaving upstream's copy alone"
    continue
  fi
  git checkout HEAD -- "$owned"
  git add -- "$owned"
  echo "==> kept the fork's $owned"
done

unresolved=$(git diff --name-only --diff-filter=U)
if [[ -n $unresolved ]]; then
  echo "error: conflicts this script will not resolve:" >&2
  echo "$unresolved" >&2
  echo "resolve them locally with scripts/sync-upstream.sh and push the branch by hand." >&2
  git merge --abort
  exit 1
fi

git commit --quiet --message "chore(sync): merge upstream through ${upstream_sha}"
echo "==> merged upstream through $upstream_sha"

# Upstream changes under these paths execute on push to master, and a clean
# merge gives the reviewer no signal that they moved. Surface them by name so a
# sync that touches CI never reads like ordinary source churn.
sensitive=$(git diff --name-only HEAD^1 HEAD -- '.github/**' 'scripts/**' 'build.rs' || true)
if [[ -n $sensitive ]]; then
  echo "==> this sync changes CI or build tooling:"
  echo "${sensitive//$'\n'/$'\n    '}" | sed '1s/^/    /'
fi

emit "synced=true"
emit "base=$upstream_sha"
# Multi-line values need heredoc syntax; a bare key=value would truncate at the
# first newline and silently drop every path after the first.
if [[ -n ${GITHUB_OUTPUT:-} && -n $sensitive ]]; then
  {
    echo "sensitive<<SYNC_EOF"
    echo "$sensitive"
    echo "SYNC_EOF"
  } >>"$GITHUB_OUTPUT"
fi
