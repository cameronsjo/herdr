from __future__ import annotations

import subprocess
import tempfile
import unittest
from pathlib import Path
from unittest import mock

import scripts.prefetch_zig_deps as prefetch

from scripts.prefetch_zig_deps import (
    Dependency,
    archive_suffix,
    git_source,
    parse_dependencies,
    process_output,
    unsafe_reason,
)


class ParseDependenciesTest(unittest.TestCase):
    def test_reads_url_and_hash_in_either_order_and_skips_path_deps(self) -> None:
        zon = """.{
            .name = .ghostty,
            .dependencies = .{
                .uucode = .{
                    .url = "https://deps.example/uucode.tar.gz",
                    .hash = "uucode-0.0.0-AAAA",
                    .lazy = true,
                },
                .zigimg = .{ .hash = "zigimg-0.1.0-BBBB", .url = "git+https://github.com/zigimg/zigimg#d695acd" },
                .local = .{ .path = "./pkg/local" },
            },
        }"""
        self.assertEqual(
            parse_dependencies(zon),
            [
                Dependency("https://deps.example/uucode.tar.gz", "uucode-0.0.0-AAAA"),
                Dependency("git+https://github.com/zigimg/zigimg#d695acd", "zigimg-0.1.0-BBBB"),
            ],
        )


class GitSourceTest(unittest.TestCase):
    def test_git_urls_drop_the_ref_query(self) -> None:
        self.assertEqual(
            git_source("git+https://github.com/ianprime0509/zig?ref=zig-gobject#08e338b"),
            ("https://github.com/ianprime0509/zig", "08e338b"),
        )

    def test_github_archives_go_through_git(self) -> None:
        self.assertEqual(
            git_source("https://github.com/vancluever/arocc/archive/f97cdfc3779aec4b.tar.gz"),
            ("https://github.com/vancluever/arocc", "f97cdfc3779aec4b"),
        )

    def test_other_downloads_stay_plain(self) -> None:
        self.assertIsNone(git_source("https://deps.files.ghostty.org/zlib-1220fed0.tar.gz"))


class ArchiveSuffixTest(unittest.TestCase):
    def test_keeps_the_download_format_zig_fetch_decodes_by(self) -> None:
        self.assertEqual(archive_suffix("https://x/harfbuzz-11.0.0.tar.xz"), ".tar.xz")
        self.assertEqual(archive_suffix("https://x/gobject-1.tar.zst"), ".tar.zst")
        self.assertEqual(archive_suffix("https://x/themes.tgz"), ".tgz")
        self.assertEqual(archive_suffix("https://x/pkg.tar.gz?download=1"), ".tar.gz")
        self.assertEqual(archive_suffix("https://x/no-extension"), ".tar.gz")


class ProcessOutputTest(unittest.TestCase):
    def test_prefers_the_error_line_over_trailing_notes(self) -> None:
        import subprocess

        err = subprocess.CalledProcessError(
            1, ["zig", "fetch"], stderr="error: hash mismatch\nnote: expected .hash = x\n"
        )
        self.assertEqual(process_output(err), "error: hash mismatch")
        located = subprocess.CalledProcessError(
            1, ["zig"], stderr="build.zig.zon:7:20: error: bad hash\nnote: see here\n"
        )
        self.assertEqual(process_output(located), "build.zig.zon:7:20: error: bad hash")
        quiet = subprocess.CalledProcessError(22, ["curl"], stderr="")
        self.assertEqual(process_output(quiet), "exit status 22")


class UnsafeReasonTest(unittest.TestCase):
    """A nested build.zig.zon comes out of a downloaded archive, so its url and
    hash reach curl, git and file paths only after these checks."""

    def test_accepts_the_shapes_the_vendored_tree_uses(self) -> None:
        for dep in [
            Dependency("https://deps.files.ghostty.org/zlib.tar.gz", "N-V-__8AAB0eQwD-0MdOEBmz7intriBR"),
            Dependency("https://x/pkg.tar.gz", "1220fed0c74e1019b3ee29edae2051788b080cd96e90d56836eea857b0b966742efb"),
            Dependency("git+https://github.com/zigimg/zigimg#d695acd97c02e57bb151e8f659d1280f5cd6ca70", "zigimg-0.1.0-lly-O6N2EABOxke8dqyzCwhtUCAafqP35zC7wsZ4Ddxj"),
        ]:
            self.assertIsNone(unsafe_reason(dep), dep)

    def test_refuses_option_injection_and_non_https_sources(self) -> None:
        for url in [
            "git+--upload-pack=touch PWNED#.",
            "git+https://github.com/a/b#--upload-pack=x",
            "file:///etc/passwd",
            "git+file:///tmp/repo#main",
            "http://deps.example/pkg.tar.gz",
            "-o/tmp/x",
        ]:
            self.assertIsNotNone(unsafe_reason(Dependency(url, "pkg-0.0.0-AAAA")), url)

    def test_refuses_hashes_that_would_leave_the_work_directory(self) -> None:
        for digest in ["../../../home/u/x", "/home/u/.config/foo", "a/b", "..", "-rf", ""]:
            self.assertIsNotNone(
                unsafe_reason(Dependency("https://x/pkg.tar.gz", digest)), digest
            )


class FailureReportingTest(unittest.TestCase):
    def test_a_failure_line_escapes_every_control_character(self) -> None:
        line = prefetch.format_failure(
            "\x1b[2Jhash", "https://x/\x1b]0;owned\x07: curl: \u202egnp"
        )
        self.assertTrue(line.isascii())
        self.assertNotIn("\x1b", line)
        self.assertNotIn("\x07", line)
        self.assertIn("\\x1b[2Jhash", line)

    def test_a_failed_curl_and_git_fallback_keep_both_errors_and_the_git_step(self) -> None:
        dep = Dependency("https://github.com/o/r/archive/abcdef1.tar.gz", "r-0.0.0-AAAA")
        curl_err = subprocess.CalledProcessError(22, ["curl"], stderr="curl: (22) 403 Forbidden\n")
        git_err = subprocess.CalledProcessError(
            128,
            ["git", "-c", "protocol.allow=never", "-C", "x", "fetch", "-q", "--", "r", "abcdef1"],
            stderr="fatal: unable to access\n",
        )
        with tempfile.TemporaryDirectory() as tmp, mock.patch.object(
            prefetch, "curl_download", side_effect=curl_err
        ), mock.patch.object(prefetch, "git_download", side_effect=git_err):
            with self.assertRaises(prefetch.FetchError) as caught:
                prefetch.download(dep, Path(tmp))
        message = str(caught.exception)
        self.assertIn("curl: curl: (22) 403 Forbidden", message)
        self.assertIn("then git fetch: fatal: unable to access", message)

    def test_a_missing_binary_is_a_fetch_error_not_a_traceback(self) -> None:
        dep = Dependency("https://deps.example/pkg.tar.gz", "pkg-0.0.0-AAAA")
        with tempfile.TemporaryDirectory() as tmp, mock.patch.object(
            prefetch, "curl_download", side_effect=FileNotFoundError(2, "No such file", "curl")
        ):
            with self.assertRaises(prefetch.FetchError) as caught:
                prefetch.download(dep, Path(tmp))
        self.assertIn("curl: No such file", str(caught.exception))


if __name__ == "__main__":
    unittest.main()
