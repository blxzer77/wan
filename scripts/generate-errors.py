#!/usr/bin/env python3
"""Generate the checked-in API error reference from wan-types."""

from __future__ import annotations

from pathlib import Path

from error_registry import load_error_definitions, load_protocol_constants


OUTPUT = Path(__file__).parents[1] / "docs" / "errors.md"


def main() -> int:
    definitions = load_error_definitions()
    constants = load_protocol_constants()
    json_rpc_codes = [
        ("JSON_RPC_PARSE_ERROR", "Parse error"),
        ("JSON_RPC_INVALID_REQUEST", "Invalid request"),
        ("JSON_RPC_METHOD_NOT_FOUND", "Method not found"),
        ("JSON_RPC_INVALID_PARAMS", "Invalid params"),
        ("JSON_RPC_INTERNAL_ERROR", "Internal error"),
        ("JSON_RPC_CANCELLED", "Cancelled"),
    ]
    json_rpc_rows = "\n".join(
        f"| `{constants[name]}` | {meaning} |" for name, meaning in json_rpc_codes
    )
    rows = "\n".join(
        f"| `{definition['name']}` | `{definition['domain']}` | `{definition['code']}` |"
        for definition in definitions
    )
    document = f"""# API Errors

This file is generated from `crates/wan-types/src/error.rs`. Only error kinds with a documented design definition are listed.

## JSON-RPC codes

| Code | Meaning |
|---:|---|
{json_rpc_rows}

## wan error codes

Wan error codes use `{constants["WAN_ERROR_END"]}..{constants["WAN_ERROR_START"]}`; `{constants["WAN_RESERVED_ERROR_END"]}..{constants["WAN_RESERVED_ERROR_START"]}` are reserved.

| Kind | Domain | Code |
|---|---|---:|
{rows}
"""
    OUTPUT.write_text(document, encoding="utf-8")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
