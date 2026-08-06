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

# --no-commit so the README rule can apply before the merge commit is written.
git merge --no-commit --no-ff upstream/master || true

for owned in "${FORK_OWNED[@]}"; do
  if [[ -n $(git ls-files --unmerged -- "$owned") ]]; then
    echo "==> keeping the fork's $owned over upstream's"
    git checkout --ours -- "$owned"
    git add -- "$owned"
  fi
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
emit "synced=true"
emit "base=$upstream_sha"
