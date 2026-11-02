#!/usr/bin/env python3
"""Verify the vendored dogdraw tree against its UPSTREAM manifest."""

from __future__ import annotations

import hashlib
from pathlib import Path
import sys


ROOT = Path(__file__).resolve().parents[1] / "vendor" / "dogdraw"


def load_manifest() -> dict[str, str]:
    manifest = ROOT / "UPSTREAM"
    hashes: dict[str, str] = {}
    in_hashes = False
    for line in manifest.read_text(encoding="utf-8").splitlines():
        if line == "SHA-256:":
            in_hashes = True
            continue
        if in_hashes and line:
            digest, relative_path = line.split(maxsplit=1)
            hashes[relative_path] = digest
    return hashes


def main() -> int:
    expected = load_manifest()
    actual_paths = {
        path.relative_to(ROOT).as_posix()
        for path in ROOT.rglob("*")
        if path.is_file() and path.name != "UPSTREAM"
    }
    expected_paths = set(expected)
    errors: list[str] = []

    for relative_path in sorted(expected_paths - actual_paths):
        errors.append(f"missing vendored file: {relative_path}")
    for relative_path in sorted(actual_paths - expected_paths):
        errors.append(f"unlisted vendored file: {relative_path}")

    for relative_path in sorted(expected_paths & actual_paths):
        path = ROOT / relative_path
        digest = hashlib.sha256(path.read_bytes()).hexdigest()
        if digest != expected[relative_path]:
            errors.append(
                f"hash mismatch: {relative_path}; expected {expected[relative_path]}, got {digest}"
            )

    if errors:
        print("\n".join(errors), file=sys.stderr)
        return 1

    print(f"vendor hash check: OK ({len(expected)} files)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
