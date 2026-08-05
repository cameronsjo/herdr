#!/usr/bin/env bash
# Rebuilds this fork's master on top of the current herdrdev/herdr master —
# the periodic sync the fork's README promises. Automates the mechanical
# part (fetch, worktree, merge, the always-keep-ours README.md conflict);
# stops for you to resolve anything else by hand.
#
# Usage: scripts/sync-upstream.sh
# Env overrides: ORIGIN_REMOTE (default: origin), FORK_REMOTE (default: fork)
set -euo pipefail

ROOT_DIR=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
ORIGIN_REMOTE=${ORIGIN_REMOTE:-origin}
FORK_REMOTE=${FORK_REMOTE:-fork}
BRANCH="sync-upstream-$(date +%Y%m%d)"
WORKTREE_DIR="$(cd -- "$ROOT_DIR/.." && pwd)/herdr-worktrees/$BRANCH"

cd "$ROOT_DIR"

for remote in "$ORIGIN_REMOTE" "$FORK_REMOTE"; do
  git remote get-url "$remote" >/dev/null 2>&1 || {
    echo "error: remote '$remote' not configured" >&2
    exit 1
  }
done

echo "==> fetching $ORIGIN_REMOTE and $FORK_REMOTE"
git fetch "$ORIGIN_REMOTE" --prune
git fetch "$FORK_REMOTE" --prune

if git worktree list --porcelain | command grep -qx "worktree $WORKTREE_DIR"; then
  echo "==> reusing existing worktree at $WORKTREE_DIR"
else
  echo "==> creating worktree at $WORKTREE_DIR (branch $BRANCH, from $FORK_REMOTE/master)"
  git worktree add "$WORKTREE_DIR" -b "$BRANCH" "$FORK_REMOTE/master"
fi

cd "$WORKTREE_DIR"

if [[ -n "$(git status --porcelain)" ]] && ! git status --porcelain | command grep -q '^UU\|^AA\|^DD'; then
  echo "error: worktree has unrelated uncommitted changes — resolve or clean it first" >&2
  exit 1
fi

echo "==> merging $ORIGIN_REMOTE/master"
if git merge "$ORIGIN_REMOTE/master" --no-edit; then
  echo "==> merge landed with no conflicts"
else
  if git status --porcelain | command grep -q '^UU README.md$'; then
    echo "==> auto-resolving README.md conflict (keeping the fork's own notice)"
    git checkout --ours README.md
    git add README.md
  fi

  remaining=$(git status --porcelain | command grep '^UU\|^AA\|^DD' || true)
  if [[ -n "$remaining" ]]; then
    echo
    echo "Conflicts remain — resolve these by hand, then re-run this script to finish:"
    echo "$remaining"
    echo
    echo "Worktree: $WORKTREE_DIR"
    exit 1
  fi

  echo "==> all conflicts resolved, completing the merge commit"
  git commit --no-edit
fi

echo
echo "Next steps:"
echo "  1. scripts/docker-check.sh   (verify the merge in $WORKTREE_DIR)"
echo "  2. git -C $WORKTREE_DIR push $FORK_REMOTE $BRANCH:master"
echo "  3. git worktree remove $WORKTREE_DIR && git branch -d $BRANCH"
