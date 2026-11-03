---
name: wan-code-review
description: Review wan changes against the repository design, task scope, and engineering rules.
---

# wan Code Review

Review the complete diff against the verified base before commenting.

Check the changed surface against these owners:

- [Repository rules](../../../AGENTS.md)
- [Crate rules](../../../crates/AGENTS.md)
- [Documentation rules](../../../docs/AGENTS.md)
- [Architecture design](../../../docs/dev/design/architecture.md)
- [Task brief](../../../docs/dev/tasks/)

Review for:

1. Scope staying within the selected task group and its design links.
2. Dependency direction and generated-file ownership.
3. Structured error handling at API boundaries and no library-only `anyhow` usage.
4. Required security boundaries, especially sandbox and external-process boundaries.
5. Tests covering the task acceptance criteria.
6. Language and documentation conventions.

Report findings by severity with file and line references. Do not suggest behavior that the design does not define; ask Yuki when the design is missing or ambiguous.
