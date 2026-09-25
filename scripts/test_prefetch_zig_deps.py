from __future__ import annotations

import unittest

from scripts.prefetch_zig_deps import (
    Dependency,
    archive_suffix,
    git_source,
    parse_dependencies,
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


if __name__ == "__main__":
    unittest.main()
