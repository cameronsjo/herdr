"""Tests for scripts/sync-upstream.sh, the fork's manual upstream sync.

Each test builds a throwaway upstream (`origin`) and fork (`fork`) whose
masters both change one file, so the merge stops on a conflict. Upstream also
edits the two fork-owned files, README.md (which the fork edits too) and the
release workflow (which it does not). The real
script is copied into a working clone and run there with a fake `date` on
PATH, so a test can stop a sync on one day and resume it on the next. Git
runs with background maintenance off, so it cannot race the temp-directory
cleanup.
"""

from __future__ import annotations

import os
import re
import shutil
import subprocess
import tempfile
import unittest
from pathlib import Path

SCRIPT_PATH = Path(__file__).resolve().parent / "sync-upstream.sh"
CI_SCRIPT_PATH = Path(__file__).resolve().parent / "sync-upstream-ci.sh"
RELEASE = ".github/workflows/release.yml"
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


class SyncUpstreamTest(unittest.TestCase):
    def setUp(self) -> None:
        self._tmp = tempfile.TemporaryDirectory()
        root = Path(os.path.realpath(self._tmp.name))
        self.root = root
        upstream = root / "upstream.git"
        fork = root / "fork.git"
        seed = root / "seed"
        for bare in (upstream, fork):
            git(root, "init", "-q", "--bare", "-b", "master", str(bare))
        git(root, "init", "-q", "-b", "master", str(seed))
        (seed / ".github" / "workflows").mkdir(parents=True)
        (seed / "f.txt").write_text("base\n")
        (seed / "README.md").write_text("base readme\n")
        (seed / RELEASE).write_text("base release\n")
        git(seed, "add", ".")
        git(seed, "commit", "-q", "-m", "base")
        git(seed, "push", "-q", str(upstream), "master")
        (seed / "f.txt").write_text("upstream\n")
        (seed / "README.md").write_text("upstream readme\n")
        (seed / RELEASE).write_text("upstream release\n")
        git(seed, "commit", "-q", "-am", "upstream change")
        git(seed, "push", "-q", str(upstream), "master")
        git(seed, "reset", "-q", "--hard", "HEAD~1")
        (seed / "f.txt").write_text("fork\n")
        (seed / "README.md").write_text("fork notice\n")
        git(seed, "commit", "-q", "-am", "fork change")
        git(seed, "push", "-q", str(fork), "master")

        self.work = root / "herdr"
        git(root, "clone", "-q", "-o", "fork", str(fork), str(self.work))
        git(self.work, "remote", "add", "origin", str(upstream))
        (self.work / "scripts").mkdir()
        shutil.copy(SCRIPT_PATH, self.work / "scripts" / "sync-upstream.sh")

        self.bin = root / "bin"
        self.bin.mkdir()
        fake_date = self.bin / "date"
        fake_date.write_text('#!/bin/sh\necho "$FAKE_DAY"\n')
        fake_date.chmod(0o755)
        self.worktrees = root / "herdr-worktrees"

    def tearDown(self) -> None:
        self._tmp.cleanup()

    def run_sync(self, day: str) -> subprocess.CompletedProcess[str]:
        env = {**GIT_ENV, "FAKE_DAY": day, "PATH": f"{self.bin}:{GIT_ENV['PATH']}"}
        return subprocess.run(
            ["bash", "scripts/sync-upstream.sh"],
            cwd=self.work,
            env=env,
            capture_output=True,
            text=True,
        )

    def resolve(self, worktree: Path) -> None:
        (worktree / "f.txt").write_text("resolved\n")
        git(worktree, "add", "f.txt")

    def test_resume_on_a_later_day_finishes_the_first_merge(self) -> None:
        first = self.run_sync("20261022")
        self.assertEqual(first.returncode, 1, first.stdout + first.stderr)
        day_one = self.worktrees / "sync-upstream-20261022"
        self.resolve(day_one)

        second = self.run_sync("20261023")
        self.assertEqual(second.returncode, 0, second.stdout + second.stderr)
        self.assertIn("resuming sync-upstream-20261022", second.stdout)
        self.assertFalse((self.worktrees / "sync-upstream-20261023").exists())
        self.assertEqual((day_one / "f.txt").read_text(), "resolved\n")
        self.assertEqual(git(day_one, "rev-list", "--count", "--merges", "HEAD"), "1")

    def test_resume_on_the_same_day_still_finishes(self) -> None:
        self.assertEqual(self.run_sync("20261022").returncode, 1)
        self.resolve(self.worktrees / "sync-upstream-20261022")
        result = self.run_sync("20261022")
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertIn("completing the merge commit", result.stdout)

    def test_two_merges_in_progress_refuse(self) -> None:
        self.assertEqual(self.run_sync("20261022").returncode, 1)
        git(self.work, "worktree", "add", "-q", "-b", "sync-upstream-20261021",
            str(self.worktrees / "sync-upstream-20261021"), "fork/master")
        other = self.worktrees / "sync-upstream-20261021"
        subprocess.run([*GIT, "merge", "origin/master"], cwd=other, env=GIT_ENV,
                       capture_output=True)
        result = self.run_sync("20261023")
        self.assertEqual(result.returncode, 1, result.stdout + result.stderr)
        self.assertIn("more than one sync worktree has a merge in progress", result.stderr)

    def test_fork_owned_files_keep_the_forks_copy(self) -> None:
        first = self.run_sync("20261022")
        self.assertEqual(first.returncode, 1, first.stdout + first.stderr)
        worktree = self.worktrees / "sync-upstream-20261022"
        self.assertIn("f.txt", first.stdout)
        self.assertNotIn("README.md", first.stdout.split("Conflicts remain")[1])
        self.resolve(worktree)

        second = self.run_sync("20261022")
        self.assertEqual(second.returncode, 0, second.stdout + second.stderr)
        # README.md conflicted; the release workflow merged cleanly, which a
        # conflict-only rule would have let through.
        self.assertEqual(git(worktree, "show", "HEAD:README.md"), "fork notice")
        self.assertEqual(git(worktree, "show", f"HEAD:{RELEASE}"), "base release")
        self.assertEqual(git(worktree, "rev-list", "--count", "--merges", "HEAD"), "1")


class ForkOwnedParityTest(unittest.TestCase):
    def test_both_sync_scripts_own_the_same_files(self) -> None:
        def owned(path: Path) -> list[str]:
            block = re.search(r"^FORK_OWNED=\((.*?)^\)", path.read_text(), re.M | re.S)
            self.assertIsNotNone(block, path)
            assert block is not None
            return re.findall(r'^\s*"([^"]+)"', block.group(1), re.M)

        self.assertEqual(owned(SCRIPT_PATH), owned(CI_SCRIPT_PATH))
        self.assertIn(RELEASE, owned(SCRIPT_PATH))


if __name__ == "__main__":
    unittest.main()
