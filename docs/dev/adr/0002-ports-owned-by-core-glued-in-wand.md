# ADR 0002：端口 trait 归 wan-core，适配器不依赖 core，由 wand 用 newtype 粘合

- 状态：已接受
- 日期：2026-11-02

## 背景
architecture.md 第 9 节的依赖规则自相矛盾：
- 允许（A1）：「wan-core 只依赖 types，并拥有端口 trait（core::ports）……vault → types（+ core::ports）；host → types、sandbox（+ core::ports）；acp、pages → types（+ core::ports）」
- 禁止：「sandbox / host 永不链接 vault / core」

core::ports 是 wan-core 的一个模块，Cargo 只能按 crate 依赖，所以「host → core::ports」就等于 host 链接 wan-core，违反禁止规则。编码代理在 M0-02 因此停下。

另据第 3 节：「协议层、会话、审批、模型、工具、调度、存储都在 wand 进程内；电脑、MCP、ACP 和远程主机在进程外，由 wand 管理」；第 9 节表中 wan-host、wan-sandbox 均为 lib。即 wan-host 是链接进 wand 的库，它驱动进程外的电脑 / Chrome（CDP pipe）/ SSH。因此「永不链接」是 crate 链接层面的规则（限制 host 代码能看到什么），不能靠「host 是独立进程」绕开。

Yuki 倾向不增加实体（奥卡姆），新增 wan-ports crate 需要有必要性。

## 决策
1. 端口 trait 仍定义在 wan-core::ports，wan-core 仍只依赖 wan-types。
2. 适配器 crate（wan-vault、wan-host、wan-acp、wan-pages）不依赖 wan-core，只暴露自己的具体 API（具体类型 + 固有方法），参数和返回值用 wan-types 里的类型。
3. wand（组合根）为每个端口写一个薄 newtype 实现，例如 `struct HostComputer(wan_host::Computer); impl wan_core::ports::Computer for HostComputer { … 转调 … }`。孤儿规则：trait 和类型都在别的 crate 时 wand 不能直接 impl，newtype 在 wand 本地，所以合法。
4. 不新建 crate。

## 后果
- 正面：三条规则都满足（core 只依赖 types；host/sandbox/vault 不链接 core；types 只放类型）；crate 数不变；适配器可以独立测试，也没有 core 的编译耦合；core 用内存 fake 测试不受影响。
- 负面：每个端口在 wand 里要多写一层转调（10 个端口，每个大概几十行）；端口签名和适配器 API 可能慢慢不一致，要靠 wand 编译期发现。
- 约束：适配器公开 API 只能用 wan-types 或自己的类型，不能用 core 类型；CI 的 cargo metadata 检查（A4）加一条「vault/host/acp/pages/sandbox 不得依赖 wan-core」。
- 重新评估信号：如果第三方插件或多个 bin 需要在不链接 core 的前提下实现同一套端口，再按第 9 节「何时再拆」新建 wan-ports。

## 考虑过的其他方案
- **端口放进 wan-types**：零新增实体，但 types 会带上 async-trait/行为，并且被 specta/schemars 生成和 TS 协议共享，破坏「协议唯一来源、只放类型」，不采用。
- **core 依赖适配器（适配器自有 API）**：codex-rs 就是这样：codex-core 直接依赖 codex-sandboxing、codex-exec-server、codex-rmcp-client、codex-login 等；codex-linux-sandbox 只依赖 codex-protocol、codex-sandboxing，codex-core 只是它的 dev-dependency。方向上不违反「host 不链接 core」，但违反「core 只依赖 types」，core 会把 chromiumoxide、SSH 等依赖带进编译，fake 测试也要绕开具体实现，不采用。
- **新建 wan-ports crate**：最标准的六边形做法，适配器直接 impl trait，不需要粘合层；代价是多一个实体，而当前粘合成本低，暂时没有必要。保留为重新评估信号触发后的方案。
- **core 用泛型 / 闭包端口**（例如 `Box<dyn Fn>` 注入）：不需要 newtype，但 10 个端口、多方法的情况下可读性差，不采用；个别单方法端口（Clock、IdGen）可以这样做。
