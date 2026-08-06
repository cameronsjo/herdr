#!/usr/bin/env python3
"""Compose release notes for a fork build.

The notes carry the artifact's sha256 so bumping the Homebrew formula in
cameronsjo/homebrew-tap is a copy, not a download-and-hash round trip.
"""

from __future__ import annotations

import argparse
import re
from pathlib import Path

SHA256_RE = re.compile(r"^sha256=(?P<digest>[0-9a-f]{64})\s", re.MULTILINE)
FIELD_RE = re.compile(r"^(?P<key>[a-z_]+)=(?P<value>.*)$", re.MULTILINE)

TEMPLATE = """\
Fork build of herdr carrying the command palette, built from `{commit}`.

Apple Silicon only — every consumer of this build runs arm64 macOS.

## Homebrew

`cameronsjo/homebrew-tap` `Formula/herdr.rb` needs `version` and the `on_arm`
block pointed here:

```ruby
version "{version}"
url "https://github.com/cameronsjo/herdr/releases/download/{tag}/{artifact}"
sha256 "{digest}"
```

## Build inputs

| field | value |
| --- | --- |
| target | `{target}` |
| libghostty-vt optimize | `{optimize}` |
| libghostty-vt SIMD | `{simd}` |
"""


def parse_build_info(text: str) -> dict[str, str]:
    fields = {m.group("key"): m.group("value").strip() for m in FIELD_RE.finditer(text)}
    digest = SHA256_RE.search(text)
    if not digest:
        raise SystemExit("BUILD_INFO.txt carries no sha256= line; refusing to publish notes without it")
    fields["digest"] = digest.group("digest")
    return fields


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--build-info", required=True, type=Path)
    parser.add_argument("--tag", required=True)
    parser.add_argument("--output", required=True, type=Path)
    parser.add_argument("--artifact", default="herdr-macos-aarch64")
    args = parser.parse_args()

    fields = parse_build_info(args.build_info.read_text(encoding="utf-8"))
    notes = TEMPLATE.format(
        commit=fields.get("commit", "unknown")[:12],
        version=args.tag.removeprefix("v"),
        tag=args.tag,
        artifact=args.artifact,
        digest=fields["digest"],
        target=fields.get("target", "unknown"),
        optimize=fields.get("libghostty_vt_optimize", "unknown"),
        simd=fields.get("libghostty_vt_simd", "unknown"),
    )
    args.output.write_text(notes, encoding="utf-8")
    print(notes)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
