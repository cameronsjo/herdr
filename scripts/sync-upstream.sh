#!/usr/bin/env bash
# Rebuilds this fork's master on top of the current herdrdev/herdr master —
# the periodic sync the fork's README promises. Automates the mechanical
# part (fetch, worktree, merge, the always-keep-ours README.md conflict);
# stops for you to resolve anything else by hand.
#
# Usage: scripts/sync-upstream.sh
# Env overrides: ORIGIN_REMOTE (default: origin), FORK_REMOTE (default: fork)
set -euo pipefail

# -P everywhere: `git worktree list` reports physical paths, so a logical one
# here would fail the reuse check below and try to re-create the worktree.
ROOT_DIR=$(cd -P -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd -P)
ORIGIN_REMOTE=${ORIGIN_REMOTE:-origin}
FORK_REMOTE=${FORK_REMOTE:-fork}
BRANCH="sync-upstream-$(date +%Y%m%d)"
WORKTREE_DIR="$(cd -P -- "$ROOT_DIR/.." && pwd -P)/herdr-worktrees/$BRANCH"

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

# Capture first: under `set -o pipefail` a `grep -q` exits at the first match
# and can SIGPIPE the producer (141), inverting this test and sending the
# resume path into `git worktree add` on a directory that already exists.
worktree_list=$(git worktree list --porcelain)
if command grep -qx "worktree $WORKTREE_DIR" <<<"$worktree_list"; then
  echo "==> reusing existing worktree at $WORKTREE_DIR"
else
  echo "==> creating worktree at $WORKTREE_DIR (branch $BRANCH, from $FORK_REMOTE/master)"
  git worktree add "$WORKTREE_DIR" -b "$BRANCH" "$FORK_REMOTE/master"
fi

cd "$WORKTREE_DIR"

# A resumed run is the normal path: the first run stops on conflicts, you fix
# them, and re-run to land the merge commit. By then the tree is dirty with
# *staged resolutions* and no `UU` entries left, so unmerged-path detection
# can't tell that state apart from unrelated dirt — MERGE_HEAD can.
if [[ -f "$(git rev-parse --git-dir)/MERGE_HEAD" ]]; then
  merge_in_progress=true
else
  merge_in_progress=false
fi

if [[ -n "$(git status --porcelain)" ]] && ! "$merge_in_progress"; then
  echo "error: worktree has unrelated uncommitted changes — resolve or clean it first" >&2
  exit 1
fi

merge_rc=0
if "$merge_in_progress"; then
  echo "==> resuming the in-progress merge of $ORIGIN_REMOTE/master"
  merge_rc=1
else
  echo "==> merging $ORIGIN_REMOTE/master"
  git merge "$ORIGIN_REMOTE/master" --no-edit || merge_rc=$?
fi

if [[ "$merge_rc" -eq 0 ]]; then
  echo "==> merge landed with no conflicts"
else
  status=$(git status --porcelain)
  if command grep -q '^UU README.md$' <<<"$status"; then
    echo "==> auto-resolving README.md conflict (keeping the fork's own notice)"
    git checkout --ours README.md
    git add README.md
  fi

  # Every unmerged porcelain state, not just the three symmetric ones —
  # a rename/delete conflict is DU/UD/AU/UA and was silently omitted.
  remaining=$(command grep -E '^(DD|AU|UD|UA|DU|AA|UU)' <<<"$(git status --porcelain)" || true)
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
echo "  1. (cd $WORKTREE_DIR && ./scripts/docker-check.sh)"
echo "     Run it from the worktree: the script mounts the tree it lives in, so the"
echo "     primary checkout's copy would test the PRE-merge tree and still print PASS."
echo "  2. git -C $WORKTREE_DIR push --no-follow-tags $FORK_REMOTE $BRANCH:master"
echo "     --no-follow-tags is required: push.followTags is set globally and the fetch"
echo "     above imports upstream's new annotated tags."
echo "  3. git merge --ff-only $BRANCH   (fast-forward master BEFORE deleting the branch,"
echo "     or git branch -d compares against HEAD and refuses)"
echo "  4. git worktree remove $WORKTREE_DIR && git branch -d $BRANCH"
