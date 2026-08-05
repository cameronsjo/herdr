#!/usr/bin/env bash
# Runs the Rust side of `just check` (fmt, clippy, nextest) inside a Linux
# container, plus the maintenance script tests natively on the host.
#
# Use this instead of `just check` on a machine where herdr can't build
# natively — e.g. a macOS host whose SDK no longer ships the arm64-macos
# libSystem slice Zig 0.15.2 needs (herdrdev/herdr#285,
# docs/gotchas/zig-macos-sdk-wall.md upstream). Requires Docker (or Colima)
# running locally.
set -euo pipefail

ROOT_DIR=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
IMAGE_TAG=herdr-docker-check:local
REGISTRY_VOLUME=herdr-docker-check-cargo-registry

# These tests fail in this container image regardless of code changes —
# confirmed by running them against a pristine origin/master checkout in the
# same image and getting the identical list, both parallel AND serialized
# (--test-threads 1), on 2026-08-05. They are NOT load/parallelism flakes:
# serializing the run doesn't rescue any of them. Root causes fall into two
# groups:
#   - `*_terminates_processes_inside_*`: rely on process-group reaping that
#     needs a subreaper; a plain `docker run` (even with --init) has none.
#   - the rest (hooks session-id tests, live_handoff tests, the TTL test):
#     fail identically with or without serialization, so it's a container
#     environment difference (missing agent CLI stubs, clock granularity,
#     loopback/networking setup), not a race.
# If a run surfaces a failure NOT on this list, treat it as real — this is a
# verified allowlist, not a place to stash new flakiness without checking.
KNOWN_ENV_FAILURES=(
  "cases::hooks::claude_hook_reports_session_id_from_stdin"
  "cases::hooks::codex_hook_reports_persisted_root_session_and_ignores_ephemeral_or_nested_sessions"
  "cases::hooks::copilot_hook_reports_session_id_from_stdin"
  "cases::hooks::devin_hook_prefers_hook_session_id_over_list"
  "cases::hooks::devin_hook_reports_session_id_from_stdin_without_state"
  "cases::hooks::devin_hook_reports_tool_session_from_list_without_state"
  "cases::panes::closing_pane_terminates_processes_inside_it"
  "cases::panes::closing_workspace_terminates_processes_inside_it"
  "cases::workspace::forced_worktree_remove_terminates_processes_inside_checkout"
  "live_handoff_preserves_http_servers_across_multiple_sessions"
  "live_handoff_preserves_keyboard_protocol_for_client_input"
  "live_handoff_preserves_modify_other_keys_for_client_input"
  "live_handoff_preserves_python_http_server"
  "terminal::state::metadata::tests::metadata_clear_only_without_ttl_does_not_extend_old_ttl"
)

if ! command -v docker >/dev/null 2>&1; then
  echo "error: docker not found on PATH (Colima counts — start it with 'colima start')" >&2
  exit 1
fi
if ! docker info >/dev/null 2>&1; then
  echo "error: docker daemon not reachable (is Colima/Docker Desktop running?)" >&2
  exit 1
fi

echo "==> building check image ($IMAGE_TAG)"
docker build -q -t "$IMAGE_TAG" "$ROOT_DIR/scripts/docker-check" >/dev/null

echo "==> cargo fmt --check, clippy, nextest (in container)"
LOG_FILE=$(mktemp)
trap 'rm -f "$LOG_FILE"' EXIT

set +e
docker run --rm \
  -v "$ROOT_DIR:/work" \
  -v "$REGISTRY_VOLUME:/opt/cargo/registry" \
  -w /work \
  "$IMAGE_TAG" \
  bash -c '
    set -euo pipefail
    rm -rf .zig-cache vendor/libghostty-vt/.zig-cache vendor/libghostty-vt/zig-out
    cargo fmt --check
    cargo clippy --all-targets --locked -- -D warnings
    cargo nextest run --locked --no-fail-fast --status-level fail --final-status-level fail --failure-output final --success-output never
  ' 2>&1 | tee "$LOG_FILE"
NEXTEST_EXIT=${PIPESTATUS[0]}
set -e

if [[ "$NEXTEST_EXIT" -ne 0 ]]; then
  if ! command grep -q "Nextest run ID" "$LOG_FILE"; then
    echo
    echo "error: cargo fmt --check or cargo clippy failed before nextest ran — see output above" >&2
    exit 1
  fi

  mapfile -t actual_failures < <(command grep -E '^ *FAIL ' "$LOG_FILE" | awk '{print $NF}' | sort -u)
  unexpected=()
  for failure in "${actual_failures[@]}"; do
    known=false
    for allowed in "${KNOWN_ENV_FAILURES[@]}"; do
      [[ "$failure" == "$allowed" ]] && known=true && break
    done
    "$known" || unexpected+=("$failure")
  done

  if [[ "${#unexpected[@]}" -gt 0 ]]; then
    echo
    echo "error: unexpected test failure(s), not on the known-environment-artifact list:" >&2
    printf '  %s\n' "${unexpected[@]}" >&2
    exit 1
  fi

  echo
  echo "note: ${#actual_failures[@]} known container-environment failure(s), not code regressions (see KNOWN_ENV_FAILURES in this script)"
fi

# Some maintenance tests shell out to cargo/git directly; make sure a
# rustup-managed cargo is on PATH even if the invoking shell never sourced
# it, and neutralize any personal global git config (e.g. forced tag
# signing) that has nothing to do with this repo's correctness.
[[ -f "$HOME/.cargo/env" ]] && source "$HOME/.cargo/env"

echo
echo "==> maintenance script tests (native, needs git/python3/node/cargo on host PATH)"
(
  cd "$ROOT_DIR"
  GIT_CONFIG_GLOBAL=/dev/null GIT_CONFIG_SYSTEM=/dev/null python3 -m unittest \
    scripts.test_agent_detection_manifest_check \
    scripts.test_changelog \
    scripts.test_config_reference_check \
    scripts.test_docs_translation_parity \
    scripts.test_hermes_integration_asset \
    scripts.test_package_windows_conpty \
    scripts.test_preview \
    scripts.test_vendor_libghostty_vt \
    scripts.test_vendor_portable_pty
)

echo
echo "docker-check: PASS"
