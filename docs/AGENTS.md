# Documentation

Supplements the root `AGENTS.md`.

## Document layers

| Location | Owns |
| --- | --- |
| Root `AGENTS.md` | Standing instructions needed in every repository session. |
| `docs/dev/` | Developer-facing design and implementation documentation. |
| `docs/dev/adr/` | Long-term architecture decisions and their trade-offs. |
| `docs/dev/design/`, `roadmap.md`, `tasks/`, `workflow.md` | Design (single authority), milestones, task status, orchestration loop — Simplified Chinese |
| `docs/user/` | Product-facing setup and usage documentation. |
| Crate `README.md` | The crate's purpose, public contract, usage, and limitations. |

Keep each fact in one home. Link to the owning document instead of copying its rule, contract, or explanation into another layer.

## Language

`AGENTS.md`, `SKILL.md`, `README.md`, code comments, and commit messages use English. Documents under `docs/user/` and `docs/dev/`, including ADRs, use Simplified Chinese. This file is an `AGENTS.md`, so it uses English.

## Anti-slop checks

Before submitting documentation, check that:

1. Rules are not repeated from another document.
2. The text does not add status labels or progress markers.
3. The text does not narrate the reasoning process or editing history.
4. Every sentence carries a concrete fact or instruction; remove filler.
5. Sentences stay short and direct.
