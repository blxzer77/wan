# wan 架构设计 v1

> 导出自 Page「wan 架构设计 v1」（https://cursor.com/canvas/store-eb702648-1a45-447b-8323-ba94eaf2c492/5c75acb3-a438-4fc1-b09e-78e64f12555c，2026-11-02 版本）。导出时已按 2026-11-02 Yuki 最新决定覆盖冲突处（见文末「导出说明」）。本文件是 wan 唯一权威的当前设计；需求见 [requirements.md](requirements.md)，路线图与任务见 [../roadmap.md](../roadmap.md)。

> 原摘要：wan 当前唯一权威的设计文档（2026-11-02 整理）：一个 Rust 守护进程照着 Grok Bot 复刻。按主题组织，每件事只写一次；已取代的决策放在附录。只讲设计，不含实现代码。

<a id="s1"></a>

## 1 概览

> **本页地位** 这是 wan 唯一权威的当前设计。[需求文档](requirements.md)只记需求和开源对照，评审页（已关闭，见 roadmap）只记审计历史；三处冲突时以本页为准。决策编号（1–40）和审计编号（A/B/N/V/H/L/P）保留，便于对照评审页。

- **形态**：一个纯 Rust 守护进程 wand（无 Node、无 Deno）【Yuki】。所有 bot、routine、队列、审批都在这一个进程里。电脑（原生 + bwrap）、MCP 服务器、ACP 编码代理都是它管理的子进程，坏了就重建。Electron 客户端自带 Node 可以；Pages 的类型检查调用随包附带的 TS7 原生 tsc 二进制。
- **用户**：只给个人用【Yuki】。没有团队、管理员、多账号；GB 的团队功能一律不做。
- **范围**：v1 = M0–M5，约 18 周；v1.1、v1.2 见路线图（B12【Yuki】）。浏览器与 Vault 填入在 v1.1；事件触发、Web Push、用量页在 v1.2；Pages 在 v1 之后。

<a id="s2"></a>

## 2 原则与标签

> **原则** 照着 Grok Bot 做：GB 有文档的行为就照做；GB 没有的 wan 不做；wan 只给个人用（2026-11-01）。偏离 GB 的地方都标明已由 Yuki 批准。

| 标签 | 含义 |
|---|---|
| 【GB】 | Grok Bot 公开资料（附出处）；「官方」= xAI 设计文章，「演讲」= Peter Piekarczyk 整理的 Lee Robinson 演讲（第三方，含推测） |
| 【GB 观察】 | 观察到但未找到文档 |
| 【Anthropic】【Claude】【Cursor】 | 对应厂商文档 |
| 【自】 | wan 自己的设计 |
| 【Yuki】 | Yuki 的决定 |

- **官方**：五个基本概念：Bots、Chats、Prompts（一次性 / Skills / Routines）、Tools、Artifacts。Tools 和 Skills 在账户内共享，Memory 和 Routines 归各个 bot，群聊共享上下文但每个 bot 记忆各自独立；用户能看到电脑的状态，能在侧栏预览，也能直接接管。
- **演讲**：每个会话一条队列，用户消息打断当前回合，其他消息排队；主循环跑在 Temporal 上，每次模型和工具调用记一步；电脑是可丢弃的 Firecracker 微虚拟机。wan 照搬语义，换成单进程能跑的实现（决策 5、8）。
- **未公开**：Grok Bot 的客户端协议。wan 用 Codex app-server 风格的 JSON-RPC（第 4 节）。

<a id="s3"></a>

## 3 组件与部署

组件：

- **客户端**：Electron（Linux / WSL2）· wand 托管的 PWA · wand CLI（B8）
- **协议层**：JSON-RPC over WS（B4）；设备令牌；控制 API 对沙箱不可达
- **会话 actor**：每会话队列与回合
- **审批管线**：规则 →（Jev，可选）→ 审查 → 用户
- **模型层**：按协议 4 个适配器；API key / OAuth 登录模块
- **工具层**：内置 + MCP + 技能
- **SQLite**：步骤日志 / 事件 / FTS5
- **文件区**：记忆 git / Library / 附件
- **Vault**：MK 包裹 DEK；条目直接用 DEK 加密
- **电脑**：原生安装 + bwrap；GHCR 镜像可选
- **浏览器 provider**：v1.1：Chrome + CDP pipe（服务器模式；本地模式 v1.x）
- **Pages 编译**：v1 之后：SWC + tsc（Library 在 v1 / M5）
- **入站端点**：v1.2：仅 /hooks，与控制 API 分离
- **子进程**：MCP 服务器 / ACP 编码代理
- **远程主机**：用户服务器 via SSH
- **调度器**：routine（cron；事件触发 v1.2）
- **通知**：WS 在线；离线 ntfy；Web Push v1.2

连线：客户端 → 协议层；协议层 → 会话 actor；会话 actor → 通知；调度器 → 会话 actor（入队）；会话 actor → 模型层；会话 actor → 审批管线；审批管线 → 模型层（Jev/审查）；会话 actor → 工具层；工具层 → 电脑；工具层 → 子进程；工具层 → 远程主机；工具层 → Vault；工具层 → 浏览器 provider；Vault → 浏览器 provider（按引用填入）；工具层 → Pages 编译；入站端点 → 调度器（事件）；会话 actor → SQLite；会话 actor → 文件区；协议层 → SQLite（事件重放）

协议层、会话、审批、模型、工具、调度、存储都在 wand 进程内；电脑、MCP、ACP 和远程主机在进程外，由 wand 管理，坏了可以丢掉重建。

- **部署**：主路径：原生安装 + bwrap（Claude Code srt 式）【Yuki】B6 / 决策 8。GHCR 镜像可选，只给无头 VPS 和 CI：非 root、命名卷；文档写明隔离更弱，警告不要挂载 ~/.ssh 或云凭据。.devcontainer 只用于开发 wan。microVM 预留在 Computer 端口后（【GB】每用户一台 Firecracker microVM，docs.x.ai/grok-bot/security-faq）。
- **电脑**：无头 Linux：shell、文件、命令行工具、编码工作目录；不装桌面、不开 VNC（决策 9）。按需启停、闲置停机、内存上限【Yuki】。
- **浏览器**：v1.1：在 Xvfb 上跑有界面的真 Chrome（不装窗口管理器），只经 CDP pipe 控制（决策 29、V1）。不加自动化标记、不伪装指纹；资料目录持久并共享；出口 IP 可配置；接管用 CDP screencast；闲置自动停；崩溃用同一资料目录重启。【Yuki】先做服务器模式，接口按两种模式设计；本地模式 v1.x，【自】做法设想：wan 独立的 Chrome 资料目录，电脑关机或睡眠时需要浏览器的 routine 记「浏览器不可用」并跳过。【GB】对照：GB 只有云电脑，「Route traffic through this computer」只是让网络出口经过用户电脑（docs.x.ai/grok-bot/settings-and-notifications）。
- **远程**：用户自己的服务器经 SSH 接入，命令同样过审批。连接用户自己的电脑（GB 的本地电脑执行）推迟到 v3。
- **连接**：远程访问（B8【Yuki】）：Tailscale serve，或局域网 + 自签名证书；不做公网反代。配对：8 位数字码 + 二维码，10 分钟有效、一次性，换取设备令牌（PWA 只放 HttpOnly cookie，Electron 放系统钥匙串）。事件入站（v1.2）见 5.2。

<a id="s4"></a>

## 4 协议与类型

- **协议**：JSON-RPC over WebSocket，Thread → Turn → Item（决策 2）。Item 按 started → delta → completed 推送；审批和表单由服务端主动发请求；Electron、PWA、CLI 用同一套协议；事件带全局 seq，断线后 `device/sync`（sinceSeq）补发。
- **命名**：B4 + N1【Yuki】：方法 `domain/verb_object`，事件 `<domain>/event/<过去式>`。固定域：session、memory、routine、vault、pages、computer、acp、notify、device、system；协议消息用 `$/`；方法名 snake_case，参数 camelCase，枚举 snake_case，允许 `_meta`；`system/initialize` 的 protocolVersion 用日期；方法与通知在 wan-types 唯一登记。例：`session/respond_card`、`session/event/item_updated`、`system/event/status_changed`、`settings/get|update|reset`、`session/start_upload`。
- **版本**：握手互换版本：主版本不同拒绝连接并提示升级；次版本只加字段、互相兼容（决策 38【Yuki】）
- **类型**：Rust 为唯一来源；specta 生成 TS（决策 16、V2【Yuki】）。specta =2.0.0-rc.27，specta-serde / typescript / util =0.0.14，放在只能手动升级的 Renovate 组；schemars 只用于工具参数和配置，utoipa 只用于 /hooks 等 HTTP 端点；CI 比对全部生成物。
- **Item**：决策 22：Item = 标签联合 message / tool_step / card / system_event / attachment【自】。卡片种类 email_draft、secure_form、approval、routine_created、question 等，必须带 fallback_text；卡片有状态 pending / submitted / expired，用带版本的 `session/respond_card` 回应，服务端推 `session/event/item_updated`。
- **状态**：【GB】单独的 `system/event/status_changed`：idle / thinking / working / waiting / blocked / done 六态 + 当前动作文字（designing-grok-bot）。【自】由回合生命周期推导：blocked = 需要用户（审批、Secure Form、接管、Vault 未解锁）；waiting = 排队或在等 helper / 锁。

<a id="s5"></a>

## 5 各子系统

<a id="s5-1"></a>

### 5.1 会话、回合、持久执行与存储

- **1. 消息到达**：客户端发 `turn/start`（消息 ID 由客户端生成），服务端去重并写入 events 表，带 seq；routine、helper 回报、渠道消息走同一入口。
- **2. 入队**：会话 actor 的 mpsc 队列：每个对话同一时刻一个回合；用户消息默认在下一个工具边界引导当前回合，可选排队；其他输入排队。优先级 P0 用户 > P1 群聊 > P2 后台，5 分钟防饿死（N3【Yuki】）。
- **3. 开始回合**：写 turn 记录；按 stable / context / volatile 三层组装上下文，profile.md 在会话开始时冻结注入；系统提示词见 P1（待定）。
- **4. 调用模型**：写一步 `model_call`（started）；流式 delta 作为 Item 事件推给在线客户端；结束写 completed 并记 usage 和 cost。
- **5. 审批**：见 5.7。每个决定都写一步。
- **6. 执行工具**：tool_use 落盘后才执行；只读的可并行，写操作串行；有副作用的步骤崩溃时结果未知，不自动重试，改为问用户。
- **7. 继续或结束**：有工具结果回到第 4 步；否则写 turn completed。超过阈值且会话闲着时安排后台压缩（只有接近上限时才阻塞，完整历史保留可检索，演讲）。
- **8. 停止与取消**：CancellationToken 树；停止不回滚；进行中的副作用步骤记 cancelled_unknown，永不自动重试，卡片给「核实 / 重做 / 忽略」（N2、N4）。

- **actor**：直接用 tokio：每会话一个 task + mpsc，外面一个 supervisor（决策 4）。不引入 actor 框架；kameo 备选。
- **步骤日志**：自写 SQLite 步骤日志，语义参照 Temporal / Restate（决策 5）。每次模型、工具、审批记一步；重启后复用已完成步骤；接口抽象，将来可换 Restate；不用 Temporal（太重，无官方 Rust SDK）。
- **存储**：SQLite WAL 存结构化数据，文件存内容，Vault 单独加密（决策 6）。SQLite：会话、事件、步骤日志、配置、routine、规则、用量、Pages 源码与版本、FTS5。文件：记忆（git Markdown）、技能、附件、Library。保留【Yuki】M5：按会话归档事件 / 步骤；inbox_events 处理后保留 30 天；FTS 定期 optimize；VACUUM 计划；用量按日汇总。
- **并发**：N3【Yuki】ResourceManager 锁。文件系统读写锁、每主机 SSH 4 个、computerUse 1 个、每个工作目录 nyan 1 个、记忆 git 独占、SQLite 单写者；固定顺序加锁，等待时不持锁。上限：全局 6 个回合（后台最多 3）、并行只读工具 8 个、每回合 helper 4 个且不嵌套。预算：80% 时后台暂停；100% 时照【GB】让正在跑的回合跑完、不开新回合。
- **超时**：N2【Yuki】。SIGTERM 后 5 秒 SIGKILL；MCP 取消 2 秒、ACP 10 秒。模型响应头 10 分钟、流空闲 5 分钟；前台命令默认 2 分钟 / 最长 10 分钟；后台命令 2 小时；MCP 启动 30 秒、调用 2 分钟（上限 30 分钟）；nyan / helper 空闲 10 分钟；回合 2 小时；routine 30 分钟；后台审批 30 分钟。
- **幂等**：N4【Yuki】。RPC 结果保存 24 小时；审批先到先得；MCP 调用只在工具只读或请求从未发出时重试 1 次；模型调用最多重试 5 次；已批准的发送走 outbox；对外写操作若 API 不支持幂等键则不自动重试。routine 补跑见 5.2。

<a id="s5-2"></a>

### 5.2 Routines 与事件触发

GB 没有公开 routine 的实现；以下除标【GB】【Yuki】处外为【自】。

- 【GB】限额（https://docs.x.ai/grok-bot/skills-routines-and-automations）：每个 bot 最多 50 个 routine；定时至少间隔 5 分钟；每个 routine 保留最近 20 条运行记录；每小时总计 300 次、单个 routine 200 次，超限报「Automation run rate limit exceeded」，不自动重试。【自】滚动一小时计数器（全局 + 每 routine），超限返回 B3 routine 段的 `routine_rate_limited`。
- 【GB】可以设带日期的一次性 routine，按准确时间运行。
- cron 也是一种事件源；事件先进事务收件箱 `inbox_events`（`dedup_key` 唯一）；表 `routines` / `inbox_events` / `routine_runs`；插入 run 与推进 `next_fire` 在同一事务；routine = 触发过滤器 + 提示词，归属单个 bot，命中的事件作为不打断的输入进会话队列。
- 【Yuki】补跑（决策 32 = N4，唯一规则；偏离 GB 已批准，GB 跑在云端未说明补跑）：UNIQUE(routine_id, scheduled_at)；停机或重启后错过的运行只补跑一次（最近一次），且只在窗口内（周期的一半，限制在 2 分钟到 2 小时，窗口算法【自】Hermes 式）；其余记 `missed_count`，窗口外记 skipped。新建的 routine 照【GB】等下一次触发。
- 重叠策略默认 skip / buffer_one；IANA 时区并处理夏令时；弹性窗口内按确定性抖动；计时用单调时钟，时钟跳变后重算。
- 【Yuki】grants（工具、主机、收件人、金额等）创建时一次性授权；范围外动作生成审批卡，过期照 GB（后台卡 30 分钟）；复用 wan-core policy 模块的 decide。routine 发出的邮件和消息存草稿等用户点 Send。
- 【GB】长期不在时先问用户是否继续运行 routine，没有回复就暂停；阈值 14 天【Yuki】（决策 33）。不做「连续失败 N 次暂停」。
- 【GB】由不可信来源（如邮件）唤起的运行若修改自己的 routine，按不可信内容审查，通常会被拦下；创建或修改 routine 可能要求用户确认。
- 【Yuki】inbox_events 处理后保留 30 天再清理。

#### 事件触发（v1.2，决策 19）

- 【GB】事件触发只算新事件，不回放历史事件。
- v1.2 事件源：webhook、GitHub、邮件、飞书（【Yuki】顶替 GB 的 Slack），加 cron；Linear / Sentry / PagerDuty 以后按需。
- 【Yuki】入站：单独的仅入站端点（只暴露 /hooks，与控制 API 分离，各自鉴权）；文档给出 Cloudflare Tunnel / Tailscale Funnel 等隧道方案；无公网时用轮询后备。【自】邮件用 Gmail API history.list 轮询，不依赖 Pub/Sub；GitHub 无公网时轮询 API；飞书长连接核实后放 v1.x。

|  | 通用 webhook（L1） | GitHub webhook |
|---|---|---|
| 鉴权 | 【GB】Bearer key；key 只在创建时显示一次，要新 key 就轮换【Yuki】 | 【Yuki】校验 X-Hub-Signature-256：用 GitHub webhook secret 对原始请求体做 HMAC-SHA256，常量时间比较 |
| 存储 | 【Yuki】只存 key 的 SHA-256 哈希，请求时比对 | secret 存 Vault（vault_ref）【自】 |
| 去重 | 【Yuki】不去重：每次调用的 dedup_key 是新生成的随机 id，每次都是新运行 | 【Yuki】按 X-GitHub-Delivery 去重 |
| HMAC / 时间窗口 | 都没有（照 GB） | 有 HMAC（GitHub 自己的机制）；无时间窗口 |
| 返回 | 【GB】200 = 已接受并已开始运行；其他任何响应 = 未开始（https://cursor.com/help/grok-bot/routines、skills-routines-and-automations）；不自动重试 | 同左 |

<a id="s5-3"></a>

### 5.3 Vault（决策 20、36）

- **GB**：登录、卡、联系信息全账户共享；秘密永不进模型上下文（https://docs.x.ai/grok-bot/vault）。Vault 与电脑分离，Reset 后仍在；GB 未公开加密方式与密钥持有方。
- **GB**：fill_credential(条目 id)：由守护进程经 CDP 填入网页（v1.1）。模型只拿到条目 id。【Yuki】条目存允许的 origin；填入前比对当前顶层 origin（含 eTLD+1），不匹配则拒绝并提示接管。
- **GB**：Secure Form 卡片（v1.1），带「保存到 Vault」勾选；顺序：Vault → 密码管理器 → Secure Form。API key 走只写的 secret 卡片。
- **GB**：一次性验证码经 Gmail 连接器或自有邮箱读取；注册时生成密码；SSO / passkey / 验证码 / 二维码交给用户接管。【自】wan 只经 Gmail 连接器读取；bot 自有邮箱以后（需求页 P2）。
- **Yuki**：v1 只有存储与 vault_ref；填入在 v1.1；CSV 导入（Google、Apple 密码管理器没有第三方 API）。本地导入；已存在的跳过；出错的行跳过。1Password / Bitwarden 直连以后。
- **Yuki**：不做 Vault 导出（GB 没有这个功能，2026-11-02）

#### 密钥方案（决策 36，2026-11-02 修订【Yuki】）

- **自**：层级：主密钥 MK 包裹数据密钥 DEK；DEK 在 SQLite 里存两份包装（MK 包一份、恢复密钥包一份）。参照 Bitwarden / 1Password / OpenBao / systemd-creds。
- **Yuki**：条目直接用 DEK 加密：XChaCha20-Poly1305，24 字节随机 nonce。AAD 保留 item_id、字段名、origin（另含格式版本、条目类型【自】）。不再用 HKDF 派生条目密钥。
- **Yuki**：解锁只有两种：OS 钥匙串，或密钥文件 + 恢复密钥。服务器默认：密钥文件放在数据目录之外（0400）；用 GHCR 镜像时以容器 secret 挂载。本地：OS 钥匙串（keyring）；Linux 无桌面会话退回密钥文件。没有口令解锁模式。【自】可选增强：宿主机 systemd-creds 用 TPM2+host 加密密钥文件；环境变量只作兼容、不推荐；TPM 默认绑定以后。
- **Yuki**：恢复密钥（Emergency Kit）初始化时只显示一次，必须确认已保存才能继续。【自】256 位随机，能独立解开 DEK；服务器只存包装结果。
- **自**：备份绝不含主密钥。备份 = 密文 + 包装后的 DEK；恢复到新机器需要 MK 文件或恢复密钥（7.3）。
- **自**：轮换：换 MK 只重包 DEK；换 DEK 后台按 key_version 重加密，之后重新生成恢复密钥
- **自**：实现：write-only API、zeroize / secrecy、审计日志。没有 get_secret；fill 在守护进程内解密并经 CDP 写入；审计只记元数据（时间、item_id、字段、目标 origin、触发的 thread / routine）。crate：chacha20poly1305、keyring、zeroize、secrecy。
- **自**：能防 / 防不住。能防：磁盘或数据卷被拷走（MK 不同盘时）、备份泄露、沙箱被攻破、提示注入索取秘密。防不住：宿主机 root 失陷、守护进程内存被读、恶意页面读取已填入的值。

<a id="s5-4"></a>

### 5.4 记忆与检索（决策 7、28）

- **检索**：FTS5 + jieba-rs 中文分词，再加子串 grep。中文常用词多为 2 字，trigram 搜不到；词典外的词退回单字切分。v1 不做向量但留好分块表；v2 加 fastembed + bge-small-zh + sqlite-vec，RRF 合并。
- **记忆**：每个 bot 一个 git 管理的 Markdown 目录。profile.md（约 2000 字，会话开始时注入）、notes/、journal/（只供检索）；所有 bot 共享一份 user.md；对话中用 memory 工具即时写；后台「做梦」任务定期整理；「忘掉」= 删除并从索引排除。
- **评测**：检索和记忆评测从第一天就建。具体设计由 Yuki 负责（待定）。
- **导入**：记忆导入（v1 之后）照 GB 流程：设置 → 通用 → 导入；复制提示词到另一个助手；粘贴回答点 Add to Memory；送到主 bot 并打开该会话【GB】。来源 docs.x.ai/grok-bot/settings-and-notifications 与 /bots；GB 提示词原文未公开（what-a-bot-remembers 返回 404）。【自】主 bot 整理进 user.md / profile.md；提示词以 Claude 官方版本为底稿改写（见下）。

> I'm moving to another service and need to export my data. List every memory you have stored about me, as well as any context you've learned about me from past conversations. Output everything in a single code block so I can easily copy it. Format each entry as: [date saved, if available] - memory content. Make sure to cover all of the following — preserve my words verbatim where possible: Instructions I've given you about how to respond (tone, format, style, 'always do X', 'never do Y'). Personal details: name, location, job, family, interests. Projects, goals, and recurring topics. Tools, languages, and frameworks I use. Preferences and corrections I've made to your behavior. Any other stored context not covered above. Do not summarize, group, or omit any entries. After the code block, confirm whether that is the complete set or if any remain.
>
> —— Claude 帮助中心，https://support.claude.com/en/articles/12123587-import-and-export-your-memory-from-claude

第三方 Tom's Guide（https://www.tomsguide.com/ai/you-can-move-your-chatgpt-memory-to-claude-in-60-seconds-heres-how）另有分类版本，未在官方页核实。

<a id="s5-5"></a>

### 5.5 技能、插件、连接器（决策 11、31）

- **插件**：连接器用标准 MCP，技能用 Markdown，插件 = 把它们打成一个包。MCP 支持 stdio 和远程 HTTP；OAuth 由 wan 完成，令牌存 Vault，调用都过审批。清单格式兼容 Codex / Claude 插件；内置工具写在 Rust 核心；优先用厂商官方 MCP。v1 预置：Gmail、Google 日历、Google Drive、GitHub、飞书，加 Office 技能；有全局代理设置；不做插件市场。bot 可以自己添加 MCP 服务器，只在私网或私有 CA 服务器时询问（B2【Yuki】）。X 连接器暂缓。
- **技能**：所有 bot 共用一个私有 skill 库；照 GB 在聊天里管理，不做单独界面。团队发布不做（仅个人）。
- **Google**：决策 31（v1.1）：自写 Google 连接器（OAuth + REST），不用 Google 官方 MCP【Yuki】。【GB】GB 用自己的「Grok」OAuth app（cursor.com/help/grok-bot/connect-apps）。每个用户在 Google Cloud 注册自己的 OAuth client（Gmail 受限 scope 需 Google 验证），wan 引导配置；用户仍可自行接任意 MCP。

> **已定【Yuki】2026-11-02** Gmail / 日历 / Drive（Google 连接器）在 v1.1；v1 只有 GitHub、飞书和用户自接的 MCP。

<a id="s5-6"></a>

### 5.6 模型层（决策 12）

- **Yuki**：按协议做，不按提供商做：四个协议适配器（OpenAI Responses、Chat Completions、Anthropic Messages、System One）。任何兼容这些协议的服务填 base URL 即可接入；消息和流式事件类型参照 Pi（pi-ai）；模型 HTTP 用 reqwest 自写薄客户端。
- **Yuki**：接入方式只有 API key 和 OAuth 登录；OAuth 登录按提供商做成可插拔模块（照抄 CLIProxyAPI，Rust 重写）。CPA 不是默认，只是用户可填的兼容端点。【Yuki】接受 OAuth 登录订阅的条款风险。
- **Yuki**：每个 bot 配 main / review / memory 三个模型；可选全局分类器 Jev。Jev：TypeSafe 托管，走 System One API /v1/systemone，用户自带 key；不使用、不打包开源权重。设置里只有模型池 + vault_ref；动态模型调度待定（B11）。v1 没有备用模型自动切换。

<a id="s5-7"></a>

### 5.7 审批（决策 10）

- **管线**：规则 →（Jev，可选【Yuki】）→ 审查 LLM → 用户。Jev 只做快速判断；未配置时规则 + 审查 LLM 两层完整工作。审查 LLM 用单独的便宜模型，只看待执行动作和最近几轮，绝不看外部内容，出错或超时就问用户；T3 默认要人批。分级借鉴 DeepSeek Harness auto-review：低风险放行；中风险只有用户明确点名动作和对象才放行；高风险拒绝；拿不准就拒绝；沙箱档位和审批策略打包成预设。
- **Yuki**：发信对齐 GB（【GB】approvals-security-and-privacy）。聊天里用户明确要求发的邮件 / 消息，过 Auto Review 后可发出（需要时出审批卡）；routine、入站或其他非该请求发起的发送一律存草稿等用户点 Send。
- **Yuki**：付款（决策 30，v1.x）完全照 GB。【GB】「A card saved in Vault fills in at checkout without asking you each time, and no setting requires your approval for each use. If you want to approve every payment yourself, don't save the card.」（docs.x.ai/grok-bot/vault）。不设上限、不设商户白名单（H1）；付款仍走同一管线，用户可加 Ask first 规则【自】；Link / 未保存的一次性卡按订单批准。
- **自**：规则：只能收紧，ask-first 优先（B11）；所有决定都写日志

<a id="s5-8"></a>

### 5.8 群聊、helper、ACP（决策 14、18）

> coordinating Bots handle routine routing and bring the user in when a decision requires judgment
>
> —— x.ai/news/designing-grok-bot

- 【GB】多个 bot 共享一条会话，每个 bot 保留自己的记忆、技能和模型；协调 bot 处理常规分派，需要判断时拉用户进来；可指定固定协调者（Chief of Staff 模式）。每群约 6 个 bot，每账号约 50 个 bot，wan 以此为默认上限。v1 默认自动协调【Yuki】。
- 【自】分派：配置了 Jev 先用 Jev，判断不了或未配置时交给 LLM。
- 【Yuki】bot 消息一律不可信，不能触发 T2 及以上；每轮 bot 间跳数与预算上限；协调分派写审计。
- helper：有独立上下文、可持久的子回合，结果作为不打断的排队消息回到主会话（演讲）。内置 research、browser（v1.1）、coding；coding 经 ACP，先接 nyan（AGPL，只经进程边界），再接 Claude Code、Codex、Pi。

<a id="s5-9"></a>

### 5.9 客户端与多设备（B8、N6）

- **Yuki**：Electron：Linux x64 / arm64 与 WSL2（AppImage + deb）。可改全部设置；e2e 用 Playwright。
- **Yuki**：wand 托管的 PWA：聊天、附件、审批、Pages（之后）、routine 列表与暂停（语音转文字已取消，2026-11-02【Yuki】）。只改通知、主题、语言、bot 开关、routine 暂停。
- **Yuki**：wand CLI：管理命令（配对、备份、恢复、doctor、settings）+ 经 Unix socket 的 chat / approve。读 LC_*，--json 为英文。
- **Yuki**：v1.1+：macOS / Windows、原生 App（做前复核技术路线【Yuki】）、网页屏幕接管、Slack / X；与 GB 的 5 处偏离已批准。客户端工作量约 5.5 周（净 3.5 周）。
- **GB**：N6 多设备照抄 GB：服务端权威快照 + seq 事件（`device/sync`），不用 CRDT。草稿只在本地；readSeq 取最大值合并；审批先到先得，其他设备置灰并显示设备名；设置带版本乐观锁，冲突回退；可续传上传 `session/start_upload` / `session/finish_upload`；离线只读、无发送队列；吊销的设备会被清空。

<a id="s5-10"></a>

### 5.10 通知（决策 24）

- 【GB】注意力状态 needs-attention / unread / working；每个 bot 一个通知开关（群聊没有）（docs.x.ai/grok-bot/settings-and-notifications）。
- 【GB】桌面在用时通知发到活动桌面，正在看的 bot 不提醒；完成的回复不推手机；等用户输入约 3 分钟后升级到手机。
- 【Yuki】「手机」= PWA + ntfy（默认不带内容，用界面语言）；Web Push 在 v1.2；Linux 桌面不做在场检测；在一处读过就撤回各处通知。推送失败最多重试 3 次，只记日志，不挡回合。

<a id="s5-11"></a>

### 5.11 bot 资料、侧栏、Library、Pages、头像（决策 23、26、34、35）

- 【GB】资料 = 名字、头像、标签（+ 模型设置）；个人 bot 没有描述字段。侧栏：自定义分组、隐藏（可恢复）、注意力状态。Cmd-K 搜 bot、会话内容、Page 标题（FTS5 + jieba；chat-and-collaboration）。
- 【GB】每个 bot 一个 Library：Today / This week / Older，可 Pin，标出已分享（files-and-results）。Library 在 v1（M5）。
- Pages（v1 之后）【GB】：单个 TSX，只能 import wan/page 组件库；数据静态嵌入不联网；写入时类型检查，失败保留上一个好版本，错误带行号；按精确字符串批量编辑；默认私有、可分享、控件状态不持久、只有 bot 能编辑；在只允许脚本的 iframe 里运行（cursor.com/help/grok-bot/pages、files-and-results、chat-and-collaboration）。
- 【Yuki】A20：Pages 不做团队分享和公开链接（偏离 GB，已批准）。【自】版本存 SQLite；SWC 转译。【Yuki】类型检查用 TypeScript 7 原生 tsc（7.0 无编程 API，只调 CLI；devblogs.microsoft.com/typescript/announcing-typescript-7-0/）；不可用时只转译并记日志。
- bot 模板（决策 34）：未定，放 v2【Yuki】。【GB】Share → Create a copy → Publish；含身份、指令、记忆、skills、routines、插件，不含电脑、登录和历史（docs.x.ai/grok-bot/bots）。团队可见与共享不做。
- 头像（决策 35）：【GB】所有 bot 共用简单造型，眼睛有表现力，各有变体和配饰，动作表示六态（designing-grok-bot）。【Yuki】v1 静态 dogdraw 小狗，每 bot 一品种（【自】；vendor 于固定 commit，V3）；v2 动画小狗：dogdraw 拆部件 + CSS 动画由 status_changed 驱动（idle 眨眼摇尾、working 快摇跳动、blocked 耳朵耷拉、done 跳一下；thinking / waiting 待设计，L4）；可选 Rive，不用 Live2D。

<a id="s5-12"></a>

### 5.12 附件、媒体、用量（决策 25、27）

- 【GB】附件限额（files-and-results）：每条消息 20 个；图片单个 25MB、合计 200MB；文档 250MB；视频 1GB。
- 【GB】图像 / 视频生成只在用户明确要求时，结果进 Library（create-media）。图像比例 1:1、4:3、3:4、16:9、9:16；视频 1–15 秒（默认 6 秒）、480p / 720p、最多 14 张参考图、4 张关键帧、含 3:2 / 2:3、MP4。
- 【Yuki】v1 只做图像生成（用户填 URL + API key），视频 v1.x；不做幻灯片、不装 LibreOffice（如做走 Google Workspace API）。【自】生成实现为模型层适配器。
- 用量【自】：SQLite 逐次调用账本（bot、会话、routine、用途、模型、输入 / 输出 / 缓存 token、延迟）；【Yuki】v1 只基于 wan 自己记录的调用；单独的用量页（v1.2），不提供聊天内查询；订阅剩余额度 v1.x（OAuth 登录拉取）。

<a id="s5-13"></a>

### 5.13 设置、错误、i18n、a11y、文档（B11、B3、N7、N8、B13）

- **设置**：B11【Yuki】SQLite settings_docs，带版本乐观锁，类型在 wan-types 里只写一次。作用域 user、bot、routine、device-local（不同步）；没有 system / policy 层和团队管理。秘密只存 vault_ref。RPC `settings/get|update|reset`（ifVersion + JSON Merge Patch）、`settings/event/changed`；冲突时回退并提示「Couldn't save. Try again.」。审计日志保留 1 年。【GB】设置项：时区（routine 用）、语言、外观、自定义审批规则、每 bot 设置。
- **错误**：B3【Yuki】。各模块 thiserror，API 边界转成统一 ApiError；anyhow 只在二进制和测试。JSON-RPC 标准码；-32800 取消；wan 码 -32000..-32099，每域 10 个，-32090..-32099 保留；字段 code、data.kind（snake_case，约 36 种，routine 段含 `routine_rate_limited`）、retryable、retryAfterMs、白名单 params、requestId；原因只进日志。用户看到「问题 + 下一步 + 请求 ID」加操作按钮；策略拒绝说明原因、在哪改、谁能改。模型看到 tool_result isError：`kind: reason. Suggestion: ...`。CI：码段、i18n key、生成 docs/errors.md、TS 漂移、边界无 anyhow、脱敏测试、insta 快照。
- **i18n**：N7【Yuki】单一 Fluent .ftl 来源；只有 zh-CN 和 en。跟随系统或手动设置，zh* → zh-CN，其余 en，回退 en。bot 用界面语言回复，除非用户用另一种语言写；模型看到英文 kind。UI 用 Intl，Rust 用 jiff；时区设置只给 routine，界面时间跟随设备。输入法组字时 Enter 不发送。CI：key、占位符、伪语言。约 5 天。
- **a11y**：N8【GB】【Yuki】已对照 GB App 0.78.1 包核实。照做：聊天 role=log、消息为带标签的 article、aria-busy、prefers-reduced-motion、forced-colors / 高对比度焦点环、:focus-visible、对话框焦点还原、缩放 0.5–3（步长 0.1）、GB 快捷键表。不做：Alt+A、播报节流、强调色对比度调整、WCAG 目标、CLI --plain、移动端专门无障碍、Orca 走查。约 3 天。
- **文档**：B13【Yuki】。docs/user（中文）、docs/dev 与 ADR（中文，Nygard 格式，4 位编号，取代不删）、生成的 errors.md 与 settings.md。README、CHANGELOG、AGENTS.md、代码注释和提交用英文；CLAUDE.md 为指向 AGENTS.md 的符号链接（.claude/skills 同样链接到 .agents/skills），不使用 .cursor 目录（【Yuki】覆盖原 B13「CLAUDE.md 一行 `@AGENTS.md`、.cursor/rules 可选」）。CI：链接检查、生成文档漂移、ADR 索引、AGENTS.md 存在。约 6.5 天。

<a id="s6"></a>

## 6 安全与威胁模型

> 未注来源处为【自】，汇总评审页 A9 的推荐（【Yuki】按推荐采纳）。

<a id="s6-1"></a>

### 6.1 沙箱（决策 37）

> Each user gets a dedicated Firecracker microVM with its own kernel, memory, and virtual devices.
>
> —— https://docs.x.ai/grok-bot/security-faq

- **GB**：GB：每用户一台 Firecracker microVM；网络默认全放行（白名单仅 Enterprise）。https://docs.x.ai/grok-bot/security
- **Yuki**：出口放行（照 GB），但屏蔽 loopback。沙箱内进程连不到本机控制 API（A9，吸取 DeepSeek Harness CVE-2026-82533 教训：沙箱里的 agent 经本机无认证接口把自己改成完全访问）。agent 永远不能给自己提权。
- **Yuki**：Linux / WSL2：fork Codex codex-rs/linux-sandbox（Apache-2.0，保留 LICENSE 与 NOTICE）。bubblewrap + no_new_privs + seccomp（屏蔽 AF_VSOCK、io_uring），.git 只读；Landlock 只作可选第二层，不作回退。
- **Yuki**：macOS：Seatbelt（profile 参照 Codex / Anthropic sandbox-runtime / Gemini CLI）；Windows：专用低权限用户 + 防火墙 + ACL，不用 AppContainer
- **Yuki**：Chrome CDP 只用 --remote-debugging-pipe（V1）。不开 TCP 端口；chromiumoxide 0.9.1 只支持 WebSocket，所以自写 PipeTransport 并复用它的类型；上游 #294 合并后改回；CI 断言没有调试端口。
- **Yuki**：沙箱策略（能碰什么）与审批策略（要不要问）分两层；没有可用沙箱时拒绝执行命令，不退回无沙箱。stdio MCP 进程也跑在 OS 沙箱里。

<a id="s6-2"></a>

### 6.2 控制面与鉴权

- 控制 API 必须认证；检查 Origin，CORS 只放行 wand 托管的 PWA 源和 Electron。
- 登录与配对限速：按来源 IP 计数，连续失败退避；配对码 8 位 + 二维码，10 分钟有效、一次性（B8【Yuki】）。
- 【Yuki】设备令牌只存 SHA-256 哈希，请求时比对；可吊销、定期轮换；恢复到新机器后旧令牌作废。
- 【Yuki】通用 webhook key 同样只存 SHA-256 哈希，只在创建时显示一次，轮换得到新 key；GitHub webhook 另用 X-Hub-Signature-256（见 5.2 对比表）。两者是不同机制。
- PWA 会话 cookie：HttpOnly + Secure + SameSite=Strict，并检查 Origin。WebSocket 握手要先鉴权：PWA 用 cookie，Electron 和 CLI 用 Bearer 设备令牌；CLI 本机可走 Unix socket。
- Electron 与 PWA 设严格 CSP（禁 inline 脚本与 eval，connect-src 只到 wand）；Pages iframe 只允许脚本。

<a id="s6-3"></a>

### 6.3 不可信内容与注入

- 外部内容（网页、邮件、webhook、bot 消息）一律标记为不可信；审查 LLM 不看外部内容。
- bot 消息不能触发 T2 及以上（5.8）；不可信来源唤起的运行改自己的 routine 通常被拦下（【GB】5.2）。
- Vault 秘密永不进模型上下文；fill 绑定 origin（5.3）。
- 日志脱敏：秘密类型用 secrecy::SecretString，写日志和步骤日志前统一脱敏（第 8 节）。

<a id="s6-4"></a>

### 6.4 供应链与许可证（B2 + N9【Yuki】）

cargo-deny 0.20.2；pnpm frozen lockfile、安装脚本白名单、pnpm audit；Actions 按 SHA 固定；Renovate 每周一次，patch 自动合并，发布满 3 天才升；CycloneDX SBOM、cosign、SLSA。NOTICE + THIRD_PARTY_LICENSES.html 由 cargo-about 0.9.3 与 license-checker-rseidelsohn 5.0.1 + manual.toml 生成，随每个产物、「关于」对话框和 `wand --licenses` 提供；发布依赖出现 GPL / AGPL 则 CI 失败。fork 的 Codex 代码保留原文件头；OAuth 照抄 CLIProxyAPI（MIT）、dogdraw（MIT）保留版权声明；nyan（AGPL）只经 ACP 进程边界。

<a id="s7"></a>

## 7 降级与恢复

<a id="s7-1"></a>

### 7.1 失败与降级（决策 39、B10【Yuki】）

参照 LangChain Deep Agents fault tolerance、ACM Queue《Failure Is a Feature》、AWS Agentic AI Lens AGENTOPS04-BP03：只重试瞬时错误，重试在 harness 层，失败后熔断并诚实降级。

- 分类：瞬时（超时、429、5xx、连接断开）→ 重试；确定性（400/401/403、校验失败）→ 不重试；写操作结果不明 → 问用户，不自动重试。
- 重试：指数退避，初始 1s，因子 2，full jitter，单次上限 30s；模型 API 最多 5 次【Yuki】，外部工具 / MCP 最多 2 次，推送最多 3 次。
- 熔断：每个外部依赖一个断路器；连续失败 5 次（或 60s 内错误率 ≥50%）打开；冷却 60s 后半开放 1 次探测。
- 降级原则：不丢数据；先重试或等待，再用 B3 模板告诉用户（status=blocked + 通知）；从中断处继续；不重做副作用。v1 没有备用模型自动切换。
- 故障表（16 种，节选）：Vault 未解锁（钥匙串不可用或缺密钥文件）→ 步骤暂停到解锁；连接器过期 → Reconnect 卡；SSH 走恢复阶梯；MCP 重启 3 次（1 / 5 / 30 秒）；磁盘满 → 停后台工作；缺 bwrap → 拒绝命令；记忆仓库损坏 → 只读 + 待写队列；SQLite 损坏 → 只读恢复模式并指向 `wand restore`；时钟跳变 → 重算 routine。v1.1（随浏览器）：Chrome 崩溃用同一资料目录重启。
- `wand doctor`（15 项检查，--fix）；/healthz、/readyz、`system/get_status`；故障注入测试（单元、每夜真实环境、每周混沌）。

<a id="s7-2"></a>

### 7.2 备份（N5【Yuki】）

每天 03:30 做 age 加密快照；保留 7 份日备、4 份周备、3 份升级前备份。内容：SQLite（VACUUM INTO）、记忆仓库（git bundle）、skills / 设置 / Library、工作目录（去掉 gitignore 的文件）、Vault 密文与包装后的 DEK、manifest；不含主密钥；Chrome 资料目录默认不备份（--include-browser 可加）。路径 ~/.local/share/wan/backups；rclone 上传需主动开启；restic 在 v1.x；不做通用数据导出（`wand export`）【Yuki】2026-11-02。`wand restore` 校验哈希，拒绝更新的 schema；每次备份后校验，每周恢复演练，每夜 CI。

<a id="s7-3"></a>

### 7.3 恢复到新机器【自】

- **1. 安装并恢复备份**：在新机器装 wand，运行 `wand restore`，校验哈希。
- **2. 解开 Vault**：放入原 MK 密钥文件，或输入恢复密钥解开 DEK，再按新机器方式（钥匙串或密钥文件）重新包装。
- **3. 吊销旧设备令牌**：旧机器发出的设备令牌全部作废（只存哈希，直接清表）。通用 webhook key 的哈希随备份恢复，仍有效；怀疑泄露时逐个轮换（已确认【Yuki】2026-11-02）。
- **4. 重新配对**：各客户端用 10 分钟配对码重新配对。

<a id="s7-4"></a>

### 7.4 升级与迁移（决策 38）

- **1. 备份**：升级前自动备份。
- **2. schema 迁移**：schema 版本存在 DB 里；按序只进不退，不支持降级。
- **3. DEK 重加密**：如有密钥格式变化，按 key_version 把旧条目重新加密。
- **失败处理**：任何一步失败 → 恢复备份并拒绝启动。【自】

<a id="s8"></a>

## 8 测试与可观测性

> 【Yuki】A7、A8、B1 按评审页推荐采纳；具体做法为【自】。

- 日志：tracing + tracing-subscriber JSON；span 字段统一 botId / threadId / turnId / stepId；秘密用 secrecy::SecretString，日志和步骤日志写入前统一脱敏；v1 不接 OTel。
- 单元测试用内存 fake（A2）；集成测试用临时目录里的真 SQLite；模型调用用 record / replay 文件；时间用可注入的 fake clock，测 routine、补跑与超时。
- 沙箱测试只在 Linux CI 跑（bwrap）；e2e 用 Playwright 跑 Electron；故障注入见 7.1；不设覆盖率门槛。
- 发布（A8）：守护进程发 Linux 二进制（主路径）与可选 GHCR 镜像（cosign 签名）；桌面用 electron-builder + GitHub Releases 自动更新。

<a id="s9"></a>

## 9 仓库、crate 与技术栈

> 【Yuki】2026-10-31 定稿：厚的模块化单体 + 薄的边界 crate。

Monorepo：根目录 Cargo workspace（crates/*）+ pnpm workspace（apps/*、packages/*）；根目录 Apache-2.0 LICENSE。nyan 是单独的 AGPL 仓库，只经 ACP 接入。

| crate | 类型 | 职责 |
|---|---|---|
| `wan-types` | lib | 协议唯一来源；specta / utoipa / schemars 生成 |
| `wan-sandbox` | lib | OS 沙箱：fork Codex linux-sandbox；macOS Seatbelt；Windows 专用用户 + 防火墙 + ACL |
| `wan-vault` | lib | 凭据存储与 origin 绑定 |
| `wan-host` | lib | 电脑 + 浏览器（CDP pipe）+ SSH |
| `wan-core` | lib | 模块：session / turn / memory / routines / notify、store、policy（审批）、tools（内置 / MCP / skills）、model（适配器 + OAuth） |
| `wan-acp` | lib | 薄 ACP 客户端（接 nyan） |
| `wan-pages` | lib | 薄的 SWC + TS7 tsc 编译管线 |
| `wand` | bin | HTTP/WS JSON-RPC，组合根 |

| 依赖规则 | 内容 |
|---|---|
| 允许（A1【Yuki】） | types 在最底层；wan-core 只依赖 types，并拥有端口 trait（core::ports）；sandbox 依赖最少；vault → types（+ core::ports）；host → types、sandbox（+ core::ports）；acp、pages → types（+ core::ports）；wand 依赖全部并组装 |
| 端口与数据（A2、A3【Yuki】） | 端口：Model、Computer、Browser、RemoteHost、SecretFill、CodingAgent、PageCompiler、Notifier、Clock、IdGen，各配内存 fake；表按模块加前缀归属，跨模块只走公开 API |
| 检查（A4【Yuki】） | CI 用 cargo metadata 校验 crate 依赖图；core 内模块只暴露 facade；架构测试禁止引用其他模块内部路径 |
| 禁止 | sandbox / host 永不链接 vault / core；types 不依赖任何 wan-*；crate 之间不能有环 |
| 何时再拆 | 只看可观察信号：需要独立发布、编译时间或依赖污染、安全审计要求零链接 |

| 目录 / 文件 | 内容 |
|---|---|
| `packages/wan-protocol` | 生成的 TS 协议类型，禁止手改 |
| `packages/wan-page-kit` | Pages 唯一允许 import 的组件库 |
| `apps/desktop` | Electron 45.2.0 + React 19.3.1 |
| `vendor/dogdraw` | 固定于 5b5f9380d368458adf6b3bb63d146b5513c942b4，带 LICENSE 与 UPSTREAM，CI 校验哈希（V3） |
| 根目录 | Cargo.toml、Cargo.lock、rust-toolchain.toml、deny.toml、justfile、package.json、pnpm-workspace.yaml、docs/、.github/workflows/ |

### 定稿技术栈（L1–L6，2026-11-02 更新 L2）

| 层 | 范围 | 选型与版本 |
|---|---|---|
| L1 核心运行时 | 语言 / 异步 / HTTP / 存储 / MCP / TLS / 模型客户端 | Rust 1.99.1（edition 2024，MSRV 1.99）；tokio 1.54.1；reqwest 0.13.5；serde 1.0.229；serde_json 1.0.154；rmcp 3.8.0；rustls 0.23.46；axum 0.8.9；sqlx 0.9.0 |
| L2 记忆、Vault 与哈希 | 分词 / 加密 / 密钥 / 哈希 | jieba-rs 0.11.0；chacha20poly1305 0.11.0；keyring 4.2.0；zeroize 1.9.1；secrecy 0.10.3；sha2 0.11.1（设备令牌与 webhook key 哈希）；hmac 0.13.0 + subtle 2.6.1（GitHub 签名、常量时间比较）；age 0.12.1（仅用于备份，N5）。已移除：hkdf、argon2【Yuki】 |
| L3 桌面客户端 | Electron / UI / Node | Electron 45.2.0（Linux x64 / arm64 + WSL2，AppImage + deb）；React 19.3.1；构建用 Node 24.x，运行时用 Electron 自带的 Node 24.22 |
| L4 Pages 编译 | 转译 / 类型检查 | swc_core 83.0.0；TypeScript 7.0.2 原生 tsc 二进制 |
| L5 电脑与沙箱 | 浏览器 / 镜像 / OS 沙箱 | chromiumoxide 0.9.1（只用类型，传输自写 PipeTransport）；Chrome stable；bubblewrap 0.13.1（主路径）；GHCR 镜像可选（Docker 29.x / Podman 6.1.4）；fork Codex linux-sandbox |
| L6 工具链与周边 | 类型生成 / OpenAPI / ACP / 测试 / 推送 / 头像 | specta =2.0.0-rc.27 + specta-serde / typescript / util =0.0.14（手动升级）；utoipa 6.0.0；schemars 1.2.3；agent-client-protocol 3.6.0；cargo-nextest 0.9.150；cargo-deny 0.20.2；cargo-about 0.9.3；license-checker-rseidelsohn 5.0.1；ntfy 2.35.0；web-push-native 0.5.0（v1.2）；dogdraw @5b5f938（vendored） |

> **锁定规则** 实现与 CI 以本表为准；升小版本需改本表并跑 CI。sha2 / hmac / subtle 版本为 2026-11-02 crates.io 最新稳定版。nyan 仓库自身的依赖升级在 nyan 仓库记录（「Phase A」是 nyan 的阶段名，不属于 wan 路线图）。

<a id="s10"></a>

## 10 路线图

> 【Yuki】B12（2026-11-01）：v1 = M0–M5，约 18 周。各里程碑的内容分配为【自】按 B12 范围整理。

| 阶段 | 内容 | 对应 |
|---|---|---|
| M0 工程地基 | monorepo、CI 与供应链、许可证、wan-types 方法登记表与错误模型、specta 类型生成、i18n 与文档骨架 | 16、B2、B3、B4、N9、N7、B13 |
| M1 地基 | SQLite 存储与只进不退迁移、事件表与 seq、会话 actor 与队列、取消与超时、步骤日志、模型适配器、协议版本握手、Vault 存储与密钥方案、设置、备份恢复 | 1–6、12、36、38、N2、N5、B11 |
| M2 单 bot 可用 | Electron + PWA + wand CLI、配对、多设备同步、ntfy、审批管线、记忆与 memory 工具、FTS5 + jieba、Item / 卡片 / status_changed | 7、10、22、24、B8、N6 |
| M3 电脑与工具 | 原生电脑 + bwrap、SSH、MCP 客户端、GitHub 连接器、附件管线、ACP 接 nyan、并发与资源锁 | 8、9、11、37、N3 |
| M4 Routines（cron）与可靠性 | routines、cron 与一次性 routine、补跑与暂停、限速、幂等、降级与 `wand doctor`、通知路由 | 21、24、32、33、39、N4、B10 |
| M5 多 bot | 多 bot、侧栏、Library、群聊（helper + bot 互发消息）、图像生成 | 14、18、26、27 |
| v1.1 | 浏览器、Vault 填入（Secure Form、接管、fill_credential）、Google 连接器（Gmail / 日历 / Drive）；macOS / Windows、原生 App（做前复核技术路线）、网页屏幕接管、Slack / X（B8） | 20、29、31 |
| v1.2 | 事件触发（通用 webhook / GitHub / 邮件 / 飞书）、Web Push、用量页 | 19、24、25 |
| 之后 | Pages、记忆导入；v1.x：购物与支付、浏览器本地模式、飞书长连接、视频生成、订阅额度、restic；v2：动画头像、bot 模板、向量检索；v3：连接用户自己的电脑 | 23、28、30、34、35 |

<a id="s11"></a>

## 11 决策表（当前）

> 只列当前有效或未定的决策；已取代的见附录 A。细节见各节。

| # | 决策 | 当前选择 | 依据 | 节 |
|---|---|---|---|---|
| 1 | 进程 | 单一纯 Rust 守护进程 + 可丢弃子进程【Yuki】 | OpenClaw、Hermes、Goose | 1 |
| 2 | 客户端协议 | JSON-RPC/WS，Thread→Turn→Item；B4 命名 | Codex app-server；GB 未公开 | 4 |
| 3 | 排队与引导 | 每对话一个回合；下一个工具边界引导，可选排队（N3） | 演讲（含推测） | 5.1 |
| 4 | actor | tokio task + mpsc | — | 5.1 |
| 5 | 持久执行 | 自写 SQLite 步骤日志 | Temporal / Restate 文档 | 5.1 |
| 6 | 存储 | SQLite WAL + 文件 + Vault；保留与归档【Yuki】 | 评审 M5 | 5.1 |
| 7 | 检索与记忆 | jieba FTS5 + grep；git Markdown | 调研页 | 5.4 |
| 8 | 部署 | 原生 + bwrap 为主；GHCR 镜像可选；microVM 预留（B6【Yuki】） | 【GB】security-faq；【Anthropic】sandbox-runtime | 3 |
| 9 | 电脑 | 无头 Linux；按需启停与内存上限【Yuki】 | 评审 M7 | 3 |
| 10 | 审批 | 规则 →（Jev 可选）→ 审查 → 用户；发信照 GB【Yuki】 | 【GB】approvals；DeepSeek Harness | 5.7 |
| 11 | 插件 | MCP + 技能 + 打包；无市场 | Codex / Claude 格式 | 5.5 |
| 12 | 模型层 | 4 个协议适配器；API key / OAuth 登录；CPA 非默认【Yuki】 | Pi、CLIProxyAPI | 5.6 |
| 14 | helper | 持久子回合，结果不打断 | 演讲 | 5.8 |
| 15 | 压缩 | 闲置时后台压缩 | 演讲 | 5.1 |
| 16 | 类型 | Rust → TS（specta，锁版本 + CI 比对）【Yuki】 | 评审 L2、V2 | 4 |
| 18 | 群聊（v1） | 复刻 GB 群聊含自动协调；bot 消息不可信【Yuki】 | 【GB】博客；评审 M2 | 5.8 |
| 19 | 事件触发（v1.2） | webhook、GitHub、邮件、飞书 + 仅入站端点 + 轮询后备【Yuki】 | 博客 | 5.2 |
| 20 | Vault | 复刻 GB；v1 存储 + vault_ref，填入 v1.1；CSV 导入；origin 绑定；无导出【Yuki】 | 【GB】vault | 5.3 |
| 21 | Routines | 统一事件模型 + 事务收件箱 + grants；GB 限额与限速；通用 webhook 照 L1；GitHub 验签 + 去重 | 【GB】skills-routines-and-automations、routines | 5.2 |
| 22 | 卡片 / 事件 / 状态 | Item 联合 + 有状态卡片 + status_changed 六态 | 【GB】博客 | 4 |
| 23 | Pages（v1 之后） | 复刻 GB 画布；无团队分享 / 公开链接（A20）；TS7 tsc【Yuki】 | 【GB】pages、files-and-results | 5.11 |
| 24 | 通知 | 复刻 GB；ntfy，Web Push v1.2 | 【GB】settings-and-notifications | 5.10 |
| 25 | 用量 | 自有账本 + 用量页（v1.2）；订阅额度 v1.x【Yuki】 | 【自】 | 5.12 |
| 26 | 资料 / 侧栏 / Library | 复刻 GB；Library 在 v1 | 【GB】files-and-results | 5.11 |
| 27 | 附件与媒体 | GB 限额；v1 图像，视频 v1.x【Yuki】 | 【GB】create-media | 5.12 |
| 28 | 记忆导入（v1 之后） | 照 GB 流程 | 【GB】settings-and-notifications | 5.4 |
| 29 | 浏览器（v1.1） | CDP pipe provider；服务器模式先做，本地模式 v1.x【Yuki】 | 【自】 | 3 |
| 30 | 购物与支付（v1.x） | 完全照 GB（H1） | 【GB】vault | 5.7 |
| 31 | Google 连接器（v1.1） | 自写 OAuth + REST【Yuki】 | 【GB】connect-apps | 5.5 |
| 32 | Routine 补跑 | 只补一次、只在半周期窗口内（2 分钟–2 小时）；新建等下一次【Yuki】偏离 GB 已批准 | 【Yuki】 | 5.2 |
| 33 | Routine 自动暂停 | 长期不在先问、不回就暂停；14 天【Yuki】 | 【GB】routines | 5.2 |
| 34 | bot 模板 | 放 v2（未定） | 【Yuki】 | 5.11 |
| 35 | bot 头像 | v1 静态 dogdraw；v2 动画【Yuki】 | 【GB】博客 | 5.11 |
| 36 | Vault 密钥 | MK 包 DEK；条目直接 DEK + XChaCha20-Poly1305；钥匙串或密钥文件 + 恢复密钥；无口令模式、无导出（2026-11-02【Yuki】） | 【自】Bitwarden / 1Password 等 | 5.3 |
| 37 | 沙箱 | OS 沙箱为主；屏蔽 loopback；CDP pipe；无沙箱拒绝命令【Yuki】 | 【GB】microVM；Codex / srt | 6.1 |
| 38 | 升级与版本 | 升级前备份；只进不退迁移；协议主版本不同拒绝【Yuki】 | 评审 H3 | 7.4 |
| 39 | 失败与降级 | 只重试瞬时错误；模型 5 次；熔断；无备用模型；B10 故障表【Yuki】 | LangChain / ACM Queue / AWS | 7.1 |
| 40 | 内置系统提示词 | 未定：下一步专门设计（P1）【Yuki】 | — | 13 |

<a id="s12"></a>

## 12 与 Grok Bot 的差距

> 来源：博客 = Designing Grok Bot；文档 = docs.x.ai/grok-bot；需求页 = 功能对照表。没有来源的功能不列。

| Grok Bot 能力 | 来源 | wan 状态 | 说明 |
|---|---|---|---|
| bot 名字、头像、标签；侧栏置顶 / 分组 / 隐藏；Cmd-K 搜索 | 博客、文档 | v1 | 5.11；头像动画 v2 |
| 头像表示六态，悬停显示当前动作 | 博客 | v1（静态）/ v2（动画） | status_changed |
| 内联卡片与系统事件 | 博客 | v1 | 4 |
| 电脑状态、侧栏预览、全屏接管 | 博客 | v1.1（随浏览器） | CDP screencast |
| Vault + Secure Form；网站验证码由 bot 读取 | 文档 | v1 存储 / v1.1 填入 | 5.3 |
| 接 1Password | 文档 | 以后 | v1 CSV 导入 |
| Routines 定时与一次性 | 博客、文档 | v1 | 5.2 |
| Routines 事件触发（GitHub、webhook、Slack、Linear、Sentry、PagerDuty、bot 邮箱） | 博客、需求页 | v1.2（部分） | webhook、GitHub、邮件、飞书 |
| Library | 文档 | v1 | 5.11 |
| Pages；分享链接 | 需求页、文档 | v1 之后；不做团队 / 公开分享 | A20 |
| 幻灯片 | 需求页 | 不做（v1） | 如做走 Google API |
| 图像 / 视频生成 | 文档 | v1 图像；视频 v1.x | 5.12 |
| Usage & Billing | 文档 | v1.2（用量页） | 5.12 |
| 通知路由 | 文档 | v1 | 5.10 |
| 设置：时区、语言、外观、审批规则、每 bot 设置 | 文档 | v1 | 5.13 |
| 记忆导入 | 文档 | v1 之后 | 5.4 |
| 附件 | 文档 | v1 | 5.12 |
| 电脑与 Drive / Gmail 互传文件 | 文档 | 部分（v1.1 随 Google 连接器） | 无专门工具 |
| bot 互发消息、群聊、协调者；约 50 个 bot | 博客 | v1 | 5.8 |
| bot 专属邮箱 | 需求页 | 以后（P2） | — |
| 聊天渠道 Slack / X | 需求页、文档 | v1.1+（个人使用） | B8 |
| 会议、语音（听写 / 语音转文字 / 实时语音）、示范学习 | 需求页、文档 | 不做 | 语音全部不做【Yuki】2026-11-02，取代 B8 语音部分 |
| 模型自动路由，无选择器 | 文档 | 刻意不同 | 按 bot 和用途选；动态调度待定 |
| 委派编码（Cursor Cloud Agent） | 需求页 | v1（ACP） | 先接 nyan |
| 连接用户自己的电脑 | 需求页 | v3 | 与浏览器本地模式不同 |
| 多账号、团队管理、多台电脑、推理钩子、TLS 代理 | 文档（目录） | 不做 | 仅个人 |
| 购物付款 | 需求页 | v1.x | 决策 30 |
| 原生手机 App、bot 模板 | 需求页、文档 | App v1.1+（做前复核技术路线【Yuki】）；模板 v2（未定） | B8、决策 34 |

商品搜索这类内置工具在本次打开的官方页面里没看到，没列；天气只是博客里卡片的例子。

<a id="s13"></a>

## 13 待定事项

- [ ] P1 内置系统提示词：下一步专门设计【Yuki】（决策 40）
- [ ] 动态模型调度：由 Yuki 与 Agent 以后设计【Yuki】（B11）；在此之前只有模型池 + vault_ref
- [ ] 检索和记忆评测的具体设计（Yuki 负责）
- [ ] 头像 thinking / waiting 动作（L4，随 v2 动画）
- [ ] bot 模板做不做（决策 34，v2）
- [ ] X 连接器实现方式（暂缓，不一定用付费官方 API；定价见附录 B）
- [ ] 订阅额度查询细节（v1.x，Yuki 实现时设计）
- [ ] 原生 App 技术路线：v1.1 开工前复核【Yuki】

<a id="appendix-a"></a>

## 附录 A 已取代（仅列结论）

> 历史讨论见评审页（已关闭）。以下内容不得实现。

- ~~决策 13 群聊（旧）~~ → 决策 18（群聊进 v1，自动协调）（2026-10-30【Yuki】）
- ~~决策 17 局域网默认、公网可选（反代、DuckDNS、Let's Encrypt、EasyTier）~~ → B8：Tailscale serve 或局域网自签名证书（2026-11-01【Yuki】）
- ~~Docker 必选 / 容器优先的服务器模式~~ → B6：原生 + bwrap 为主，GHCR 镜像可选（决策 8、37）（2026-11-01【Yuki】）
- ~~无沙箱时每条命令都问~~ → 无沙箱则拒绝命令（B10）（2026-11-01【Yuki】）
- ~~事件触发进 v1；Web Push 进 v1~~ → 均在 v1.2（B12）（2026-11-01【Yuki】）
- ~~旧 M1–M6 里程碑~~ → B12：M0–M5 + v1.1 / v1.2（2026-11-01【Yuki】）
- ~~约 16 个 crate 方案~~ → 厚模块化单体 + 8 个 crate（2026-10-31【Yuki】）
- ~~Pages 类型检查用常驻 Deno 侧车~~ → TS7 原生 tsc（2026-10-30【Yuki】）
- ~~RPC 命名 noun/verb、`card.respond` / `item.updated` / `bot.status`、snake_case 参数~~ → B4：`domain/verb_object`、`<domain>/event/<过去式>`、参数 camelCase（2026-11-01 / 11-02【Yuki】）
- ~~通用 webhook HMAC + 时间窗口 + 去重；401 / 409 返回码~~ → L1：Bearer key、200 = 已开始，其他 = 未开始；不去重（2026-11-01 / 11-02【GB】【Yuki】）
- ~~补跑：重启后只补 15 分钟内最近一次（N4 旧）~~ → 决策 32 唯一规则（半周期窗口）（2026-11-02【Yuki】）
- ~~「routine 运行中不能再建 routine」（GB 观察）~~ → GB 规则：不可信来源唤起的运行改自己的 routine 按不可信审查（2026-11-02【GB】）
- ~~模型重试 10 次（N4）/ 3 次（决策 39）~~ → 统一 5 次（2026-11-02【Yuki】）
- ~~备用模型自动切换~~ → v1 无；以后由动态模型调度处理（2026-11-02【Yuki】）
- ~~Vault 条目密钥 = HKDF-SHA256(DEK, item_id)~~ → 条目直接用 DEK 加密（XChaCha20-Poly1305，24 字节随机 nonce）（2026-11-02【Yuki】）
- ~~Vault 口令解锁模式（MK = Argon2id(密码)）~~ → 只有钥匙串或密钥文件 + 恢复密钥（2026-11-02【Yuki】）
- ~~Vault age 加密导出 / scrypt 口令导出 / CLI 明文导出~~ → 不做 Vault 导出（GB 没有）（2026-11-02【Yuki】）
- ~~B8：PWA 语音转文字（v1）、实时语音（v1.1+）~~ → 语音全部不做（2026-11-02【Yuki】）
- ~~`wand export` 通用数据导出~~ → 不做；异地备份只用 restic（2026-11-02【Yuki】）
- ~~已存卡付款加上限、商户白名单等护栏~~ → 完全照 GB（H1）（2026-10-30 / 11-01【Yuki】）
- ~~v1 不做单独 CLI、只留管理子命令（B8 旧推荐）~~ → wand CLI = 管理命令 + chat / approve（2026-11-01【Yuki】）
- ~~「连续失败 N 次暂停 routine」~~ → 删除；只按长期不在暂停（2026-10-30【Yuki】）
- ~~B13：CLAUDE.md 只有一行 `@AGENTS.md`；.cursor/rules 可选~~ → CLAUDE.md 为 AGENTS.md 的符号链接，无 .cursor（2026-11-02【Yuki】）

<a id="appendix-b"></a>

## 附录 B 调研结论

| 主题 | 结论 | 来源 |
|---|---|---|
| OpenJev 权重 | 不使用；只用 TypeSafe 托管 Jev | 2026-10-29 |
| TypeSafe Jev | jev-1.14.0 / 1.13.0 按输入 $0.042 / 百万 token，输出免费；上下文 64k/请求、32k/问题，长上下文 header 后 262k。MCA 禁止蒸馏、训练模仿模型、作为独立服务转售。wan 只作为客户端接入（用户自带 key） | docs.typesafe.ai/models、MCA |
| Restate | server BSL 1.1（发布后 4 年转 Apache-2.0），只禁止公共 Restate 平台服务；Rust SDK MIT。可作为可选组件，默认不捆绑；v1 仍用自研 SQLite 持久化 | LICENSE 全文 |
| Google Workspace MCP | Developer Preview，需加入预览计划；每个产品一个远程 server。已决定自写连接器（决策 31）。【一般常识，未在文档看到】External 测试模式 refresh token 7 天过期 | Google 文档 |
| X API | 按量付费：读 post $0.005、读 user $0.010、读 DM $0.010；发 post $0.015（带链接 $0.20）、发 DM $0.015；Owned Reads $0.001；每月最多 300 万条读取。官方 X MCP 在 api.x.com/mcp，需要用户自己的开发者 app | docs.x.com |
| tsgo | TS7 已发布，`tsgo` 更名 `tsc` | devblogs.microsoft.com、typescript-go #4576 |

## 相关页面与来源

- [wan 需求文档](requirements.md)
- [wan 设计核对与评审（审计历史，5 轮已全部关闭）](https://cursor.com/canvas/store-eb702648-1a45-447b-8323-ba94eaf2c492/ce2c3c0f-57eb-4374-9fe9-aa4f6e1cfb5b)
- [wan：bot 头像调研](https://cursor.com/canvas/store-eb702648-1a45-447b-8323-ba94eaf2c492/45d7edd3-79c9-41af-aa9f-cb124e51512b)
- [wan：架构调研](https://cursor.com/canvas/store-eb702648-1a45-447b-8323-ba94eaf2c492/e676b66e-83da-4dee-a727-95816bdf319e)
- [Designing Grok Bot（官方）](https://x.ai/news/designing-grok-bot)
- [Grok Bot Routines 帮助](https://cursor.com/help/grok-bot/routines)
- [Skills, routines and automations](https://docs.x.ai/grok-bot/skills-routines-and-automations)
- [What actually happens when you message Grok Bot（第三方）](https://www.peterp.me/articles/what-actually-happens-when-you-message-grok-bot/)
- [Codex app-server README](https://github.com/openai/codex/blob/main/codex-rs/app-server/README.md)
- [Codex linux-sandbox](https://github.com/openai/codex/blob/main/codex-rs/linux-sandbox/README.md)
- [pi-ai README](https://github.com/badlogic/pi-mono/blob/main/packages/ai/README.md)
- [TypeSafe System One](https://docs.typesafe.ai/concepts/system-one)
- [GitHub：Validating webhook deliveries](https://docs.github.com/en/webhooks/using-webhooks/validating-webhook-deliveries)
- [DSH CVE-2026-82533 报道](https://thehackernews.com/2026/09/deepseek-harness-flaw-let-ai-agents.html)


## 导出说明

- 来源 Page 的「附录 A」表格已压缩为上面的「已取代」清单；其余各节按原文导出，组件化排版改为 Markdown。
- 覆盖项（2026-11-02【Yuki】）：B13 文档约定改为 CLAUDE.md 符号链接、无 .cursor。其余最新决定（无语音、Google 连接器 v1.1、无通用导出 / 仅 restic、无 Vault 导出、无 HKDF、无 Argon2、令牌与 webhook key 只存 SHA-256、GitHub 验签与去重、通用 webhook 不去重、routine 限额与补跑、模型重试 5 次、inbox_events 30 天、14 天暂停、原生 App 复核）原 Page 已一致，未改动。
- 待定项（第 13 节）中的 P1 内置系统提示词与动态模型调度：**等待 Yuki 设计，不得实现**。
