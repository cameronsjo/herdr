"""Tests for the unexpected-failure retry logic in scripts/docker-check.sh.

These exercise the real bash functions (run_nextest_filter,
classify_nextest_failures) by sourcing docker-check.sh — which skips its
`docker build` / `docker run` main body when sourced rather than executed
(the `[[ "${BASH_SOURCE[0]}" == "${0}" ]]` guard) — and then redefining
run_nextest_filter *after* the source, standing in for a real nextest
retry. Redefining after source (never before) matters: the real
implementation installs unconditionally, and a stray `export -f
run_nextest_filter` loading before this script must never be able to
shadow it — the test's own stub follows that same rule so it can't
accidentally rely on the vulnerable ordering it's meant to guard against.
No Docker or cargo nextest is required to run these.
"""

from __future__ import annotations

import subprocess
import tempfile
import unittest
from contextlib import contextmanager
from pathlib import Path

SCRIPT_PATH = Path(__file__).resolve().parent / "docker-check.sh"

# A single realistic nextest FAIL line, as observed from a live run:
#   FAIL [   0.019s] (1/1) herdr::cli app::state::tests::some_test
# `docker-check.sh` extracts field 5 (the binary id) and everything after
# it (the test name, which may itself contain spaces).
FAIL_LINE_TEMPLATE = "        FAIL [   0.019s] (1/1) {binary} {name}"

LOG_PREAMBLE = "Nextest run ID deadbeef-0000-0000-0000-000000000000 with nextest profile: default\n"

DEFAULT_BINARY = "herdr::cli"


def _write_log(tmp_path: Path, failures: list[tuple[str, str]]) -> Path:
    """`failures` is a list of (binary_id, test_name) pairs."""
    log_file = tmp_path / "nextest.log"
    lines = [LOG_PREAMBLE]
    for binary, name in failures:
        lines.append(FAIL_LINE_TEMPLATE.format(binary=binary, name=name) + "\n")
    log_file.write_text("".join(lines), encoding="utf-8")
    return log_file


def _run_classification(
    log_file: Path, stub_body: str
) -> subprocess.CompletedProcess[str]:
    """Sources docker-check.sh (real run_nextest_filter installed), then
    redefines run_nextest_filter with `stub_body` (a bash function body
    receiving $1=binary_id $2=test_name), then calls
    classify_nextest_failures against `log_file` and prints STILL_FAILING.
    """
    script = f"""
set -euo pipefail

source "{SCRIPT_PATH}"

run_nextest_filter() {{
  local binary_id="$1"
  local test_name="$2"
  echo "retry-invoked: $binary_id | $test_name" >&2
{stub_body}
}}

classify_nextest_failures "{log_file}"

echo "STILL_FAILING_COUNT=${{#STILL_FAILING[@]}}"
for entry in "${{STILL_FAILING[@]}}"; do
  echo "STILL_FAILING_ENTRY=$entry"
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
        name = "agent_start_stops_retrying_when_the_pane_shell_stays_busy"
        with _tmp_dir() as tmp_path:
            log_file = _write_log(tmp_path, [(DEFAULT_BINARY, name)])
            result = _run_classification(log_file, "  return 0")

        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("STILL_FAILING_COUNT=0", result.stdout)
        self.assertNotIn("STILL_FAILING_ENTRY", result.stdout)
        self.assertIn(f"retry-invoked: {DEFAULT_BINARY} | {name}", result.stderr)

    def test_failure_that_repeats_on_retry_is_still_reported(self) -> None:
        """The negative case: a test failing twice (full run + retry) must
        still surface as a real failure, never silently swallowed."""
        with _tmp_dir() as tmp_path:
            log_file = _write_log(
                tmp_path, [(DEFAULT_BINARY, "a_genuinely_broken_test")]
            )
            result = _run_classification(log_file, "  return 1")

        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("STILL_FAILING_COUNT=1", result.stdout)
        self.assertIn(
            f"STILL_FAILING_ENTRY={DEFAULT_BINARY}\ta_genuinely_broken_test",
            result.stdout,
        )

    def test_known_env_failure_is_never_retried(self) -> None:
        """Failures already on the allowlist skip the retry entirely — no
        run_nextest_filter call, no report."""
        with _tmp_dir() as tmp_path:
            log_file = _write_log(
                tmp_path,
                [
                    (
                        DEFAULT_BINARY,
                        "cases::hooks::claude_hook_reports_session_id_from_stdin",
                    )
                ],
            )
            result = _run_classification(log_file, "  return 1")

        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("STILL_FAILING_COUNT=0", result.stdout)
        self.assertNotIn("retry-invoked", result.stderr)

    def test_mixed_flake_and_real_failure_reports_only_the_real_one(self) -> None:
        with _tmp_dir() as tmp_path:
            log_file = _write_log(
                tmp_path,
                [
                    (DEFAULT_BINARY, "flaky_under_load"),
                    (DEFAULT_BINARY, "a_genuinely_broken_test"),
                ],
            )
            stub = """
  case "$test_name" in
    flaky_under_load) return 0 ;;
    *) return 1 ;;
  esac
"""
            result = _run_classification(log_file, stub)

        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("STILL_FAILING_COUNT=1", result.stdout)
        self.assertIn(
            f"STILL_FAILING_ENTRY={DEFAULT_BINARY}\ta_genuinely_broken_test",
            result.stdout,
        )
        self.assertNotIn("flaky_under_load", result.stdout)

    def test_retry_filter_matching_zero_tests_is_a_hard_failure_not_a_flake(
        self,
    ) -> None:
        """Critical 1 (herdrdev/herdr#25 review): a retry filter that
        matches nothing must never read as a passing retry. This stub
        simulates `--no-tests=fail`'s NO_TESTS_RUN exit (4) — any nonzero
        exit, including 4, must land the failure in STILL_FAILING."""
        with _tmp_dir() as tmp_path:
            log_file = _write_log(
                tmp_path, [(DEFAULT_BINARY, "renamed_or_unmatched_test")]
            )
            result = _run_classification(log_file, "  return 4")

        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("STILL_FAILING_COUNT=1", result.stdout)
        self.assertIn(
            f"STILL_FAILING_ENTRY={DEFAULT_BINARY}\trenamed_or_unmatched_test",
            result.stdout,
        )

    def test_retry_is_scoped_to_the_failing_binary_not_test_name_alone(
        self,
    ) -> None:
        """Critical 2 (herdrdev/herdr#25 review): two binaries can define a
        test with the same name. A real regression in one binary must not
        retry-pass just because an unscoped `test(=name)` filter would also
        match the other binary's healthy copy.

        The stub only recognizes the correctly *binary-scoped* call
        ($1=the failing binary) and fails it — as a real regression should.
        Any call arriving without that binary_id (the pre-fix behavior,
        which discarded the binary and only passed the test name) falls
        through to the `*)` arm, which passes — simulating an unscoped
        filter hitting the other binary's healthy copy. If
        classify_nextest_failures regresses to filtering by test name
        alone, this test starts asserting the wrong thing and fails."""
        shared_name = "shared_test_name_across_binaries"
        failing_binary = "herdr::cli"
        with _tmp_dir() as tmp_path:
            log_file = _write_log(tmp_path, [(failing_binary, shared_name)])
            stub = f"""
  case "$binary_id" in
    {failing_binary}) return 1 ;;
    *) return 0 ;;
  esac
"""
            result = _run_classification(log_file, stub)

        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("STILL_FAILING_COUNT=1", result.stdout)
        self.assertIn(
            f"STILL_FAILING_ENTRY={failing_binary}\t{shared_name}", result.stdout
        )
        self.assertIn(f"retry-invoked: {failing_binary} | {shared_name}", result.stderr)

    def test_test_name_containing_spaces_survives_extraction(self) -> None:
        """Nit 5: proc-macro-generated test names can contain spaces; the
        extraction must not truncate to the last whitespace-separated
        field."""
        name_with_spaces = "some proc macro generated case name"
        with _tmp_dir() as tmp_path:
            log_file = _write_log(tmp_path, [(DEFAULT_BINARY, name_with_spaces)])
            result = _run_classification(log_file, "  return 1")

        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn(
            f"STILL_FAILING_ENTRY={DEFAULT_BINARY}\t{name_with_spaces}",
            result.stdout,
        )

    def test_run_nextest_filter_installs_the_real_implementation_by_default(
        self,
    ) -> None:
        """Important 3/4: sourcing the script with nothing pre-defining
        run_nextest_filter must install the real docker-backed
        implementation unconditionally — never silently no-op-installed."""
        script = f"""
set -euo pipefail
source "{SCRIPT_PATH}"
declare -f run_nextest_filter
"""
        result = subprocess.run(
            ["bash", "-c", script], capture_output=True, text=True, check=False
        )

        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("docker run", result.stdout)
        self.assertIn("binary_id(=", result.stdout)
        self.assertIn("--no-tests=fail", result.stdout)


@contextmanager
def _tmp_dir():
    with tempfile.TemporaryDirectory() as raw:
        yield Path(raw)


if __name__ == "__main__":
    unittest.main()
