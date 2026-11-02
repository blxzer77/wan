#!/usr/bin/env python3
"""Check API error uniqueness, code ranges, and library anyhow usage."""

from __future__ import annotations

import json
import re
import subprocess
import sys

from error_registry import load_domain_ranges, load_error_definitions, load_protocol_constants

SNAKE_CASE = re.compile(r"^[a-z][a-z0-9]*(?:_[a-z0-9]+)*$")


def check_error_registry() -> list[str]:
    definitions = load_error_definitions()
    constants = load_protocol_constants()
    domain_ranges = load_domain_ranges()
    wan_start = constants["WAN_ERROR_START"]
    wan_end = constants["WAN_ERROR_END"]
    reserved_start = constants["WAN_RESERVED_ERROR_START"]
    reserved_end = constants["WAN_RESERVED_ERROR_END"]
    errors: list[str] = []
    names: set[str] = set()
    codes: set[int] = set()

    ordered_ranges = sorted(domain_ranges.values())
    for start, end in ordered_ranges:
        if end - start != 9:
            errors.append(f"error domain range is not ten codes wide: {start}..{end}")
    for (_, previous_end), (next_start, _) in zip(ordered_ranges, ordered_ranges[1:]):
        if next_start <= previous_end:
            errors.append(f"overlapping error domain ranges: {previous_end} and {next_start}")

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
        if not wan_start <= code <= wan_end:
            errors.append(f"error code outside wan range: {name}={code}")
        if reserved_start <= code <= reserved_end:
            errors.append(f"error code uses reserved range: {name}={code}")
        if domain not in domain_ranges:
            errors.append(f"unknown error domain: {domain}")
        else:
            start, end = domain_ranges[domain]
            if not start <= code <= end:
                errors.append(f"error code outside {domain} range: {name}={code}")
    return errors


def check_library_anyhow() -> list[str]:
    metadata = json.loads(
        subprocess.check_output(["cargo", "metadata", "--format-version", "1"], text=True)
    )
    package_names = {package["id"]: package["name"] for package in metadata["packages"]}
    nodes = {node["id"]: node for node in metadata["resolve"]["nodes"]}
    errors: list[str] = []

    for package in metadata["packages"]:
        if not any("lib" in target["kind"] for target in package["targets"]):
            continue
        root = package["id"]
        pending = [root]
        visited: set[str] = set()
        while pending:
            current = pending.pop()
            if current in visited:
                continue
            visited.add(current)
            if package_names.get(current) == "anyhow":
                errors.append(f"library crate {package['name']} transitively depends on anyhow")
                break
            for dependency in nodes.get(current, {}).get("deps", []):
                kinds = dependency.get("dep_kinds", [])
                if any(kind.get("kind") in (None, "normal", "build") for kind in kinds):
                    pending.append(dependency["pkg"])
    return errors


def main() -> int:
    try:
        errors = check_error_registry() + check_library_anyhow()
    except (KeyError, ValueError, subprocess.CalledProcessError) as error:
        print(f"error model check failed: {error}", file=sys.stderr)
        return 1
    if errors:
        print("\n".join(errors), file=sys.stderr)
        return 1
    print("error model check: OK")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
