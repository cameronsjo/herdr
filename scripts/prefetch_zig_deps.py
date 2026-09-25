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

Everything below the vendored tree is untrusted input: a package's own
build.zig.zon comes out of a downloaded archive. So a dependency is refused
unless its hash and URL have the expected shape (https only, no leading `-`,
no path characters in the hash), and a download is trusted, and its own
build.zig.zon read, only once `zig fetch` computes exactly the pinned hash.

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
# Zig package hashes: legacy multihash hex (1220...) or name-version-digest.
# No path separators, so a hash is always a single file name.
_SAFE_HASH = re.compile(r"[A-Za-z0-9][A-Za-z0-9_.+-]{0,199}")
_SAFE_REV = re.compile(r"[A-Za-z0-9][A-Za-z0-9_./-]{0,199}")


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


def unsafe_reason(dep: Dependency) -> str | None:
    """Why a dependency from an untrusted build.zig.zon must not be fetched,
    or None when its hash and URL are safe to hand to curl, git and paths."""
    if not _SAFE_HASH.fullmatch(dep.hash) or ".." in dep.hash:
        return "hash is not a plain Zig package hash"
    if not (dep.url.startswith("https://") or dep.url.startswith("git+https://")):
        return "only https:// and git+https:// sources are fetched"
    source = git_source(dep.url)
    if source is not None:
        _, rev = source
        if not _SAFE_REV.fullmatch(rev) or ".." in rev:
            return "git revision has an unexpected shape"
    return None


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
    try:
        env = subprocess.run([zig, "env"], check=True, capture_output=True, text=True).stdout
    except (OSError, subprocess.CalledProcessError) as err:
        raise SystemExit(
            f"cannot run `{zig} env` ({err}); install Zig 0.16.0 or set $ZIG to its binary"
        ) from None
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


def curl_download(url: str, target: Path) -> None:
    subprocess.run(
        [
            "curl",
            "-fsSL",
            "--proto",
            "=https",
            "--proto-redir",
            "=https",
            "--retry",
            "3",
            "-o",
            str(target),
            url,
        ],
        check=True,
        capture_output=True,
    )


def git_download(repo: str, rev: str, checkout: Path, target: Path) -> None:
    # https only, and `--` so neither argument can be read as an option.
    git = ["git", "-c", "protocol.allow=never", "-c", "protocol.https.allow=always"]
    subprocess.run([*git, "init", "-q", str(checkout)], check=True, capture_output=True)
    subprocess.run(
        [*git, "-C", str(checkout), "fetch", "-q", "--depth", "1", "--", repo, rev],
        check=True,
        capture_output=True,
    )
    subprocess.run(
        [*git, "-C", str(checkout), "archive", "--prefix=pkg/", "-o", str(target), "FETCH_HEAD"],
        check=True,
        capture_output=True,
    )


class FetchError(Exception):
    """A download that failed on every route, naming each route's own error."""


def download(dep: Dependency, workdir: Path) -> tuple[Path, str]:
    """Fetches a dependency whose hash and URL passed `unsafe_reason`.
    Returns the archive and the route that produced it ("curl" or "git")."""
    source = git_source(dep.url)
    errors = []
    if not dep.url.startswith("git+"):
        # A plain download first, GitHub archives included; git is the
        # fallback for a proxy that refuses archive downloads.
        target = workdir / f"{dep.hash}{archive_suffix(dep.url)}"
        try:
            curl_download(dep.url, target)
            return target, "curl"
        except subprocess.CalledProcessError as err:
            errors.append(f"curl: {process_output(err)}")
            if source is None:
                raise FetchError("; ".join(errors)) from None
    if source is None:
        raise FetchError("no download route")
    repo, rev = source
    target = workdir / f"{dep.hash}.git.tar.gz"
    try:
        git_download(repo, rev, workdir / f"{dep.hash}.git", target)
    except subprocess.CalledProcessError as err:
        step = next((arg for arg in err.cmd if arg in {"init", "fetch", "archive"}), "git")
        errors.append(f"git {step}: {process_output(err)}")
        raise FetchError("; then ".join(errors)) from None
    return target, "git"


def process_output(err: subprocess.CalledProcessError) -> str:
    """The failing command's own words, which `str(err)` leaves out: its
    last `error:`/`fatal:`/`curl:` line when it has one, else its last line."""
    output = err.stderr or err.stdout or b""
    text = output.decode(errors="replace") if isinstance(output, bytes) else output
    lines = [line.strip() for line in text.splitlines() if line.strip()]
    if not lines:
        return f"exit status {err.returncode}"
    marked = [
        line for line in lines if line.lower().startswith(("error", "fatal", "curl:"))
    ]
    return (marked or lines)[-1]


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
            reason = unsafe_reason(dep)
            if reason:
                failed[dep.hash] = f"{dep.url}: refused, {reason}"
                continue
            source = cached_archive(cache, dep.hash)
            if source is None:
                try:
                    archive, route = download(dep, workdir)
                except FetchError as err:
                    failed[dep.hash] = f"{dep.url}: {err}"
                    continue
                try:
                    computed = subprocess.run(
                        [zig, "fetch", str(archive)],
                        cwd=stub,
                        check=True,
                        capture_output=True,
                        text=True,
                    ).stdout.strip()
                except subprocess.CalledProcessError as err:
                    failed[dep.hash] = f"{dep.url}: zig fetch (via {route}): {process_output(err)}"
                    continue
                # Trust nothing inside a download until Zig's own content hash
                # matches the pinned one; a mismatch is a failure, not a fetch.
                if computed != dep.hash:
                    failed[dep.hash] = (
                        f"{dep.url}: expected {dep.hash}, zig computed "
                        f"{computed or 'no hash'} (via {route}); the mismatched copy "
                        "is cached under its own hash and never used"
                    )
                    continue
                source = cached_archive(cache, dep.hash)
                if source is None:
                    failed[dep.hash] = f"{dep.url}: zig fetch did not store {dep.hash}"
                    continue
                fetched += 1
                print(f"fetched {dep.hash}")
            zon = package_zon(source)
            if zon:
                queue.extend(parse_dependencies(zon))
    for digest, reason in failed.items():
        # URLs and stderr lines come from downloaded content; escape control
        # characters so a hostile one cannot rewrite the terminal.
        # The hash of a refused dependency is untrusted too, so escape the
        # whole line once.
        line = f"FAILED {digest} ({reason})"
        print(line.encode("unicode_escape", "backslashreplace").decode("ascii"), file=sys.stderr)
    print(f"{len(seen)} dependencies, {fetched} fetched, {len(failed)} failed; cache at {cache}")
    return 1 if failed else 0


if __name__ == "__main__":
    raise SystemExit(main())
