"""Tests for scripts/upstream-drift.sh, the fork's on-demand drift check.

Each test builds a throwaway upstream and fork (bare repos) plus a working
clone whose `origin` and `fork` remotes point at them, then runs the real
script there. Git runs with background maintenance off, so it cannot race the
temp-directory cleanup.
"""

from __future__ import annotations

import os
import subprocess
import tempfile
import unittest
from pathlib import Path

SCRIPT_PATH = Path(__file__).resolve().parent / "upstream-drift.sh"
GIT = ["git", "-c", "maintenance.auto=false", "-c", "gc.auto=0"]
GIT_ENV = {
    **os.environ,
    "GIT_AUTHOR_NAME": "t",
    "GIT_AUTHOR_EMAIL": "t@example.com",
    "GIT_COMMITTER_NAME": "t",
    "GIT_COMMITTER_EMAIL": "t@example.com",
    "GIT_CONFIG_GLOBAL": os.devnull,
    "GIT_CONFIG_NOSYSTEM": "1",
}


def git(cwd: Path, *args: str) -> str:
    return subprocess.run(
        [*GIT, *args], cwd=cwd, env=GIT_ENV, check=True, capture_output=True, text=True
    ).stdout.strip()


class UpstreamDriftTest(unittest.TestCase):
    def setUp(self) -> None:
        self._tmp = tempfile.TemporaryDirectory()
        root = Path(self._tmp.name)
        self.upstream = root / "upstream.git"
        self.fork = root / "fork.git"
        self.work = root / "work"
        for bare in (self.upstream, self.fork):
            git(root, "init", "-q", "--bare", "-b", "master", str(bare))
        git(root, "init", "-q", "-b", "master", str(self.work))
        git(self.work, "commit", "-q", "--allow-empty", "-m", "base")
        git(self.work, "remote", "add", "origin", str(self.upstream))
        git(self.work, "remote", "add", "fork", str(self.fork))
        git(self.work, "push", "-q", "origin", "master")
        git(self.work, "push", "-q", "fork", "master")

    def tearDown(self) -> None:
        self._tmp.cleanup()

    def run_drift(self) -> subprocess.CompletedProcess[str]:
        return subprocess.run(
            ["bash", str(SCRIPT_PATH)],
            cwd=self.work,
            env=GIT_ENV,
            capture_output=True,
            text=True,
        )

    def test_level_fork_with_no_tags_exits_0(self) -> None:
        result = self.run_drift()
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertIn("fork behind:  0 commits", result.stdout)
        self.assertIn("newest upstream tag: (none found", result.stdout)

    def test_only_palette_tags_reads_as_none_found(self) -> None:
        git(self.work, "tag", "v0.8.2-palette.3")
        result = self.run_drift()
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertIn("newest upstream tag: (none found", result.stdout)

    def test_behind_fork_names_newest_non_palette_tag_and_exits_1(self) -> None:
        git(self.work, "tag", "v0.8.1")
        git(self.work, "commit", "-q", "--allow-empty", "-m", "upstream work")
        git(self.work, "tag", "v0.8.2")
        git(self.work, "tag", "v0.8.2-palette.3")
        git(self.work, "push", "-q", "origin", "master")
        result = self.run_drift()
        self.assertEqual(result.returncode, 1, result.stdout + result.stderr)
        self.assertIn("fork behind:  1 commits", result.stdout)
        self.assertIn("newest upstream tag: v0.8.2\n", result.stdout)

    def test_missing_fork_remote_exits_2(self) -> None:
        git(self.work, "remote", "remove", "fork")
        result = self.run_drift()
        self.assertEqual(result.returncode, 2, result.stdout + result.stderr)
        self.assertIn("no remote named 'fork'", result.stderr)


if __name__ == "__main__":
    unittest.main()
