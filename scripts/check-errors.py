#!/usr/bin/env python3
"""Check API error uniqueness, code ranges, and library anyhow usage."""

from __future__ import annotations

import json
import re
import subprocess
import sys

from error_registry import load_error_definitions

WAN_START = -32_089
WAN_END = -32_000
RESERVED_START = -32_099
RESERVED_END = -32_090
DOMAIN_RANGES = {"Routine": (-32_029, -32_020)}
SNAKE_CASE = re.compile(r"^[a-z][a-z0-9]*(?:_[a-z0-9]+)*$")


def check_error_registry() -> list[str]:
    definitions = load_error_definitions()
    errors: list[str] = []
    names: set[str] = set()
    codes: set[int] = set()
    for definition in definitions:
        name = str(definition["name"])
        code = int(definition["code"])
        domain = str(definition["domain"])
        if not SNAKE_CASE.fullmatch(name):
            errors.append(f"error kind is not snake_case: {name}")
        if name in names:
            errors.append(f"duplicate error kind: {name}")
        if code in codes:
            errors.append(f"duplicate error code: {code}")
        names.add(name)
        codes.add(code)
        if not WAN_START <= code <= WAN_END:
            errors.append(f"error code outside wan range: {name}={code}")
        if RESERVED_START <= code <= RESERVED_END:
            errors.append(f"error code uses reserved range: {name}={code}")
        if domain not in DOMAIN_RANGES:
            errors.append(f"unknown error domain: {domain}")
        else:
            start, end = DOMAIN_RANGES[domain]
            if not start <= code <= end:
                errors.append(f"error code outside {domain} range: {name}={code}")
    return errors


def check_library_anyhow() -> list[str]:
    metadata = json.loads(
        subprocess.check_output(["cargo", "metadata", "--format-version", "1"], text=True)
    )
    errors: list[str] = []
    for package in metadata["packages"]:
        if not any("lib" in target["kind"] for target in package["targets"]):
            continue
        for dependency in package["dependencies"]:
            if dependency["name"] == "anyhow" and dependency["kind"] != "dev":
                errors.append(f"library crate {package['name']} depends on anyhow")
    return errors


def main() -> int:
    errors = check_error_registry() + check_library_anyhow()
    if errors:
        print("\n".join(errors), file=sys.stderr)
        return 1
    print("error model check: OK")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
