#!/usr/bin/env python3
"""Generate the checked-in API error reference from wan-types."""

from __future__ import annotations

from pathlib import Path

from error_registry import load_error_definitions


OUTPUT = Path(__file__).parents[1] / "docs" / "errors.md"


def main() -> int:
    definitions = load_error_definitions()
    rows = "\n".join(
        f"| `{definition['name']}` | `{definition['domain']}` | `{definition['code']}` |"
        for definition in definitions
    )
    document = f"""# API Errors

This file is generated from `crates/wan-types/src/error.rs`. Only error kinds with a documented design definition are listed.

## JSON-RPC codes

| Code | Meaning |
|---:|---|
| `-32700` | Parse error |
| `-32600` | Invalid request |
| `-32601` | Method not found |
| `-32602` | Invalid params |
| `-32603` | Internal error |
| `-32800` | Cancelled |

## wan error codes

Wan error codes use `-32000..-32089`; `-32090..-32099` are reserved.

| Kind | Domain | Code |
|---|---|---:|
{rows}
"""
    OUTPUT.write_text(document, encoding="utf-8")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
