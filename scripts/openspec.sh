#!/usr/bin/env bash
# Run the pinned OpenSpec CLI against this repo's openspec/ tree.
#
# The version and every transitive package's integrity hash live in
# scripts/openspec/package-lock.json, so this installs exactly what was
# reviewed (npm ci, install scripts off) instead of whatever `npx` resolves
# today. CI's openspec job runs it the same way.
#
# Usage:
#   scripts/openspec.sh validate --all --strict --no-interactive
#   scripts/openspec.sh list --specs
#   scripts/openspec.sh <any openspec subcommand>
#
# `init` and `update` also run scripts/openspec-localize.sh afterwards.
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
pin="$root/scripts/openspec"
bin="$pin/node_modules/.bin/openspec"

# Reinstall when the lockfile is newer than the install, so a bumped pin is
# picked up without a manual clean.
if [[ ! -x "$bin" || "$pin/package-lock.json" -nt "$pin/node_modules/.package-lock.json" ]]; then
  npm ci --prefix "$pin" --ignore-scripts --no-audit --no-fund --loglevel=error >&2
fi

# No telemetry and no npm update check. The first run (or a bumped pin) still
# downloads the pinned packages with npm ci.
export OPENSPEC_TELEMETRY="${OPENSPEC_TELEMETRY:-0}"
export OPENSPEC_NO_UPDATE_CHECK="${OPENSPEC_NO_UPDATE_CHECK:-1}"
# Subcommands that reach outside this repo: `feedback` files a GitHub issue
# upstream with your gh login, `completion` writes your shell rc, `config`
# edits the global config or opens $EDITOR, `workset` launches an opener,
# `store` (but its read-only list/ls/doctor) registers, creates or deletes a
# store elsewhere on disk, and `--store <id>` on any command works in that
# store's folder instead of this repo. None is part of the spec workflow, so
# they need an explicit opt-in. This is a guard for the usual ways out, not a
# sandbox: the CLI still runs as you.
# The first argument that is not an option is the subcommand (`--no-color
# feedback` is still feedback).
for arg in "$@"; do
  case "$arg" in
    --store | --store=*)
      if [[ "${OPENSPEC_ALLOW_OUTSIDE:-}" != 1 ]]; then
        echo "openspec.sh: '--store' works outside this repo; rerun with OPENSPEC_ALLOW_OUTSIDE=1 to allow it" >&2
        exit 2
      fi
      ;;
  esac
done
sub=""
for arg in "$@"; do
  case "$arg" in
    -*) ;;
    *) sub="$arg"; break ;;
  esac
done
case "$sub" in
  feedback | completion | config | workset)
    if [[ "${OPENSPEC_ALLOW_OUTSIDE:-}" != 1 ]]; then
      echo "openspec.sh: '$sub' reaches outside this repo; rerun with OPENSPEC_ALLOW_OUTSIDE=1 to allow it" >&2
      exit 2
    fi
    ;;
  store)
    # The store's own subcommand is the next argument that is not an option.
    action=""
    seen=0
    for arg in "$@"; do
      if [[ $seen == 0 ]]; then
        [[ "$arg" == store ]] && seen=1
        continue
      fi
      case "$arg" in
        -*) ;;
        *) action="$arg"; break ;;
      esac
    done
    case "$action" in
      "" | list | ls | doctor) ;;
      *)
        if [[ "${OPENSPEC_ALLOW_OUTSIDE:-}" != 1 ]]; then
          echo "openspec.sh: 'store $action' reaches outside this repo; rerun with OPENSPEC_ALLOW_OUTSIDE=1 to allow it" >&2
          exit 2
        fi
        ;;
    esac
    ;;
  context)
    # `context --code-workspace <path> --force` overwrites any file with
    # workspace JSON, and the skills pre-allow `context`.
    for arg in "$@"; do
      case "$arg" in
        --code-workspace | --code-workspace=*)
          if [[ "${OPENSPEC_ALLOW_OUTSIDE:-}" != 1 ]]; then
            echo "openspec.sh: 'context --code-workspace' writes outside this repo; rerun with OPENSPEC_ALLOW_OUTSIDE=1 to allow it" >&2
            exit 2
          fi
          ;;
      esac
    done
    ;;
esac

cd "$root"
case "$sub" in
  init | update)
    # Both rewrite the generated skills with a bare, unpinned
    # `Bash(openspec:*)` grant, which the OpenSpec workflow rejects. Localize
    # them in the same run, even after a failed CLI run that may have written
    # some of them. The CLI's failure wins the exit status.
    status=0
    "$bin" "$@" || status=$?
    localize=0
    bash "$root/scripts/openspec-localize.sh" || localize=$?
    if [[ $status -ne 0 ]]; then
      exit "$status"
    fi
    exit "$localize"
    ;;
esac
exec "$bin" "$@"
