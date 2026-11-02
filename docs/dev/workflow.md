# 编排流程（coding agent 自助）

> 计划的唯一入口。路线图见 [roadmap.md](roadmap.md)，任务见 [tasks/](tasks/)，设计见 [design/](design/)。

## 循环

1. 读根目录 `AGENTS.md`，再读 [roadmap.md](roadmap.md)。
2. 按 M0 → M5 顺序，在 tasks/ 中找**第一个**状态为 `todo` 且所有依赖都是 `done` 的任务。
3. 读该任务链接的设计章节，按任务里的 **agent brief** 执行：从最新 `origin/main` 拉分支、改代码、本地通过检查。
4. 在同一 PR 里把该任务状态改为 `done`（合并即生效）。
5. 向 `main` 开 PR：Conventional Commits 标题；正文四段 Motivation / Changes / Testing / AI Usage（用 wan-pr-description 技能）。
6. 按 brief 的 Report back 汇报，然后停下。Yuki 审查并 squash 合并；PR 未合并前不开始下一个任务。
7. 若上一个 PR 漏改状态，在下一个 PR 的第一处改动里补上。

## 规则

- **一个会话 / 一个 PR 只做一个任务**；PR 保持可审查的大小，超出就停下请 Yuki 拆分。
- **从不自己合并**，不推送到 main，不改分支保护。
- **不开工 `blocked` 任务**，也不为它写占位实现。
- **设计有歧义或缺失时问 Yuki**，不要自己发明；GB 没有文档的行为不做，不自加功能。
- **设计变更**（含偏离 GB）先写 ADR（wan-adr 技能），经 Yuki 批准合并后再改设计文档与任务。
- **一处一事**：设计只在 design/，状态只在 tasks/，不在别处复述。
