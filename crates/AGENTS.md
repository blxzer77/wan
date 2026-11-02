# Crate Development

Supplements the root `AGENTS.md`.

## Dependency direction

`wan-types` is the lowest layer and contains protocol types only. The crate graph must remain acyclic.

- `wan-sandbox` and `wan-host` must never depend on `wan-vault` or `wan-core`.
- `wan-core` is a modular monolith and owns the port traits.
- `wan-acp` connects process-boundary integrations through ACP.
- `wand` is the binary and composition root.
- All other edges must follow `wan architecture design v1`; do not infer new edges. TODO: record the complete graph when M0 creates the crates.

## Protocol types

`wan-types` contains types only: no service logic, orchestration, or transport implementation. Rust is the sole source of truth for protocol types. Generated TypeScript is derived output and must never be edited by hand.

## Error handling

Each module defines its own error type with `thiserror`. Convert module errors to `ApiError` at API boundaries. Use `anyhow` only in binaries and tests; library crates define structured errors instead.

## Unsafe and sandbox code

Every `unsafe` block or unsafe abstraction must explain why it is needed, which invariant makes it safe, and where its safety boundary is. Sandbox-related code must explain the confinement requirement, the trusted inputs, and the reason the implementation belongs in that crate. Keep the explanation in an adjacent English comment or a linked developer document.

## External process boundaries

nyan remains a separate AGPL repository. Integrate it only through the ACP process boundary; do not copy, vendor, link, or reimplement nyan source in this repository.

## Tests

Keep unit tests beside the module they exercise. Put crate integration tests in that crate's `tests/` directory. Put cross-crate behavior tests with the crate that owns the behavior. TODO: define end-to-end test placement when M0 establishes the test harness.
