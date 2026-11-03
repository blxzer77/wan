# AGENTS.md

wan is an Apache-2.0, self-hosted personal agent that recreates Grok Bot for one person.

- **Plan:** start at [docs/dev/workflow.md](docs/dev/workflow.md); it tells you which task to pick and how to ship it.
- **Task groups per PR:** take the first `todo` task in [docs/dev/tasks/](docs/dev/tasks/) whose deps are `done`; group it with other small, logically related tasks whose deps are `done`; never start `blocked` tasks; never self-merge.
- **Design is fixed:** [docs/dev/design/architecture.md](docs/dev/design/architecture.md) is the single design authority; ask Yuki instead of inventing, and change it only via an approved ADR.

## Repository layout

- `crates/` — Rust crates.
- `crates/wan-types/` — protocol types and their sole source of truth.
- `crates/wan-sandbox/` — sandbox crate.
- `crates/wan-vault/` — vault crate.
- `crates/wan-host/` — host crate.
- `crates/wan-core/` — modular monolith and owner of port traits.
- `crates/wan-acp/` — ACP crate.
- `crates/wan-pages/` — pages crate.
- `crates/wand/` — binary and composition root.
- `apps/` — application packages. TODO: define applications in M1.
- `packages/` — shared TypeScript packages. TODO: define packages in M1.
- `vendor/` — vendored source; read-only under the [vendor policy](#vendored-source).
- `docs/user/` — Simplified Chinese user documentation.
- `docs/dev/` — Simplified Chinese developer documentation.
- `docs/dev/adr/` — Simplified Chinese architecture decision records.
- `.agents/skills/` — reusable agent workflows.
- `.claude/skills` — compatibility symlink to `.agents/skills`.
- `.github/workflows/` — GitHub Actions workflows. TODO: add CI in M1.

## Commands

Use `just` for project commands. The recipes do not exist yet:

- `just fmt` — TODO: define in M1.
- `just test` — TODO: define in M1.
- `just lint` — TODO: define in M1.
- `just ci` — TODO: define in M1.

## Secrets

Never commit secrets. Never print secrets in logs, command output, tests, issues, PRs, or review comments.

## Conventions

- **Language and docs.** Write AGENTS.md, SKILL.md, README files, code comments, and commit messages in English; write `docs/user/`, `docs/dev/`, and ADRs in Simplified Chinese. ([details](docs/AGENTS.md#language))
- **Crate dependencies.** Keep the dependency direction acyclic; `wan-types` is lowest, and `wan-sandbox` and `wan-host` never depend on `wan-vault` or `wan-core`. ([details](crates/AGENTS.md#dependency-direction))
- **Protocol types.** Rust is the sole source for protocol types; generated TypeScript is output and must never be edited by hand. ([details](crates/AGENTS.md#protocol-types))
- **Error handling.** Modules define `thiserror` errors, API boundaries use `ApiError`, and `anyhow` is limited to binaries and tests. ([details](crates/AGENTS.md#error-handling))
- **Unsafe and sandbox code.** Explain the reason, relied-on invariant, and safety boundary for every `unsafe` or sandbox-related implementation. ([details](crates/AGENTS.md#unsafe-and-sandbox-code))
- **Release licensing.** Published artifacts must not include GPL or AGPL dependencies. ([license details](NOTICE))
- **Vendored source.** Treat `vendor/` as read-only and update it only according to `vendor/UPSTREAM`; the file is TODO until vendoring begins. ([details](#vendored-source))
- **Nyan boundary.** Keep nyan in its separate AGPL repository and connect only through an ACP process boundary; never copy its code here. ([details](crates/AGENTS.md#external-process-boundaries))
- **Conventional Commits.** Use `type(scope): summary` with an imperative summary and no unrelated changes. ([details](https://www.conventionalcommits.org/en/v1.0.0/))

## Skills

- [`wan-pre-push-checks`](.agents/skills/wan-pre-push-checks/SKILL.md) — use before pushing or opening a PR to select the smallest relevant checks.
- [`wan-adr`](.agents/skills/wan-adr/SKILL.md) — use when recording a long-term decision that code and normal documentation cannot explain.

## Vendored source

`vendor/` is read-only. When vendoring begins, update it only by following the repository-level `vendor/UPSTREAM` instructions. TODO: add that file before the first vendored dependency.

## Editing these instructions

Keep each fact in one place and link to its owner instead of repeating it. Keep rules as short as clarity allows. A child `AGENTS.md` supplements this file and must not repeat root rules. Edit the real `AGENTS.md`; `CLAUDE.md` is only a compatibility symlink.
