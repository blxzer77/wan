# wan 路线图

> 里程碑定义来自 [设计第 10 节](design/architecture.md#s10)（【Yuki】B12，2026-11-01：v1 = M0–M5，约 18 周）。本文件只写每个里程碑的目标、范围、退出条件与依赖；设计细节一律链接到 [design/](design/)，任务与状态在 [tasks/](tasks/)，执行流程见 [workflow.md](workflow.md)。
>
> 设计评审（5 轮）已全部关闭，结论已并入设计文档；不再需要阅读评审页。

## 总览

| 里程碑 | 名称 | 依赖 | 任务 |
|---|---|---|---|
| [M0](#m0) | 工程地基 | — | [tasks/m0.md](tasks/m0.md) |
| [M1](#m1) | 地基 | M0 | [tasks/m1.md](tasks/m1.md) |
| [M2](#m2) | 单 bot 可用 | M1 | [tasks/m2.md](tasks/m2.md) |
| [M3](#m3) | 电脑与工具 | M2 | [tasks/m3.md](tasks/m3.md) |
| [M4](#m4) | Routines（cron）与可靠性 | M3 | [tasks/m4.md](tasks/m4.md) |
| [M5](#m5) | 多 bot | M4 | [tasks/m5.md](tasks/m5.md) |

里程碑按顺序推进；任务级依赖以 tasks/ 中的「依赖」列为准，可以跨里程碑提前开工的只有依赖已全部 done 的任务。

**未设计、不得实现**（等待 Yuki 设计）：P1 内置系统提示词（[M1-17](tasks/m1.md#m1-17-p1-内置系统提示词)）、动态模型调度（[M1-18](tasks/m1.md#m1-18-动态模型调度)）、记忆与检索评测（[M2-08](tasks/m2.md#m2-08-记忆与检索评测)）。见 [设计第 13 节](design/architecture.md#s13)。

<a id="m0"></a>
## M0 工程地基

- **目标**：让后续每个 PR 都有可重复的构建、检查与生成物比对。
- **范围（内）**：治理（AGENTS.md、技能、ADR，已完成）；monorepo（Cargo + pnpm workspace、justfile）；CI 与供应链门禁、许可证生成；vendor/dogdraw；wan-types 方法登记表与错误模型；specta 类型生成；i18n 与文档骨架。对应 16、B2、B3、B4、N9、N7、B13，见 [4](design/architecture.md#s4)、[5.13](design/architecture.md#s5-13)、[6.4](design/architecture.md#s6-4)、[9](design/architecture.md#s9)。
- **范围（外）**：任何业务逻辑、存储、网络服务；发布签名（cosign / SLSA）。
- **退出条件**：`just ci` 在 CI 上为 main 必需检查；依赖图、GPL / AGPL、dogdraw 哈希、调试端口、生成物（TS、errors.md）、i18n、文档链接检查全部在 CI 中生效。
- **依赖**：无。

<a id="m1"></a>
## M1 地基

- **目标**：单进程守护进程能持久地跑回合：存储、事件、队列、步骤日志、模型调用、Vault、设置、备份都可用（无 UI）。
- **范围（内）**：M1 治理遗留（apps/AGENTS.md、wan-code-review 与 wan-pr-description 技能、文档字数预算门禁）；SQLite 与只进不退迁移、事件表与 seq、会话 actor 与队列、取消与超时、步骤日志、模型适配器（含 OAuth 登录）、协议版本握手、Vault 存储与密钥方案（含 CSV 导入）、设置、备份恢复。对应 1–6、12、36、38、N2、N5、B11，见 [5.1](design/architecture.md#s5-1)、[5.3](design/architecture.md#s5-3)、[5.6](design/architecture.md#s5-6)、[7.2–7.4](design/architecture.md#s7-2)。
- **范围（外）**：设备配对与鉴权（M2）、重试与熔断（M4）、系统提示词与动态模型调度（blocked）、Vault 填入与 Secure Form（v1.1）、Vault 导出与通用数据导出（不做）。
- **退出条件**：集成测试覆盖：崩溃后步骤重放、迁移、Vault 加解密与轮换、备份往返、四个模型适配器 record / replay；本机经 WS 完成 `system/initialize` 与一个只有模型调用的回合。
- **依赖**：M0。

<a id="m2"></a>
## M2 单 bot 可用

- **目标**：用户能在 Electron、PWA、CLI 上与一个 bot 聊天、审批、被提醒，bot 有记忆。
- **范围（内）**：Electron + PWA + wand CLI、配对、多设备同步、ntfy、审批管线、记忆与 memory 工具、FTS5 + jieba、Item / 卡片 / status_changed。对应 7、10、22、24、B8、N6，见 [4](design/architecture.md#s4)、[5.4](design/architecture.md#s5-4)、[5.7](design/architecture.md#s5-7)、[5.9](design/architecture.md#s5-9)、[6.2](design/architecture.md#s6-2)。
- **范围（外）**：语音（全部不做）、Web Push（v1.2）、macOS / Windows / 原生 App（v1.1+）、付款（v1.x）、记忆导入（v1 之后）、向量检索（v2）。
- **退出条件**：Playwright e2e：配对 → 聊天 → 批准卡片 → 记忆写入与检索；两台设备同步与审批竞争正确；离线 ntfy 推送。
- **依赖**：M1。

<a id="m3"></a>
## M3 电脑与工具

- **目标**：bot 能在沙箱电脑和用户服务器上做事，能用 MCP 与 GitHub，能把编码委派给 nyan。
- **范围（内）**：原生电脑 + bwrap、SSH、MCP 客户端、GitHub 连接器、附件管线、ACP 接 nyan、并发与资源锁。对应 8、9、11、37、N3，见 [3](design/architecture.md#s3)、[5.5](design/architecture.md#s5-5)、[5.8](design/architecture.md#s5-8)、[6.1](design/architecture.md#s6-1)。
- **范围（外）**：浏览器与 CDP（v1.1）、Google 连接器（v1.1）、macOS Seatbelt / Windows 沙箱（随 v1.1 平台）、Claude Code / Codex / Pi 接入（以后）、连接用户电脑（v3）。
- **退出条件**：Linux CI 沙箱测试通过（loopback 屏蔽、.git 只读、无沙箱拒绝）；经审批的 SSH 与 MCP 调用；nyan 经 ACP 完成一次编码 helper 任务；资源锁无死锁测试。
- **依赖**：M2。

<a id="m4"></a>
## M4 Routines（cron）与可靠性

- **目标**：无人值守的定时工作可靠运行，故障时诚实降级。
- **范围（内）**：routines、cron 与一次性 routine、补跑与暂停、限速、幂等、降级与 `wand doctor`、通知路由。对应 21、24、32、33、39、N4、B10，见 [5.2](design/architecture.md#s5-2)、[5.10](design/architecture.md#s5-10)、[7.1](design/architecture.md#s7-1)。
- **范围（外）**：事件触发（webhook / GitHub / 邮件 / 飞书）与 /hooks 入站端点（v1.2）。
- **退出条件**：fake clock 测试覆盖限额（50 / 20 / 300 / 200 / 5 分钟）、补跑窗口、14 天暂停、DST；故障注入测试覆盖重试次数与熔断；`wand doctor` 可运行。
- **依赖**：M3。

<a id="m5"></a>
## M5 多 bot

- **目标**：多个 bot 各有身份与记忆，能互发消息、在群聊里协作；Library 与图像生成可用。v1 完成。
- **范围（内）**：多 bot、侧栏、Library、群聊（helper + bot 互发消息）、图像生成。对应 14、18、26、27，见 [5.8](design/architecture.md#s5-8)、[5.11](design/architecture.md#s5-11)、[5.12](design/architecture.md#s5-12)。
- **范围（外）**：Pages（v1 之后）、动画头像与 bot 模板（v2）、视频生成（v1.x）、团队功能（不做）。
- **退出条件**：e2e：建第二个 bot、群聊中自动协调分派、bot 消息不能触发 T2+、图像结果进 Library。
- **依赖**：M4。

## v1 之后（不在本计划内，不建任务）

v1.1、v1.2、v1.x、v2、v3 的内容见 [设计第 10 节](design/architecture.md#s10)。开工前由 Yuki 拆任务；原生 App 技术路线须在 v1.1 开工前复核【Yuki】。

## 待 Yuki 决定（计划缺口）

设计没有把以下 v1 内容分配到里程碑，tasks/ 中暂未建任务：上下文压缩（决策 15，[5.1](design/architecture.md#s5-1)）、共享 skill 库与技能管理（[5.5](design/architecture.md#s5-5)）、飞书与 Office 技能预置（[5.5](design/architecture.md#s5-5)）、用量账本（[5.12](design/architecture.md#s5-12)）、后台「做梦」记忆整理（[5.4](design/architecture.md#s5-4)）、可选 GHCR 镜像与发布流程（cosign / SLSA / electron-builder，[8](design/architecture.md#s8)）。
