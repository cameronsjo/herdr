#!/usr/bin/env bash
# Runs the Rust side of `just check` (fmt, clippy, nextest) inside a Linux
# container, plus the maintenance script tests natively on the host.
#
# Use this instead of `just check` on a machine where herdr can't build
# natively — e.g. a macOS host whose SDK no longer ships the arm64-macos
# libSystem slice Zig 0.15.2 needs (herdrdev/herdr#285,
# docs/gotchas/zig-macos-sdk-wall.md upstream). Requires Docker (or Colima)
# running locally.
#
# An unexpected nextest failure (not on KNOWN_ENV_FAILURES below) is retried
# once, alone, before being reported: a load-contention flake passes on that
# retry, a genuine regression fails both times (herdrdev/herdr#25). See
# run_nextest_filter / classify_nextest_failures and their test coverage in
# scripts/test_docker_check.py.
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

# run_nextest_filter FILTER_EXPR
# Runs `cargo nextest run` inside the check container, scoped to the given
# nextest filter expression (e.g. `test(=cases::hooks::some_test)`), and
# returns its exit status. This is the real implementation; it's only
# installed when nothing has already defined the function, which is what
# lets a test source this file (skipping execution — see the BASH_SOURCE
# guard below) after supplying its own stand-in, and exercise
# classify_nextest_failures below without Docker or a real nextest run.
if ! declare -F run_nextest_filter >/dev/null; then
  run_nextest_filter() {
    local filter="$1"
    docker run --rm \
      -v "$ROOT_DIR:/work" \
      -v "$REGISTRY_VOLUME:/opt/cargo/registry" \
      -w /work \
      "$IMAGE_TAG" \
      bash -c 'cargo nextest run --locked -E "$1" --no-fail-fast --status-level fail --final-status-level fail --failure-output final --success-output never' _ "$filter"
  }
fi

# classify_nextest_failures LOG_FILE
# Reads the FAIL lines out of a completed nextest run's log, drops anything
# on KNOWN_ENV_FAILURES, then retries each remaining (unexpected) failure
# once in isolation via run_nextest_filter with an exact-match filter — a
# load-contention flake passes alone; a genuine regression fails again.
# Populates the global arrays ACTUAL_FAILURES and STILL_FAILING (the subset
# of unexpected failures that failed twice).
classify_nextest_failures() {
  local log_file="$1"

  mapfile -t ACTUAL_FAILURES < <(command grep -E '^ *FAIL ' "$log_file" | awk '{print $NF}' | sort -u)

  local -a unexpected=()
  local failure known allowed
  for failure in "${ACTUAL_FAILURES[@]}"; do
    known=false
    for allowed in "${KNOWN_ENV_FAILURES[@]}"; do
      [[ "$failure" == "$allowed" ]] && known=true && break
    done
    "$known" || unexpected+=("$failure")
  done

  STILL_FAILING=()
  for failure in "${unexpected[@]}"; do
    echo "note: retrying unexpected failure in isolation: $failure" >&2
    if run_nextest_filter "test(=$failure)"; then
      echo "note: $failure passed on retry — treated as a flake, not reported" >&2
    else
      echo "note: $failure failed again on retry — real failure" >&2
      STILL_FAILING+=("$failure")
    fi
  done
}

if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
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
      # cargo only reruns build.rs when one of its declared rerun-if-changed
      # inputs changes — deleting zig-out is not one of them, so on a checkout
      # where only .rs sources changed since the last run, cargo trusts the
      # stale fingerprint and skips the rebuild, leaving link errors
      # ("cannot find -lghostty-vt") on an otherwise-correct build. Touching
      # build.rs forces the rerun every time, matching the always-wiped zig-out
      # above.
      touch build.rs
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

    classify_nextest_failures "$LOG_FILE"

    if [[ "${#STILL_FAILING[@]}" -gt 0 ]]; then
      echo
      echo "error: unexpected test failure(s), not on the known-environment-artifact list and still failing after a retry in isolation:" >&2
      printf '  %s\n' "${STILL_FAILING[@]}" >&2
      exit 1
    fi

    echo
    echo "note: ${#ACTUAL_FAILURES[@]} failure(s) in the full run were not real regressions (on the known-environment allowlist, or a load-contention flake that passed when retried alone)"
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
      scripts.test_docker_check \
      scripts.test_docs_translation_parity \
      scripts.test_hermes_integration_asset \
      scripts.test_package_windows_conpty \
      scripts.test_preview \
      scripts.test_vendor_libghostty_vt \
      scripts.test_vendor_portable_pty
  )

  echo
  echo "docker-check: PASS"
fi
