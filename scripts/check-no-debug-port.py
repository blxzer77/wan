#!/usr/bin/env python3
"""Reject the TCP Chrome debugging-port flag in implementation and config files."""

from __future__ import annotations

import os
from pathlib import Path
import sys


IGNORED_DIRECTORIES = {".git", ".pnpm-store", "node_modules", "target"}
IGNORED_SUFFIXES = {".md"}


def main() -> int:
    root = Path(__file__).resolve().parents[1]
    forbidden = "--remote-debugging" + "-port"
    matches: list[str] = []

    for directory, subdirectories, filenames in os.walk(root):
        subdirectories[:] = [name for name in subdirectories if name not in IGNORED_DIRECTORIES]
        for filename in filenames:
            path = Path(directory) / filename
            if path.suffix.lower() in IGNORED_SUFFIXES:
                continue
            try:
                content = path.read_bytes()
            except OSError:
                continue
            if forbidden.encode() in content:
                matches.append(str(path.relative_to(root)))

    if matches:
        print(f"forbidden Chrome debugging port flag found in: {', '.join(sorted(matches))}", file=sys.stderr)
        return 1

    print("Chrome debugging port guard: OK")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
