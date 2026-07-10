# YunXi Stage 4G Full Core Agent Parity One-Pass Development Report

生成时间：2026-07-10 13:13:33 +08:00

## 核心结论

Stage 4F 之后，YunXi Agent 已经具备默认自主运行路径，并且默认依赖树不包含
`vendor/codex-rs`、`codex-*` crate 或 `yunxi-agent-codex`。MCP、skills、
multi-agent 也已经从 schema-only 状态推进到 YunXi 自有 runtime 的初步可执行
能力。

但这仍不是 Codex CLI 端核心 Agent 能力的完整复刻。下一版 Stage 4G 的目标应当
定义为：

```text
以 extracted/codex-core-agent-sources 和 vendor/codex-rs 为行为参考，
在不恢复上游运行时依赖的前提下，尽量一次性把 Codex CLI headless Agent
核心能力复刻进 YunXi-owned crates，使 YunXi 默认 runtime 具备接近 Codex CLI
核心 Agent 的真实工作能力。
```

这里的“一次性”不是指不分模块，而是指在一个开发阶段里连续构建完整能力面：
先整体迁移和实现，不在单点上过度停留；完成构建后再统一验证。

## 当前基线

当前已完成能力：

- 默认 `yunxi-agent-cli` 不依赖 `yunxi-agent-codex`。
- 默认依赖图不包含 `codex-*` crate。
- `vendor/codex-rs` 仅作为源码参考和 parity 输入。
- `extracted/codex-core-agent-sources` 已包含 12 个核心 Agent parity 层的迁移源码。
- `yunxi-agent-runtime` 已有 provider/tool loop、AGENTS.md 注入、history restore、compact entry。
- `yunxi-agent-provider` 已有 OpenAI-compatible request/response/stream fixture 基础。
- `yunxi-agent-tools` 已有 tool registry、router、dispatch trace 和 `CompositeToolRuntime`。
- `yunxi-agent-tools` 默认可执行 shell、patch、tool_search、view_image、request_user_input 边界。
- `yunxi-agent-mcp` 已有 in-memory runtime 和 workspace `.yunxi/mcp-runtime.json` seed。
- `yunxi-agent-skills` 已有 skill discovery、metadata、injection。
- `yunxi-agent-multi-agent` 已有 in-memory registry 和基础生命周期命令。
- `yunxi-agent-storage` 已有 session/history/rollout 基础。

当前缺口的性质已经从“是否能摆脱上游依赖”转变为：

```text
YunXi 已经独立，但还需要把 Codex CLI 核心 Agent 的深层 runtime 行为、
真实 I/O 能力、事件协议、持久化状态和 CLI 细节继续补齐。
```

## Stage 4G 硬性约束

本阶段必须遵守以下约束：

- 默认 YunXi runtime 不得依赖 `vendor/codex-rs`。
- 默认依赖图不得包含 `codex-*` crate。
- 默认 `yunxi-agent-cli` 不得依赖 `yunxi-agent-codex`。
- 允许读取 `vendor/codex-rs` 和 `extracted/codex-core-agent-sources` 作为源码参考。
- 模型/provider 层继续保持可替换，不能把 runtime 绑定死到 OpenAI。
- 不引入 TUI、desktop app、cloud tasks、doctor、update、completion、marketplace、release installer。
- 先整体构建，构建中不做频繁验证；完成后统一运行验证门。
- 每个迁移能力必须落到 YunXi-owned crate，并有 fixture、单元测试或集成测试。
- 复制实质性上游源码时保留许可和来源说明。
- 每次阶段结束清理编译中间产物。
- 每次任务结束追加桌面开发日志。

## Stage 4G 总目标

Stage 4G 的目标是把剩余核心 Agent 能力压成一个大阶段完成：

1. 完整 streaming turn loop。
2. 真实 provider HTTP/SSE transport。
3. 深度 exec process manager。
4. 完整 sandbox/approval/escalation policy。
5. Codex-style patch parity。
6. 深度 context/prompt/compact。
7. 真实 MCP stdio/http client。
8. 完整 skills/plugins/dynamic tools。
9. 真实 multi-agent runtime。
10. 完整 rollout/thread state/resume。
11. Protocol/event mapping 全量对齐。
12. Headless CLI 行为 parity。

本阶段验收的核心表述：

```text
在默认构建中，YunXi Agent 可以不依赖 Codex CLI 上游源码，执行一个包含
模型 streaming、工具调用、shell/patch/MCP/skill/multi-agent、approval/sandbox、
history/resume/rollout 和 JSONL 输出的完整 headless Agent 回合。
```

## 4G.1 Protocol / Streaming / Event Mapping

### 目标

把协议层从“基础事件类型”升级为“Codex CLI core Agent event parity 底座”。

### 需要修改

- `crates/yunxi-agent-protocol/src/lib.rs`
- `crates/yunxi-agent-provider/src/lib.rs`
- `crates/yunxi-agent-runtime/src/lib.rs`
- `crates/yunxi-agent-cli/src/main.rs`

### 参考源码

- `extracted/codex-core-agent-sources/yunxi-agent-protocol/protocol-event-mapping`
- `extracted/codex-core-agent-sources/yunxi-agent-runtime/runtime-thread-turn`
- `vendor/codex-rs/core/src/event_mapping.rs`
- `vendor/codex-rs/core/src/client_common.rs`
- `vendor/codex-rs/core/src/turn_metadata.rs`

### 构建内容

- 补齐 input item、response item、content item、tool call item、reasoning item。
- 补齐 response started/delta/completed/failed/cancelled event。
- 补齐 turn metadata、thread id、turn id、item id、call id。
- 统一 provider stream event 到 `yunxi-agent-protocol::StreamEvent`。
- CLI JSONL 输出只从 protocol/runtime event 映射产生。
- 保留稳定协议版本字段，避免后续事件 shape 反复破坏 CLI 输出。

### 验收

- fixture 可以模拟 assistant delta、reasoning delta、tool argument delta、tool completed。
- runtime 可以把 stream events 映射成 `AgentEvent` 和 JSONL。
- CLI JSONL 输出在 shell、patch、MCP、skill、multi-agent 回合中保持一行一个事件。

## 4G.2 Provider Live Transport

### 目标

把 provider 从 fixture/transport boundary 推进到默认可配置的真实 HTTP/SSE 能力，同时保持模型层可替换。

### 需要修改

- `crates/yunxi-agent-provider/src/lib.rs`
- `crates/yunxi-agent-runtime/src/lib.rs`
- `crates/yunxi-agent-cli/src/main.rs`

### 参考源码

- `extracted/codex-core-agent-sources/yunxi-agent-provider/provider-transport-streaming`
- `vendor/codex-rs/core/src/client.rs`
- `vendor/codex-rs/core/src/responses_retry.rs`
- `vendor/codex-rs/model-provider`
- `vendor/codex-rs/http-client`

### 构建内容

- 增加 provider config loader：base_url、api_key、model、timeout、stream 开关。
- 增加 HTTP JSON transport 实现。
- 增加 SSE parser 和 streaming response adapter。
- 增加 retry/backoff：429、5xx、timeout、connection reset。
- 增加错误分类：auth、rate_limit、server、network、invalid_response、unsupported_model。
- 增加模型能力描述：tools、parallel_tool_calls、reasoning、stream_usage。
- live provider 测试默认仍使用 fixture/mock transport，不访问真实网络。

### 验收

- fixture transport 可模拟 streaming response。
- provider 可以产生 protocol stream events。
- runtime 可在 streaming provider 下完成 tool loop。
- 默认测试不需要真实 API key。

## 4G.3 Exec Process Manager

### 目标

把 shell 执行从“一次性 output()”升级为 Codex-style unified exec 生命周期。

### 需要修改

- `crates/yunxi-agent-exec/src/lib.rs`
- `crates/yunxi-agent-tools/src/lib.rs`
- `crates/yunxi-agent-runtime/src/lib.rs`
- `crates/yunxi-agent-core/src/event.rs`

### 参考源码

- `extracted/codex-core-agent-sources/yunxi-agent-exec/exec-shell`
- `vendor/codex-rs/core/src/exec.rs`
- `vendor/codex-rs/core/src/unified_exec`
- `vendor/codex-rs/core/src/shell.rs`
- `vendor/codex-rs/core/src/shell_snapshot.rs`
- `vendor/codex-rs/shell-command`

### 构建内容

- 新增 `ExecManager`，负责 spawn、stdout/stderr streaming、stdin、timeout、cancel。
- 新增 `ExecHandle`，支持 long-running command 的后续控制。
- 新增 `ExecLifecycleEvent` 到 runtime event 的完整映射。
- 支持 command canonicalization、platform shell argv、cwd/env 合并。
- 支持 output limits：head/tail 截断、line count、byte count、truncation marker。
- 支持 command duration、exit status、signal/termination reason。
- 文件变更 snapshot 从工具层下沉到 exec/patch 统一 file-change detector。

### 验收

- shell command 产生 started、stdout delta、stderr delta、completed。
- timeout command 产生 timed_out 和 failed/declined 状态。
- stdin fixture 可以向进程写入内容。
- Windows `cmd /C` 和非 Windows `sh -c` 行为分别有 fixture。

## 4G.4 Sandbox / Approval / Escalation

### 目标

把 sandbox/approval 从简单拒绝策略升级为 Codex-style risk evaluation 和 approval lifecycle。

### 需要修改

- `crates/yunxi-agent-sandbox/src/lib.rs`
- `crates/yunxi-agent-tools/src/lib.rs`
- `crates/yunxi-agent-runtime/src/lib.rs`
- `crates/yunxi-agent-core/src/config.rs`
- `crates/yunxi-agent-core/src/event.rs`

### 参考源码

- `extracted/codex-core-agent-sources/yunxi-agent-sandbox/sandbox-approval`
- `vendor/codex-rs/core/src/exec_policy.rs`
- `vendor/codex-rs/execpolicy`
- `vendor/codex-rs/sandboxing`
- `vendor/codex-rs/windows-sandbox-rs`
- `vendor/codex-rs/linux-sandbox`
- `vendor/codex-rs/shell-escalation`

### 构建内容

- 增加 command risk classifier：read、write、delete、network、credential、process control。
- 增加 approval request event、approval decision event、approval denied event。
- 增加 non-interactive host policy：never/on-request/on-failure/untrusted 的精确行为。
- 增加 escalation request：需要更高 sandbox/network 权限时返回可解释事件。
- 增加 sandbox backend selection：read-only、workspace-write、danger-full-access。
- 增加 network policy：disabled、workspace/inherit、enabled。
- 平台 runner 先以 facade + fixture 实现，随后再深化到真实 Windows/Linux runner。

### 验收

- destructive command 在 on-request 下产生 approval required。
- read-only sandbox 拒绝写文件。
- workspace-write 拒绝 workspace 外 cwd 和路径逃逸。
- danger-full-access 允许执行并记录 bypass backend。
- approval denied 会把 tool result 返回 provider，runtime 不崩溃。

## 4G.5 Patch Full Parity

### 目标

把 apply_patch 从当前可用子集提升到 Codex-style grammar 和 diagnostics parity。

### 需要修改

- `crates/yunxi-agent-patch/src/lib.rs`
- `crates/yunxi-agent-patch/assets/apply_patch.lark`
- `crates/yunxi-agent-tools/src/lib.rs`

### 参考源码

- `extracted/codex-core-agent-sources/yunxi-agent-patch/patch`
- `vendor/codex-rs/core/src/apply_patch.rs`
- `vendor/codex-rs/apply-patch`
- `vendor/codex-rs/core/src/tools/handlers/apply_patch.rs`
- `vendor/codex-rs/core/src/tools/runtimes/apply_patch.rs`

### 构建内容

- 支持 add、delete、update、move、update+move。
- 支持多 hunk、EOF marker、上下文匹配、缩进保持。
- 支持精确 diagnostics：缺少 Begin/End、路径非法、上下文不匹配、重复文件。
- 统一 JSON patch 和 Codex patch 的 file-change report。
- patch runtime 产生 patch started、file changed、patch completed/failed。

### 验收

- Codex-style add/update/delete/move fixture 通过。
- 错误 fixture 的 message 稳定。
- patch 不能逃逸 workspace。

## 4G.6 Context / Prompt / Compact

### 目标

把 prompt/context 从基础 AGENTS.md + deterministic compact 推进到 Codex context manager parity。

### 需要修改

- `crates/yunxi-agent-context/src/lib.rs`
- `crates/yunxi-agent-runtime/src/lib.rs`
- `crates/yunxi-agent-storage/src/lib.rs`
- `crates/yunxi-agent-cli/src/main.rs`

### 参考源码

- `extracted/codex-core-agent-sources/yunxi-agent-context/prompt-context-agents-compact`
- `vendor/codex-rs/core/src/context`
- `vendor/codex-rs/core/src/context_manager`
- `vendor/codex-rs/context-fragments`
- `vendor/codex-rs/prompts`
- `vendor/codex-rs/file-search`
- `vendor/codex-rs/core/src/compact*.rs`

### 构建内容

- 引入 prompt asset registry。
- 补齐 AGENTS.md hierarchy manager 和 invalidation。
- 补齐 context fragments：workspace、instructions、tools、files、history、compact summary。
- 补齐 file mention parser 和 workspace search。
- 引入 approximate tokenizer facade，后续可替换为精确 tokenizer。
- 实现 context window state：normal、pressure、needs_compaction、compacted。
- 实现 compact request/response flow，可通过 provider 生成摘要，也可 fixture fallback。
- 增加 prompt debug 输出，方便对齐 Codex 行为。

### 验收

- AGENTS.md 多层目录按 root -> cwd 顺序注入。
- file mention 能把存在文件加入 context。
- history 超预算时触发 compact flow。
- compact 后 resume 可重建 context。

## 4G.7 Real MCP Runtime

### 目标

把 MCP 从 seed/in-memory 推进到真实 stdio/http client 和 tool/resource discovery。

### 需要修改

- `crates/yunxi-agent-mcp/src/lib.rs`
- `crates/yunxi-agent-tools/src/lib.rs`
- `crates/yunxi-agent-runtime/src/lib.rs`
- `crates/yunxi-agent-cli/src/main.rs`

### 参考源码

- `extracted/codex-core-agent-sources/yunxi-agent-mcp/mcp`
- `vendor/codex-rs/codex-mcp`
- `vendor/codex-rs/rmcp-client`
- `vendor/codex-rs/core/src/mcp.rs`
- `vendor/codex-rs/core/src/mcp_tool_call.rs`
- `vendor/codex-rs/core/src/mcp_tool_approval_templates.rs`

### 构建内容

- 加载 workspace/global MCP config。
- 实现 stdio client：spawn server、JSON-RPC initialize、list tools/resources、call tool。
- 实现 HTTP client facade：先支持 mock/http fixture，再深化 auth。
- tool discovery 结果注入 dynamic tool metadata。
- MCP tool call 走 sandbox/approval policy。
- 支持 elicitation 和 auth status event。
- 保留 `.yunxi/mcp-runtime.json` 作为离线 fixture seed。

### 验收

- 本地 mock MCP stdio server 可完成 initialize/list/call。
- resource list/read 可以返回内容。
- destructive MCP tool 会触发 approval event。

## 4G.8 Skills / Plugins / Dynamic Tools

### 目标

把 skills 从“读取 SKILL.md”推进到 Codex-style skill/plugin/dynamic tool runtime。

### 需要修改

- `crates/yunxi-agent-skills/src/lib.rs`
- `crates/yunxi-agent-tools/src/lib.rs`
- `crates/yunxi-agent-runtime/src/lib.rs`
- 可选新增：`crates/yunxi-agent-plugins`

### 参考源码

- `extracted/codex-core-agent-sources/yunxi-agent-skills/skills-plugins-dynamic-tools`
- `vendor/codex-rs/core/src/skills.rs`
- `vendor/codex-rs/core-skills`
- `vendor/codex-rs/skills`
- `vendor/codex-rs/plugin`
- `vendor/codex-rs/core-plugins`
- `vendor/codex-rs/core/src/plugins`

### 构建内容

- skill catalog 支持多根目录、禁用项、错误收集。
- skill frontmatter 支持 policy、dependencies、tools、interface。
- skill invocation 支持显式工具调用和隐式注入。
- plugin manifest/cache 支持 skill 和 MCP server 声明。
- `tool_search` 改为搜索 dynamic tool metadata，而不是只搜文件名。
- `request_user_input` 保持 host boundary，CLI/headless 非交互下返回 structured declined。
- `view_image` 返回 structured local image item。

### 验收

- skill 可加载、注入、调用，并记录来源。
- plugin manifest 可暴露 skill 和 MCP server。
- tool_search 可找到 deferred tool metadata。

## 4G.9 Multi-Agent Runtime

### 目标

把 multi-agent 从 in-memory registry 推进到真实子 agent runtime 和 agent graph。

### 需要修改

- `crates/yunxi-agent-multi-agent/src/lib.rs`
- `crates/yunxi-agent-runtime/src/lib.rs`
- `crates/yunxi-agent-tools/src/lib.rs`
- `crates/yunxi-agent-storage/src/lib.rs`

### 参考源码

- `extracted/codex-core-agent-sources/yunxi-agent-multi-agent/multi-agent`
- `vendor/codex-rs/core/src/agent`
- `vendor/codex-rs/core/src/agent_communication.rs`
- `vendor/codex-rs/core/src/tools/handlers/multi_agents`
- `vendor/codex-rs/core/src/tools/handlers/multi_agents_v2`
- `vendor/codex-rs/agent-graph-store`

### 构建内容

- agent graph store 持久化 parent/child、status、task、role、budget。
- spawn 创建子 runtime context，继承 cwd、approval、sandbox、provider config、context budget。
- wait 读取子 agent 状态和最终输出。
- send_message/follow_up 追加子 agent thread 输入。
- interrupt/cancel 传播到 exec/provider/tool loop。
- multi-agent rollout 共享 budget，并写入 parent session。

### 验收

- spawn -> wait -> completed 可以端到端通过。
- interrupt 可以把子 agent 标记 interrupted 并停止后续 loop。
- parent session 可以看到子 agent lifecycle event。

## 4G.10 Storage / Rollout / Resume

### 目标

把 storage 从 session/history 基础推进到 Codex-style rollout replay 和 thread state。

### 需要修改

- `crates/yunxi-agent-storage/src/lib.rs`
- `crates/yunxi-agent-runtime/src/lib.rs`
- `crates/yunxi-agent-cli/src/main.rs`

### 参考源码

- `extracted/codex-core-agent-sources/yunxi-agent-storage/storage-rollout-resume`
- `vendor/codex-rs/core/src/rollout.rs`
- `vendor/codex-rs/core/src/rollout_budget.rs`
- `vendor/codex-rs/core/src/thread_rollout_truncation.rs`
- `vendor/codex-rs/thread-store`
- `vendor/codex-rs/state`
- `vendor/codex-rs/core/src/state_db_bridge.rs`

### 构建内容

- JSONL rollout recorder 每个 runtime event 都可持久化。
- rollout replay 可重建 thread timeline。
- thread metadata 支持 title、archive、pin、fork、parent。
- truncation 策略按 max items/max bytes 保留最新关键事件。
- resume 不只恢复 prompt/history，还恢复 tool/context/rollout state facade。
- CLI 增加 `sessions rollout`、`sessions fork`、`sessions pin/archive` parity 修正。

### 验收

- 一个多工具回合可写入 rollout JSONL。
- replay 后事件数量和关键事件一致。
- fork 后 parent/child metadata 正确。

## 4G.11 Headless CLI Parity

### 目标

只复刻 Codex CLI headless Agent 核心行为，不扩展非核心产品面。

### 需要修改

- `crates/yunxi-agent-cli/src/main.rs`
- `crates/yunxi-agent-core/src/config.rs`
- `crates/yunxi-agent-core/src/event.rs`

### 参考源码

- `extracted/codex-core-agent-sources/yunxi-agent-cli/cli-headless-core`
- Codex CLI headless 相关参数解析和 JSONL 输出源码

### 构建内容

- 对齐核心 flags：cwd、model、provider、approval、sandbox、jsonl、resume、config。
- 错误码稳定化：empty prompt、config error、provider error、tool error、approval denied。
- JSONL 输出只使用 protocol/runtime event。
- 默认 backend 永远是 YunXi。
- `--backend codex` 继续只作为兼容提示，不进入默认能力。
- live provider 需要显式配置，测试默认 fixture。

### 验收

- `yunxi-agent-cli --jsonl "..."` 可以完成 streaming/tool 回合。
- `sessions list/show/history/rollout/resume/fork/archive/pin` 可用。
- 缺配置时错误清楚，不回退到 Codex。

## 推荐执行方式

Stage 4G 采用“大阶段、分模块、统一验证”的执行方式：

1. 先构建 protocol/event/streaming，因为它是所有能力的统一出口。
2. 接着构建 exec/sandbox/approval，因为这是 Agent 自主执行和安全策略的底座。
3. 然后构建 provider live transport，让 runtime 能真实接模型流。
4. 然后构建 patch/context/storage，使代码修改和长上下文运行可靠。
5. 然后构建 MCP/skills/multi-agent，使外部工具生态和子 agent 能力成型。
6. 最后收敛 CLI 行为和 JSONL 输出。
7. 全部构建完后统一验证。

在执行阶段允许按 crate 内部拆小提交，但不应在某个单点反复卡住。遇到真实平台 runner
或 live 网络细节过深时，应先完成 facade + fixture parity，再进入下一层能力，避免阻塞
整个 4G 大阶段。

## 统一验证门

Stage 4G 构建完成后统一运行：

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

额外独立性验证：

```powershell
Rename-Item vendor\codex-rs vendor\codex-rs.disabled
cargo test
cargo check -p yunxi-agent-cli
cargo build -p yunxi-agent-cli
cargo tree -p yunxi-agent-cli
Rename-Item vendor\codex-rs.disabled vendor\codex-rs
```

额外行为验证：

```powershell
cargo run -p yunxi-agent-cli -- --backend yunxi --jsonl "explain this project"
cargo run -p yunxi-agent-cli -- --backend yunxi --jsonl "use shell, patch, mcp, skill, and multi-agent fixtures"
cargo run -p yunxi-agent-cli -- sessions list
cargo run -p yunxi-agent-cli -- sessions rollout <session-id>
```

依赖边界必须满足：

```text
cargo tree -p yunxi-agent-cli
不得出现 vendor/codex-rs
不得出现 codex-* crate
不得出现 yunxi-agent-codex
```

验证完成后必须执行：

```powershell
cargo clean
```

## 完成定义

Stage 4G 只有同时满足以下条件，才可以认为完成：

- 默认 `yunxi-agent-cli` 可在无上游 Codex runtime 依赖下运行。
- 默认依赖树无 `vendor/codex-rs`、`codex-*`、`yunxi-agent-codex`。
- provider streaming、tool loop、exec lifecycle、approval/sandbox、patch、MCP、skills、
  multi-agent、storage/rollout、resume、CLI JSONL 均有 YunXi-owned 实现。
- 所有核心能力都有 fixture 或测试覆盖。
- 禁用 `vendor/codex-rs` 后默认测试、check、CLI build 仍通过。
- 桌面开发日志记录本阶段工作流程。
- 编译中间产物已清理。

## 风险与取舍

### 风险 1：真实平台 sandbox 过深

Windows/Linux sandbox runner 可能牵涉系统权限和平台差异。本阶段优先完成 runner
selection、policy、event、fixture parity；真实 OS-level 强隔离可以作为 Stage 4G 后续深化点，
但不能阻塞 exec/approval 主链路。

### 风险 2：真实 MCP 协议细节过多

MCP stdio/http/auth/elicitation 较深。本阶段先完成 stdio JSON-RPC mock server 端到端和
HTTP facade，再逐步扩充 auth 与 elicitation。

### 风险 3：Provider live 网络不稳定

默认测试不得依赖真实网络。live transport 必须通过 fixture/mock transport 验证，真实 API
只做 opt-in smoke。

### 风险 4：一次阶段范围过大

Stage 4G 是大阶段，但执行时必须以 crate 边界推进。每个模块完成构建后不反复打磨细枝末节，
统一验证时再集中修正编译、测试和集成问题。

## 下一步执行入口

下一步应直接进入 Stage 4G 构建：

```text
开始工作后，先构建 protocol/event/streaming，再构建 exec/sandbox/approval，
随后 provider、patch/context/storage、MCP/skills/multi-agent、CLI 收敛。
中间不做频繁验证，完成全部构建后统一测试、检查、构建、依赖边界验证和清理。
```

第一批应优先打开的文件：

- `crates/yunxi-agent-protocol/src/lib.rs`
- `crates/yunxi-agent-runtime/src/lib.rs`
- `crates/yunxi-agent-provider/src/lib.rs`
- `crates/yunxi-agent-exec/src/lib.rs`
- `crates/yunxi-agent-sandbox/src/lib.rs`
- `crates/yunxi-agent-tools/src/lib.rs`
- `crates/yunxi-agent-cli/src/main.rs`

第一批应参考的迁移源码目录：

- `extracted/codex-core-agent-sources/yunxi-agent-protocol`
- `extracted/codex-core-agent-sources/yunxi-agent-runtime`
- `extracted/codex-core-agent-sources/yunxi-agent-provider`
- `extracted/codex-core-agent-sources/yunxi-agent-exec`
- `extracted/codex-core-agent-sources/yunxi-agent-sandbox`
- `extracted/codex-core-agent-sources/yunxi-agent-tools`
