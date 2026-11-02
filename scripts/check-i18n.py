#!/usr/bin/env python3
"""Check Fluent key/placeholder parity and generate a pseudo-locale."""

from __future__ import annotations

import argparse
from pathlib import Path
import re


ROOT = Path(__file__).parents[1]
LOCALES = ROOT / "locales"
KEY = re.compile(r"^(?P<key>[a-z][a-z0-9_-]*)\s*=\s*(?P<value>.*)$")
PLACEHOLDER = re.compile(r"\{\s*\$([A-Za-z][A-Za-z0-9_-]*)\s*\}")
TRANSLATIONS = str.maketrans({
    "a": "ɑ", "e": "ë", "i": "ï", "o": "ø", "u": "ü",
    "A": "Å", "E": "Ë", "I": "Ï", "O": "Ø", "U": "Ü",
})


def parse_locale(path: Path) -> dict[str, tuple[str, set[str]]]:
    messages: dict[str, tuple[str, set[str]]] = {}
    for line_number, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
        if not line.strip() or line.lstrip().startswith("#"):
            continue
        match = KEY.fullmatch(line)
        if match is None:
            raise ValueError(f"{path}:{line_number}: unsupported Fluent line")
        value = match.group("value")
        messages[match.group("key")] = (value, set(PLACEHOLDER.findall(value)))
    return messages


def pseudo_value(value: str) -> str:
    placeholders: list[str] = []

    def protect(match: re.Match[str]) -> str:
        placeholders.append(match.group(0))
        return f"\x00{len(placeholders) - 1}\x00"

    protected = PLACEHOLDER.sub(protect, value).translate(TRANSLATIONS)
    for index, placeholder in enumerate(placeholders):
        protected = protected.replace(f"\x00{index}\x00", placeholder)
    return f"⟦{protected}⟧"


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--pseudo-output", type=Path, required=True)
    args = parser.parse_args()

    english = parse_locale(LOCALES / "en.ftl")
    chinese = parse_locale(LOCALES / "zh-CN.ftl")
    errors: list[str] = []
    if set(english) != set(chinese):
        errors.append(f"locale key sets differ: en={sorted(english)}, zh-CN={sorted(chinese)}")
    for key in sorted(set(english) & set(chinese)):
        if english[key][1] != chinese[key][1]:
            errors.append(f"placeholder sets differ for {key}")

    if errors:
        print("\n".join(errors))
        return 1

    args.pseudo_output.parent.mkdir(parents=True, exist_ok=True)
    args.pseudo_output.write_text(
        "\n".join(f"{key} = {pseudo_value(english[key][0])}" for key in sorted(english)) + "\n",
        encoding="utf-8",
    )
    print(f"i18n check: OK ({len(english)} keys; pseudo locale generated)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
