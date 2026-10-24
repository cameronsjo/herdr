"""Tests for .githooks/commit-msg, the conventional-subject hook.

Each test builds a throwaway repo with the real hook and checker installed
and two diverging branches, so `git merge` has to make a merge commit. Git
runs with background maintenance off, so it cannot race the temp-directory
cleanup.
"""

from __future__ import annotations

import os
import shutil
import subprocess
import tempfile
import unittest
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent
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


class CommitMsgHookTest(unittest.TestCase):
    def setUp(self) -> None:
        self._tmp = tempfile.TemporaryDirectory()
        self.repo = Path(os.path.realpath(self._tmp.name)) / "repo"
        self.git("init", "-q", "-b", "master", str(self.repo), cwd=self.repo.parent)
        (self.repo / ".githooks").mkdir()
        (self.repo / "scripts").mkdir()
        shutil.copy(REPO_ROOT / ".githooks" / "commit-msg", self.repo / ".githooks")
        shutil.copy(REPO_ROOT / "scripts" / "conventional_commits.py", self.repo / "scripts")
        (self.repo / ".githooks" / "commit-msg").chmod(0o755)
        self.git("config", "core.hooksPath", ".githooks")
        self.commit("a.txt", "base\n", "chore: base")
        self.git("checkout", "-q", "-b", "side")
        self.commit("b.txt", "side\n", "feat: side")
        self.git("checkout", "-q", "master")

    def tearDown(self) -> None:
        self._tmp.cleanup()

    def git(self, *args: str, cwd: Path | None = None, check: bool = True) -> subprocess.CompletedProcess[str]:
        return subprocess.run(
            [*GIT, *args], cwd=cwd or self.repo, env=GIT_ENV, check=check, capture_output=True, text=True
        )

    def commit(self, name: str, text: str, message: str, check: bool = True) -> subprocess.CompletedProcess[str]:
        (self.repo / name).write_text(text)
        self.git("add", name)
        return self.git("commit", "-q", "-m", message, check=check)

    def test_rejects_a_non_conventional_subject(self) -> None:
        result = self.commit("c.txt", "x\n", "bad subject", check=False)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("invalid commit subject", result.stdout + result.stderr)

    def test_accepts_a_clean_merge(self) -> None:
        self.commit("c.txt", "master\n", "fix: master")
        result = self.git("merge", "--no-edit", "side", check=False)
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertEqual(self.git("rev-list", "--count", "--merges", "HEAD").stdout.strip(), "1")

    def test_accepts_a_merge_committed_after_a_conflict(self) -> None:
        self.commit("b.txt", "master\n", "fix: master")
        self.assertNotEqual(self.git("merge", "--no-edit", "side", check=False).returncode, 0)
        (self.repo / "b.txt").write_text("resolved\n")
        self.git("add", "b.txt")
        result = self.git("commit", "--no-edit", check=False)
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertEqual(self.git("rev-list", "--count", "--merges", "HEAD").stdout.strip(), "1")


if __name__ == "__main__":
    unittest.main()
