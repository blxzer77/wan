#!/usr/bin/env python3
"""Render a stable HTML license report from license-checker JSON."""

from __future__ import annotations

import html
import json
from pathlib import Path
import sys
import tomllib


def package_name(package_key: str) -> tuple[str, str]:
    name, version = package_key.rsplit("@", 1)
    return name, version


def main() -> int:
    if len(sys.argv) != 4:
        print(
            "usage: generate-third-party-licenses.py CHECKER_JSON MANUAL_TOML OUTPUT_HTML",
            file=sys.stderr,
        )
        return 2

    checker_path, manual_path, output_path = map(Path, sys.argv[1:])
    packages = json.loads(checker_path.read_text(encoding="utf-8"))
    manual = tomllib.loads(manual_path.read_text(encoding="utf-8"))
    overrides = manual.get("packages", {})

    rows: list[str] = []
    for package_key in sorted(packages):
        package = packages[package_key]
        name, version = package_name(package_key)
        override = overrides.get(package_key, {})
        license_name = override.get("license", package.get("licenses", "UNKNOWN"))
        repository = override.get("repository", package.get("repository", ""))
        if isinstance(repository, dict):
            repository = repository.get("url", "")
        repository_html = (
            f'<a href="{html.escape(repository, quote=True)}">source</a>'
            if repository
            else ""
        )
        rows.append(
            "<tr>"
            f"<td>{html.escape(name)}</td>"
            f"<td>{html.escape(version)}</td>"
            f"<td>{html.escape(str(license_name))}</td>"
            f"<td>{repository_html}</td>"
            "</tr>"
        )

    body = "\n".join(rows) or '<tr><td colspan="4">No packages reported.</td></tr>'
    document = f'''<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <title>wan third-party licenses</title>
  <style>body {{ font-family: sans-serif; margin: 2rem; }} table {{ border-collapse: collapse; width: 100%; }} th, td {{ border: 1px solid #ccc; padding: .4rem; text-align: left; }}</style>
</head>
<body>
  <h1>wan third-party licenses</h1>
  <table>
    <thead><tr><th>Package</th><th>Version</th><th>License</th><th>Repository</th></tr></thead>
    <tbody>
{body}
    </tbody>
  </table>
</body>
</html>
'''
    output_path.write_text(document, encoding="utf-8")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
