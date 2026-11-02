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


def load_error_definitions() -> list[dict[str, str | int]]:
    source = ERROR_SOURCE.read_text(encoding="utf-8")
    return [
        {
            "kind": match.group("kind"),
            "name": match.group("name"),
            "domain": match.group("domain"),
            "code": int(match.group("code").replace("_", "")),
        }
        for match in ERROR_DEFINITION.finditer(source)
    ]
