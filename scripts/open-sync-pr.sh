#!/usr/bin/env bash
# Pushes a sync branch and opens its pull request.
#
# The token is supplied to the push explicitly rather than persisted into
# .git/config by actions/checkout, so a repo-write credential never sits in a
# tree that has upstream code merged into it.
#
# Environment: GH_TOKEN, BRANCH, BASE, SENSITIVE, GITHUB_REPOSITORY
set -euo pipefail

: "${GH_TOKEN:?GH_TOKEN is required}"
: "${BRANCH:?BRANCH is required}"
: "${BASE:?BASE is required}"
: "${GITHUB_REPOSITORY:?GITHUB_REPOSITORY is required}"
SENSITIVE=${SENSITIVE:-}

# A plain push, deliberately. --force-with-lease looks like protection but gives
# none here: nothing ever fetched this branch, so there is no remote-tracking ref
# for the lease to compare against and it degrades to a plain force. A non-fast-
# forward rejection is the outcome we actually want — it means today's branch
# already exists and possibly carries a hand-resolved conflict, which is exactly
# what must not be overwritten.
if ! git push --no-follow-tags \
  "https://x-access-token:${GH_TOKEN}@github.com/${GITHUB_REPOSITORY}.git" \
  "HEAD:refs/heads/${BRANCH}"; then
  echo "error: could not fast-forward ${BRANCH}; it already exists and has diverged." >&2
  echo "A previous run or a human already pushed there. Inspect it before re-running." >&2
  exit 1
fi

if gh pr view "$BRANCH" --json number >/dev/null 2>&1; then
  echo "pull request already open for $BRANCH; branch updated in place"
  exit 0
fi

review_note="This sync touched source only."
title="chore(sync): merge upstream through ${BASE}"
if [[ -n $SENSITIVE ]]; then
  review_note="**This sync changes CI or build tooling — review these before anything else:**

\`\`\`
${SENSITIVE}
\`\`\`

These paths execute on push to master. Read them before reopening the PR to
trigger CI, because reopening runs the merged code."
  title="chore(sync): merge upstream through ${BASE} (touches CI)"
fi

gh pr create --base master --head "$BRANCH" --title "$title" --body "Automated upstream sync from herdrdev/herdr through \`${BASE}\`.

${review_note}

Conflicts are auto-resolved only in files this fork owns outright (its README
and its release workflow). Any other conflict fails the run instead of picking
a side, so a clean run means nothing of the fork's was silently overwritten.

CI does not run on this PR — GitHub does not trigger workflows for pull requests
opened with \`GITHUB_TOKEN\`. Close and reopen it to get a run before merging."
