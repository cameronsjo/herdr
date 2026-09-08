#!/usr/bin/env bash
#
# Cut a fork release: tag fork/master and let .github/workflows/release.yml
# build the arm64 macOS binary, publish it, and dispatch a formula bump to
# cameronsjo/homebrew-tap.
#
# Usage:
#   scripts/cut-fork-release.sh v0.9.0-palette.1     # cut a release
#   scripts/cut-fork-release.sh --dry-run v0.9.0-palette.1
#                                                    # run every precondition,
#                                                    # tag and push nothing
#   scripts/cut-fork-release.sh --watch v0.9.0-palette.1
#                                                    # resume watching one
#                                                    # already pushed
#
# There is no confirmation prompt: the bare form tags and pushes. Use --dry-run
# to exercise the guards. (Written after a session ran the bare form intending
# to test them and cut a real release.)
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
# A PUBLISHED palette tag is never reused. softprops/action-gh-release updates
# an existing release in place and overwrites the asset, so a delete-and-
# re-push gives the binary a new sha256 while every tap formula is still pinned
# to the old one. `brew upgrade` then fails a checksum on every installed
# machine with an error that reads like a corrupted download. Fix a bad build by
# cutting the next counter: v0.9.0-palette.2.
#
# A tag that exists ONLY LOCALLY was never published and is safe to discard --
# `git tag -d <tag>` and rerun. The script tells these two states apart rather
# than sending you to burn a counter for an interrupted run.
#
# If `release` succeeded but `tap-bump` failed, rerun just that job:
#   gh run rerun --failed <release-run-id>
# The tap receiver is idempotent, so re-firing is safe. But tap-bump downloads
# BUILD_INFO.txt from an artifact with retention-days: 7. Past that window the
# rerun dies at the download step and a new tag is the only path.
#
# A leaked upstream tag is a silent downgrade, not a confusion. A plain v0.9.0
# on the fork sorts ABOVE v0.9.0-palette.1 in Homebrew's ordering, so brew would
# pin a palette-less build and never upgrade back. Step 3 refuses on one. Two
# gates back this up: release.yml's tag filter and TAG_RE in
# scripts/fork_release_notes.py -- keep all three palette-only together.

set -euo pipefail

FORK_REMOTE="${FORK_REMOTE:-fork}"
FORK_REPO="${FORK_REPO:-cameronsjo/herdr}"
TAP_REPO="${TAP_REPO:-cameronsjo/homebrew-tap}"
FORMULA_PATH="${FORMULA_PATH:-Formula/herdr.rb}"

die() {
  printf 'cut-fork-release: %s\n' "$1" >&2
  exit 1
}

step() {
  printf '\n==> %s\n' "$1"
}

WATCH_ONLY=""
DRY_RUN=""
while [ $# -gt 0 ]; do
  case "$1" in
    --watch) WATCH_ONLY="yes"; shift ;;
    --dry-run) DRY_RUN="yes"; shift ;;
    --) shift; break ;;
    -*) die "unknown option '$1'; expected --watch or --dry-run" ;;
    *) break ;;
  esac
done

TAG="${1:-}"
[ -n "$TAG" ] || die "usage: scripts/cut-fork-release.sh [--watch|--dry-run] vMAJOR.MINOR.PATCH-palette.N"
[ -z "$WATCH_ONLY" ] || [ -z "$DRY_RUN" ] \
  || die "--watch and --dry-run are mutually exclusive"

# ---------------------------------------------------------------------------
# 1. Tag shape. Stricter than an unanchored grep: [[ =~ ]] matches the whole
#    string, so an embedded newline cannot smuggle a second line past it.
#    Palette segment is mandatory -- see the leaked-tag note in the header.
# ---------------------------------------------------------------------------
step "Checking tag shape"
if ! [[ "$TAG" =~ ^v[0-9]+\.[0-9]+\.[0-9]+-palette\.[0-9]+$ ]]; then
  die "refusing tag '$TAG': expected vMAJOR.MINOR.PATCH-palette.N"
fi
BASE_VERSION="${TAG%-palette.*}"
printf 'OK    %s (upstream base %s)\n' "$TAG" "$BASE_VERSION"

# ---------------------------------------------------------------------------
# 2. Preconditions. Everything that can fail closed does so before any push.
#    Every remote read is captured into a variable and its own exit status
#    checked BEFORE the match: `remote | grep -q` under `set -o pipefail`
#    returns grep's 1 when the remote call fails, so the guard would read a
#    failed query as "no match" and proceed. That is the wrong direction for
#    every check in this section.
# ---------------------------------------------------------------------------
step "Checking gh authentication"
gh auth status >/dev/null 2>&1 \
  || die "gh is not authenticated (run: gh auth login) -- cannot check credentials or watch runs"
printf 'OK    authenticated\n'

step "Checking the remote"
remote_url="$(git remote get-url "$FORK_REMOTE" 2>/dev/null)" \
  || die "no remote named '$FORK_REMOTE' (override with FORK_REMOTE=)"
# Normalise both SSH and HTTPS spellings to owner/name, then compare exactly.
# A substring match would accept cameronsjo/herdr-mirror, or any host at all.
remote_slug="$(printf '%s' "$remote_url" \
  | sed -e 's#^git@[^:]*:##' -e 's#^https://[^/]*/##' -e 's#\.git$##')"
[ "$remote_slug" = "$FORK_REPO" ] \
  || die "remote '$FORK_REMOTE' resolves to '$remote_slug', not '$FORK_REPO' -- refusing to tag it"
printf 'OK    %s -> %s\n' "$FORK_REMOTE" "$remote_slug"

# Tracked files only. Step 5 tags fork/master explicitly, so nothing in the
# working tree can reach the release either way -- this is an "are you where you
# think you are" guard, and untracked scratch files are not that signal.
step "Checking the working tree"
if [ -n "$(git status --porcelain --untracked-files=no)" ]; then
  git status --short --untracked-files=no >&2
  die "tracked files are modified; commit or stash before cutting a release"
fi
printf 'OK    clean\n'

step "Fetching $FORK_REMOTE"
git fetch --no-tags "$FORK_REMOTE" master \
  || die "could not fetch $FORK_REMOTE/master -- refusing to tag against a stale ref"
remote_tags="$(git ls-remote --tags "$FORK_REMOTE")" \
  || die "could not list tags on $FORK_REPO -- refusing to tag blind"
printf 'OK    fetched (%s remote tag refs)\n' "$(printf '%s\n' "$remote_tags" | grep -c 'refs/tags/' || true)"

# ---------------------------------------------------------------------------
# 3. Tag availability. Local-only and published are DIFFERENT states: only the
#    published one is unreusable, and telling an operator to burn a counter
#    after an interrupted run wastes a release for no reason.
# ---------------------------------------------------------------------------
step "Checking the tag is unused"
tag_on_remote=""
printf '%s\n' "$remote_tags" | grep -q "refs/tags/$TAG\$" && tag_on_remote="yes" || true

if [ -n "$tag_on_remote" ]; then
  # Published is exactly the state --watch exists for: rejoining a cut whose
  # push succeeded and whose watching half was interrupted.
  [ -n "$WATCH_ONLY" ] \
    || die "$TAG is already published on $FORK_REPO. A published palette tag is never reused -- re-pushing overwrites the asset while installed formulae stay pinned to the old sha256. Cut the next counter instead. (To rejoin a cut already in flight: --watch $TAG)"
  tagged="$(printf '%s\n' "$remote_tags" \
    | sed -n "s#^\([0-9a-f]*\)[[:space:]]*refs/tags/$TAG^{}\$#\1#p")"
  [ -n "$tagged" ] || tagged="$(printf '%s\n' "$remote_tags" \
    | sed -n "s#^\([0-9a-f]*\)[[:space:]]*refs/tags/$TAG\$#\1#p")"
  printf 'OK    %s is published (%s) -- resuming at the watch step\n' "$TAG" "$tagged"
elif git rev-parse -q --verify "refs/tags/$TAG" >/dev/null; then
  [ -z "$WATCH_ONLY" ] \
    || die "$TAG exists locally but was never pushed, so there is nothing to watch. Discard it (git tag -d $TAG) and rerun without --watch."
  die "$TAG exists locally but is NOT on $FORK_REPO -- an interrupted run, not a published release. Safe to discard: git tag -d $TAG, then rerun."
else
  [ -z "$WATCH_ONLY" ] \
    || die "--watch needs a tag already pushed to $FORK_REPO; $TAG is on neither side."
  printf 'OK    %s is free\n' "$TAG"
fi

# ---------------------------------------------------------------------------
# 4. Leaked-upstream-tag check, derived from the tag under cut rather than a
#    hardcoded version list -- a hardcoded one silently stops firing at the next
#    upstream minor bump, and a decaying guard that reports green is worse than
#    no guard. Any PLAIN vX.Y.Z on the fork that sorts at or above this
#    release's base version is a downgrade waiting to happen.
# ---------------------------------------------------------------------------
if [ -n "$WATCH_ONLY" ]; then
  # Steps 4-7 decide whether a tag SHOULD be cut. Under --watch it already was,
  # so re-running them can only produce a refusal about a decision already made.
  step "Checking tap visibility"
  gh api "repos/$TAP_REPO" >/dev/null 2>&1 \
    || die "cannot read $TAP_REPO -- the verification below needs read access to it"
  printf 'OK    %s readable\n' "$TAP_REPO"
else

step "Checking for leaked upstream tags"
plain_tags="$(printf '%s\n' "$remote_tags" \
  | sed -n 's#.*refs/tags/\(v[0-9][0-9.]*\)$#\1#p' \
  | sort -u)"
leaked=""
while IFS= read -r candidate; do
  [ -n "$candidate" ] || continue
  if [ "$candidate" = "$BASE_VERSION" ]; then
    leaked="$leaked $candidate"
    continue
  fi
  # If BASE_VERSION sorts first, candidate is the higher version.
  if [ "$(printf '%s\n%s\n' "$candidate" "$BASE_VERSION" | sort -V | head -1)" = "$BASE_VERSION" ]; then
    leaked="$leaked $candidate"
  fi
done <<EOF
$plain_tags
EOF

if [ -n "$leaked" ]; then
  die "LEAK: plain upstream tag(s)$leaked are on $FORK_REPO and sort at or above $BASE_VERSION. Homebrew would pin one of them over $TAG and never upgrade back. Delete them first: git push --no-follow-tags $FORK_REMOTE --delete <tag>"
fi
printf 'OK    CLEAN (no plain tag at or above %s)\n' "$BASE_VERSION"

# ---------------------------------------------------------------------------
# 5. Release credentials. tap-bump runs AFTER release, so a missing credential
#    fails after the GitHub release is already published -- and the only clean
#    recovery from that is a new tag. Names are matched exactly: an unanchored
#    match would accept a leftover RELEASE_APP_ID_OLD.
# ---------------------------------------------------------------------------
step "Checking release credentials"
var_names="$(gh variable list --repo "$FORK_REPO" --json name --jq '.[].name')" \
  || die "could not list repo variables on $FORK_REPO (permissions? network?) -- not a statement about the credentials themselves"
secret_names="$(gh secret list --repo "$FORK_REPO" --json name --jq '.[].name')" \
  || die "could not list repo secrets on $FORK_REPO (permissions? network?) -- not a statement about the credentials themselves"
printf '%s\n' "$var_names" | grep -qx 'RELEASE_APP_ID' \
  || die "repo variable RELEASE_APP_ID is missing on $FORK_REPO; tap-bump would fail after publishing"
printf '%s\n' "$secret_names" | grep -qx 'RELEASE_APP_PRIVATE_KEY' \
  || die "secret RELEASE_APP_PRIVATE_KEY is missing on $FORK_REPO; tap-bump would fail after publishing"
printf 'OK    RELEASE_APP_ID + RELEASE_APP_PRIVATE_KEY present\n'

step "Checking tap visibility"
gh api "repos/$TAP_REPO" >/dev/null 2>&1 \
  || die "cannot read $TAP_REPO -- the post-release verification needs read access to it. Fix access before cutting, or the release will publish with no way to confirm it landed."
printf 'OK    %s readable\n' "$TAP_REPO"

if [ -n "$DRY_RUN" ]; then
  printf '\nPASS  every precondition for %s holds. Nothing was tagged or pushed.\n' "$TAG"
  printf '      Cut it for real with: scripts/cut-fork-release.sh %s\n' "$TAG"
  exit 0
fi

# ---------------------------------------------------------------------------
# 6. Tag fork/master explicitly. Never HEAD: this script may be run from a
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
# 7. Push. --no-follow-tags because push.followTags is global and the fork
#    carries upstream's tags locally; without it this push leaks them.
# ---------------------------------------------------------------------------
step "Pushing $TAG"
git push --no-follow-tags "$FORK_REMOTE" "refs/tags/$TAG"
printf 'OK    pushed. From here, rejoin with: scripts/cut-fork-release.sh --watch %s\n' "$TAG"

fi  # end of the cut-only half; --watch resumes here

# ---------------------------------------------------------------------------
# 8. Watch the Release run. --branch "$TAG" is a sound correlation here because
#    a tag push sets head_branch to the tag name and a tag is never reused --
#    unlike the tap, which has no such handle (see step 9).
# ---------------------------------------------------------------------------
step "Waiting for the Release run to appear"
run_id=""
for _ in $(seq 1 30); do
  run_id="$(gh run list --repo "$FORK_REPO" --workflow Release --branch "$TAG" \
    --limit 1 --json databaseId --jq '.[0].databaseId // empty')" || run_id=""
  [ -n "$run_id" ] && break
  sleep 10
done
[ -n "$run_id" ] || die "no Release run appeared for $TAG after 5 minutes. The tag is pushed; check https://github.com/$FORK_REPO/actions and rejoin with --watch $TAG."
run_url="https://github.com/$FORK_REPO/actions/runs/$run_id"
printf 'OK    run %s -- %s\n' "$run_id" "$run_url"

step "Watching the Release run"
if ! gh run watch "$run_id" --repo "$FORK_REPO" --exit-status; then
  # gh run watch exits non-zero for its OWN failures (dropped connection, token
  # expiry, API 5xx) exactly as it does for a failed run. Re-query before
  # verdicting, or a transient network blip burns a release counter.
  state="$(gh run view "$run_id" --repo "$FORK_REPO" --json status,conclusion \
    --jq '"\(.status):\(.conclusion)"' 2>/dev/null || echo "unknown:unknown")"
  case "$state" in
    completed:success)
      printf 'NOTE  watch dropped, but run %s completed successfully.\n' "$run_id"
      ;;
    completed:*)
      printf '\nFAIL  Release run %s finished %s -- %s\n' "$run_id" "$state" "$run_url" >&2
      printf '      Do NOT re-push %s. Fix the cause and cut the next counter.\n' "$TAG" >&2
      exit 1
      ;;
    *)
      die "lost the connection while watching run $run_id, which is still '$state'. Nothing is decided; rejoin with: scripts/cut-fork-release.sh --watch $TAG"
      ;;
  esac
fi

# ---------------------------------------------------------------------------
# 9. Verify the tap. NOT by reading the tap's newest workflow run: the tap
#    carries several formulae, so its most recent "Update Formula" run is
#    usually somebody else's and already `completed:success` -- a poll on it
#    passes on iteration one, before this dispatch could have been received,
#    and would print PASS for a formula that never moved. The formula's own
#    bytes are the only signal unambiguously about this release.
# ---------------------------------------------------------------------------
step "Reading the published digest"
release_body="$(gh release view "$TAG" --repo "$FORK_REPO" --json body --jq '.body')" \
  || die "could not read the release body for $TAG -- the release exists; verify the tap by hand"
digest="$(printf '%s' "$release_body" | grep -oE '[0-9a-f]{64}' | head -1)"
[ -n "$digest" ] || die "the release notes for $TAG carry no sha256 -- verify the tap by hand"
printf 'OK    sha256=%s\n' "$digest"

step "Waiting for $TAP_REPO to carry $TAG"
version="${TAG#v}"
tap_ok=""
for _ in $(seq 1 30); do
  formula="$(gh api -H "Accept: application/vnd.github.raw" \
    "repos/$TAP_REPO/contents/$FORMULA_PATH" 2>/dev/null || true)"
  if printf '%s' "$formula" | grep -qF "\"$version\"" \
    && printf '%s' "$formula" | grep -qF "$digest"; then
    tap_ok="yes"
    break
  fi
  sleep 10
done

if [ -z "$tap_ok" ]; then
  printf '\nFAIL  %s/%s still does not carry version %s and sha256 %s after 5 minutes.\n' \
    "$TAP_REPO" "$FORMULA_PATH" "$version" "$digest" >&2
  printf '      The release itself published fine -- only the formula bump is missing.\n' >&2
  printf '      Rerun the failed job: gh run rerun --failed %s\n' "$run_id" >&2
  printf '      (Only works inside the artifact retention window; see this script header.)\n' >&2
  printf '      Tap runs: https://github.com/%s/actions\n' "$TAP_REPO" >&2
  exit 1
fi

# ---------------------------------------------------------------------------
# 10. One verdict, plus everything the operator has to carry forward.
# ---------------------------------------------------------------------------
cat <<EOF

PASS  $TAG released and $TAP_REPO bumped.

  tag         $TAG
  commit      $tagged
  release     https://github.com/$FORK_REPO/releases/tag/$TAG
  sha256      $digest
  formula     $TAP_REPO $FORMULA_PATH version "$version"
  run         $run_url

Confirm the asset downloads and hashes (brew info would report the version
string whether or not it does):

  brew update && brew fetch herdr
EOF
