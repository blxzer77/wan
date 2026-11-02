---
name: wan-pre-push-checks
description: Select the smallest relevant checks before pushing a wan branch or opening a pull request.
---

# wan Pre-Push Checks

Use this skill before pushing a branch or opening a pull request. Select evidence from the changed paths and run the smallest set that can catch a regression in that surface.

## Procedure

1. Confirm the branch and inspect the complete diff against its verified base.
2. Classify the changed surface: Rust, generated protocol types, TypeScript, documentation, vendored source, or governance.
3. Run only the narrowest available checks for that surface. Use `just` recipes when they exist.
4. Report only commands actually run and their results. Do not describe a TODO command as executed.

## Current command status

The repository has no `justfile` yet. `just fmt`, `just test`, `just lint`, and `just ci` are TODO until the workspace and M1 tooling exist. Do not invent a passing result for an unavailable command.

For governance-only changes in the current skeleton, `git diff --check` and `git diff --cached --check` are the available formatting checks; run them only when applicable.
