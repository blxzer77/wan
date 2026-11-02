# API Errors

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
| `routine_rate_limited` | `Routine` | `-32020` |
