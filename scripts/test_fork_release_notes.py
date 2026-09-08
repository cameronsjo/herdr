"""Tests for the tag gate in scripts/fork_release_notes.py.

The palette segment being mandatory is a security property, not a style
preference: an upstream tag reaching this fork composes valid release notes if
the gate accepts it, and a bare v0.9.0 sorts above v0.9.0-palette.1 in
Homebrew's ordering, so the tap would pin a palette-less build and never
upgrade back. The failure is a silent downgrade, so it needs a test that goes
red rather than a comment.

These call the module directly; no network, no git, no GitHub.
"""

from __future__ import annotations

import unittest
from pathlib import Path

from scripts import fork_release_notes

BUILD_INFO = """\
Mach-O 64-bit executable arm64
commit=0123456789abcdef0123456789abcdef01234567
target=aarch64-apple-darwin
libghostty_vt_optimize=ReleaseSafe
libghostty_vt_simd=false
sha256=""" + ("a" * 64) + "  herdr-macos-aarch64\n"


class TagShapeTests(unittest.TestCase):
    def test_accepts_a_palette_tag(self) -> None:
        self.assertIsNotNone(fork_release_notes.TAG_RE.fullmatch("v0.9.0-palette.1"))
        self.assertIsNotNone(fork_release_notes.TAG_RE.fullmatch("v10.20.30-palette.42"))

    def test_rejects_a_palette_less_upstream_tag(self) -> None:
        # The regression this gate exists for. The fork's remote carries every
        # upstream tag, so these are the exact strings that can reach it.
        for tag in ("v0.9.0", "v0.8.2", "v1.0.0"):
            with self.subTest(tag=tag):
                self.assertIsNone(fork_release_notes.TAG_RE.fullmatch(tag))

    def test_rejects_the_dotless_historical_shape(self) -> None:
        # release.yml's header records an early tag as 0.8.0-palette1. That
        # shape is not accepted and must not be reintroduced.
        self.assertIsNone(fork_release_notes.TAG_RE.fullmatch("v0.8.0-palette1"))

    def test_rejects_shapes_that_could_escape_the_ruby_literal(self) -> None:
        # The tag is interpolated into a Ruby string in the release notes.
        for tag in ('v0.9.0-palette.1"', "v0.9.0-palette.1'; system('id')", "v0.9.0-palette.1\nx"):
            with self.subTest(tag=tag):
                self.assertIsNone(fork_release_notes.TAG_RE.fullmatch(tag))

    def test_rejects_a_prerelease_that_is_not_palette(self) -> None:
        for tag in ("v0.9.0-rc.1", "v0.9.0-palette.x", "v0.9.0-palette."):
            with self.subTest(tag=tag):
                self.assertIsNone(fork_release_notes.TAG_RE.fullmatch(tag))

    def test_rejects_a_trailing_newline(self) -> None:
        # Python's $ matches before a trailing newline; fullmatch does not.
        self.assertIsNone(fork_release_notes.TAG_RE.fullmatch("v0.9.0-palette.1\n"))

    def test_rejects_non_ascii_digits(self) -> None:
        # \d is Unicode-wide. These reach a Ruby string literal in the notes.
        self.assertIsNone(fork_release_notes.TAG_RE.fullmatch("v١.٢.٣-palette.1"))


class ComposeTests(unittest.TestCase):
    def _compose(self, tag: str, tmp: Path) -> str:
        build_info = tmp / "BUILD_INFO.txt"
        build_info.write_text(BUILD_INFO, encoding="utf-8")
        out = tmp / "RELEASE_NOTES.md"
        argv = [
            "--build-info", str(build_info),
            "--tag", tag,
            "--output", str(out),
        ]
        import sys

        saved = sys.argv
        sys.argv = ["fork_release_notes.py", *argv]
        try:
            fork_release_notes.main()
        finally:
            sys.argv = saved
        return out.read_text(encoding="utf-8")

    def test_refuses_a_palette_less_tag(self) -> None:
        import tempfile

        with tempfile.TemporaryDirectory() as d:
            with self.assertRaises(SystemExit) as caught:
                self._compose("v0.9.0", Path(d))
            self.assertIn("v0.9.0", str(caught.exception))
            self.assertIn("palette", str(caught.exception))

    def test_composes_for_a_palette_tag(self) -> None:
        import tempfile

        with tempfile.TemporaryDirectory() as d:
            notes = self._compose("v0.9.0-palette.1", Path(d))
        self.assertIn('version "0.9.0-palette.1"', notes)
        self.assertIn("releases/download/v0.9.0-palette.1/herdr-macos-aarch64", notes)
        self.assertIn("a" * 64, notes)

    def test_refuses_build_info_with_no_digest(self) -> None:
        import tempfile

        with tempfile.TemporaryDirectory() as d:
            tmp = Path(d)
            (tmp / "BUILD_INFO.txt").write_text("commit=abc\n", encoding="utf-8")
            with self.assertRaises(SystemExit):
                fork_release_notes.parse_build_info("commit=abc\n")


if __name__ == "__main__":
    unittest.main()
