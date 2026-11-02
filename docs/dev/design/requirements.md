# wan：私人 Agent 需求文档与开源方案对照

> 导出自 Page「wan：私人 Agent 需求文档与开源方案对照」（https://cursor.com/canvas/store-eb702648-1a45-447b-8323-ba94eaf2c492/f0f77b6d-7792-42da-a41b-33c0083c4824，2026-11-02 版本）。只记需求和开源对照；当前设计以 [architecture.md](architecture.md) 为准，冲突时以设计为准。导出时的修正见文末「导出说明」。

> 原摘要：wan（汪）是一个守护进程全 Rust（客户端与 Pages 用 TS）、自托管、多 bot 协作的通用私人助理，定位是复刻 Grok Bot：wan 只提供通用能力，编码任务委派给用户接入的外部 Coding Agent。

## 定位

> **总体定位（已定）** wan 就是复刻 Grok Bot：Grok Bot 有的功能就做，Grok Bot 没有的就不做。区别只在实现方式：给自己用、开源、守护进程全 Rust（客户端与 Pages 用 TS）、自托管。

wan（汪）是一个先给自己用、之后开源的私人助理平台，和 Yuki 的 Coding Agent nyan（喵）是一对：一个喵一个汪。wan 除写代码外什么都管。每个实例只服务一个用户，跑在自己的服务器上，用自己的模型 API key，守护进程全部用 Rust 写，客户端与 Pages 用 TS（客户端先用 Electron）。wan 只提供通用能力（电脑 / shell / SSH、连接器、routines、记忆、skills、审批），具体干什么由用户配置的 bot 决定，比如服务器管理、订阅管理、文件管理。多个 bot 共用一台“自己的电脑”，各有记忆，能互相发消息、在群聊里协作。编码任务通过可插拔接口，直接委派给用户接入的外部 Coding Agent（例如 Claude Code、Codex、Cursor CLI、OpenCode，或者 Yuki 自己的 nyan）。

## AI 友好（全局原则）

> 项目几乎全部由 AI Coding Agent 开发，所以 AI 友好是所有技术选型的第一考量

| 原则 | 具体做法 |
|---|---|
| 主流技术栈 | 选强类型、文档齐全、训练数据多的技术：Rust（tokio / axum / sqlx）和 TypeScript（Electron / PWA） |
| 少用语言 | 只用 Rust + TS 两种；Swift / Kotlin 只在必要的原生模块里用 |
| 显式 schema | 协议和 API 都有显式 schema：Rust 类型自动生成 TS 类型（specta 锁定版本 + CI 比对生成物，设计文档决策 16【Yuki】），HTTP 接口出 OpenAPI（utoipa），配置和工具参数出 JSON Schema（schemars） |
| Monorepo + 约定 | 【Yuki】2026-10-31 定稿：Cargo workspace（crates/*：7 个 lib + 1 个 bin `wand`）+ pnpm workspace（apps/desktop、packages/wan-protocol、packages/wan-page-kit）；Apache-2.0；nyan 是单独的 AGPL 仓库，只经 ACP 接入。根目录和各模块都放 `AGENTS.md`，写清架构、命名、怎么跑测试。详见 [设计：仓库、crate 与技术栈](architecture.md#s9) |
| 快速、可验证的反馈 | `cargo nextest`、`cargo clippy -D warnings`、`tsc --noEmit`、lint、端到端测试；CI 和本地用同一条命令 |
| 小模块 | 拆小的是 crate 内的模块（crate 本身按「厚单体 + 薄边界」只有 8 个），边界清楚，一个 agent 一次就能读完一个模块；用 trait / 接口隔开，方便打桩测试 |

## 需求清单

> 第三列已对齐设计文档路线图（B12，2026-11-02）：P0 / v1 = M0–M5；另有 v1.1、v1.2、之后（v1.x / v2 / v3）、不做

| 领域 | 需求 | 优先级 |
|---|---|---|
| 非功能 | AI 友好：主流强类型技术栈、少用语言、显式 schema、monorepo + AGENTS.md、强测试和快速反馈、小模块（见上节） | P0（全局） |
| 非功能 | 守护进程全部用 Rust 实现：后端、核心、运行时、工具；性能优先 | P0（硬性） |
| 非功能 | 守护进程纯 Rust：不依赖 Python / Node / Deno 运行时；非 Rust 项目只当设计参考【Yuki】 | P0（硬性） |
| 非功能 | 非 Rust 部分：Electron 客户端（自带 Node，可以）、Pages（TSX，类型检查用随发行包附带的 TS7 原生 tsc 二进制）、Claude Code / Codex 的 ACP 适配器（外部进程）；见设计文档决策 23【Yuki】 | P0 |
| 非功能 | 自托管；单用户实例；开源，许可证 Apache-2.0 | P0 |
| 非功能 | 升级与版本【Yuki】：schema 版本存在 DB 里；升级前自动备份，再按序跑只进不退的迁移（不支持降级），失败则恢复备份并拒绝启动；客户端 / 服务端握手互换协议版本，主版本不同拒绝连接并提示升级，次版本只加字段、互相兼容（设计文档决策 38） | P0（v1） |
| 非功能 | 模型接入【Yuki】：按协议做、不按提供商做——四个协议适配器 OpenAI Responses、Chat Completions、Anthropic Messages、System One；接入方式只有 API key 和 OAuth 登录两种；OAuth 登录按提供商，照抄 CLIProxyAPI 的实现用 Rust 重写成可插拔登录模块；CPA 不是默认（设计文档决策 12）；用户自己配各端点的 base URL / 凭据 / 模型列表；按 bot 选默认模型，审核模型（Jev，可选）、推理模型、生图模型分开配置；默认流式；若用户自己填了兼容网关（如 CLIProxyAPI / CPA，可选、非默认、不是依赖），容忍它在非流式响应外面包的一层 {"data": ...} | P0 |
| 非功能 | 不专门支持本地模型（Grok Bot 没有这个功能）；需要时通过兼容 Responses 的 base URL 或网关接入 | — |
| 非功能 | 远程访问（【Yuki】B8）：Tailscale serve，或局域网 + 自签名证书；设备配对（8 位码 + 二维码，10 分钟、一次性）、长期 token 可吊销、登录限速；不做公网反代（设计文档决策 17 已被 B8 取代） | P0 |
| 定位 | 定时任务 / routines（cron、带日期的一次性 routine【GB】） | P0（v1 / M4） |
| 定位 | 杂事处理、想法收集整理 | P0 |
| 定位 | 不做专门的资讯功能（对齐 Grok Bot）：用户在 bot 里描述要什么，bot 自己建 routine 抓取；wan 保证网页搜索 / 抓取、RSS 等连接器和 routines 这些通用能力（X 连接器暂缓，设计文档决策 11） | P0 |
| Coding Agent | 可插拔的 Coding Agent 接口，编码任务直接委派过去，wan 自己不写代码 | P0 |
| Coding Agent | 第一个接入的 ACP Coding Agent：nyan（Yuki 自己的） | P0 |
| Coding Agent | 后续接入 Claude Code、Codex、Pi，走同一个可插拔接口；用户可以配置多个 | P1 |
| Coding Agent | 与 nyan 生态深度集成（通用 ACP 接口仍是其他 agent 的主接口） | P2 |
| 客户端 | Electron（TS）桌面端，Fedora 优先，瘦客户端 | P0 |
| 客户端 | 原生手机 App（iOS + Android）；v1 的手机入口是 PWA（B8） | v1.1+ |
| 客户端 | 手机端逐步换成 SwiftUI / Compose 界面 + 共享 Rust 核心 | P2 |
| 客户端 | 条件允许时用 Rust 重写客户端 | P2 |
| 多 bot | 多个角色 bot，各自角色与记忆，由用户配置 | P0 |
| 多 bot | 侧边栏置顶 / 分组 / 隐藏；跨对话搜索 | P0 |
| 多 bot | bot 之间互发消息、协作 / 委派 | P0（v1） |
| 多 bot | 多 bot 群聊（Grok Bot 式自动协调，可指定固定协调者；约 6 bot/群、约 50 bot/账号） | P0（v1） |
| 多 bot | 开箱只有一个默认 bot，不预装模板（对齐 Grok Bot）；servers manager 等只作为文档示例 | P0 |
| 电脑 | Computer trait：执行命令、文件、浏览器、截图 | P0（命令 / 文件 v1；浏览器与截图 v1.1） |
| 电脑 | 后端 1：原生安装 + bwrap（主路径，Claude Code srt 式）；GHCR 镜像可选，只给无头 VPS 与 CI，隔离更弱（【Yuki】B6） | P0 |
| 电脑 | 无电脑档：只调 API | P0 |
| 电脑 | 所有 bot 共享一台电脑：文件系统、工具、浏览器登录；每个 bot 一个桌面 | 共享电脑 v1（浏览器登录 v1.1）；每 bot 一块桌面 v2 |
| 电脑 | 本地（单 PC）部署模式：守护进程跑在用户电脑上并驱动独立 Chrome；v1 只做服务器模式，但模块接口按两种模式设计【Yuki】 | P1（v1.x，决策 29） |
| 安全 | 沙箱（决策 37 修订【Yuki】）：出口全放行同 Grok Bot，但屏蔽 loopback；Linux/WSL2 复用 Codex linux-sandbox（bubblewrap + no_new_privs + seccomp，.git 只读）；macOS Seatbelt；Windows 低权限专用用户 + 防火墙 + ACL；GHCR 镜像可选；CDP 只用 pipe；沙箱策略与审批策略分层；无沙箱则拒绝命令（B10） | P0（v1） |
| 电脑 | 把用户自己的电脑当远程机器连接（Grok Bot 的「连接用户电脑」） | P2（v3） |
| 电脑 | 后端 3：托管云沙箱（E2B / Daytona / Modal），只留接口给社区实现 | P2 |
| 通用工具 | SSH 访问远程主机（可配主机与凭据范围） | P0 |
| 通用工具 | 推送（不买 Apple 开发者账号）：在线走 WebSocket，离线走 ntfy（默认不带内容）；PWA Web Push 在 v1.2（【Yuki】B8、B12） | P0 |
| 通用工具 | 原生 APNs 推送，以后视情况 | P2 |
| 通用工具 | 按 bot 用规则限定能用的工具和主机（用来划数据边界；bot 之间不是安全边界） | P0 |
| 连接器 | MCP 连接器（GitHub、飞书、用户自接 MCP）；Gmail / 日历 / Drive 走自写 Google 连接器；Slack 在 v1.1+ | P0（v1）；Google v1.1 |
| 记忆 | 自动写入，不需要用户确认，写入时简短告知；在聊天里查看、纠正、删除（不做记忆界面；skills 同样在聊天里管理） | P0 |
| 记忆 | 从其他 AI 助手导入记忆（不做导出；也不做通用数据导出，备份异地用 restic【Yuki】） | 之后（v1 之后，决策 28） |
| 记忆 | bot 自己的 profile（常驻上下文） | P0 |
| 记忆 | 共享用户记忆：所有 bot 可见，带来源和日期 | P0 |
| 记忆 | journal：按日期记录，最近几条进上下文 | P0 |
| 记忆 | 召回检索旧记忆：SQLite FTS5 + jieba + grep（v1）；向量 fastembed + sqlite-vec（v2）；见设计文档决策 7 | P0（v1 关键词） |
| 记忆 | skills：全局共享的可复用流程 | P0 |
| 记忆 | Pages：Library 里可以反复迭代的报告和看板 | Library v1（M5）；Pages 之后 |
| 文件 | 不做知识库（Grok Bot 也没有）：bot 的产出就是共享电脑上的普通文件 | P0 |
| 知识 | Markdown 知识目录（git 同步、索引）：可选，后续再开发 | P2 |
| Vault | 加密单独存放凭据、账号密码、卡、地址等联系方式，绝不写入记忆；bot 按需调用填入，用户可以打开编辑；不做导出【Yuki】 | P0（存储 v1；填入 / Secure Form v1.1） |
| Vault | Google / Apple 密码 CSV 本地导入（两者没有第三方 API）；1Password / Bitwarden 直连以后 | P0（v1 CSV） |
| 购物 | 购物与支付：完全照 Grok Bot，Vault 已存卡付款不需要确认、不加额外护栏【GB】【Yuki】；付款走同一审批管线，用户可加 Ask first 规则；一次性卡每笔确认；推迟到 v1.x【Yuki】（v1 只做浏览、Vault 登录填入、表单填写） | P1（v1.x） |
| Vault | 密钥方案（设计文档决策 36，2026-11-02 修订【Yuki】）：主密钥包裹 DEK；条目直接用 DEK 加密（XChaCha20-Poly1305，24 字节随机 nonce，AAD 含 item_id、字段名、origin）；解锁只用 OS 钥匙串或密钥文件 + 恢复密钥（无口令模式）；恢复密钥只显示一次且必须确认已保存；备份不含主密钥；不做 Vault 导出 | P0（v1） |
| 事件 | 事件触发入站：单独的仅入站端点（只暴露 /hooks，与控制 API 分离）+ 隧道部署文档 + 无公网时轮询后备（Gmail history.list、GitHub API）【Yuki】；通用 webhook 用 Bearer key（只存哈希、不去重），GitHub 校验 X-Hub-Signature-256 并按 X-GitHub-Delivery 去重 | v1.2 |
| 用量 | v1：基于 wan 自己记录的调用做完整用量与成本统计【Yuki】 | P0（v1） |
| 用量 | 订阅剩余额度：OAuth 登录各订阅拉取；Yuki 接受可能违反部分厂商条款的风险；推迟到 v1.x【Yuki】 | P1（v1.x） |
| 工具 | 图像生成：可选内置工具，用户自己配置服务 URL 和 API key【Yuki】 | P0（v1） |
| 工具 | 视频生成（同样用户配 URL + API key）【Yuki】 | P1（v1.x） |
| 非功能 | 完整的模型用量与成本统计：按 bot、routine、模型、日期统计 token 和花费，带预算规则（N3） | 账本与预算 v1；用量页 v1.2 |
| 客户端 | 手机能力（日历、通讯录、位置、提醒）开放给 bot，随原生手机 App | 之后（随原生 App，v1.1+） |
| 连接器 | bot 专属邮箱：可选连接器，核心不内置 | P2 |
| 多 bot | bot 模板（导出 / 分享）：做不做未定，放 v2【Yuki】 | P1（v2，未定） |
| 安全 | 高风险或不可逆操作前审批（发消息、删除、付款、改服务器） | P0 |
| 安全 | 操作日志 / 审计，支持写回滚步骤 | P0 |
| 安全 | 密钥和凭据只放 Vault，不进模型上下文 | P0 |

## Grok Bot 功能对照

> 调研日期 2026-10-29。只列用户能看到的功能。来源：官方文档 docs.x.ai/grok-bot、官方 changelog、发布博客、第三方评测，以及 Grok Bot 作为产品本身的第一手介绍；链接见文末“参考链接”

| 功能 | Grok Bot 怎么做（要点） | 我们的决定 | 备注 |
|---|---|---|---|
| 多个 bot | 每个 bot 有自己的名字、头像、标签（角色）和独立对话；有一个主 bot；侧边栏支持置顶、分组（桌面和手机同步）、隐藏（隐藏后 routine 照常运行）；iPhone 上可以复制 bot | v1 | 已定：侧边栏置顶 / 分组 / 隐藏都进 v1。头像（更新 2026-10-30【Yuki】）：可爱的动画小狗，多品种，动作表示六种状态，尽量做好，放 v2。见设计文档决策 35 |
| bot 互发消息、群聊 | bot 之间互发消息、移交任务；群聊里可以 @ 某个 bot，也可以禁止 bot 互聊；支持线程和表情回应 | v1 | 已定（2026-10-30【Yuki】）：1:1 复刻 Grok Bot 群聊，含自动协调 / 分派，留在 v1；默认照 Grok Bot 自动协调（协调 bot 做常规分派，需要判断时拉用户进来），可指定固定协调者；上限约 6 bot/群、约 50 bot/账号。见设计文档决策 18 |
| 记忆 | 自动保存，不先问，通常在回复里提一句；每个 bot 各自记忆，名字、时区等少量用户事实所有 bot 共享；按日期的工作日志；个人 bot 没有记忆界面，在聊天里问、改、删；能从其他 AI 助手导入记忆；官方强调记忆不是事实来源 | v1（导入在 v1 之后） | 已定：自动写入 + 简短告知；和 Grok Bot 一样不做记忆界面，在聊天里查看 / 纠正 / 删除；从其他助手导入记忆在 v1 之后（B12）；不做导出 |
| skills | 所有 bot 共用一个私有 skill 库，另有 Connect apps 里的官方打包 skills；团队可以发布 skill | v1 | 已定：和 Grok Bot 一样在聊天里管理，不做单独界面；团队发布 skill：不做（仅个人） |
| 示范学习 | Teach a task：录下最长 10 分钟的浏览器操作，生成一个 skill 草稿，再由用户补充规则后定时运行（逐步开放） | 不做 | 已定：使用场景以服务器 / CLI / API 为主 |
| Pages | Library 里的报告、备忘、看板，可以一起编辑，或让 bot 持续更新；可置顶；能生成分享链接；还能做幻灯片（导出 PPTX / Google Slides） | Library v1；Pages v1 之后 | 已定：Pages 保留；更新（2026-10-30）：照 Grok Bot 画布做组件化 TSX。【GB】默认私有、可分享、控件状态不持久、只有 Bot 能编辑（https://cursor.com/help/grok-bot/pages）；Library：Today / This week / Older、Pin、已分享（https://docs.x.ai/grok-bot/files-and-results）；只允许脚本的 iframe、单个 TSX 已有 GB 引用（A20）。幻灯片 v1 不做。见设计文档决策 23、26、27 |
| Routines | 按 cron 定时（间隔至少 5 分钟），或由事件触发：Slack、GitHub、Linear、Sentry、PagerDuty、bot 邮箱、webhook；关机后照常无人值守运行；可以暂停、测试、编辑、删除，并有运行记录；每个 bot 最多 50 个、每个 routine 保留最近 20 条运行记录；每小时总计 300 次、单个 routine 200 次，超限报「Automation run rate limit exceeded」且不自动重试；事件触发只算新事件；支持带日期的一次性 routine（【GB】https://cursor.com/help/grok-bot/routines、https://docs.x.ai/grok-bot/skills-routines-and-automations；已同步到设计文档决策 21） | v1：cron + 一次性 routine；v1.2：事件触发（webhook、GitHub、邮件、飞书代替 Slack）；Linear / Sentry / PagerDuty 以后（见设计文档 5.2） | 已定：创建时一次性授权，超出范围的动作跳过或存草稿。更新 2026-11-02：错过的运行会补跑【Yuki】（偏离 GB，已批准；GB 在云端运行，未说明补跑）：只补一次（最近一次），且只在窗口内，窗口 = 周期的一半，限制在 2 分钟到 2 小时；新建 routine 照【GB】等下一次。长期不在时先问是否继续运行，没回复就暂停【GB，https://cursor.com/help/grok-bot/routines】，阈值 14 天【Yuki】。由不可信来源（如邮件）唤起的运行修改自己的 routine 按不可信审查，通常被拦下；创建或修改 routine 可能要求确认【GB】。通用 webhook：Bearer key，200 = 已开始运行、其他响应 = 未开始【GB】；不去重，key 只存 SHA-256 哈希。GitHub webhook 是另一套：校验 X-Hub-Signature-256（HMAC-SHA256）并按 X-GitHub-Delivery 去重【Yuki】。inbox_events 处理后保留 30 天【Yuki】。见设计文档决策 21、32、33 |
| 共享云电脑 | 每个用户一台持久的云端 Linux 电脑，所有 bot 共享文件、浏览器登录和命令行凭据；每个 bot 一块屏幕；可以实时观看，遇到验证码或 2FA 时由用户接管；可以重置 | v1：原生安装 + bwrap 沙箱（命令 + 文件，B6；GHCR 镜像可选）；v1.1：浏览器（服务器模式；本地模式 v1.x，决策 29）；每 bot 一块桌面 v2 | 两边一致：bot 之间不是安全边界 |
| 连接用户自己的电脑 | 需要先开启；默认每条命令都问，也可设成总是允许或从不允许；还能让流量经桌面转发 | 后续（v3） | 注意与「本地部署模式」区分：后者是 wan 整个跑在用户电脑上，v1.x【Yuki】（设计文档决策 29） |
| 连接器 / 插件 | 在 Connect apps 里按需安装并登录授权（Gmail、GitHub、Slack、Notion、Figma 等），也支持自定义 MCP；部分应用在聊天里显示为交互卡片；不连 X 账号也能只读搜索 X 公开内容 | v1（rmcp） | X 搜索不内置，按通用原则走连接器。Google（更新 2026-10-30【Yuki】）：自己写 OAuth + REST 连接器，不用 Google 官方 MCP；每个用户注册自己的 Google Cloud OAuth client（Gmail 受限 scope 需 Google 验证），wan 引导配置；【GB】Grok Bot 也用自己的「Grok」OAuth app（https://cursor.com/help/grok-bot/connect-apps）；用户仍可自行接任意 MCP。见设计文档决策 31 |
| Vault | 存登录、卡、地址、电话、邮箱；Secure Form 收集密码和 API key，值对 bot 不可见；接入 1Password（每次登录在 1Password 里批准）；支付账户（Link、X Money） | 存储 v1；填入 / Secure Form v1.1 | 已定（更新 2026-10-30 / 11-02）：逐条复刻 Grok Bot Vault；不做导出（GB 没有）；Google / Apple 密码没有第三方 API，v1 用 CSV 本地导入；1Password / Bitwarden 直连以后。见设计文档决策 20 |
| 审批 | Auto Review 默认开启；用自然语言写 Ask first / Allow 规则；审批卡有“允许一次 / 总是允许 / 拒绝”；bot 可以在卡片上提议增改规则；只有用户自己的消息算请求；routine 发出的邮件和消息存成草稿，等用户点 Send | v1：规则 +（可选 Jev）+ 审查 LLM 自动审核 | 【Yuki】发信对齐 Grok Bot：聊天里用户明确要求发的，过 Auto Review 后可发出；routine / 入站等非该请求发起的一律草稿等 Send。例外（【Yuki】照 Grok Bot）：Vault 已存卡付款不需确认、不加额外护栏，付款仍走审批管线以便加 Ask first；一次性卡每笔确认 |
| 聊天渠道 | Slack（团队共享 bot 可以加进 Slack）、X Chat、在 X 上 @bot 发起任务 | Slack：v1.1 或之后（个人使用）；团队共享：不做（仅个人） | 【Yuki】B8：Slack / X 在 v1.1+；v1 不做聊天渠道 |
| bot 专属邮箱 | 每个账号可以领一个 @mail.grokbot.com 地址，收发邮件，支持 +tag 和来信触发 routine；外发要审批或先存草稿 | 后续（可选连接器） | 已定：核心不内置；自托管要自己解决收发信（域名、MX、发信信誉） |
| 会议 | 给 bot 一个会议链接，让它加入 Meet / Zoom / Teams，会后读摘要和转录（第一手介绍；官方文档没有单独页面；第三方 10 月 7 日报道开发版里出现了这个设置）；Zoom 插件可以拉转录 | 不做 | 已定 |
| 语音 | 听写；bot 用语音备忘回复（附转录）；和单个 bot 实时语音通话；iPhone 支持 Siri / 快捷指令 | 不做 | 【Yuki】2026-11-02：听写 / 语音转文字、语音消息、实时语音都不做；取代 B8 的语音部分（已取代） |
| 图像 / 视频生成 | 根据描述或图片生成图像和视频 | 图像 v1；视频 v1.x【Yuki】 | 已定：和 Grok Bot 一样提供；用户自己配置服务 URL 和 API key；v1 只做图像生成【Yuki】。【GB】只在明确要求时生成、进 Library；图像 1:1/4:3/3:4/16:9/9:16；视频 1–15s 默认 6s、480p/720p、≤14 参考图、≤4 关键帧、含 3:2/2:3、MP4（https://cursor.com/help/grok-bot/create-media · https://docs.x.ai/grok-bot/files-and-results） |
| 购物与付款 | 在网站上比价、加购物车；用 Vault 里的卡自动结账；用 Link 一次性卡时，每笔都要批准 | v1.x【Yuki】 | 已定（更新 2026-10-30【Yuki】）：推迟到 v1.x，v1 只做浏览、Vault 登录填入、表单填写；做时完全照 Grok Bot。【GB】「A card saved in Vault fills in at checkout without asking you each time, and no setting requires your approval for each use. If you want to approve every payment yourself, don't save the card.」（https://docs.x.ai/grok-bot/vault）。不加额外护栏；付款走同一审批管线，用户可加 Ask first 规则；未保存的一次性卡每笔确认。见设计文档决策 30 |
| 委派编码 | 把编码任务交给 Cursor Cloud Agent，结果卡片里附带文件和 PR 链接 | v1（ACP，先接 nyan） | 已定 |
| 让 bot 用手机上的功能 | 日历、联系人、位置、闹钟 / 计时器；iPhone 上还有提醒事项和 Apple 家庭 | 之后（随原生 App，v1.1+） | 随原生手机 App 一起做；手机技术路线见「手机端」节 |
| bot 模板与分享 | 开箱只有一个主 bot，没有预装模板；可以生成模板链接（公开或仅团队），对方添加后得到一份副本；团队还能共享同一个 bot | 模板：v2（未定）；团队可见与团队共享：不做（仅个人） | 更新 2026-10-30【Yuki】：做不做未定，放 v2。【GB】Share → Create a copy → Publish；模板含身份、指令、记忆、skills、routines、插件，不含电脑、登录和历史（https://docs.x.ai/grok-bot/bots）。团队共享不做。见设计文档决策 34 |
| 模型选择 | 没有模型选择器，自动路由 | v1：按协议的四个适配器（Responses、Chat Completions、Anthropic Messages、System One），API key 或 OAuth 登录接入，CPA 非默认【Yuki】，按 bot 和用途选模型 | 有意和 Grok Bot 不同；Gemini 原生协议不做（用户如需要，可自行填一个兼容端点，如 CLIProxyAPI；这是可选做法，不是默认依赖） |
| 套餐与用量 | 包含在订阅里，按周的用量池；有 Usage & Billing 页面 | v1：用量账本与预算规则；用量页 v1.2；不做计费和套餐 | 已定：按 bot、routine、模型、日期统计 token 和花费，带预算提醒 |
| 客户端 | 桌面端 macOS / Windows / Linux，手机端 iPhone / Android；Cmd/Ctrl+K 跨对话搜索 | v1：Electron（Linux / WSL2）+ PWA + wand CLI；macOS / Windows / 原生手机 v1.1 或之后（【Yuki】B8） | 已定：跨对话搜索进 v1 |
| 服务器管理 | 没有内置，由用户自己配置 bot | 不做内置 | 见“示例 bot：servers manager” |
| 知识库 | 没有 | 不做 | Markdown 知识目录后续可选 |
| 团队 / 企业管理 | 管理员控制、身份与访问、私有网络 | 不做 | 单用户自托管 |

## 架构草图

组件：

- **Electron 客户端 (TS)**：桌面优先；之后可改写成 Rust
- **iOS / Android App**：v1.1+（B8）；技术路线待复核
- **推送通道**：WebSocket / ntfy；Web Push v1.2
- **PWA 与 wand CLI**：v1 手机入口是 PWA；CLI 经 Unix socket（B8）
- **入站端点**：v1.2：仅 /hooks；隧道或轮询后备
- **wan Gateway / API**：Rust：HTTP + WebSocket 控制端口 + 单独入站端点；Tailscale serve 或局域网自签名证书；设备 token
- **Bot 运行时**：用户配置的多个 bot、群聊、bot 间消息
- **调度器**：cron、一次性 routine；事件触发 v1.2
- **审批与审计**：风险分级、操作日志、回滚步骤
- **记忆 / skills / Pages**：每 bot 一个 git Markdown 记忆目录 + user.md；FTS5 + jieba 召回；Library v1，Pages 之后
- **Vault**：分层加密（决策 36），不进记忆
- **模型适配层**：按协议四个适配器；API key / 按提供商的 OAuth 登录模块
- **连接器 (MCP)**：GitHub / 飞书 / 自接 MCP；Google v1.1
- **Coding Agent 适配层**：ACP 优先；退而用 CLI headless / A2A
- **外部 Coding Agent**：v1 只接 nyan；后续 Claude Code / Codex / Pi
- **Computer trait**：命令、文件、浏览器、截图、SSH
- **原生电脑 + bwrap**：主路径；GHCR 镜像可选（B6）
- **本地部署模式**：v1.x【Yuki】；OS 沙箱（决策 37）
- **云沙箱接口**：E2B / Daytona / Modal，社区实现
- **Markdown 知识目录（后续，可选）**：git 同步、索引；v1 不做

连线：Electron 客户端 (TS) → wan Gateway / API；iOS / Android App → wan Gateway / API；wan Gateway / API → Bot 运行时；入站端点 → 调度器（事件）；调度器 → Bot 运行时（触发）；Bot 运行时 → 审批与审计（高风险操作）；审批与审计 → 推送通道；Bot 运行时 → 记忆 / skills / Pages；Bot 运行时 → Vault（按需填入）；Bot 运行时 → 模型适配层；Bot 运行时 → 连接器 (MCP)；Bot 运行时 → Coding Agent 适配层（编码任务）；Coding Agent 适配层 → 外部 Coding Agent；Bot 运行时 → Computer trait；Computer trait → 原生电脑 + bwrap；Computer trait → 本地部署模式；Computer trait → 云沙箱接口；原生电脑 + bwrap → Markdown 知识目录（后续，可选）

服务端是一个单用户的 Rust 进程，所有状态都在服务器上，客户端只负责显示和发起审批。Computer trait 抽象出 `exec`、`read_file`/`write_file`、`browser`、`screenshot`；“无电脑档”就是它的空实现。wan 不内置任何具体领域的模块，服务器管理之类的功能都是在这些通用能力之上配置出来的 bot。

## Coding Agent 接口选型

| 方案 | 定位 | 核实情况 | 评价 |
|---|---|---|---|
| ACP（Agent Client Protocol，Zed 发起） | 主适配器 | 官方 Rust SDK `agent-client-protocol` 3.6.0，2026-10-28 发版；原生支持的有 Gemini CLI、Cursor CLI、OpenCode、Goose、Copilot CLI 等；Claude Code、Codex 通过官方适配器接入 | 基于 stdio 的 JSON-RPC，有会话、流式输出、权限请求，正好对上我们的审批流程；覆盖面最广 |
| CLI headless 模式 | 后备 1 | `claude -p --output-format stream-json`、`codex exec --json`（JSONL 事件）已核实；cursor-agent、opencode、gemini 也有类似模式（来自第三方文档） | 接入最简单，但各家输出格式不同，要逐个写解析器 |
| A2A（Agent2Agent） | 后备 2（远程） | 规范仓库 `a2aproject/A2A` 和官方 `a2aproject/a2a-rs` 存在；crates.io 上的 `a2a-rs` 是社区实现 | 适合跨机器、以服务形式部署的 agent；本机调用显得太重 |
| MCP | 补充 | rmcp 已核实 | 适合把“编码”当成一个工具来调；对长任务、会话和权限交互的表达不如 ACP |

> **建议** 主接口用 ACP：wan 在 Rust 里做 ACP 客户端，用户在配置里填 agent 的启动命令就能接入。不支持 ACP 的 agent 退而用 CLI headless 适配器；跑在其他机器上的 agent 用 A2A。已定：v1 只接 nyan（ACP）；后续接 Claude Code、Codex、Pi，走同一个可插拔接口。注意 Claude Code / Codex 的 ACP 适配器是 npm 包，但它们作为外部进程运行，不进 wan 核心。

## 手机端（原生 iOS / Android）

> 【Yuki】B8：v1 手机入口是 PWA，原生 App 在 v1.1+。下面的调研与「RN / Expo 先行」路线是 B8 之前的决定；v1.1 做原生 App 前复核（已确认【Yuki】2026-11-02）

### 大厂的手机端怎么做

| 产品 | iOS | Android | 来源与可信度 |
|---|---|---|---|
| ChatGPT | Swift + SwiftUI / UIKit | Kotlin + Jetpack Compose | Android：OpenAI 招聘（共享 Kotlin 平台，同时支撑 ChatGPT 和 Sora）+ droidcon 2024 演讲，一手来源；iOS：Tech Dev Notes 第三方二进制分析（2026-10-23） |
| Claude | Swift + SwiftUI / UIKit | Kotlin + Jetpack Compose | iOS：Anthropic 招聘要求 Swift / UIKit / SwiftUI + Tech Dev Notes 分析（2026-10-23）；Android：只有 FlyTrap 第三方目录和搜索摘要，未逐项核实 |
| Grok | Swift + SwiftUI / UIKit | Kotlin + Jetpack Compose | iOS：xAI iOS 招聘（Swift / SwiftUI / UIKit / Combine，Rust 是加分项）+ Tech Dev Notes 分析（2026-10-24）；Android：Tech Dev Notes 分析 |
| Grok Bot | React Native（Hermes）+ Expo 模块 + SwiftUI | React Native（Hermes）+ Expo / Nitro / Reanimated + Compose | 官方没有公开技术栈；以上来自 Tech Dev Notes 第三方二进制分析（iOS v1.0.0，2026-08-11；Android v1.5.0，2026-09-02） |

结论：成熟的大产品双端都是纯原生；Grok Bot 是新产品，用 React Native + Expo 快速覆盖两端，需要原生能力的地方用 Swift / Kotlin 模块补。

### 流式、推送、后台

| 主题 | 大厂的做法（证据） | 我们的建议 |
|---|---|---|
| 流式聊天 | 各家模型 API 用 SSE；App 内部用什么协议没有公开（未知）。ChatGPT iOS 带 LiveKit（WebRTC），用于语音（第三方分析） | 服务端到客户端用 WebSocket（双向：消息、审批、状态），SSE 作单向后备；按事件序号断线续传 |
| 推送 | APNs / FCM。ChatGPT iOS 有 Notification Service Extension、后台推送、时效性通知；Claude iOS 和 Grok Bot Android 都带 Firebase Cloud Messaging（均为第三方分析） | 已定：不买 Apple 开发者账号。在线走 WebSocket；v1 离线只走 ntfy（默认不带内容）；PWA Web Push（iOS 16.4+，需添加到主屏幕）在 v1.2（【Yuki】B8、B12；设计文档决策 24）；原生 APNs 以后视情况 |
| 后台任务 | iOS 不允许 App 长时间在后台运行。Grok Bot 官方文档：关掉 App 后工作在云端继续（docs.x.ai）。Claude iOS 新版加入了 Background Tasks 框架（第三方分析） | 所有 agent 工作都在服务端跑；手机只收推送、拉增量、做审批 |

> **自托管开源要注意** iOS 原生推送要 Apple 开发者账号和 APNs 密钥（$99 / 年），所以 v1 不走 APNs，v1 只用 ntfy（Web Push 在 v1.2），开源用户也不用自己申请账号。注意：PWA 必须先“添加到主屏幕”才能收推送。

### 方案对比

| 方案 | 早期速度 | 原生程度 | 和 Rust 的配合 | 评价 |
|---|---|---|---|---|
| Expo / React Native | 最快：复用 Electron 的 TS（API 客户端、类型、状态） | 原生控件，但界面不是 Swift / Kotlin 写的 | 可以用 `uniffi-bindgen-react-native` 接 Rust | Grok Bot 走的就是这条路 |
| SwiftUI + Compose + Rust 核心（UniFFI） | 慢：要写两套界面 | 最原生 | 最好：UniFFI 0.32.2，由 Mozilla 维护 | ChatGPT / Claude / Grok 的方向，再加上共享 Rust 核心 |
| KMP / Compose Multiplatform | 中等 | 原生（Kotlin） | 多了一层 Kotlin，和 Rust 核心重复 | 不推荐 |
| Flutter + flutter_rust_bridge | 快 | 界面自绘，不算原生 | 好（frb 2.14 beta） | 和“原生”的诉求冲突 |

### 建议路线：先快，后好

- **第一阶段：Expo / React Native（快）**：复用 Electron 的 TS 代码；只做聊天、群聊、审批、推送、查看页面和文件；需要原生能力时用 Expo Modules 写 Swift / Kotlin。
- **第二阶段：抽出 Rust 核心**：把协议、同步、离线缓存、加密抽成一个 Rust crate，通过 `uniffi-bindgen-react-native` 接进 App；之后的 Rust 桌面客户端也能复用同一个 crate。
- **第三阶段：换成双端原生（更好）**：用 UniFFI 生成 Swift / Kotlin 绑定，逐屏换成 SwiftUI / Compose，先换流式聊天渲染这类性能热点，最终双端纯原生。

> **B8 之前的决定，v1.1 做原生 App 前需复核** 第一阶段用 React Native / Expo，和 Electron 共用一套 TS 和自动生成的类型，AI 只需要维护一套代码；之后逐屏改写成原生界面。

## 审批与安全

> 调研日期 2026-10-28；除特别标注外，都来自各家官方文档或官方博客

### 各家怎么做

| 产品 | 模式 / 分级 | 规则 | 隔离 | 自动审核与其他 |
|---|---|---|---|---|
| Claude Code | default（手动）/ acceptEdits / plan / auto / dontAsk / bypassPermissions；只读默认放行，写文件和执行命令要问 | `Tool(specifier)`，例如 `Bash(npm run *)`、`WebFetch(domain:x)`、`mcp__server__*`；按 deny → ask → allow 的顺序，先命中的生效；任何一层的 deny 都不能被覆盖；层级为 managed > 命令行 > local > project > user | 沙箱同时隔离文件系统和网络（bubblewrap / Seatbelt，网络走域名白名单代理）；官方明说命令文本规则不是安全边界 | auto 模式：输入侧有注入探针，输出侧由分类器逐个审核动作（只看用户消息和工具调用，不看工具输出）；PreToolUse hook 可以拦截；SDK 的 `canUseTool` 能返回 allow / deny、修改输入、保存规则；dontAsk 模式下没人可问就拒绝 |
| Codex CLI | 沙箱模式（read-only / workspace-write / danger-full-access）和审批策略（on-request / never / 细粒度）分开设置 | 按命令前缀设 allow / prompt / forbid | Seatbelt；Linux 用 bubblewrap + seccomp（Landlock 已是旧方案）；平台无法强制沙箱时拒绝执行 | `approvals_reviewer = auto_review`：由审核 agent 代替用户批 |
| Cursor | Auto-review 运行模式 | 白名单直接跑；用 permissions.json 设定哪些自动放行、哪些要审核 | 能放进沙箱的 shell 命令进沙箱；sandbox.json 配网络域名和路径 | 不能进沙箱的命令交给分类器（来自搜索摘要，未逐项核实） |
| Gemini CLI | Trusted folders：不受信任的目录进入安全模式 | 不受信任时不加载项目设置、env、MCP，也不自动接受 | Docker / Podman / Seatbelt 沙箱 | — |
| Claude in Chrome | 手动批准 / 自动批准 / 跳过所有审批 | 站点级权限；屏蔽高风险类别网站 | — | 内容探针加动作分类器，和 Claude Code auto 用同一套机制；发布、购买、分享个人数据、下载、输入敏感信息这类高风险动作需要确认；连续被拦截就退回逐步询问 |
| ChatGPT agent（已下线，设计仍可参考） | 改变外部状态之前（购买、发邮件）先确认 | — | 云端浏览器 | Watch Mode：登录邮箱或银行这类敏感场景下，用户一离开就暂停；接管模式下用户输入密码时不截图 |
| Grok Bot | Auto Review 默认开启；卡片有“允许一次 / 总是允许 / 拒绝” | 自然语言写的 Ask first 规则和 Allow 规则各最多 20 条，两者都命中时 Ask first 胜；“总是允许”会存成一条规则 | 一台共享云电脑；官方明说不同 bot 之间不是安全边界 | 只有用户自己的消息算请求，邮件、网页里的文字不算；后台任务的卡片 30 分钟过期，聊天里的卡片不过期；routine 发的邮件先存草稿；群聊里只有提问人能批；密码和密钥走 Secure Form 或 Vault，不让模型看到 |
| OpenClaw / Hermes Agent | OpenClaw 的 exec 有 allowlist 和 ask 两种模式；Hermes 对危险命令要审批 | `openclaw approvals allowlist add`；Hermes 的 `command_allowlist` 会永久静默放行 | 可选 Docker 沙箱 | 做法较简单，可参考的部分有限 |

### Anthropic 给出的原则

1. 人要保留控制权：高风险或不可逆的动作之前要人批准，并且随时能停（trustworthy agents 框架）。
2. 权限由程序强制执行，不靠提示词；写在 CLAUDE.md 里只能影响模型想做什么，不能限制它能做什么。
3. 沙箱要同时隔离文件系统和网络：只隔离一边，被注入的 agent 仍然能把数据带出去。
4. 分类器是逐个动作的控制，不是隔离边界；无人值守运行仍然需要沙箱。
5. 防注入要多层叠加：模型训练、内容探针、动作分类器、高风险确认、最小权限；同时避免给 agent “帮我处理所有邮件”这类宽泛指令。

### 建议设计

| 等级 | 例子 | 默认处理 |
|---|---|---|
| T0 只读 | 读文件、搜索、读连接器数据、在 SSH 上执行白名单里的只读命令 | 自动放行 |
| T1 沙箱内可逆写 | 在电脑工作区写文件、在 bwrap 沙箱里跑命令（出口照 GB 放行，屏蔽 loopback） | 自动放行，沙箱就是边界 |
| T2 对外但可撤回 | 建草稿、改日历、创建 routine、委派给 Coding Agent | 先看规则；没有命中规则时由自动审核分类器决定放行还是问用户 |
| T3 对外可见或不可逆 | 发消息 / 邮件、发布、删除、付款、改权限或凭据、对服务器做写操作 | 一律要人批；分类器永远不能自动放行。例外【Yuki】【GB】：用 Vault 已存卡付款照 Grok Bot 不需确认（用户可加 Ask first 规则收紧）；聊天里用户明确要求发的邮件 / 消息过 Auto Review 后可发出（routine 等仍草稿） |

- **1. 动作分级由工具声明**：每个 Rust 工具声明自己的等级、能否撤销、主要目标（路径、域名、主机、收件人）。MCP 工具的注解（例如 readOnlyHint）只当参考，第三方 MCP 服务一律按最低信任处理。
- **2. 规则语法照搬 Claude Code**：`tool(specifier)`，例如 `shell(git status)`、`ssh(host:nas)`、`mcp__gmail__send_*`、`browser(domain:*.bank.com)`。效果只有 deny / ask / allow 三种，按 deny → ask → allow 的顺序先命中先生效。规则存在 settings_docs（B11，带版本），类型在 wan-types 里定义。
- **3. 层级：只能收紧**：从高到低：内置安全底线（不能改）> 全局用户策略 > bot 策略 > routine 授权 > 会话临时授权。任何一层的 deny 或 ask 都不能被下层的 allow 覆盖。每个 bot 能做什么，只靠规则来限制。
- **4. 安全靠沙箱，命令规则只用来少弹几次**：shell 跑在 OS 沙箱（bwrap）里；出口照 GB 放行，但屏蔽 loopback（决策 37）。SSH 没有沙箱：默认 ask，只有按主机配置的只读命令白名单才自动放行。
- **5. 防注入：标记每段内容的来源**：上下文里每段内容都标上来源：用户、bot 自己、不受信任（网页、邮件、工具输出）。只有用户的消息算请求；由不受信任内容引出的 T2 及以上动作一律问用户（学 Grok Bot）。
- **6. 自动审核：规则 +（可选 Jev）+ 审查 LLM（已定【Yuki】）**：Jev 可选：用户有就用，没有时跳过第二层，规则 + 审查 LLM 照样完整工作。第一层是规则：先评估，deny 永远胜出，后面的层推翻不了。第二层（可选）是 Jev 决策模型：在 allow / ask / deny 里快速选一个，置信度高就直接采用。第三层兜底：置信度低、命令复杂（比如多步 shell 管道）或者风险等级高，就升级给会推理的大模型审核，或者直接问用户。不管哪一层，T3 都不会被自动放行（例外：Vault 已存卡付款照 Grok Bot 免确认，除非命中用户的 Ask first 规则）；分类器只看用户消息和工具调用，不看不受信任的工具输出。
- **7. 无人值守的 routine（已定）**：运行时完全自动，不弹审批。创建 routine 时一次性授予权限和范围，范围内的动作直接执行。超出范围的动作跳过或存为草稿，写进本次运行结果，由用户决定要不要扩大范围。
- **8. 手机审批体验（已定）**：推送里带按钮：允许一次 / 总是允许（生成一条规则，默认永久有效，直到用户删除）/ 拒绝。卡片列出目标、范围、具体值，并提示“不可撤销”。T3 仍然必须由人明确批准（Vault 已存卡付款除外，见上），但不要求生物识别：在推送或 App 里点一下就算批准；T3 不提供“总是允许”，默认选中拒绝。群聊里只有发起人能批。
- **9. 审计与回滚**：每个动作都写进只追加的审计日志：哪个 bot、由什么触发、命中哪条规则、谁批的、输入是什么、结果如何。可逆动作同时记下回滚步骤，例如文件快照、变更前备份。审批撤销不了已经做完的事，所以不可逆动作先出草稿或 dry-run 结果。
- **10. 密钥与 bot 边界（已定）**：密钥和凭据存在 Vault，执行工具时才注入，永远不进模型上下文。所有 bot 和 Grok Bot 一样，共用一台电脑和一套凭据，每个 bot 的限制只靠规则执行。明确一点：bot 之间不是安全边界。

- **Jev 是什么**：TypeSafe AI 的决策模型（2026-09 发布）：针对预先定义好的选项，一次前向计算就给出每个选项的校准概率；响应约 70–500 ms，输入每百万 token $0.042，输出免费（来源：Braintrust 介绍文章）
- **接入方式**：可选【Yuki】：只用 TypeSafe 托管的 Jev（用户自带 key），不使用 OpenJev；置信度阈值可以配置
- **已知局限（arXiv 2610.24929）**：需要中间推理步骤才能做的决定，它做不了；给它一个过时的状态摘要，它会照着走，自己也察觉不到；它的置信度能标出自己犯的错，但标不出从错误输入继承来的错
- **已知局限（Braintrust 文章）**：置信度衡量的是概率有多集中，不等于答对的概率；它默认不把输入当成恶意内容，被注入的指令能左右结果，所以“只看用户消息和工具调用”这条护栏必不可少

> **实现要点（Rust、AI 友好）** wan-core 里的 policy 模块，核心是一个纯函数 `decide(action, context, rules) -> Decision`，没有 IO，方便 AI 写大量单元测试和属性测试；审批卡片和审计记录的类型用 specta 共享给客户端。

## 记忆与 Vault

> 已定（Yuki，2026-10-29），参考 Grok Bot。存储实现以设计文档决策 7（记忆）和 36（Vault 密钥）为准

| 层 | 内容 | 怎么进上下文 | 规则 |
|---|---|---|---|
| bot profile | 这个 bot 的角色、偏好、所管领域的状态 | 常驻 | 工作方式以 bot 自己的 profile 为准 |
| journal | 这个 bot 按日期记的工作记录 | 最近几条进上下文，更早的靠召回 | — |
| 共享用户记忆 | 名字、语言、时区等用户事实，以及工作方式偏好 | 所有 bot 可见 | 每条带来源和日期；用户事实冲突时以最新为准 |
| skills | 全局共享的可复用流程 | 按需加载 | 所有 bot 共用 |
| Pages | Library 里可以反复迭代的报告和看板 | 按需打开 | 大块输出放这里，不塞进记忆 |
| Vault（单独存放） | 凭据、账号密码、卡、地址等联系方式 | 永远不进上下文 | 加密存放，绝不写入记忆；bot 按需调用并直接填入；用户可以打开编辑 |

1. 自动写入：和 Grok Bot 一样，bot 自己决定记什么，不需要用户确认，写入时简短告知一句；用户随时可以查看、纠正、删除。
2. 存储与召回：每个 bot 一个 git 管理的 Markdown 目录（profile.md、notes/、journal/），另有所有 bot 共享的 user.md；索引在 SQLite FTS5 + jieba，再加子串 grep；向量检索 v2（fastembed + sqlite-vec）。见设计文档决策 7。
3. 防注入：记忆里的内容一律当作不可信数据，读回上下文时按“不受信任”标记来源，不能当成用户的请求。
4. 不做知识库：Grok Bot 没有知识库，我们也不做。v1 只有记忆各层、skills、Library 和 Vault（Pages 在 v1 之后）；bot 的产出就是共享电脑上的普通文件。
5. 后续再开发（可选）：专门的 Markdown 知识目录，支持 git 同步、建索引、用系统默认应用打开编辑。

## 基础库选型（定稿）

> 【Yuki】2026-10-31 定稿；完整精确版本以设计文档「定稿技术栈」为准。nyan 仓库自身的升级在 nyan 仓库记录。

| 层 | 选型与版本 |
|---|---|
| L1 核心运行时 | Rust 1.99.1（edition 2024，MSRV 1.99）；tokio 1.54.1；reqwest 0.13.5；serde 1.0.229；serde_json 1.0.154；rmcp 3.8.0；rustls 0.23.46；axum 0.8.9；sqlx 0.9.0；actor = tokio task + mpsc；模型 HTTP 用 reqwest 自写 |
| L2 记忆、Vault 与哈希 | jieba-rs 0.11.0；chacha20poly1305 0.11.0；keyring 4.2.0；zeroize 1.9.1；secrecy 0.10.3；sha2 0.11.1；hmac 0.13.0 + subtle 2.6.1（GitHub 签名）；age 0.12.1（仅备份）。已移除 hkdf、argon2【Yuki】2026-11-02 |
| L3 桌面客户端 | Electron 45.2.0；React 19.3.1；Node 24（maintenance） |
| L4 Pages 编译 | swc_core 83.0.0；TypeScript 7.0.2 原生 tsc |
| L5 电脑与沙箱 | chromiumoxide 0.9.1；Chrome stable；bubblewrap 0.13.1；GHCR 镜像可选（Docker 29.x / Podman 6.1.4）；fork Codex linux-sandbox；CDP 自写 PipeTransport（V1） |
| L6 工具链与周边 | specta 2.0.0-rc.27；utoipa 6.0.0；schemars 1.2.3；ACP 3.6.0；cargo-nextest 0.9.150；web-push-native 0.5.0（v1.2）；ntfy 2.35.0；dogdraw（vendored，见设计文档） |
| 仍作参考、未锁定进 L1–L6 | bollard（Docker API）、russh（SSH）、uniffi / uniffi-bindgen-react-native（手机绑定）、sqlite-vec + fastembed（v2 向量召回） |

权威表见 [architecture.md 第 9 节「定稿技术栈」](architecture.md#s9)。

## 开源项目设计参考

> 以下全部只参考设计，不直接复用或依赖

| 项目 | 语言 | 学它的什么设计 |
|---|---|---|
| Hermes Agent | Python | 单进程网关加内置 cron；`delegate_task` 子 agent；可插拔的终端后端（一个长期运行、跨会话共享的容器）；`MEMORY.md` / `USER.md` 和 skills 目录 |
| OpenClaw | TS | Gateway 控制面；多 agent 绑定路由；每个 agent 独立的工作区、身份和工具白名单；automations 的持久化与补跑；ACP 接入 |
| Kortix（原 Suna） | TS / Python | 一个会话一台云电脑；cron 与签名 webhook 触发器；配置用 git 管理 |
| nanobot | Python | 轻量网关常驻后台；渠道接入的抽象；只给自己的账号开放访问 |
| Agent Zero | Python | Docker 里的 XFCE 桌面 + 浏览器（参考每个 bot 一块屏幕）；按项目隔离记忆和密钥 |
| OpenHuman | Rust | 一台小机器跑大量 agent 的资源模型；记忆引擎决定保留和压缩什么 |
| OpenJarvis | Python | 按成本和延迟路由模型；从使用记录里自我改进 |
| Letta | Python | 分层记忆（核心记忆块 + 归档记忆）；agent 自己编辑记忆 |
| Goose | Rust | 以 MCP 扩展为中心的工具体系；ACP 服务端实现 |
| Khoj | Python | 对个人文档做检索；定时整理资讯 |
| browser-use | Python | 页面 DOM 提取、元素标注、动作循环 |
| Cua | Python / Swift | “操作电脑”的驱动抽象和跨系统沙箱 |
| OpenHands | Python | 沙箱中的任务执行和事件流 |

## 示例 bot：servers manager

> wan 不内置、也不预装；这只是写在文档里的示例，用户用通用能力自己配出来

| 这个 bot 要做的事 | 用到的通用能力 |
|---|---|
| 经 Tailscale 等组网后用 SSH 管理多台机器 | SSH 工具 + 主机清单（写在 bot 记忆或配置里） |
| 读取 Beszel 主机探针和 Gatus 服务探针（用户自己部署） | HTTP / MCP 连接器 |
| 定时巡检、主动告警，推到手机 | routine（cron）+ 推送通道（ntfy） |
| 改动前审批、先备份、记变更日志、能回滚 | 审批 + 操作日志 + skill（备份和回滚流程） |
| 资源规划（什么服务放在哪台机器） | bot 记忆 + Pages |
| 公司机器只上报主机指标，不碰公司数据 | 用规则限定这个 bot 能访问的主机和工具 |

## 生态：与 nyan 深度集成

> 以下是建议，放在后期做；其他 Coding Agent 仍然走通用 ACP 接口

1. 共享记忆和 skills：nyan 能读到用户共享记忆（语言、时区、偏好），双方的 skills 可以互相调用。
2. 统一审批：nyan 的权限请求走 wan 的审批引擎和推送通道，在同一个地方批准。
3. nyan 作为 bot 加入群聊：像其他 bot 一样被 @、汇报进度、和其他 bot 协作。
4. 共享模型网关：两边共用同一个模型适配层和 API key，统一计量用量。
5. 其他可能：共用电脑或工作区、统一的任务和操作日志。

## 需要自己造的部分

1. 多 bot 共用一台电脑：共享文件系统和浏览器登录（v1.1），并有并发与锁的规则（N3）；每个 bot 一块桌面在 v2。
2. bot 间消息总线和群聊语义：谁回复、什么时候 @、怎么防止 bot 之间无限对话。
3. 审批引擎：风险分级（读 / 写 / 不可逆）、推到客户端和推送通道、超时处理、审计记录。
4. Coding Agent 适配层：ACP 客户端、CLI headless 解析器、判断什么算编码任务、交接时带什么上下文、结果怎么回传。
5. 权限模型：按 bot 设定工具 / 主机白名单和凭据范围（数据边界靠它来保证）。
6. 分层记忆：profile、journal、共享用户记忆的自动写入、冲突处理、召回，以及查看 / 纠正 / 删除。
7. Vault：加密存储，执行工具时直接填入，不经过模型，用户可以编辑。
8. Rust 版 Computer 端口和原生 + bwrap 后端（浏览器 v1.1：Chrome/Xvfb + CDP pipe screencast 与截图，不装桌面、不开 VNC）；microVM 预留在端口后；复用 Codex linux-sandbox 的 OS 沙箱（决策 37）；接口兼顾 v1.x 的本地模式。
9. 模型适配层：按协议的四个适配器（Responses、Chat Completions、Anthropic Messages、System One）、API key / OAuth 登录两种接入（OAuth 按提供商、照抄 CLIProxyAPI 的 Rust 登录模块）、多端点配置、按 bot 和用途选模型、兼容网关的响应包装。
10. 远程访问（B8）：Tailscale serve 或局域网自签名证书；设备配对（10 分钟）、长期 token 吊销、登录限速。
11. 客户端（B8）：Electron（全部设置）、PWA（聊天、附件、审批、routine 列表与暂停；不做语音）、wand CLI（管理 + chat / approve）。

## 分阶段路线

> **v1 范围（【Yuki】B12，2026-11-01）** v1 = M0–M5，约 18 周；之后依次是 v1.1、v1.2 和更后。各里程碑的内容分配为【自】按 B12 整理，详见 [architecture.md 第 10 节](architecture.md#s10) 与 [../roadmap.md](../roadmap.md)。

| 阶段 | 内容 |
|---|---|
| M0 工程地基 | monorepo、CI 与供应链、许可证、wan-types 方法登记表与错误模型、i18n（zh-CN / en）与文档骨架（B13：docs/user、docs/dev、ADR、AGENTS.md） |
| M1 地基 | SQLite 与只进不退迁移、事件与 seq、会话 actor、取消与超时、步骤日志、模型适配器、协议握手、Vault 存储、设置、备份恢复 |
| M2 单 bot 可用 | 客户端（B8）：Electron（Linux / WSL2）、wand 托管的 PWA、wand CLI；配对、多设备同步、ntfy；审批、记忆、FTS5 + jieba、卡片与 system/event/status_changed |
| M3 电脑与工具 | 原生安装 + bwrap、SSH、MCP、GitHub、附件、ACP 接 nyan、并发与资源锁 |
| M4 Routines（cron）与可靠性 | cron routine、补跑与暂停、幂等、降级与 `wand doctor`、通知路由 |
| M5 多 bot | 多 bot、侧栏、Library、群聊（helper 子回合 + bot 互发消息）、图像生成 |
| v1.1 | 浏览器、Vault 填入、Google 连接器；macOS / Windows、原生 App（做前复核技术路线【Yuki】）、网页屏幕接管、Slack / X 均在 v1.1 或之后（B8）；语音不做【Yuki】 |
| v1.2 | 事件触发（webhook / GitHub / 邮件 / 飞书）、Web Push、用量页 |
| 之后 | Pages、记忆导入（Library 在 v1 / M5，以设计为准）；仍属 v1.x 的：购物与支付、本地（单 PC）模式、飞书长连接、视频生成、订阅剩余额度 |
| 下一步设计 | P1：内置系统提示词【Yuki】 |

> **附录：旧路线（存档，已被上表 B12 取代）** 2026-11-01 之前的 MVP / v2 / v3 清单。主要变化：Docker 电脑后端 → 原生 + bwrap（B6）；公网单端口 → Tailscale serve 或局域网（B8）；Web Push、事件触发、用量页 → v1.2；浏览器、Vault 填入、Google 连接器 → v1.1；Pages 与记忆导入 → v1 之后；RN 手机端 → 原生 App v1.1+；Vault 导出取消【Yuki】。仍有效的远期项：v2 = 向量召回、每 bot 一块桌面、动画头像、bot 模板（未定）、接入 Claude Code / Codex / Pi；v3 = 连接用户电脑、云沙箱接口、Markdown 知识目录、A2A、原生 APNs、bot 邮箱连接器、与 nyan 深度集成、Rust 重写客户端、正式开源。

## 待 Yuki 确认的问题

开放问题只在 [architecture.md 第 13 节](architecture.md#s13) 维护，这里不重复。目前包括：检索评测、额度细节、bot 模板、X 连接器；头像 thinking/waiting 动画（L4，并入决策 35）。M8 的 GB 原文链接已补。

## 参考链接

- [Agent Client Protocol](https://agentclientprotocol.com/get-started/agents)
- [ACP Rust SDK](https://github.com/agentclientprotocol/rust-sdk)
- [A2A 规范](https://github.com/a2aproject/A2A)
- [a2a-rs（官方）](https://github.com/a2aproject/a2a-rs)
- [Claude Code headless](https://code.claude.com/docs/en/headless)
- [Codex 非交互模式](https://learn.chatgpt.com/docs/non-interactive-mode)
- [Hermes Agent](https://github.com/NousResearch/hermes-agent)
- [OpenClaw](https://github.com/openclaw/openclaw)
- [Kortix（原 Suna）](https://github.com/kortix-ai/suna)
- [nanobot](https://github.com/HKUDS/nanobot)
- [Agent Zero](https://github.com/agent0ai/agent-zero)
- [OpenHuman](https://github.com/tinyhumansai/openhuman)
- [OpenJarvis](https://github.com/open-jarvis/OpenJarvis)
- [Letta](https://github.com/letta-ai/letta)
- [Goose](https://github.com/aaif-goose/goose)
- [Khoj](https://github.com/khoj-ai/khoj)
- [browser-use](https://github.com/browser-use/browser-use)
- [Cua](https://github.com/trycua/cua)
- [OpenHands](https://github.com/OpenHands/OpenHands)
- [ChatGPT iOS 技术栈（第三方分析）](https://techdevnotes.com/apps/ios/chatgpt/6448311069/tech-stack)
- [Claude iOS 技术栈（第三方分析）](https://techdevnotes.com/apps/ios/claude/6473753684/tech-stack)
- [Grok iOS 技术栈（第三方分析）](https://techdevnotes.com/apps/ios/grok/6670324846/tech-stack)
- [Grok Bot iOS 技术栈（第三方分析）](https://techdevnotes.com/apps/ios/grok-bot/6794501026/tech-stack)
- [Grok Bot Android 技术栈（第三方分析）](https://techdevnotes.com/apps/android/grok-bot/ai.x.grok.bot/tech-stack)
- [OpenAI Android 招聘（Kotlin 平台）](http://www.openai.com/careers/android-engineer-chatgpt-mobile-infrastructure-san-francisco/)
- [droidcon 2024：从零构建 ChatGPT App](https://it.droidcon.com/droidcon-2024/talk/FZASWP/)
- [Anthropic iOS 招聘](https://job-boards.greenhouse.io/anthropic/jobs/4572744008)
- [xAI iOS 招聘](http://job-boards.greenhouse.io/xai/jobs/4985386007)
- [Grok Bot 手机端文档](https://docs.x.ai/grok-bot/mobile)
- [Claude Code 权限配置](https://code.claude.com/docs/en/permissions)
- [Claude Code auto 模式是怎么做的](https://www.anthropic.com/engineering/claude-code-auto-mode)
- [Claude Code 沙箱](https://www.anthropic.com/engineering/claude-code-sandboxing)
- [Agent SDK 权限与 canUseTool](https://code.claude.com/docs/en/agent-sdk/permissions)
- [Claude Code hooks](https://code.claude.com/docs/en/hooks)
- [Anthropic：安全可信 agent 框架](https://www.anthropic.com/news/our-framework-for-developing-safe-and-trustworthy-agents)
- [Anthropic：浏览器使用中的注入防御](https://www.anthropic.com/research/prompt-injection-defenses)
- [Claude in Chrome 权限指南](https://support.claude.com/en/articles/12902446-claude-in-chrome-permissions-guide)
- [Codex 审批与安全](https://learn.chatgpt.com/docs/agent-approvals-security)
- [Gemini CLI Trusted Folders](https://geminicli.com/docs/cli/trusted-folders/)
- [Cursor 运行模式](https://cursor.com/docs/agent/security/run-modes)
- [ChatGPT agent 系统卡：注入防御](https://deploymentsafety.openai.com/chatgpt-agent/promptinjection)
- [Grok Bot 审批、安全与隐私](https://docs.x.ai/grok-bot/approvals-security-and-privacy)
- [OpenClaw exec 审批](https://docs.openclaw.ai/tools/exec-approvals/)
- [What is Jev?（Braintrust）](https://www.braintrust.dev/articles/what-is-jev)
- [What Can't Jev Decide?（arXiv 2610.24929）](https://arxiv.org/html/2610.24929)
- [Grok Bot 文档总览](https://docs.x.ai/grok-bot/overview)
- [Grok Bot：创建和管理 Bot（含记忆、模板）](https://docs.x.ai/grok-bot/bots)
- [Grok Bot：消息与协作（语音、群聊）](https://docs.x.ai/grok-bot/chat-and-collaboration)
- [Grok Bot：电脑与应用（邮箱、本地电脑）](https://docs.x.ai/grok-bot/computer-and-apps)
- [Grok Bot：文件与结果（Pages、图像视频、Library）](https://docs.x.ai/grok-bot/files-and-results)
- [Grok Bot：创建媒体（图像 / 视频参数）](https://cursor.com/help/grok-bot/create-media)
- [Grok Bot：Pages](https://cursor.com/help/grok-bot/pages)
- [Grok Bot：Skills 与 Routines](https://docs.x.ai/grok-bot/skills-routines-and-automations)
- [Grok Bot：Vault](https://docs.x.ai/grok-bot/vault)
- [Grok Bot：设置与通知](https://docs.x.ai/grok-bot/settings-and-notifications)
- [Grok Bot：FAQ](https://docs.x.ai/grok-bot/faq)
- [Grok Bot Changelog](https://x.ai/changelog/bot)
- [Introducing Grok Bot](https://x.ai/news/introducing-grok-bot)
- [Grok Bot 101](https://x.ai/bot/guides/grok-bot-101)
- [第三方：Grok Bot Meetings 开发版报道](https://www.progressiverobot.com/2026/10/07/grok-bot-meetings-join-google-meet-zoom/)
- [第三方：Grok Bot 7 天评测](https://aliakbarfakhri.com/grok-bot-review/)
- [第三方：七个 bot 的团队实测](https://aimaker.substack.com/p/grok-bot-ai-agent-team)

- [rmcp（官方 Rust MCP SDK）](https://github.com/modelcontextprotocol/rust-sdk)
- [axum](https://github.com/tokio-rs/axum)
- [sqlx](https://github.com/launchbadge/sqlx)
- [sqlite-vec](https://github.com/asg017/sqlite-vec)
- [jieba-rs](https://github.com/messense/jieba-rs)
- [bollard](https://github.com/fussybeaver/bollard)
- [russh](https://github.com/warp-tech/russh)
- [chromiumoxide](https://github.com/mattsse/chromiumoxide)
- [headless_chrome](https://github.com/rust-headless-chrome/rust-headless-chrome)
- [thirtyfour](https://github.com/stevepryde/thirtyfour)
- [fantoccini](https://github.com/jonhoo/fantoccini)
- [genai](https://github.com/jeremychone/rust-genai)
- [rig](https://github.com/0xPlaygrounds/rig)
- [Beszel](https://github.com/henrygd/beszel)
- [Gatus](https://github.com/TwiN/gatus)
- [ntfy](https://github.com/binwiederhier/ntfy)
- [UniFFI（Mozilla）](https://github.com/mozilla/uniffi-rs)
- [uniffi-bindgen-react-native](https://github.com/jhugman/uniffi-bindgen-react-native)
- [flutter_rust_bridge](https://github.com/fzyzcjy/flutter_rust_bridge)
- [Expo](https://github.com/expo/expo)
- [specta](https://github.com/specta-rs/specta)
- [utoipa](https://github.com/juhaku/utoipa)
- [schemars](https://github.com/GREsau/schemars)
- [cargo-nextest](https://github.com/nextest-rs/nextest)


## 导出说明

- 「分阶段路线」表：原 Page 把 Library 列在「之后」、M5 未列 Library，与设计（Library 在 v1 / M5）冲突，已按设计修正。
- 指向设计 Page 的链接改为仓库内相对链接；「手机端」一节的 RN / Expo 路线是 B8 之前的结论，v1.1 做原生 App 前需复核【Yuki】。
- P1 内置系统提示词、动态模型调度：等待 Yuki 设计，不得实现。
