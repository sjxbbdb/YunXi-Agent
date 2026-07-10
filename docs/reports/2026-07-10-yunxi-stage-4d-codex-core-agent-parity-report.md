# YunXi Stage 4D Codex Core Agent Parity Report

## 核心结论

Stage 4C 已经证明：

```text
默认 YunXi CLI 可以在禁用 vendor/codex-rs 后构建、测试和运行。
```

但 Stage 4C 还没有复刻 Codex CLI 端的完整核心 Agent 能力。下一阶段
Stage 4D 的目标应当明确收窄为：

```text
照 Codex CLI 端源码机械移植核心 Agent 能力，让 YunXi 默认运行路径在
不依赖上游 Codex crate 的前提下具备 Codex CLI 的核心 Agent 行为。
```

模型层不作为本阶段的深度复刻目标。YunXi 只保留 provider 抽象、请求/
响应/流式事件协议、工具调用协议和可替换模型接口。后续接入其他 Agent
模型接口时，只替换 provider adapter，不改 Agent runtime、tools、storage、
approval、sandbox、AGENTS.md、MCP、skills、session 等核心能力。

## 当前基线

已经完成：

- 默认 `yunxi-agent-cli` 不依赖 `yunxi-agent-codex`。
- 默认 `cargo tree -p yunxi-agent-cli` 不包含 `codex-*`、`vendor/codex-rs`
  或 `yunxi-agent-codex`。
- 禁用 `vendor/codex-rs` 后，默认测试、check、build、smoke 均通过。
- `yunxi-agent-runtime` 已有 YunXi 自有 provider/tool loop。
- `yunxi-agent-provider` 已有 OpenAI-compatible request/response fixture 层。
- `yunxi-agent-tools` 已有 shell policy gate 和 constrained patch。
- `yunxi-agent-storage` 已有 file-backed session record。
- `yunxi-agent-cli` 已有 `sessions list` 和 `sessions show`。

仍缺失的 Codex CLI 核心 Agent 能力：

- 完整 Codex turn/session/thread runtime。
- 完整 AGENTS.md 加载、合并、缓存和注入。
- 完整 system/developer/user prompt 构造和上下文装配。
- 上下文窗口、token budget、compact、history restore。
- 完整 streaming model response loop 和 tool-call dispatch。
- 完整 shell/unified exec、stdin 写入、输出截断、pty/terminal 行为。
- 完整 approval、exec policy、sandbox policy、网络策略和提权路径。
- 完整 apply_patch parser/runtime，而不是 Stage 4C 的受限 JSON patch。
- 完整 MCP runtime、资源读取、工具发现、认证和 elicitation。
- 完整 skills/runtime/plugin/tool-search/view-image/request-user-input 等工具面。
- 完整 rollout、thread-store、message-history、resume/fork/archive/pin。
- Codex CLI 端的多 agent/spawn/wait/message/followup/interrupt 能力。
- 与 Codex CLI 事件协议等价的运行时事件和 JSONL 输出。

## Stage 4D 目标

Stage 4D 不是“继续小修小补”，而是一次 Codex core agent parity 移植。

完成条件：

- YunXi 默认 backend 具备 Codex CLI 端核心 Agent 行为。
- 默认 YunXi crate 不依赖 `vendor/codex-rs` 或任何 `codex-*` crate。
- `vendor/codex-rs` 只作为源码参考和对照测试输入存在。
- 能删除或重命名 `vendor/codex-rs` 后通过默认构建验证。
- 模型 provider 可替换；runtime 不绑定 OpenAI 或 Codex provider。
- Codex CLI 端核心 agent fixture/行为测试被迁移到 YunXi 自有测试。
- CLI 用户体验至少覆盖 headless Agent 的核心运行、工具、session 和恢复路径。

## 复刻策略

采用“先照搬、后去 Codex 化”的策略。

第一原则：

```text
能机械移植的先机械移植，不在 Stage 4D 中重新发明 Agent runtime。
```

执行方式：

- 从 `vendor/codex-rs` 逐模块复制核心逻辑到 YunXi 自有 crate。
- 保留算法、状态机、错误边界、测试 fixture 和关键行为。
- 修改命名空间、public facade 和 crate 边界，避免默认依赖 `codex-*` crate。
- 对必须复制的源码保留许可证和来源说明。
- 先让行为等价，再逐步清理命名、拆文件和去 Codex 化。
- 模型 provider 层保持抽象，只复刻请求/响应/stream/tool-call 协议边界。

不推荐的方式：

- 不要再手写简化版 runtime。
- 不要继续用一个 constrained patch 假装完整 patch 能力。
- 不要把 MCP、skills、session resume 延后到抽象层之外。
- 不要把 Codex compatibility crate 重新接回默认 CLI。

## Codex 源码到 YunXi 模块映射

### Runtime / Thread / Turn

Codex 来源：

- `vendor/codex-rs/core/src/codex_thread.rs`
- `vendor/codex-rs/core/src/thread_manager.rs`
- `vendor/codex-rs/core/src/session/*`
- `vendor/codex-rs/core/src/client_common.rs`
- `vendor/codex-rs/core/src/client.rs`
- `vendor/codex-rs/core/src/responses_retry.rs`
- `vendor/codex-rs/core/src/event_mapping.rs`
- `vendor/codex-rs/core/src/turn_metadata.rs`

YunXi 目标：

- `crates/yunxi-agent-runtime`
- 必要时新增 `crates/yunxi-agent-protocol`

迁移内容：

- thread/session 生命周期。
- turn queue、input queue、idle/start/interrupt 状态。
- provider stream event loop。
- tool-call scheduling。
- final response 聚合。
- runtime error/recovery。
- JSONL/event 输出映射。

### Prompt / Context / AGENTS.md

Codex 来源：

- `vendor/codex-rs/core/src/agents_md.rs`
- `vendor/codex-rs/core/src/agents_md_manager.rs`
- `vendor/codex-rs/core/src/prompt_debug.rs`
- `vendor/codex-rs/core/src/context/*`
- `vendor/codex-rs/core/src/context_manager/*`
- `vendor/codex-rs/context-fragments`
- `vendor/codex-rs/prompts`
- `vendor/codex-rs/file-search`

YunXi 目标：

- 新增 `crates/yunxi-agent-context`
- `crates/yunxi-agent-runtime`

迁移内容：

- `AGENTS.md` 查找、读取、层级合并和缓存。
- system/developer/user instructions 组装。
- workspace metadata、git info、文件搜索和 mention 解析。
- prompt debug 和上下文片段注入。

### Compact / History / Token Budget

Codex 来源：

- `vendor/codex-rs/core/src/compact.rs`
- `vendor/codex-rs/core/src/compact_remote.rs`
- `vendor/codex-rs/core/src/compact_remote_v2.rs`
- `vendor/codex-rs/core/src/compact_token_budget.rs`
- `vendor/codex-rs/message-history`
- `vendor/codex-rs/core/src/session/token_budget.rs`
- `vendor/codex-rs/core/src/session/context_window.rs`

YunXi 目标：

- `crates/yunxi-agent-context`
- `crates/yunxi-agent-storage`
- `crates/yunxi-agent-runtime`

迁移内容：

- message history reconstruction。
- context window 管理。
- compact trigger 和 compact request。
- token budget 计算。
- resume 时恢复历史和压缩状态。

### Tools / Router / Dispatch

Codex 来源：

- `vendor/codex-rs/core/src/tools/*`
- `vendor/codex-rs/core/src/function_tool.rs`
- `vendor/codex-rs/tools`
- `vendor/codex-rs/core/src/mcp_tool_call.rs`
- `vendor/codex-rs/core/src/mcp_tool_exposure.rs`

YunXi 目标：

- `crates/yunxi-agent-tools`
- 必要时新增 `crates/yunxi-agent-tool-router`

迁移内容：

- tool registry。
- tool schema/spec 生成。
- tool routing。
- tool lifecycle events。
- tool dispatch trace。
- failed/declined/completed 状态一致性。

### Shell / Exec / Sandbox / Approval

Codex 来源：

- `vendor/codex-rs/core/src/exec.rs`
- `vendor/codex-rs/core/src/unified_exec/*`
- `vendor/codex-rs/core/src/shell.rs`
- `vendor/codex-rs/core/src/shell_snapshot.rs`
- `vendor/codex-rs/core/src/command_canonicalization.rs`
- `vendor/codex-rs/core/src/exec_policy.rs`
- `vendor/codex-rs/exec`
- `vendor/codex-rs/exec-server`
- `vendor/codex-rs/execpolicy`
- `vendor/codex-rs/sandboxing`
- `vendor/codex-rs/linux-sandbox`
- `vendor/codex-rs/windows-sandbox-rs`
- `vendor/codex-rs/shell-command`
- `vendor/codex-rs/shell-escalation`

YunXi 目标：

- `crates/yunxi-agent-tools`
- 新增 `crates/yunxi-agent-sandbox`
- 新增 `crates/yunxi-agent-exec`

迁移内容：

- shell command parsing/canonicalization。
- exec command lifecycle。
- stdin 写入。
- stdout/stderr aggregation。
- output truncation。
- workspace snapshot 和 diff。
- approval request/decision。
- sandbox policy。
- Windows/Linux sandbox runner。
- network policy。
- escalation path。

### Patch

Codex 来源：

- `vendor/codex-rs/core/src/apply_patch.rs`
- `vendor/codex-rs/core/src/tools/handlers/apply_patch.rs`
- `vendor/codex-rs/core/src/tools/handlers/apply_patch.lark`
- `vendor/codex-rs/core/src/tools/runtimes/apply_patch.rs`
- `vendor/codex-rs/apply-patch`
- `vendor/codex-rs/git-utils/src/apply.rs`

YunXi 目标：

- `crates/yunxi-agent-tools`
- 新增 `crates/yunxi-agent-patch`

迁移内容：

- Codex apply_patch grammar。
- patch parser。
- file add/update/delete。
- patch failure diagnostics。
- changed-file tracking。
- patch event parity。

### MCP

Codex 来源：

- `vendor/codex-rs/codex-mcp`
- `vendor/codex-rs/rmcp-client`
- `vendor/codex-rs/core/src/mcp.rs`
- `vendor/codex-rs/core/src/mcp_tool_call.rs`
- `vendor/codex-rs/core/src/mcp_tool_approval_templates.rs`
- `vendor/codex-rs/core/src/mcp_openai_file.rs`
- `vendor/codex-rs/core/src/session/mcp.rs`
- `vendor/codex-rs/core/src/session/mcp_runtime.rs`

YunXi 目标：

- 新增 `crates/yunxi-agent-mcp`
- `crates/yunxi-agent-tools`
- `crates/yunxi-agent-runtime`

迁移内容：

- MCP server config。
- stdio/http MCP connection。
- list/read resources。
- tool search。
- tool invocation。
- auth/elicitation。
- MCP sandbox state。
- MCP approval flow。

### Skills / Plugins / Dynamic Tools

Codex 来源：

- `vendor/codex-rs/core/src/skills.rs`
- `vendor/codex-rs/core-skills`
- `vendor/codex-rs/skills`
- `vendor/codex-rs/plugin`
- `vendor/codex-rs/core-plugins`
- `vendor/codex-rs/core/src/plugins/*`
- `vendor/codex-rs/core/src/tools/handlers/dynamic.rs`
- `vendor/codex-rs/core/src/tools/handlers/tool_search.rs`
- `vendor/codex-rs/core/src/tools/handlers/view_image.rs`
- `vendor/codex-rs/core/src/tools/handlers/request_user_input.rs`

YunXi 目标：

- 新增 `crates/yunxi-agent-skills`
- 新增 `crates/yunxi-agent-plugins`
- `crates/yunxi-agent-tools`

迁移内容：

- skill discovery。
- skill metadata。
- skill injection。
- implicit/explicit skill invocation。
- dynamic tool metadata。
- tool search。
- request user input。
- view image。
- plugin config/cache。

### Storage / Rollout / Resume

Codex 来源：

- `vendor/codex-rs/core/src/rollout.rs`
- `vendor/codex-rs/core/src/rollout_budget.rs`
- `vendor/codex-rs/core/src/thread_rollout_truncation.rs`
- `vendor/codex-rs/thread-store`
- `vendor/codex-rs/state`
- `vendor/codex-rs/core/src/state_db_bridge.rs`
- `vendor/codex-rs/external-agent-sessions`

YunXi 目标：

- `crates/yunxi-agent-storage`

迁移内容：

- thread metadata。
- rollout recorder。
- session listing。
- archive/unarchive。
- pin/fork/resume。
- thread path lookup。
- session truncation。
- state DB bridge 或 YunXi 等价存储。

### Multi-Agent

Codex 来源：

- `vendor/codex-rs/core/src/agent/*`
- `vendor/codex-rs/core/src/agent_communication.rs`
- `vendor/codex-rs/core/src/tools/handlers/multi_agents/*`
- `vendor/codex-rs/core/src/tools/handlers/multi_agents_v2/*`
- `vendor/codex-rs/agent-graph-store`

YunXi 目标：

- 新增 `crates/yunxi-agent-multi-agent`
- `crates/yunxi-agent-runtime`
- `crates/yunxi-agent-storage`
- `crates/yunxi-agent-tools`

迁移内容：

- spawn agent。
- wait agent。
- send message。
- follow-up task。
- interrupt agent。
- list agents。
- agent graph。
- sub-agent session/source inheritance。
- rollout budget 共享。

## 建议目标架构

Stage 4D 后，默认 YunXi workspace 应收敛为：

```text
yunxi-agent-cli
  -> yunxi-agent-core
  -> yunxi-agent-runtime
       -> yunxi-agent-protocol
       -> yunxi-agent-provider
       -> yunxi-agent-context
       -> yunxi-agent-tools
       -> yunxi-agent-exec
       -> yunxi-agent-sandbox
       -> yunxi-agent-patch
       -> yunxi-agent-mcp
       -> yunxi-agent-skills
       -> yunxi-agent-storage
       -> yunxi-agent-multi-agent
```

仍然禁止出现在默认依赖图：

```text
yunxi-agent-codex
vendor/codex-rs/*
codex-* crates
```

`vendor/codex-rs` 可以继续留在仓库中作为源码参考，但默认构建必须在它被重
命名或删除时仍然通过。

## Stage 4D 实施切片

### Stage 4D.1: Source Parity Inventory

目的：

建立 Codex core agent 能力到 YunXi crate 的逐文件迁移索引。

产物：

- `docs/extraction-index/codex-core-agent-parity-map.md`
- 每个 Codex 源文件对应：
  - 能力分类
  - YunXi 目标 crate
  - 是否必须机械移植
  - 是否只需 fixture/测试
  - 是否属于非核心产品面

验收：

- 所有 `vendor/codex-rs/core/src` 下核心 agent 文件都有归类。
- `exec`、`tools`、`apply-patch`、`codex-mcp`、`skills`、`thread-store`、
  `rollout`、`message-history` 均有迁移目标。

### Stage 4D.2: Protocol And Event Parity

目的：

先把 Codex CLI agent 的事件、tool call、input item、response item 抽成
YunXi 协议层。

产物：

- `yunxi-agent-protocol`
- YunXi-owned input/event/tool-call/response item types
- JSONL serialization parity tests

验收：

- Codex fixture event 可以解析为 YunXi event。
- YunXi event 可以输出稳定 JSONL。
- 不依赖 `codex-protocol`。

### Stage 4D.3: Runtime Thread And Turn Parity

目的：

机械移植 Codex thread/session/turn 状态机。

产物：

- `yunxi-agent-runtime` 内的 thread manager、turn loop、input queue、status。
- session source、interrupt、idle/start turn 行为。

验收：

- 可启动新 thread。
- 可执行多 turn。
- 可 interrupt。
- 可从 tool result 回到 provider。
- 事件序列与 Codex fixture 对齐。

### Stage 4D.4: Prompt, AGENTS.md, Context, Compact

目的：

复刻 Codex 的上下文装配能力。

产物：

- `yunxi-agent-context`
- AGENTS.md loader/manager
- prompt/context fragment builder
- token budget/context window
- compact/history reconstruction

验收：

- 多层 AGENTS.md 合并顺序正确。
- prompt fixture 与 Codex 输出等价。
- 超上下文时触发 compact。
- resume 后可恢复历史上下文。

### Stage 4D.5: Tool Registry And Router Parity

目的：

复刻 Codex tool registry、schema、router、lifecycle。

产物：

- YunXi tool registry。
- tool spec/schema builder。
- dispatch trace。
- tool lifecycle event。

验收：

- shell、patch、MCP、skills、multi-agent 工具可统一注册和路由。
- tool schema fixture 与 Codex 等价。

### Stage 4D.6: Shell, Exec, Approval, Sandbox Parity

目的：

把 Stage 4C 简化 shell policy 升级为 Codex 等价的 exec/sandbox/approval
执行层。

产物：

- `yunxi-agent-exec`
- `yunxi-agent-sandbox`
- command canonicalization
- exec policy
- sandbox runner
- stdin/output/truncation
- network policy

验收：

- shell command 支持 start/update/completed event。
- approval required/denied/approved 路径齐全。
- workspace-write/read-only/danger-full-access 行为可测。
- Windows 路径和 sandbox 行为有 fixture 或平台测试。

### Stage 4D.7: Full Apply Patch Parity

目的：

用 Codex apply_patch grammar 和 runtime 替换 Stage 4C constrained patch。

产物：

- `yunxi-agent-patch`
- apply_patch parser
- patch runtime
- diagnostics
- file-change events

验收：

- Codex apply_patch fixture 全部通过。
- add/update/delete/move/error diagnostics 可测。
- runtime patch event 与 Codex CLI 行为等价。

### Stage 4D.8: MCP Parity

目的：

复刻 Codex MCP runtime。

产物：

- `yunxi-agent-mcp`
- MCP config
- stdio/http client
- resource list/read
- tool discovery/invocation
- MCP approval/elicitation

验收：

- 本地 mock MCP server 可完成 list/read/call。
- MCP tool 可被 runtime 调用并返回 provider loop。
- 默认测试不需要外部网络。

### Stage 4D.9: Skills And Plugin Runtime Parity

目的：

复刻 Codex skills/plugin/dynamic tool 能力。

产物：

- `yunxi-agent-skills`
- `yunxi-agent-plugins`
- skill discovery/loading/injection
- dynamic tool metadata
- tool_search/request_user_input/view_image

验收：

- skill fixture 可加载。
- 显式 skill mention 可触发注入。
- 隐式 skill invocation 事件可生成。
- dynamic tools 可被 provider 看到并调用。

### Stage 4D.10: Storage, Rollout, Resume, Fork

目的：

把 Stage 4C 简单 session JSON 升级为 Codex 等价 session/thread/rollout
存储。

产物：

- thread metadata
- rollout recorder
- session list/show/resume/archive/pin/fork
- history restore
- thread truncation

验收：

- CLI 可 resume 指定 session。
- 可 list/show/archive/pin/fork。
- rollout 可重建 turn history。
- 禁用 `vendor/codex-rs` 后仍通过。

### Stage 4D.11: Multi-Agent Parity

目的：

复刻 Codex CLI 的多 agent 工具体系。

产物：

- `yunxi-agent-multi-agent`
- spawn/wait/message/followup/interrupt/list
- agent graph store
- sub-agent runtime inheritance

验收：

- 主 agent 可 spawn 子 agent。
- wait 可收集结果。
- message/followup 可触发子 agent turn。
- 子 agent session 可持久化并恢复。

### Stage 4D.12: CLI Core Parity

目的：

把上面能力接入 `yunxi-agent-cli`，形成 Codex CLI headless agent 等价默认
路径。

产物：

- CLI flags 对齐核心 agent 行为。
- JSON/JSONL 输出稳定。
- session resume/list/show/archive/pin/fork。
- approval prompt 或非交互 approval policy。
- provider selection 只经过 YunXi provider abstraction。

验收：

- 用户可用 YunXi CLI 完成 Codex CLI 常见 headless agent 工作流。
- 不通过 `yunxi-agent-codex`。

## 验证门槛

Stage 4D 仍遵守用户确认过的硬约束：

```text
先构建，中间无需反复验证；构建完成后统一验证。
不要在一个点上浪费大量时间。
```

统一验证命令：

```powershell
cargo fmt -- --check
cargo test
cargo check -p yunxi-agent-core
cargo check -p yunxi-agent-protocol
cargo check -p yunxi-agent-provider
cargo check -p yunxi-agent-context
cargo check -p yunxi-agent-tools
cargo check -p yunxi-agent-exec
cargo check -p yunxi-agent-sandbox
cargo check -p yunxi-agent-patch
cargo check -p yunxi-agent-mcp
cargo check -p yunxi-agent-skills
cargo check -p yunxi-agent-storage
cargo check -p yunxi-agent-runtime
cargo check -p yunxi-agent-cli
cargo build -p yunxi-agent-cli
cargo run -p yunxi-agent-cli -- --cwd <temp> --backend yunxi --jsonl "explain this project"
cargo tree -p yunxi-agent-cli
git diff --check
```

独立性验证：

```powershell
Rename-Item vendor\codex-rs vendor\codex-rs.disabled
cargo test
cargo check -p yunxi-agent-cli
cargo build -p yunxi-agent-cli
cargo tree -p yunxi-agent-cli
Rename-Item vendor\codex-rs.disabled vendor\codex-rs
```

依赖断言：

```text
默认 cargo tree 不得包含 yunxi-agent-codex、vendor/codex-rs 或 codex-* crate。
```

行为断言：

- Codex fixture 测试迁移到 YunXi 后通过。
- YunXi CLI 默认路径可以跑完整 agent loop。
- shell、patch、MCP、skills、AGENTS.md、resume、compact、多 agent 至少各有
  一个端到端 fixture。

## 风险和处理

源码量过大：

Codex core 能力很大，Stage 4D 必须按能力域机械迁移，不应边迁移边重构。
每个切片只做命名空间替换、依赖替换、测试迁移和最小编译修复。

默认依赖回流：

任何直接引用 `codex-*` crate 的做法都可能破坏独立性。迁移时允许看
`vendor/codex-rs`，但不允许默认 crate 依赖它。

模型接口变化：

模型层保留 provider abstraction。OpenAI-compatible 只是当前 adapter，后续
其他 Agent 模型接口通过 adapter 接入。

许可证和来源：

机械移植 Codex 源码时必须保留许可证合规信息和来源说明。YunXi 后续可以再
做命名、边界和实现细节的自主化重写。

Windows 平台复杂度：

Windows sandbox、path normalization、PowerShell/cmd 行为要优先保留 Codex
逻辑，不要用简单 POSIX 假设替代。

## 非目标

- 不复刻 TUI。
- 不复刻桌面 app。
- 不复刻云任务产品面。
- 不复刻 marketplace/update/doctor/completion/release installer。
- 不把 `yunxi-agent-codex` 接回默认 CLI。
- 不在 Stage 4D 中重写 provider 生态；只保留可替换 provider 边界。

## 建议下一步

下一步直接进入 Stage 4D.1：

```text
建立 Codex core agent parity map。
```

先把 `vendor/codex-rs` 中与核心 Agent 能力相关的源码逐项索引到 YunXi 目标
crate，再按这个索引进行机械迁移。这样可以避免继续在单点上反复纠结，也能
保证最终目标没有被稀释：

```text
完整复刻 Codex CLI 端核心 Agent 能力，同时保持 YunXi 默认运行不依赖上游源码。
```

## Implementation Update: Foundation Slice

Stage 4D implementation has started with a build-first foundation slice.

Implemented in this slice:

- Added `docs/extraction-index/codex-core-agent-parity-map.md` as the source
  migration index for Codex core agent parity.
- Added `yunxi-agent-protocol` for YunXi-owned runtime protocol, response item,
  tool-call, and JSONL event types.
- Added `yunxi-agent-context` for AGENTS.md hierarchy loading and context bundle
  assembly.
- Added `yunxi-agent-sandbox` for approval, sandbox, cwd, and network policy
  decisions.
- Added `yunxi-agent-exec` for command canonicalization and output aggregation
  primitives.
- Added `yunxi-agent-patch` for constrained JSON patch and Codex-style
  `*** Begin Patch` application.
- Added `yunxi-agent-mcp` for MCP configuration, resource, and tool invocation
  interfaces.
- Added `yunxi-agent-skills` for skill discovery, metadata, and invocation
  interfaces.
- Added `yunxi-agent-multi-agent` for multi-agent command and registry
  interfaces.
- Routed `yunxi-agent-tools` patch execution through `yunxi-agent-patch`.
- Routed shell output aggregation through `yunxi-agent-exec`.
- Routed runtime startup through `yunxi-agent-context` so AGENTS.md
  instructions enter provider messages before the user prompt.
- Added provider-to-protocol tool-call conversion.
- Added storage rollout/thread metadata types.
- Added `yunxi-agent-cli parity map` to expose the parity index from the CLI.

This slice does not claim full Codex CLI core parity. It creates the autonomous
YunXi-owned surfaces required to continue mechanical migration without adding
`codex-*` or `vendor/codex-rs` dependencies to the default graph.

Verification status:

- Unified post-build verification passed on 2026-07-10.
- Disabled-vendor independence verification passed on 2026-07-10 after
  temporarily renaming `vendor/codex-rs` to `vendor/codex-rs.disabled`.
- Default `cargo tree -p yunxi-agent-cli` contains no `yunxi-agent-codex`,
  `vendor/codex-rs`, or `codex-*` crates.

Commands verified:

- `cargo fmt -- --check`
- `cargo test`
- `cargo check -p yunxi-agent-core`
- `cargo check -p yunxi-agent-protocol`
- `cargo check -p yunxi-agent-provider`
- `cargo check -p yunxi-agent-context`
- `cargo check -p yunxi-agent-sandbox`
- `cargo check -p yunxi-agent-exec`
- `cargo check -p yunxi-agent-patch`
- `cargo check -p yunxi-agent-tools`
- `cargo check -p yunxi-agent-mcp`
- `cargo check -p yunxi-agent-skills`
- `cargo check -p yunxi-agent-multi-agent`
- `cargo check -p yunxi-agent-storage`
- `cargo check -p yunxi-agent-runtime`
- `cargo check -p yunxi-agent-cli`
- `cargo build -p yunxi-agent-cli`
- `cargo run -p yunxi-agent-cli -- --cwd <temp> --backend yunxi --jsonl "explain this project"`
- `cargo run -p yunxi-agent-cli -- parity map`
- `cargo tree -p yunxi-agent-cli`
- `git diff --check`
