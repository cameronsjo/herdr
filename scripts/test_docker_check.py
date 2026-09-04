"""Tests for the unexpected-failure retry logic in scripts/docker-check.sh.

These exercise the real bash functions (run_nextest_filter,
classify_nextest_failures) by sourcing docker-check.sh — which skips its
`docker build` / `docker run` main body when sourced rather than executed
(the `[[ "${BASH_SOURCE[0]}" == "${0}" ]]` guard) — after stubbing
run_nextest_filter with a fixture standing in for a real nextest retry. No
Docker or cargo nextest is required to run these.
"""

from __future__ import annotations

import subprocess
import tempfile
import unittest
from contextlib import contextmanager
from pathlib import Path

SCRIPT_PATH = Path(__file__).resolve().parent / "docker-check.sh"

# A single realistic nextest FAIL line, as observed from a live run:
#   FAIL [   0.019s] (1/1) herdr::bin/herdr app::state::tests::some_test
# `docker-check.sh` extracts the trailing whitespace-separated field (the
# dotted module::test_name path) with `awk '{print $NF}'`.
FAIL_LINE_TEMPLATE = "        FAIL [   0.019s] (1/1) herdr::cli {name}"

LOG_PREAMBLE = "Nextest run ID deadbeef-0000-0000-0000-000000000000 with nextest profile: default\n"


def _write_log(tmp_path: Path, failing_test_names: list[str]) -> Path:
    log_file = tmp_path / "nextest.log"
    lines = [LOG_PREAMBLE]
    for name in failing_test_names:
        lines.append(FAIL_LINE_TEMPLATE.format(name=name) + "\n")
    log_file.write_text("".join(lines), encoding="utf-8")
    return log_file


def _run_classification(
    log_file: Path, retry_outcomes: dict[str, bool]
) -> subprocess.CompletedProcess[str]:
    """Sources docker-check.sh with a stubbed run_nextest_filter, then calls
    classify_nextest_failures against `log_file` and prints STILL_FAILING.

    `retry_outcomes` maps a bare test name to whether its retry should pass
    (True) or fail again (False); the stub inspects the incoming
    `test(=<name>)` filter to decide which outcome applies.
    """
    outcome_cases = "\n".join(
        f'    "test(={name})") {"return 0" if passes else "return 1"} ;;'
        for name, passes in retry_outcomes.items()
    )
    script = f"""
set -euo pipefail

run_nextest_filter() {{
  local filter="$1"
  echo "retry-invoked: $filter" >&2
  case "$filter" in
{outcome_cases}
    *) echo "unexpected filter: $filter" >&2; return 1 ;;
  esac
}}

source "{SCRIPT_PATH}"

classify_nextest_failures "{log_file}"

echo "STILL_FAILING_COUNT=${{#STILL_FAILING[@]}}"
for name in "${{STILL_FAILING[@]}}"; do
  echo "STILL_FAILING_ENTRY=$name"
done
"""
    return subprocess.run(
        ["bash", "-c", script],
        capture_output=True,
        text=True,
        check=False,
    )


class DockerCheckRetryTests(unittest.TestCase):
    def test_flaky_failure_passes_on_retry_and_is_not_reported(self) -> None:
        """The issue's acceptance bar: a test that fails once (the full run)
        and passes once retried in isolation is a flake, not a failure."""
        with _tmp_dir() as tmp_path:
            log_file = _write_log(
                tmp_path,
                ["agent_start_stops_retrying_when_the_pane_shell_stays_busy"],
            )
            result = _run_classification(
                log_file,
                {"agent_start_stops_retrying_when_the_pane_shell_stays_busy": True},
            )

        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("STILL_FAILING_COUNT=0", result.stdout)
        self.assertNotIn("STILL_FAILING_ENTRY", result.stdout)
        self.assertIn(
            "retry-invoked: test(=agent_start_stops_retrying_when_the_pane_shell_stays_busy)",
            result.stderr,
        )

    def test_failure_that_repeats_on_retry_is_still_reported(self) -> None:
        """The negative case: a test failing twice (full run + retry) must
        still surface as a real failure, never silently swallowed."""
        with _tmp_dir() as tmp_path:
            log_file = _write_log(tmp_path, ["a_genuinely_broken_test"])
            result = _run_classification(
                log_file, {"a_genuinely_broken_test": False}
            )

        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("STILL_FAILING_COUNT=1", result.stdout)
        self.assertIn("STILL_FAILING_ENTRY=a_genuinely_broken_test", result.stdout)

    def test_known_env_failure_is_never_retried(self) -> None:
        """Failures already on the allowlist skip the retry entirely — no
        run_nextest_filter call, no report."""
        with _tmp_dir() as tmp_path:
            log_file = _write_log(
                tmp_path,
                ["cases::hooks::claude_hook_reports_session_id_from_stdin"],
            )
            result = _run_classification(log_file, {})

        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("STILL_FAILING_COUNT=0", result.stdout)
        self.assertNotIn("retry-invoked", result.stderr)

    def test_mixed_flake_and_real_failure_reports_only_the_real_one(self) -> None:
        with _tmp_dir() as tmp_path:
            log_file = _write_log(
                tmp_path, ["flaky_under_load", "a_genuinely_broken_test"]
            )
            result = _run_classification(
                log_file,
                {"flaky_under_load": True, "a_genuinely_broken_test": False},
            )

        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("STILL_FAILING_COUNT=1", result.stdout)
        self.assertIn("STILL_FAILING_ENTRY=a_genuinely_broken_test", result.stdout)
        self.assertNotIn("STILL_FAILING_ENTRY=flaky_under_load", result.stdout)


@contextmanager
def _tmp_dir():
    with tempfile.TemporaryDirectory() as raw:
        yield Path(raw)


if __name__ == "__main__":
    unittest.main()
