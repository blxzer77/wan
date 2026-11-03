#!/usr/bin/env python3
"""Check documented line budgets and report files without a defined budget."""

from __future__ import annotations

from pathlib import Path


ROOT = Path(__file__).parents[1]
ROOT_AGENTS_LIMIT = 150
SKIPPED_PARTS = {".git", "node_modules", "target", "vendor", ".claude"}


def tracked_documents() -> list[Path]:
    documents: list[Path] = []
    for pattern in ("AGENTS.md", "**/AGENTS.md", "**/SKILL.md"):
        for path in ROOT.glob(pattern):
            if not path.is_file() or any(part in SKIPPED_PARTS for part in path.relative_to(ROOT).parts):
                continue
            if path not in documents:
                documents.append(path)
    return sorted(documents)


def main() -> int:
    errors: list[str] = []
    unspecified: list[str] = []
    for path in tracked_documents():
        relative = path.relative_to(ROOT)
        line_count = len(path.read_text(encoding="utf-8").splitlines())
        if relative == Path("AGENTS.md"):
            if line_count > ROOT_AGENTS_LIMIT:
                errors.append(
                    f"{relative}: {line_count} lines exceeds the limit of {ROOT_AGENTS_LIMIT}"
                )
        else:
            unspecified.append(f"{relative}: {line_count} lines (no limit defined)")

    if errors:
        print("\n".join(errors))
        return 1
    print(f"doc budget check: OK (root AGENTS.md <= {ROOT_AGENTS_LIMIT} lines)")
    if unspecified:
        print("undetermined budgets:")
        print("\n".join(unspecified))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
