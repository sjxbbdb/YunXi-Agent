# YunXi Stage 4E Core Agent Parity Gap Development Report

## 核心结论

当前 YunXi Agent 已经完成了从 Codex CLI 上游源码依赖中剥离出来的关键基础工作：

- 默认 `yunxi-agent-cli` 不依赖 `yunxi-agent-codex`。
- 默认依赖图不包含 `codex-*` crate。
- 临时禁用 `vendor/codex-rs` 后，默认构建、测试、CLI 构建和依赖树验证可以通过。
- YunXi 已经具备自有 runtime、provider 边界、session/history、AGENTS.md 注入、基础 compact、shell/patch 工具、工具注册表、工具 schema、工具 router 和 dispatch trace。

但这还不是 Codex CLI 核心 Agent 能力的完整复刻。当前状态应定义为：

```text
YunXi 已具备独立 Agent 骨架和部分核心能力，但仍缺 Codex CLI 深层 agent runtime、streaming、exec、sandbox、MCP、skills、multi-agent、rollout 和完整协议 parity。
```

Stage 4E 的目标不是重新讨论方向，而是把缺口收敛成可执行迁移清单：

```text
继续以 vendor/codex-rs 为源码参考，先整体抽取和复刻 Codex CLI 核心 Agent 能力，再逐步去 Codex 化；默认 YunXi 运行路径仍不得依赖上游源码或 codex-* crate。
```

## 当前基线

最近完成的 Stage 4D 进展：

- `yunxi-agent-tools` 新增 YunXi 自有 `ToolRegistry`、`ToolSpec`、`ToolRouter`、`ToolDispatchTrace`。
- provider 请求中的工具 schema 已从 `yunxi-agent-tools` 默认注册表生成，不再在 provider 中硬编码 shell/patch。
- runtime 工具调用路径已变为：provider tool call -> YunXi `ToolRequest` -> YunXi router -> dispatch trace -> tool runtime。
- 默认模型可见工具包括 `shell`、`patch`、`mcp`、`skill`。
- `mcp/skill arguments_json` 解析已避免二次 JSON 引号包裹。
- 统一验证和禁用 vendor 独立性验证已通过。

这说明 YunXi 的默认路径已经可以脱离上游源码运行，但还不能说具备 Codex CLI 的全部 Agent 核心能力。

## 12 个缺口层面

### 1. 完整 Turn Loop / Streaming 协议层

当前状态：

- `yunxi-agent-runtime` 已有基础 provider/tool loop。
- 能把 provider 返回的 tool call 路由到工具 runtime，再把 tool result 送回 provider。
- runtime 事件仍是简化的同步回合事件。

缺口：

- Codex 的 response item 流式处理尚未完整复刻。
- 缺少增量 assistant message、增量 reasoning、增量 tool call、tool result streaming。
- 缺少 turn metadata、取消、中断、恢复、provider retry 和 event mapping parity。
- 缺少 Codex CLI JSONL 对 response item 的完整稳定映射。

目标产物：

- 在 `yunxi-agent-protocol` 中补齐 input item、response item、stream event、turn metadata。
- 在 `yunxi-agent-runtime` 中补齐 streaming turn loop。
- 在 `yunxi-agent-cli` 中补齐 JSONL 输出 parity。

优先级：高。

### 2. 真实模型 Provider 传输层

当前状态：

- `yunxi-agent-provider` 有 OpenAI-compatible request JSON 和 response fixture parser。
- provider 边界已经独立，后期可以替换其他模型接口。
- 真实 HTTP/SSE transport 还没有成为默认可用能力。

缺口：

- 缺少 HTTP client transport。
- 缺少 SSE streaming parser。
- 缺少 retry/backoff、timeout、rate-limit、错误分类。
- 缺少模型能力协商，例如是否支持工具、reasoning、parallel tool calls、stream usage。
- 缺少 provider 级别的稳定 fixture 和 mock server。

目标产物：

- `yunxi-agent-provider` 增加 transport trait 和 OpenAI-compatible HTTP adapter。
- 保持模型层可替换，不把 runtime 绑定到 OpenAI。
- 所有 live provider 验证保持 opt-in，不进入默认测试。

优先级：高。

### 3. Shell / Exec 深度能力

当前状态：

- `yunxi-agent-tools` 能执行 shell。
- `yunxi-agent-exec` 已有命令规范化和输出聚合基础。
- 能报告命令状态、输出和文件变化。

缺口：

- 缺少 Codex 的 unified exec 生命周期。
- 缺少实时 stdout/stderr event。
- 缺少 stdin 写入。
- 缺少 timeout、进程中断、长任务管理。
- 缺少 shell snapshot、PTY/terminal 行为、环境变量处理。
- 输出截断策略还只是基础版本。

目标产物：

- 把 `vendor/codex-rs/core/src/exec.rs`、`unified_exec/**`、`shell.rs`、`shell_snapshot.rs`、`command_canonicalization.rs` 抽到 `yunxi-agent-exec`。
- runtime 工具事件从一次性完成升级到 start/update/completed 事件链。

优先级：最高。

### 4. Sandbox / Approval 完整策略

当前状态：

- 已有 `ToolPolicy`、`ApprovalDecision`、`SandboxPolicy`。
- 能根据 `AgentConfig` 拒绝需要 approval 或越界 workspace 的工具请求。

缺口：

- 缺少 Codex exec policy 的完整风险判定。
- 缺少 approval request/response 生命周期。
- 缺少 escalation 路径。
- 缺少 network policy。
- 缺少 Windows/Linux 平台 sandbox runner。
- 缺少命令风险模板和 MCP approval template。

目标产物：

- 抽取 `vendor/codex-rs/core/src/exec_policy.rs`、`execpolicy/**`、`sandboxing/**`、`windows-sandbox-rs/**`、`linux-sandbox/**`、`shell-escalation/**` 到 `yunxi-agent-sandbox`。
- `yunxi-agent-runtime` 增加 approval event 和非交互 policy 行为。

优先级：最高。

### 5. Patch 完整复刻

当前状态：

- `yunxi-agent-patch` 支持 constrained JSON patch。
- 已支持一部分 Codex-style `*** Begin Patch` add/update/delete。
- patch tool 已接入 runtime。

缺口：

- Codex apply_patch grammar 尚未完整复刻。
- 缺少完整 Lark grammar 资产。
- 缺少复杂 update hunk、move、EOF 行、错误恢复、精确 diagnostics。
- 缺少与 Codex fixture 对齐的错误提示。

目标产物：

- 抽取 `vendor/codex-rs/core/src/apply_patch.rs`、`tools/handlers/apply_patch.rs`、`apply_patch.lark`、`tools/runtimes/apply_patch.rs`、`apply-patch/**` 到 `yunxi-agent-patch`。
- 用 Codex fixture 建立 add/update/delete/move/error parity 测试。

优先级：高。

### 6. Context / Prompt / Compact 深度复刻

当前状态：

- `yunxi-agent-context` 已有 AGENTS.md hierarchy loading。
- runtime 会把 AGENTS.md 注入 provider messages。
- 已有 session history restore、近似 token budget 和 deterministic compact summary。

缺口：

- 缺少 Codex prompt assets。
- 缺少 context fragments。
- 缺少完整 prompt assembly。
- 缺少准确 tokenizer。
- 缺少 compact remote prompt、compact token budget、context window 状态机。
- 缺少 file search、mention 解析、workspace metadata、prompt debug。

目标产物：

- 抽取 `agents_md.rs`、`agents_md_manager.rs`、`context/**`、`context_manager/**`、`context-fragments/**`、`prompts/**`、`compact*.rs`、`compact_token_budget.rs` 到 `yunxi-agent-context`。
- 建立 prompt fixture parity 测试。

优先级：高。

### 7. MCP 真实调用层

当前状态：

- `yunxi-agent-mcp` 有配置、resource、tool invocation 的接口轮廓。
- provider/tool schema 中已暴露 `mcp`。
- runtime 能识别 MCP tool request，但默认执行仍是 declined。

缺口：

- 缺少 MCP client。
- 缺少 stdio/http connection。
- 缺少 server config loading。
- 缺少 list/read resources。
- 缺少 tool discovery 和 tool invocation。
- 缺少 MCP auth、elicitation、approval template、sandbox state。

目标产物：

- 抽取 `codex-mcp/**`、`rmcp-client/**`、`core/src/mcp.rs`、`mcp_tool_call.rs`、`mcp_tool_approval_templates.rs`、`session/mcp*.rs` 到 `yunxi-agent-mcp`。
- 用本地 mock MCP server 做端到端 fixture。

优先级：高。

### 8. Skills / Plugins / Dynamic Tools

当前状态：

- `yunxi-agent-skills` 有基础 metadata/discovery 接口。
- provider/tool schema 中已暴露 `skill`。
- runtime 能识别 skill tool request，但默认执行仍是 declined。

缺口：

- 缺少完整 skill loading。
- 缺少 skill instruction 注入。
- 缺少显式/隐式 skill invocation。
- 缺少 plugin manifest/cache。
- 缺少 dynamic tool metadata。
- 缺少 `tool_search`、`request_user_input`、`view_image` 等工具 handler。

目标产物：

- 抽取 `core/src/skills.rs`、`core-skills/**`、`skills/**`、`plugin/**`、`core-plugins/**`、`plugins/**`、`tools/handlers/tool_search.rs`、`request_user_input.rs`、`view_image.rs`。
- 必要时新建 `yunxi-agent-plugins`，避免 `yunxi-agent-skills` 变成过大 crate。

优先级：高。

### 9. Multi-Agent 能力

当前状态：

- `yunxi-agent-multi-agent` 有命令和 registry 的基础接口。
- runtime/tool registry 还没有真实接入 multi-agent handlers。

缺口：

- 缺少 spawn/wait/message/follow-up/interrupt/list。
- 缺少 agent graph store。
- 缺少 sub-agent runtime inheritance。
- 缺少 rollout budget 共享。
- 缺少 multi-agent tool schema 和 lifecycle event。

目标产物：

- 抽取 `core/src/agent/**`、`agent_communication.rs`、`tools/handlers/multi_agents/**`、`multi_agents_v2/**`、`agent-graph-store/**` 到 `yunxi-agent-multi-agent`。
- 在 `yunxi-agent-tools` 注册 multi-agent tool specs。

优先级：中高。

### 10. Storage / Rollout / Resume 完整事件回放

当前状态：

- `yunxi-agent-storage` 已有 session record、history、resume/fork/archive/pin 基础。
- CLI 已有 session lifecycle 命令。

缺口：

- 缺少 Codex rollout event parity。
- 缺少完整 JSONL rollout recorder。
- 缺少 thread store parity。
- 缺少 rollout truncation。
- 缺少 state DB bridge 或 YunXi 等价存储。
- resume/fork 仍是简化历史恢复，不是完整 runtime state reconstruction。

目标产物：

- 抽取 `rollout.rs`、`rollout_budget.rs`、`thread_rollout_truncation.rs`、`thread-store/**`、`state/**`、`state_db_bridge.rs` 到 `yunxi-agent-storage`。
- CLI 的 `sessions history`、`sessions rollout` 与 runtime event store 对齐。

优先级：中高。

### 11. Protocol / Event Mapping 全量对齐

当前状态：

- `yunxi-agent-core` 和 `yunxi-agent-protocol` 已有基础事件、tool call 和 JSONL 类型。
- 已覆盖 command、patch、MCP、file changed、completed 等基础事件。

缺口：

- 缺少完整 Codex input item / response item。
- 缺少 tool lifecycle event parity。
- 缺少 approval event parity。
- 缺少 provider streaming event parity。
- 缺少 fixture-backed event mapping。
- 缺少稳定协议版本边界。

目标产物：

- 抽取 `core/src/event_mapping.rs`、`client_common.rs`、`turn_metadata.rs` 中的协议映射到 `yunxi-agent-protocol`。
- 所有 CLI JSONL 输出从 `yunxi-agent-protocol` 统一输出。

优先级：最高。

### 12. CLI 行为细节复刻

当前状态：

- `yunxi-agent-cli` 有基础 prompt、backend selection、JSONL、parity map、session commands。
- default backend 已不依赖 Codex 上游源码。

缺口：

- Codex CLI headless 的 flag、输出模式、错误码、resume 体验还未完全对齐。
- approval prompt 交互未完整实现。
- provider selection/config loading 不完整。
- 非 TUI 核心命令仍有缺口。
- JSONL 输出还未等价 Codex 核心事件协议。

目标产物：

- 只复刻 headless Agent 核心 CLI。
- 继续排除 TUI、desktop、cloud tasks、marketplace、update、doctor、completion、release installer。
- CLI 行为以 YunXi 自有 runtime 为唯一默认路径。

优先级：中高。

## 推荐实施顺序

### Stage 4E.1: Protocol / Event / Streaming 底座

先补协议，否则后面 exec、MCP、skills、multi-agent 的事件都会返工。

范围：

- `yunxi-agent-protocol`
- `yunxi-agent-runtime`
- `yunxi-agent-provider`
- `yunxi-agent-cli` JSONL 输出

验收：

- YunXi 拥有 input item、response item、stream event、turn metadata。
- provider fixture 可以产生 streaming response items。
- CLI JSONL 由 protocol 统一输出。

### Stage 4E.2: Exec / Sandbox / Approval 深度复刻

这是 Agent 真正能自主工作和安全执行的底座。

范围：

- `yunxi-agent-exec`
- `yunxi-agent-sandbox`
- `yunxi-agent-tools`
- `yunxi-agent-runtime`

验收：

- shell 支持 start/update/completed 事件。
- approval required/approved/declined 路径完整。
- read-only/workspace-write/danger-full-access 行为有 fixture。
- Windows 路径和 shell 行为有独立测试。

### Stage 4E.3: Patch / Context / Compact 完整复刻

这一步提升代码修改和长上下文运行能力。

范围：

- `yunxi-agent-patch`
- `yunxi-agent-context`
- `yunxi-agent-storage`
- `yunxi-agent-runtime`

验收：

- Codex apply_patch grammar fixture 通过。
- AGENTS.md、prompt assets、context fragments、compact fixture 通过。
- resume 后 context reconstruction 更接近 Codex 行为。

### Stage 4E.4: MCP / Skills / Dynamic Tools

这一步补齐 Codex CLI 的外部工具生态。

范围：

- `yunxi-agent-mcp`
- `yunxi-agent-skills`
- 可能新增 `yunxi-agent-plugins`
- `yunxi-agent-tools`
- `yunxi-agent-runtime`

验收：

- mock MCP server 端到端通过。
- skill fixture 可以加载、注入、触发。
- `tool_search`、`request_user_input`、`view_image` 进入 YunXi registry/router。

### Stage 4E.5: Storage / Rollout / Multi-Agent / CLI Parity

这一步补齐长任务、多 agent 和用户可见体验。

范围：

- `yunxi-agent-storage`
- `yunxi-agent-multi-agent`
- `yunxi-agent-cli`
- `yunxi-agent-runtime`

验收：

- rollout 可重放 turn history。
- spawn/wait/message/follow-up/interrupt/list 端到端通过。
- CLI resume/list/show/archive/pin/fork/history/rollout 与 runtime state 对齐。

## 硬约束

继续保持用户确认过的强约束：

```text
先构建，中间无需检验，不要在一个点上浪费大量时间；
构建完成之后统一检验；
核心指标是复刻所有 Codex CLI 端核心 Agent 能力；
默认 YunXi 不对上游源码产生依赖；
模型层面保留接口，后期替换其他 Agent 的模型层接口。
```

工程约束：

- `vendor/codex-rs` 只作为源码参考和 fixture 对照。
- 默认 crate graph 不得包含 `codex-*` crate。
- 默认 CLI 不得依赖 `yunxi-agent-codex`。
- 每个迁移能力都要有 YunXi 自有 fixture 或单元测试。
- 复制上游实质源码时保留许可证和来源说明。
- 不复刻 TUI、desktop app、cloud tasks、marketplace、update、doctor、completion、release installer。

## 统一验证门槛

每个阶段构建完成后统一执行：

```powershell
cargo fmt
cargo fmt -- --check
cargo test
cargo check --workspace
cargo build -p yunxi-agent-cli
cargo run -p yunxi-agent-cli -- parity map
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
默认 cargo tree -p yunxi-agent-cli 不得包含 yunxi-agent-codex、vendor/codex-rs 或 codex-* crate。
```

## 下一步建议

下一轮不建议再做泛泛的“还差什么”讨论，直接进入 Stage 4E.1：

```text
补齐 protocol / event mapping / provider streaming / runtime streaming turn loop。
```

原因：

- 这层是后续 exec、MCP、skills、multi-agent 的共同事件底座。
- 先补协议可以减少后续返工。
- 这层完成后，YunXi 的 Agent 行为会从“同步回合骨架”升级为“接近 Codex CLI 的流式 Agent runtime”。

完成 Stage 4E.1 后，再按 `exec/sandbox/approval`、`patch/context/compact`、`MCP/skills`、`storage/multi-agent/CLI` 的顺序推进。
