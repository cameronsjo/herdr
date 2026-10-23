"""Tests for the outside-the-repo guard in scripts/openspec.sh.

Each test copies the wrapper into a throwaway root whose pinned install is a
fake `openspec` that only records its arguments, so nothing is installed or
run for real. The fake install's lockfile is newer than the pin's, so the
wrapper skips `npm ci`.
"""

from __future__ import annotations

import os
import shutil
import subprocess
import tempfile
import unittest
from pathlib import Path

SCRIPT_PATH = Path(__file__).resolve().parent / "openspec.sh"


@unittest.skipUnless(os.name == "posix", "the fake CLI is a bash script")
class OpenSpecWrapperTest(unittest.TestCase):
    def setUp(self) -> None:
        self._tmp = tempfile.TemporaryDirectory()
        self.root = Path(self._tmp.name)
        scripts = self.root / "scripts"
        pin = scripts / "openspec"
        bin_dir = pin / "node_modules" / ".bin"
        bin_dir.mkdir(parents=True)
        shutil.copy(SCRIPT_PATH, scripts / "openspec.sh")
        (pin / "package-lock.json").write_text("{}\n")
        installed = pin / "node_modules" / ".package-lock.json"
        installed.write_text("{}\n")
        os.utime(pin / "package-lock.json", (1, 1))
        self.ran = self.root / "ran.txt"
        fake = bin_dir / "openspec"
        fake.write_text(f'#!/usr/bin/env bash\nprintf "%s\\n" "$*" > "{self.ran}"\n')
        fake.chmod(0o755)

    def tearDown(self) -> None:
        self._tmp.cleanup()

    def run_wrapper(self, *args: str, allow: bool = False) -> subprocess.CompletedProcess[str]:
        self.ran.unlink(missing_ok=True)
        env = {k: v for k, v in os.environ.items() if k != "OPENSPEC_ALLOW_OUTSIDE"}
        if allow:
            env["OPENSPEC_ALLOW_OUTSIDE"] = "1"
        return subprocess.run(
            ["bash", str(self.root / "scripts" / "openspec.sh"), *args],
            cwd=self.root,
            env=env,
            capture_output=True,
            text=True,
            check=False,
        )

    def test_store_writes_need_the_opt_in(self) -> None:
        for args in (
            ["store", "setup", "demo", "--path", "/elsewhere/demo"],
            ["store", "register", "/elsewhere/demo"],
            ["store", "unregister", "demo"],
            ["store", "remove", "demo", "--yes"],
            ["--no-color", "store", "--json", "remove", "demo"],
        ):
            with self.subTest(args=args):
                result = self.run_wrapper(*args)
                self.assertEqual(result.returncode, 2, result.stderr)
                self.assertIn("OPENSPEC_ALLOW_OUTSIDE=1", result.stderr)
                self.assertFalse(self.ran.exists())

    def test_store_reads_run(self) -> None:
        for args in (["store", "list"], ["store", "ls"], ["store", "doctor"], ["store", "--help"]):
            with self.subTest(args=args):
                result = self.run_wrapper(*args)
                self.assertEqual(result.returncode, 0, result.stderr)
                self.assertEqual(self.ran.read_text(), " ".join(args) + "\n")

    def test_store_option_needs_the_opt_in(self) -> None:
        for args in (
            ["new", "change", "x", "--store", "demo"],
            ["list", "--store=demo"],
        ):
            with self.subTest(args=args):
                result = self.run_wrapper(*args)
                self.assertEqual(result.returncode, 2, result.stderr)
                self.assertIn("OPENSPEC_ALLOW_OUTSIDE=1", result.stderr)
                self.assertFalse(self.ran.exists())
        result = self.run_wrapper("list", "--store", "demo", allow=True)
        self.assertEqual(result.returncode, 0, result.stderr)

    def test_opt_in_allows_a_store_write(self) -> None:
        result = self.run_wrapper("store", "remove", "demo", "--yes", allow=True)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(self.ran.read_text(), "store remove demo --yes\n")

    def test_feedback_still_needs_the_opt_in(self) -> None:
        result = self.run_wrapper("feedback", "hi")
        self.assertEqual(result.returncode, 2, result.stderr)
        self.assertFalse(self.ran.exists())


if __name__ == "__main__":
    unittest.main()
