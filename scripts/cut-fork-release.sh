#!/usr/bin/env bash
#
# Cut a fork release: tag fork/master and let .github/workflows/release.yml
# build the arm64 macOS binary, publish it, and dispatch a formula bump to
# cameronsjo/homebrew-tap.
#
# Usage:
#   scripts/cut-fork-release.sh v0.9.0-palette.1
#
# Tag shape is v<upstream-version>-palette.<n>: upstream's version from
# Cargo.toml, unchanged, plus the fork's build counter. The counter resets on
# every upstream version bump, so the first build on upstream 0.9.0 is
# v0.9.0-palette.1 regardless of how high the 0.8.2 counter ran.
#
# ---------------------------------------------------------------------------
# Recovery rules -- the judgment this script deliberately does not make
# ---------------------------------------------------------------------------
#
# A palette tag is NEVER reused. softprops/action-gh-release updates an
# existing release in place and overwrites the asset, so a delete-and-re-push
# gives the binary a new sha256 while every tap formula is still pinned to the
# old one. `brew upgrade` then fails a checksum on every installed machine with
# an error that reads like a corrupted download. Fix a bad build by cutting the
# next counter: v0.9.0-palette.2.
#
# If `release` succeeded but `tap-bump` failed, rerun just that job:
#   gh run rerun --failed <release-run-id>
# The tap receiver is idempotent (it exits 0 when the formula is unchanged), so
# re-firing is safe. But tap-bump downloads BUILD_INFO.txt from an artifact with
# retention-days: 7. Past that window the rerun dies at the download step and a
# new tag is the only path.
#
# A leaked upstream tag is a silent downgrade, not a confusion. release.yml
# triggers on a palette-shaped tag only, but a plain `v0.9.1` pushed to the fork
# would sort ABOVE v0.9.0-palette.1 in Homebrew's ordering, so brew would pin a
# palette-less build and never upgrade back. Step 3 below refuses on one.

set -euo pipefail

FORK_REMOTE="${FORK_REMOTE:-fork}"
FORK_REPO="${FORK_REPO:-cameronsjo/herdr}"
TAP_REPO="${TAP_REPO:-cameronsjo/homebrew-tap}"

die() {
  printf 'cut-fork-release: %s\n' "$1" >&2
  exit 1
}

step() {
  printf '\n==> %s\n' "$1"
}

TAG="${1:-}"
[ -n "$TAG" ] || die "usage: scripts/cut-fork-release.sh vMAJOR.MINOR.PATCH-palette.N"

# ---------------------------------------------------------------------------
# 1. Tag shape. Stricter than scripts/fork_release_notes.py on purpose: the
#    validator tolerates a palette-less tag for historical reasons, this script
#    refuses to mint one.
# ---------------------------------------------------------------------------
step "Checking tag shape"
if ! printf '%s' "$TAG" | grep -qE '^v[0-9]+\.[0-9]+\.[0-9]+-palette\.[0-9]+$'; then
  die "refusing tag '$TAG': expected vMAJOR.MINOR.PATCH-palette.N"
fi
printf 'OK    %s\n' "$TAG"

# ---------------------------------------------------------------------------
# 2. Preconditions. Everything that can fail closed does so before any push.
# ---------------------------------------------------------------------------
step "Checking the remote"
remote_url="$(git remote get-url "$FORK_REMOTE" 2>/dev/null)" \
  || die "no remote named '$FORK_REMOTE' (override with FORK_REMOTE=)"
case "$remote_url" in
  *"$FORK_REPO"*) ;;
  *) die "remote '$FORK_REMOTE' is $remote_url, not $FORK_REPO -- refusing to tag it" ;;
esac
printf 'OK    %s -> %s\n' "$FORK_REMOTE" "$remote_url"

# Tracked files only. Step 5 tags fork/master explicitly, so nothing in the
# working tree can reach the release either way -- this is a "are you where you
# think you are" guard, and untracked scratch files are not that signal.
step "Checking the working tree"
if [ -n "$(git status --porcelain --untracked-files=no)" ]; then
  git status --short --untracked-files=no >&2
  die "tracked files are modified; commit or stash before cutting a release"
fi
printf 'OK    clean\n'

step "Fetching $FORK_REMOTE"
git fetch --no-tags "$FORK_REMOTE" master
git fetch "$FORK_REMOTE" "refs/tags/*:refs/tags/*" 2>/dev/null || true
printf 'OK    fetched\n'

step "Checking the tag is unused"
if git rev-parse -q --verify "refs/tags/$TAG" >/dev/null; then
  die "$TAG already exists locally -- a palette tag is never reused, cut the next counter"
fi
if git ls-remote --tags "$FORK_REMOTE" "refs/tags/$TAG" | grep -q .; then
  die "$TAG already exists on $FORK_REPO -- a palette tag is never reused, cut the next counter"
fi
printf 'OK    %s is free\n' "$TAG"

# ---------------------------------------------------------------------------
# 3. Leaked-upstream-tag check, as a verdict rather than ~150 refs to eyeball.
#    Scoped to the versions in play: the fork carries ~150 tags inherited at
#    fork creation, and an unscoped listing always looks alarming.
# ---------------------------------------------------------------------------
step "Checking for leaked upstream tags"
if git ls-remote --tags "$FORK_REMOTE" \
  | grep 'refs/tags/v' \
  | grep -v -- '-palette\.' \
  | grep -qE 'v0\.(8\.2|9\.[0-9]+)$'; then
  die "LEAK: an upstream tag for a version in play is present on $FORK_REPO; delete it (push --no-follow-tags --delete) before releasing"
fi
printf 'OK    CLEAN\n'

# ---------------------------------------------------------------------------
# 4. Release credentials. tap-bump runs AFTER release, so a missing credential
#    fails after the GitHub release is already published -- and the only clean
#    recovery from that is a new tag. Check before tagging, not after.
# ---------------------------------------------------------------------------
step "Checking release credentials"
gh variable list --repo "$FORK_REPO" | grep -q 'RELEASE_APP_ID' \
  || die "repo variable RELEASE_APP_ID is missing on $FORK_REPO; tap-bump would fail after publishing"
gh secret list --repo "$FORK_REPO" | grep -q 'RELEASE_APP_PRIVATE_KEY' \
  || die "secret RELEASE_APP_PRIVATE_KEY is missing on $FORK_REPO; tap-bump would fail after publishing"
printf 'OK    RELEASE_APP_ID + RELEASE_APP_PRIVATE_KEY present\n'

# ---------------------------------------------------------------------------
# 5. Tag fork/master explicitly. Never HEAD: this script may be run from a
#    worktree or a feature branch, and a tag one commit off ships a binary that
#    looks right and is not.
# ---------------------------------------------------------------------------
target="$(git rev-parse "$FORK_REMOTE/master")"
step "Tagging $FORK_REMOTE/master ($target)"
git tag -a "$TAG" "$FORK_REMOTE/master" -m "herdr $TAG (fork build with the command palette)"

tagged="$(git rev-parse "$TAG^{}")"
if [ "$tagged" != "$target" ]; then
  git tag -d "$TAG" >/dev/null
  die "tag landed on $tagged, expected $target -- tag deleted, nothing pushed"
fi
printf 'OK    %s -> %s\n' "$TAG" "$tagged"

# ---------------------------------------------------------------------------
# 6. Push. --no-follow-tags because push.followTags is global and the fork
#    carries upstream's tags locally; without it this push leaks them.
# ---------------------------------------------------------------------------
step "Pushing $TAG"
git push --no-follow-tags "$FORK_REMOTE" "refs/tags/$TAG"
printf 'OK    pushed\n'

# ---------------------------------------------------------------------------
# 7. Watch the release, then the tap. A dispatch returning 204 means queued,
#    not applied, so the tap's own run is what proves the formula moved.
# ---------------------------------------------------------------------------
step "Waiting for the Release run to appear"
run_id=""
for _ in $(seq 1 30); do
  run_id="$(gh run list --repo "$FORK_REPO" --workflow Release --branch "$TAG" \
    --limit 1 --json databaseId --jq '.[0].databaseId // empty')"
  [ -n "$run_id" ] && break
  sleep 10
done
[ -n "$run_id" ] || die "no Release run appeared for $TAG after 5 minutes; check https://github.com/$FORK_REPO/actions"
printf 'OK    run %s -- https://github.com/%s/actions/runs/%s\n' "$run_id" "$FORK_REPO" "$run_id"

step "Watching the Release run"
gh run watch "$run_id" --repo "$FORK_REPO" --exit-status || {
  printf '\nFAIL  Release run %s did not succeed.\n' "$run_id" >&2
  printf '      Do NOT re-push %s. Fix the cause and cut the next counter.\n' "$TAG" >&2
  exit 1
}

step "Checking the tap receiver"
tap_ok=""
for _ in $(seq 1 30); do
  tap_state="$(gh run list --repo "$TAP_REPO" --workflow "Update Formula" \
    --limit 1 --json status,conclusion --jq '.[0] | "\(.status):\(.conclusion)"' 2>/dev/null || true)"
  case "$tap_state" in
    completed:success) tap_ok="yes"; break ;;
    completed:*) die "tap receiver finished $tap_state -- see https://github.com/$TAP_REPO/actions" ;;
  esac
  sleep 10
done
[ -n "$tap_ok" ] || die "tap receiver did not complete in 5 minutes -- see https://github.com/$TAP_REPO/actions"

printf '\nPASS  %s released and %s bumped. Verify the asset with: brew update && brew fetch herdr\n' \
  "$TAG" "$TAP_REPO"
