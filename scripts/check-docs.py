#!/usr/bin/env python3
"""Check repository Markdown links, ADR indexing, and AGENTS.md coverage."""

from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path
import re
from urllib.parse import unquote, urlsplit


ROOT = Path(__file__).parents[1]
SKIPPED_PARTS = {".git", "node_modules", "target", "vendor"}
LINK = re.compile(r"(?<!!)[^\[]*\[[^\]]+\]\(\s*(?P<target><[^>]+>|[^)\s]+)")
HEADING = re.compile(r"^#{1,6}\s+(?P<text>.+?)\s*#*\s*$")
EXPLICIT_ID = re.compile(r'<a\s+(?:id|name)=["\'](?P<id>[^"\']+)["\']', re.IGNORECASE)
FENCE = re.compile(r"^\s*(`{3,}|~{3,})")
ADR_NAME = re.compile(r"^[0-9]{4}-.+\.md$")
ADR_LINK = re.compile(r"\]\((?P<target>[^)#]+)(?:#[^)]*)?\)")


@dataclass(frozen=True)
class MarkdownFile:
    path: Path
    lines: list[str]


def markdown_files() -> list[MarkdownFile]:
    files: list[MarkdownFile] = []
    for path in ROOT.rglob("*.md"):
        if not path.is_file() or any(part in SKIPPED_PARTS for part in path.relative_to(ROOT).parts):
            continue
        files.append(MarkdownFile(path, path.read_text(encoding="utf-8").splitlines()))
    return sorted(files, key=lambda item: item.path.as_posix())


def anchors(lines: list[str]) -> set[str]:
    found: set[str] = set()
    counts: dict[str, int] = {}
    in_fence = False
    for line in lines:
        fence = FENCE.match(line)
        if fence:
            in_fence = not in_fence
            continue
        if in_fence:
            continue

        for match in EXPLICIT_ID.finditer(line):
            found.add(match.group("id"))

        heading = HEADING.match(line)
        if heading:
            text = re.sub(r"[`*_~]", "", heading.group("text"))
            text = re.sub(r"<[^>]+>", "", text).lower()
            slug = re.sub(r"[^\w\s-]", "", text, flags=re.UNICODE)
            slug = re.sub(r"\s+", "-", slug.strip())
            if slug:
                occurrence = counts.get(slug, 0)
                counts[slug] = occurrence + 1
                found.add(slug if occurrence == 0 else f"{slug}-{occurrence}")
    return found


def is_external(target: str) -> bool:
    parsed = urlsplit(target)
    return bool(parsed.scheme or target.startswith("//"))


def check_links(files: list[MarkdownFile]) -> list[str]:
    errors: list[str] = []
    anchors_by_path = {item.path: anchors(item.lines) for item in files}

    for item in files:
        in_fence = False
        for line_number, line in enumerate(item.lines, 1):
            fence = FENCE.match(line)
            if fence:
                in_fence = not in_fence
                continue
            if in_fence:
                continue
            for match in LINK.finditer(line):
                target = match.group("target")
                if target.startswith("<") and target.endswith(">"):
                    target = target[1:-1]
                if is_external(target):
                    continue

                parsed = urlsplit(target)
                relative_path = unquote(parsed.path)
                destination = item.path.parent / relative_path if relative_path else item.path
                destination = destination.resolve()
                if not destination.is_relative_to(ROOT):
                    errors.append(f"{item.path.relative_to(ROOT)}:{line_number}: link escapes repository: {target}")
                    continue
                if not destination.exists():
                    errors.append(f"{item.path.relative_to(ROOT)}:{line_number}: missing link target: {target}")
                    continue
                if parsed.fragment and parsed.fragment not in anchors_by_path.get(destination, set()):
                    errors.append(
                        f"{item.path.relative_to(ROOT)}:{line_number}: missing anchor: {target}"
                    )
    return errors


def check_adr_index() -> list[str]:
    errors: list[str] = []
    directory = ROOT / "docs/dev/adr"
    index = directory / "README.md"
    if not index.is_file():
        return ["docs/dev/adr/README.md: missing ADR index"]

    index_text = index.read_text(encoding="utf-8")
    for path in sorted(directory.glob("*.md")):
        if path.name == "README.md":
            continue
        if not ADR_NAME.fullmatch(path.name):
            errors.append(f"docs/dev/adr/{path.name}: ADR filename must start with four digits")
        elif not any(
            match.group("target") == path.name for match in ADR_LINK.finditer(index_text)
        ):
            errors.append(f"docs/dev/adr/README.md: ADR is not indexed: {path.name}")
    return errors


def check_agents_files() -> list[str]:
    errors: list[str] = []
    for relative in ("AGENTS.md", "apps/AGENTS.md", "docs/AGENTS.md", "crates/AGENTS.md"):
        if not (ROOT / relative).is_file():
            errors.append(f"{relative}: required AGENTS.md is missing")
    return errors


def main() -> int:
    files = markdown_files()
    errors = check_links(files) + check_adr_index() + check_agents_files()
    if errors:
        print("\n".join(errors))
        return 1
    print(f"docs check: OK ({len(files)} Markdown files checked)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
