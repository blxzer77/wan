# AGENTS.md

## Repository layout

- `crates/` — Rust workspace crates.
- `apps/` — application packages.
- `packages/` — shared packages.
- `vendor/` — vendored dependencies.
- `docs/user/` — user documentation.
- `docs/dev/adr/` — architecture decision records.
- `.github/workflows/` — GitHub Actions workflows.

## Project rules

- This is an Apache-2.0 project for personal, self-hosted use.
- `wan architecture design v1` is the sole authority for architecture decisions. TODO: add the canonical design document.
- Rust is the single source of truth for protocol types; generated TypeScript must not be edited by hand.
- Release artifacts must not include GPL or AGPL dependencies.
- README, CHANGELOG, AGENTS.md, code comments, and commit messages are written in English.
- `docs/user/` and `docs/dev/`, including ADRs, are written in Simplified Chinese.
- Use Conventional Commits for commit messages.
- Before committing, run `just ci`.

TODO: Define the remaining coding, testing, and release details.
