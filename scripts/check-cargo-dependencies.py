#!/usr/bin/env python3
"""Check the workspace crate graph against the documented M0 rules."""

from __future__ import annotations

import json
import subprocess
import sys

EXPECTED: dict[str, set[str]] = {
    "wan-types": set(),
    "wan-sandbox": set(),
    "wan-vault": {"wan-types"},
    "wan-host": {"wan-types", "wan-sandbox"},
    "wan-core": {"wan-types"},
    "wan-acp": {"wan-types"},
    "wan-pages": {"wan-types"},
    "wand": {
        "wan-types",
        "wan-sandbox",
        "wan-vault",
        "wan-host",
        "wan-core",
        "wan-acp",
        "wan-pages",
    },
}


def load_graph() -> dict[str, set[str]]:
    raw = subprocess.check_output(
        ["cargo", "metadata", "--format-version", "1"], text=True
    )
    metadata = json.loads(raw)
    workspace_ids = set(metadata["workspace_members"])
    packages = {
        package["id"]: package["name"]
        for package in metadata["packages"]
        if package["id"] in workspace_ids
    }
    names = set(packages.values())
    if names != set(EXPECTED):
        missing = sorted(set(EXPECTED) - names)
        extra = sorted(names - set(EXPECTED))
        raise ValueError(f"workspace crates differ; missing={missing}, extra={extra}")

    graph: dict[str, set[str]] = {}
    for node in metadata["resolve"]["nodes"]:
        package_name = packages.get(node["id"])
        if package_name is None:
            continue
        graph[package_name] = {
            packages[dependency["pkg"]]
            for dependency in node["deps"]
            if dependency["pkg"] in packages
        }
    return graph


def check_cycles(graph: dict[str, set[str]]) -> None:
    visiting: set[str] = set()
    visited: set[str] = set()

    def visit(name: str) -> None:
        if name in visiting:
            raise ValueError(f"dependency cycle detected at {name}")
        if name in visited:
            return
        visiting.add(name)
        for dependency in graph[name]:
            visit(dependency)
        visiting.remove(name)
        visited.add(name)

    for name in graph:
        visit(name)


def main() -> int:
    try:
        graph = load_graph()
        for name, expected in EXPECTED.items():
            actual = graph[name]
            if actual != expected:
                raise ValueError(
                    f"{name} dependencies differ; expected={sorted(expected)}, "
                    f"actual={sorted(actual)}"
                )
        check_cycles(graph)
    except (OSError, subprocess.CalledProcessError, KeyError, TypeError, ValueError) as error:
        print(f"cargo dependency check failed: {error}", file=sys.stderr)
        return 1

    print("cargo dependency graph: OK")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
