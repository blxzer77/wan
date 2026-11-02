from __future__ import annotations

import re
from pathlib import Path


ERROR_SOURCE = Path(__file__).parents[1] / "crates" / "wan-types" / "src" / "error.rs"
ERROR_DEFINITION = re.compile(
    r"ErrorDefinition::new\(\s*"
    r"ErrorKind::(?P<kind>[A-Za-z0-9_]+),\s*"
    r'"(?P<name>[^"]+)",\s*'
    r"ErrorDomain::(?P<domain>[A-Za-z0-9_]+),\s*"
    r"(?P<code>-?[0-9_]+),\s*\)",
    re.MULTILINE,
)
CONSTANT = re.compile(r"pub const (?P<name>[A-Z0-9_]+): i32 = (?P<value>-?[0-9_]+);")
DOMAIN_BRANCH = re.compile(r"Self::(?P<domain>[A-Za-z0-9_]+) => (?P<value>-?[0-9_]+),")


def source() -> str:
    return ERROR_SOURCE.read_text(encoding="utf-8")


def load_error_definitions() -> list[dict[str, str | int]]:
    return [
        {
            "kind": match.group("kind"),
            "name": match.group("name"),
            "domain": match.group("domain"),
            "code": int(match.group("code").replace("_", "")),
        }
        for match in ERROR_DEFINITION.finditer(source())
    ]


def load_protocol_constants() -> dict[str, int]:
    return {
        match.group("name"): int(match.group("value").replace("_", ""))
        for match in CONSTANT.finditer(source())
    }


def load_domain_ranges() -> dict[str, tuple[int, int]]:
    text = source()
    ranges: dict[str, tuple[int, int]] = {}
    for method in ("code_start", "code_end"):
        block = re.search(
            rf"pub const fn {method}\(self\).*?\{{(?P<body>.*?)\n    \}}",
            text,
            re.DOTALL,
        )
        if block is None:
            raise ValueError(f"could not find ErrorDomain::{method}")
        branches = {
            match.group("domain"): int(match.group("value").replace("_", ""))
            for match in DOMAIN_BRANCH.finditer(block.group("body"))
        }
        for domain, value in branches.items():
            start, end = ranges.get(domain, (value, value))
            ranges[domain] = (value, end) if method == "code_start" else (start, value)
    return ranges
