#!/usr/bin/env python3
"""Fill the Zig package cache for the vendored libghostty-vt build.

`build.rs` runs `zig build` inside `vendor/libghostty-vt`, and Zig fetches
every `build.zig.zon` dependency (and theirs) over the network. Behind a
restrictive proxy that fetch fails with an HTTP or git error from
`build.zig.zon`, and build.rs then reports only that the Zig build failed.

This script walks the dependency graph from the vendored build.zig.zon,
reading each package's own build.zig.zon so nested dependencies are covered.
A package already cached is skipped; a missing one is downloaded with `curl`
(or `git`, for `git+https` URLs and GitHub archives a proxy refuses) and handed
to `zig fetch`, which stores it under its content hash. Any dependency that
cannot be fetched makes the script exit non-zero.

Usage: python3 scripts/prefetch_zig_deps.py   (honours $ZIG, default `zig`)
"""

from __future__ import annotations

import os
import re
import subprocess
import sys
import tarfile
import tempfile
from dataclasses import dataclass
from pathlib import Path

PROJECT_ROOT = Path(__file__).resolve().parent.parent
VENDOR_ROOT = PROJECT_ROOT / "vendor" / "libghostty-vt"

_BLOCK = re.compile(r"\.\{([^{}]*)\}", re.S)
_URL = re.compile(r'\.url\s*=\s*"([^"]+)"')
_HASH = re.compile(r'\.hash\s*=\s*"([^"]+)"')
_GITHUB_ARCHIVE = re.compile(
    r"^https://github\.com/([^/]+/[^/]+)/archive/([0-9a-f]{7,40})\.tar\.gz$"
)


@dataclass(frozen=True)
class Dependency:
    url: str
    hash: str


def parse_dependencies(zon: str) -> list[Dependency]:
    """Every `.{ ... }` block in a build.zig.zon that names both a url and a hash."""
    found = []
    for block in _BLOCK.findall(zon):
        url = _URL.search(block)
        digest = _HASH.search(block)
        if url and digest:
            found.append(Dependency(url.group(1), digest.group(1)))
    return found


def git_source(url: str) -> tuple[str, str] | None:
    """(repository, revision) to fetch with git, or None for a plain download.

    `git+https://host/repo?ref=x#rev` is git by definition. A GitHub archive
    URL is also fetched through git: some proxies refuse archive downloads
    while allowing the git protocol, and the archive's content (hence its Zig
    hash) is the same tree.
    """
    if url.startswith("git+"):
        rest = url[len("git+") :]
        repo, _, rev = rest.partition("#")
        return repo.split("?", 1)[0], rev
    match = _GITHUB_ARCHIVE.match(url)
    if match:
        return f"https://github.com/{match.group(1)}", match.group(2)
    return None


_ARCHIVE_SUFFIXES = (".tar.gz", ".tar.xz", ".tar.zst", ".tgz", ".zip")


def archive_suffix(url: str) -> str:
    """The archive extension to save a download under. `zig fetch` picks its
    decoder from the file name, so an .xz or .zst saved as .tar.gz fails."""
    path = url.split("?", 1)[0].split("#", 1)[0].lower()
    return next((suffix for suffix in _ARCHIVE_SUFFIXES if path.endswith(suffix)), ".tar.gz")


def zig_binary() -> str:
    return os.environ.get("ZIG", "zig")


def global_cache_dir(zig: str) -> Path:
    override = os.environ.get("ZIG_GLOBAL_CACHE_DIR")
    if override:
        return Path(override)
    env = subprocess.run([zig, "env"], check=True, capture_output=True, text=True).stdout
    match = re.search(r'global_cache_dir"?\s*[=:]\s*"([^"]+)"', env)
    if not match:
        raise SystemExit(f"could not find global_cache_dir in `{zig} env` output")
    return Path(match.group(1))


PKG_DIR = VENDOR_ROOT / "zig-pkg"


def cached_archive(cache: Path, digest: str) -> Path | None:
    """The cached copy of a package, if any. Zig 0.16 stores `p/<hash>.tar.gz`
    and extracts it per project into `zig-pkg/<hash>`; older layouts used an
    extracted `p/<hash>` directory."""
    for candidate in (cache / "p" / f"{digest}.tar.gz", cache / "p" / digest, PKG_DIR / digest):
        if candidate.exists():
            return candidate
    return None


def package_zon(source: Path) -> str | None:
    """A package's own build.zig.zon, read from its directory or tarball."""
    if source.is_dir():
        zon = source / "build.zig.zon"
        return zon.read_text(errors="replace") if zon.exists() else None
    try:
        with tarfile.open(source) as archive:
            for member in archive:
                # The package root is the top directory (or the archive root).
                if member.isfile() and member.name.count("/") <= 1 and member.name.endswith(
                    "build.zig.zon"
                ):
                    data = archive.extractfile(member)
                    return data.read().decode(errors="replace") if data else None
    except (tarfile.TarError, OSError) as err:
        # Python's tarfile cannot read every format Zig can (.tar.zst before
        # 3.14). The package itself is cached; only its own dependencies go
        # unchecked, so say which.
        print(f"warning: cannot read {source.name} ({err}); its dependencies are not checked",
              file=sys.stderr)
    return None


def download(dep: Dependency, workdir: Path) -> Path:
    source = git_source(dep.url)
    # `git archive` below always writes gzip; a download keeps its own format.
    target = workdir / f"{dep.hash}{'.tar.gz' if source else archive_suffix(dep.url)}"
    if source is None:
        subprocess.run(
            ["curl", "-fsSL", "--retry", "3", "-o", str(target), dep.url], check=True
        )
        return target
    repo, rev = source
    checkout = workdir / f"{dep.hash}.git"
    subprocess.run(["git", "init", "-q", str(checkout)], check=True)
    subprocess.run(
        ["git", "-C", str(checkout), "fetch", "-q", "--depth", "1", repo, rev], check=True
    )
    subprocess.run(
        ["git", "-C", str(checkout), "archive", "--prefix=pkg/", "-o", str(target), "FETCH_HEAD"],
        check=True,
    )
    return target


def main() -> int:
    zig = zig_binary()
    cache = global_cache_dir(zig)
    failed: dict[str, str] = {}
    fetched = 0
    # Seed from every build.zig.zon in the vendored tree: ghostty's local
    # `.path` packages under pkg/ name URL dependencies of their own. The
    # extracted zig-pkg/ copies are reached through the walk instead.
    queue = [
        dep
        for zon in VENDOR_ROOT.rglob("build.zig.zon")
        if PKG_DIR not in zon.parents
        for dep in parse_dependencies(zon.read_text(errors="replace"))
    ]
    seen: set[str] = set()
    with tempfile.TemporaryDirectory(prefix="herdr-zig-prefetch-") as tmp:
        workdir = Path(tmp)
        # `zig fetch` must run inside a project; an empty build.zig is enough.
        stub = workdir / "stub"
        stub.mkdir()
        (stub / "build.zig").write_text(
            'const std = @import("std");\npub fn build(b: *std.Build) void { _ = b; }\n'
        )
        # Walk the dependency graph: each package's own build.zig.zon names
        # the next layer, whether the package was cached already or not.
        while queue:
            dep = queue.pop()
            if dep.hash in seen:
                continue
            seen.add(dep.hash)
            source = cached_archive(cache, dep.hash)
            if source is None:
                try:
                    source = download(dep, workdir)
                    subprocess.run(
                        [zig, "fetch", str(source)], cwd=stub, check=True, capture_output=True
                    )
                    fetched += 1
                    print(f"fetched {dep.hash}")
                except subprocess.CalledProcessError as err:
                    failed[dep.hash] = f"{dep.url}: {err}"
                    continue
            zon = package_zon(source)
            if zon:
                queue.extend(parse_dependencies(zon))
    for digest, reason in failed.items():
        print(f"FAILED {digest} ({reason})", file=sys.stderr)
    print(f"{len(seen)} dependencies, {fetched} fetched, {len(failed)} failed; cache at {cache}")
    return 1 if failed else 0


if __name__ == "__main__":
    raise SystemExit(main())
