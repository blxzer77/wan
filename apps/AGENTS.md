# Application Development

Applications supplement the repository rules in [the root AGENTS.md](../AGENTS.md) and the documentation rules in [docs/AGENTS.md](../docs/AGENTS.md).

## Layout

- Put each application in its own package under `apps/`.
- `apps/desktop/` is the Electron application. Its platform scope and end-to-end test requirement are defined in [the architecture](../docs/dev/design/architecture.md#s5-9).
- Keep application code, package scripts, and application tests inside the owning package. Shared TypeScript code belongs in `packages/`.

## Protocol types

Rust is the source of truth for protocol types. The generated output in `packages/wan-protocol/` must not be edited by hand; update the Rust definitions and regenerate it instead. See [the crate protocol rules](../crates/AGENTS.md#protocol-types).

## Checks

Run these commands from the repository root:

- `pnpm --filter @wan/desktop typecheck` checks the desktop package.
- `pnpm exec tsc --noEmit` checks all TypeScript files included by the workspace configuration.
- Electron end-to-end tests use Playwright as required by [the architecture](../docs/dev/design/architecture.md#s5-9).
