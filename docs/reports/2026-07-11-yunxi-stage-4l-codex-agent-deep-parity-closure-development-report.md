# YunXi Stage 4L Codex Agent Deep Parity Closure Development Report

生成时间：2026-07-11 07:54:37 +08:00

## 核心结论

Stage 4K 之后，YunXi Agent 已经具备独立默认 backend、真实 DeepSeek provider
stream/non-stream smoke、provider stream -> tool loop -> child runtime -> storage ->
JSONL 的自主链路。默认 `yunxi-agent-cli` 依赖树仍不得包含 `vendor/codex-rs`、
`codex-*` 或 `yunxi-agent-codex`。

下一版 Stage 4L 的目标不是再证明 YunXi 能独立运行，而是把 Codex CLI headless
Agent 的深层行为语义继续机械迁移到 YunXi-owned crates。当前缺口集中在：
thread/session/turn 状态机、provider feature matrix、exec-server/unified_exec、真实
platform sandbox enforcement、interactive approval、MCP auth/elicitation/capability
negotiation、skills/plugins runtime、context manager/compact/rollout、multi-agent v2
协作语义、Codex JSONL protocol 全量映射和跨平台 parity harness。

Stage 4L 的工作方式继续遵守用户硬性约束：先整体迁移和构建，中途不做频繁测试，
不在单点上长时间卡住；全部构建完成后统一运行最终验证门。遇到复杂平台特性时先
落 YunXi-owned facade、fixture 和兼容层，确保主线能力继续推进。

## 当前基线

已具备能力：

- `yunxi-agent-provider`：OpenAI-compatible / DeepSeek profile、stream/non-stream、
  SSE decoder、tool call delta 聚合、provider error classification/redaction。
- `yunxi-agent-runtime`：默认 YunXi backend、provider loop、tool loop、child runtime、
  child scoped stream、cancellation/provider error/storage/context 事件。
- `yunxi-agent-tools`：shell、patch、MCP、skill、multi-agent、tool search、view image、
  request user input facade、sandbox runner 和 MCP lifecycle runtime events。
- `yunxi-agent-mcp`：workspace MCP config、session manager、stdio/http JSON-RPC facade、
  reuse/cancel/shutdown/health hooks。
- `yunxi-agent-sandbox`：approval/sandbox/cwd/network policy、escalation event、
  platform runner diagnostic。
- `yunxi-agent-storage`：file-backed session、history restore、session graph、
  runtime state snapshot、archive/pin/fork 基础。
- `yunxi-agent-context`：AGENTS.md hierarchy、file mention context、basic history
  compact entry、prompt debug/context window facade。
- `yunxi-agent-multi-agent`：agent graph、spawn_run、child runtime boundary、parent/child
  session metadata。
- `yunxi-agent-cli`：`--backend yunxi`、`--provider-live`、JSONL、sessions
  list/show/history/resume/graph、DeepSeek live smoke script。

Stage 4K 统一验证口径：

- workspace Rust tests/check/build 已通过。
- Stage 4K offline JSONL fixtures 已通过。
- DeepSeek `deepseek-v4-flash` stream 和 non-stream live smoke 已通过。
- 默认 CLI 依赖树不包含上游 Codex runtime。
- secret scan 未发现 API key 或 bearer token 模式。

## 硬性约束

- 默认 YunXi runtime 不得依赖 `vendor/codex-rs`。
- 默认依赖图不得包含 `codex-*` crate。
- 默认 `yunxi-agent-cli` 不得依赖 `yunxi-agent-codex`。
- `vendor/codex-rs` 和 `extracted/codex-core-agent-sources` 只能作为行为参考、
  源码迁移输入和 fixture 对照，不能重新进入默认运行时依赖。
- 模型/provider 层必须继续保持可替换接口，不能把 runtime 绑定到 OpenAI 或
  DeepSeek。
- 真实 API key 只能从 `<private-api-file>` 读取到临时环境变量。
- 任何命令输出、日志、JSONL、错误消息、测试快照、提交和报告都不得包含 API key。
- 构建阶段先整体接线，不做中途频繁测试；全部构建完成后统一运行最终验证门。
- 遇到复杂单点时先落 facade、fixture 或兼容层并继续推进，不在单点上消耗过多时间。
- 最终验证后必须清理 `target`、`.yunxi` 和临时 smoke 输出文件。
- 每次任务结束必须追加桌面开发日志：
  `C:\Users\admin\Desktop\YunXi Agent开发日志.md`。
- 不引入 TUI、desktop app、cloud tasks、doctor、update、completion、marketplace、
  installer 等非 headless core surfaces。

## Stage 4L 总目标

Stage 4L 要把 YunXi Agent 从“自主链路可用”推进到“Codex CLI headless Agent 深层行为
接近等价”。完成后应满足：

1. runtime 拥有更接近 Codex 的 thread/session/turn 状态机和 turn metadata。
2. provider boundary 支持 Codex 常用 feature matrix：Responses-style item mapping、
   reasoning/usage、retry bucket、provider capabilities、request metadata。
3. shell/exec 进入 unified_exec/exec-server parity facade，具备 stdin、poll、cancel、
   output limit、shell snapshot 和长期 handle 语义。
4. sandbox 从 diagnostic facade 推进到平台 runner enforcement facade，覆盖 Windows
   runner、network policy、no-sandbox approval 和 escalation fallback。
5. approval 支持 per-tool approval key、session approval cache、granular approval、
   permission request payload 和非交互策略。
6. MCP 支持 auth status、elicitation boundary、approval template、capability negotiation、
   tools cache 和更完整的 stdio/http lifecycle。
7. skills/plugins 支持 core skill assets、plugin manifest/runtime、extension tool executor
   和 dynamic schema parity。
8. context/compact 支持 Codex context manager、context fragments、prompt assets、
   token budget、auto compact 和 prompt debug snapshot 的深层行为。
9. storage/rollout 支持 rollout recorder、message-history、thread-store、state bridge、
   truncation 和复杂 parent/child graph 重建。
10. multi-agent 支持 wait/message/follow-up/interrupt/list、agent communication、v2
    action routing、budget sharing 和 sub-agent activity events。
11. protocol/JSONL 覆盖 Codex headless core 的 exec/patch/MCP/collab/sub-agent runtime
    payload。
12. parity harness 可以在禁用上游 runtime 的情况下验证上述能力；真实 DeepSeek smoke
    仍作为 provider live gate，不作为离线测试的必要前提。

## 上游源码参考范围

继续以这些源码作为机械迁移参考，迁移结果必须落到 YunXi-owned crates：

- `vendor/codex-rs/core/src/codex_thread.rs`
- `vendor/codex-rs/core/src/thread_manager.rs`
- `vendor/codex-rs/core/src/session/**`
- `vendor/codex-rs/core/src/client.rs`
- `vendor/codex-rs/core/src/client_common.rs`
- `vendor/codex-rs/core/src/responses_retry.rs`
- `vendor/codex-rs/core/src/event_mapping.rs`
- `vendor/codex-rs/core/src/tools/**`
- `vendor/codex-rs/core/src/unified_exec/**`
- `vendor/codex-rs/exec/**`
- `vendor/codex-rs/exec-server/**`
- `vendor/codex-rs/execpolicy/**`
- `vendor/codex-rs/sandboxing/**`
- `vendor/codex-rs/windows-sandbox-rs/**`
- `vendor/codex-rs/linux-sandbox/**`
- `vendor/codex-rs/shell-escalation/**`
- `vendor/codex-rs/apply-patch/**`
- `vendor/codex-rs/codex-mcp/**`
- `vendor/codex-rs/rmcp-client/**`
- `vendor/codex-rs/core/src/mcp*.rs`
- `vendor/codex-rs/core/src/skills.rs`
- `vendor/codex-rs/core-skills/**`
- `vendor/codex-rs/skills/**`
- `vendor/codex-rs/plugin/**`
- `vendor/codex-rs/core-plugins/**`
- `vendor/codex-rs/message-history/**`
- `vendor/codex-rs/thread-store/**`
- `vendor/codex-rs/state/**`
- `vendor/codex-rs/agent-graph-store/**`
- `vendor/codex-rs/core/src/agent/**`
- `vendor/codex-rs/core/src/agent_communication.rs`

## 构建面 1：Thread / Session / Turn 状态机深化

目标文件：

- `crates/yunxi-agent-runtime/src/lib.rs`
- `crates/yunxi-agent-core/src/event.rs`
- `crates/yunxi-agent-protocol/src/lib.rs`
- `crates/yunxi-agent-storage/src/lib.rs`

构建内容：

- 新增 YunXi-owned `ThreadRuntimeState`、`TurnRuntimeState`、`TurnMetadata` facade。
- 把当前单回合执行路径拆成 thread start、turn start、provider request、tool loop、
  turn finish、session save 的显式状态流。
- 记录 cwd、model、provider、approval policy、sandbox policy、child depth、context
  phase、resume source 和 cancellation state。
- 为 provider failure、tool failure、cancelled、depth limit、storage failure 建立稳定状态。
- 输出 `thread_state`、`turn_state`、`turn_metadata` JSONL runtime events。

完成口径：

- 离线 fixture 能重建一次 parent turn 和一次 child turn 的完整状态序列。
- 状态事件不依赖上游 Codex types。
- `sessions show/history/graph` 能消费新增 metadata 而不破坏旧 session 文件。

## 构建面 2：Provider Feature Matrix 与 Responses-style Item Mapping

目标文件：

- `crates/yunxi-agent-provider/src/lib.rs`
- `crates/yunxi-agent-runtime/src/lib.rs`
- `crates/yunxi-agent-protocol/src/lib.rs`
- `crates/yunxi-agent-provider/tests/provider_tests.rs`

构建内容：

- 扩展 provider capability：responses API support、parallel tool calls、reasoning
  delta、usage delta、stream usage、tool schema strictness、request compression flag。
- 新增 provider-neutral response item mapping：assistant text、reasoning、tool call、
  dynamic tool call、MCP tool call、error、usage。
- 保持 DeepSeek profile 的 OpenAI-compatible chat path，同时为后续 provider 预留
  Responses-style mapping。
- 将 retry bucket 明确拆为 auth、rate_limit、server、network、timeout、bad_request、
  unsupported_schema、unsupported_model。
- fixture 覆盖 reasoning/usage/tool call 的 stream 和 non-stream 双路径。

完成口径：

- provider tests 可以证明新增 item mapping 不绑定 DeepSeek。
- DeepSeek smoke 继续通过。
- provider error JSONL 不暴露 key、header 或请求体。

## 构建面 3：Unified Exec / Exec Server Facade

目标文件：

- `crates/yunxi-agent-exec/src/lib.rs`
- `crates/yunxi-agent-tools/src/lib.rs`
- `crates/yunxi-agent-runtime/src/lib.rs`
- `crates/yunxi-agent-protocol/src/lib.rs`

构建内容：

- 新增 YunXi-owned `UnifiedExecRequest`、`UnifiedExecAttempt`、`ExecServerSession`
  facade。
- shell tool 使用 unified exec facade 记录 begin/stdout/stderr/end/poll/cancel events。
- 支持 stdin write、output polling、timeout cancel、long-running handle status。
- 保留当前直接 shell 执行路径作为 fallback，避免平台 helper 不完整时阻塞全局迁移。
- 将 shell snapshot 信息写入 exec runtime event，用于后续 resume/debug。

完成口径：

- fixture 覆盖 stdout/stderr delta、stdin、timeout、cancel、non-zero exit。
- CLI JSONL 有 Codex-style exec begin/end 事件。
- 平台 helper 缺失时返回 structured diagnostic，不 panic。

## 构建面 4：Platform Sandbox Enforcement Facade

目标文件：

- `crates/yunxi-agent-sandbox/src/lib.rs`
- `crates/yunxi-agent-tools/src/lib.rs`
- `crates/yunxi-agent-runtime/src/lib.rs`
- `crates/yunxi-agent-protocol/src/lib.rs`

构建内容：

- 将 `SandboxRunnerDiagnostic` 扩展为 `SandboxAttemptRecord`，记录 requested sandbox、
  materialized backend、cwd、workspace roots、network mode、escalation reason。
- Windows 路径先落 runner facade 和 diagnostic，保留真实隔离实现的 hook。
- Linux 路径先落 bwrap/landlock facade 和 fixture，不把平台依赖硬接到默认测试。
- managed network、network denial cancellation、no-sandbox escalation 进入统一事件。
- sandbox 失败时能继续走 approval/escalation fallback，而不是直接吞掉工具结果。

完成口径：

- read-only、workspace-write、danger-full-access、network-disabled 都有 fixture。
- Windows 平台输出 runner status 和 escalation-needed reason。
- 默认依赖树不引入上游 sandbox crate。

## 构建面 5：Interactive Approval 与 Granular Permission Cache

目标文件：

- `crates/yunxi-agent-sandbox/src/lib.rs`
- `crates/yunxi-agent-tools/src/lib.rs`
- `crates/yunxi-agent-runtime/src/lib.rs`
- `crates/yunxi-agent-cli/src/main.rs`

构建内容：

- 新增 approval key cache：shell command key、patch target key、MCP tool key、
  child agent key。
- 支持 per-session approved、declined、ask-again、approval unavailable 状态。
- 非交互 CLI 使用 policy 直接决定 approve/decline；交互 host 先通过 facade 暴露事件。
- permission request payload 包含 command、cwd、sandbox、network、tool name、target paths。
- patch 多文件 approval 以所有 touched paths 为 key，匹配 Codex session approval 语义。

完成口径：

- 同一 session 内重复已批准命令不再重复请求 approval。
- granular approval fixture 可以区分 shell、patch、MCP、child agent。
- JSONL 输出 approval requested/completed，不泄露环境变量值。

## 构建面 6：MCP Auth / Elicitation / Capability Negotiation

目标文件：

- `crates/yunxi-agent-mcp/src/lib.rs`
- `crates/yunxi-agent-tools/src/lib.rs`
- `crates/yunxi-agent-runtime/src/lib.rs`
- `crates/yunxi-agent-cli/src/main.rs`

构建内容：

- 扩展 `McpSessionManager`：auth status、capabilities、tools cache、resources cache、
  elicitation request、approval template。
- stdio/http runtime 都输出 initialized、capability_negotiated、auth_required、
  elicitation_requested、tool_started、tool_completed、shutdown events。
- workspace MCP config 支持 bearer-token-env-var、OAuth metadata facade、local/remote
  environment resolution。
- Codex Apps tools cache 先落 YunXi-owned cache facade，不接入产品面。
- 失败 bucket 区分 config error、auth required、transport error、tool error、
  schema error、timeout、cancelled。

完成口径：

- fixture MCP server 可覆盖 list tools、read resource、call tool、elicitation request、
  auth required 和 reused session。
- child runtime 复用 parent workspace MCP session manager。
- JSONL 可以看到 MCP lifecycle 细粒度事件。

## 构建面 7：Skills / Plugins Runtime Deepening

目标文件：

- `crates/yunxi-agent-skills/src/lib.rs`
- `crates/yunxi-agent-tools/src/lib.rs`
- `crates/yunxi-agent-runtime/src/lib.rs`
- `crates/yunxi-agent-cli/src/main.rs`

构建内容：

- 将 core skill assets、workspace skills、plugin skills 统一到 `SkillRuntimeCatalog`。
- plugin manifest 解析补齐 command、MCP seed、tool metadata、display name、version。
- 新增 extension tool executor facade，输出 model-visible schema 和 dispatch result。
- skill invocation 支持 instruction injection、resource load、tool call boundary 和
  structured failure。
- tool_search 返回 source、kind、score、deferred/loadable metadata。

完成口径：

- workspace skill、plugin skill、plugin MCP、extension tool 各有 fixture。
- runtime 能把 provider 请求的 dynamic skill tool 路由到正确 runtime。
- 缺失 skill 或 manifest 错误返回 structured diagnostic。

## 构建面 8：Context Manager / Compact / Prompt Assets

目标文件：

- `crates/yunxi-agent-context/src/lib.rs`
- `crates/yunxi-agent-runtime/src/lib.rs`
- `crates/yunxi-agent-storage/src/lib.rs`
- `crates/yunxi-agent-protocol/src/lib.rs`

构建内容：

- 新增 `ContextManagerState`：agents.md fragments、file mentions、history fragments、
  skill injections、MCP/tool summaries、compact summary。
- 迁移 prompt assets facade：compact prompt、patch prompt、developer instruction
  fragment、current time reminder。
- token budget 记录 input estimate、output reserve、compaction threshold、used ratio。
- auto compact 在预算超限时生成 deterministic summary，并保存到 session record。
- prompt debug snapshot 输出到 JSONL 和 session metadata。

完成口径：

- fixture 能证明 AGENTS.md 层级、file mention、history restore、skill injection 和 compact
  summary 都进入 prompt assembly。
- resume 后上下文顺序稳定。
- compact 后 session history 可重建，不丢 parent/child metadata。

## 构建面 9：Storage / Rollout / Thread Store Parity

目标文件：

- `crates/yunxi-agent-storage/src/lib.rs`
- `crates/yunxi-agent-runtime/src/lib.rs`
- `crates/yunxi-agent-cli/src/main.rs`
- `crates/yunxi-agent-protocol/src/lib.rs`

构建内容：

- 新增 rollout record：input item、response item、tool event、runtime event、
  state snapshot。
- session file 写入 thread metadata、turn metadata、parent/child graph、archive/pin/fork
  lineage、compact summary。
- `sessions history` 支持按 turn、tool、child、runtime event 过滤。
- `sessions resume` 使用 rollout 重建 prompt context，而不是只拼接旧 prompt 文本。
- `sessions fork` 记录 fork source 和 new lineage。

完成口径：

- 复杂 parent -> child -> fork -> resume fixture 可重建 graph 和 history。
- rollout truncation fixture 能证明过长历史会被稳定裁剪。
- 旧 session 文件仍可读取。

## 构建面 10：Multi-Agent v2 协作语义

目标文件：

- `crates/yunxi-agent-multi-agent/src/lib.rs`
- `crates/yunxi-agent-tools/src/lib.rs`
- `crates/yunxi-agent-runtime/src/lib.rs`
- `crates/yunxi-agent-storage/src/lib.rs`

构建内容：

- 补齐 wait、message、follow-up、interrupt、list action。
- 新增 agent communication mailbox，支持 parent -> child、child -> parent、sibling
  metadata routing。
- child run 支持 inherited provider/tools/storage/context 和 scoped cancellation。
- 记录 sub-agent activity events：spawn begin/end、interaction begin/end、waiting
  begin/end、close begin/end。
- 支持 rollout budget sharing 和 child depth guard。

完成口径：

- multi-agent fixture 覆盖 spawn_run、wait、message、follow-up、interrupt、list。
- parent JSONL 能看到 child scoped stream 和 sub-agent activity。
- storage graph 能恢复所有 child session 和 message lineage。

## 构建面 11：Protocol / JSONL Full Shape

目标文件：

- `crates/yunxi-agent-protocol/src/lib.rs`
- `crates/yunxi-agent-core/src/event.rs`
- `crates/yunxi-agent-runtime/src/lib.rs`
- `crates/yunxi-agent-cli/src/main.rs`

构建内容：

- 补齐 runtime event payload：exec begin/end、patch begin/end、MCP begin/end、collab
  agent begin/end、sub-agent activity、context state、turn state、provider usage。
- 为所有事件定义 stable snake_case type 和 versioned envelope。
- CLI JSONL 错误输出继续走 structured error，不输出未脱敏 stderr。
- 事件映射保留向后兼容，旧 `item`、`tool_started`、`tool_completed` 不破坏。

完成口径：

- protocol round-trip tests 覆盖新增事件。
- CLI fixture 每行都是合法 JSON。
- JSONL 可被下游解析器按 type/version 消费。

## 构建面 12：Parity Harness / Disabled Vendor / Live Gate

目标文件：

- `crates/yunxi-agent-runtime/tests/runtime_tests.rs`
- `crates/yunxi-agent-cli/tests/jsonl_tests.rs`
- `crates/yunxi-agent-provider/tests/provider_tests.rs`
- `crates/yunxi-agent-tools/tests/tool_tests.rs`
- `scripts/provider/deepseek-live-smoke.ps1`
- `docs/extraction-status.md`

构建内容：

- 新增 Stage 4L offline mega fixture，覆盖 provider -> context -> tool -> exec ->
  sandbox -> approval -> MCP -> skill -> multi-agent -> storage -> JSONL。
- 新增 disabled-vendor verification script 或 documented command，临时隔离
  `vendor/codex-rs` 后运行默认 workspace check。
- DeepSeek live smoke 保持独立脚本，不作为 `cargo test` 必需前提。
- 增加 dependency keyword scan：`codex`、`vendor`、`yunxi-agent-codex` 不得进入默认
  CLI tree。
- 结束后自动清理 `target`、`.yunxi` 和临时 smoke 输出。

完成口径：

- 离线 mega fixture 通过即可证明 Stage 4L 主链路。
- DeepSeek live gate 单独通过即可证明真实 provider。
- 依赖扫描和 secret scan 必须通过。

## 最终统一验证门

Stage 4L 构建完成后一次性运行：

```powershell
cargo fmt
cargo fmt -- --check
cargo test
cargo check --workspace
cargo build -p yunxi-agent-cli
cargo run -p yunxi-agent-cli -- parity map
cargo run -p yunxi-agent-cli -- --backend yunxi --jsonl "run stage 4l deep parity fixture"
cargo tree -p yunxi-agent-cli
# Run the repository secret-pattern scan used by previous stages without
# recording the secret regex text in this report.
git diff --check
.\scripts\provider\deepseek-live-smoke.ps1 -Model deepseek-v4-flash
.\scripts\provider\deepseek-live-smoke.ps1 -Model deepseek-v4-flash -NoStream
cargo clean
Remove-Item -Recurse -Force .\.yunxi -ErrorAction SilentlyContinue
```

验收时必须记录：

- 每条命令退出码。
- Stage 4L fixture JSONL 行数和事件计数。
- DeepSeek live smoke 是否通过；失败时记录 classification，不记录密钥。
- `cargo tree` 默认依赖树是否命中上游依赖关键词。
- secret scan 是否无匹配。
- `target` 和 `.yunxi` 是否已清理。

## 完成定义

Stage 4L 完成时必须同时满足：

- YunXi 默认 CLI 在无上游 runtime 依赖的情况下具备更深层的 Codex headless Agent 行为。
- 12 个构建面都有 YunXi-owned 类型、runtime 接线和 fixture 或测试覆盖。
- provider、tool、MCP、sandbox、approval、context、storage、multi-agent、protocol 的状态
  能通过 JSONL 被观察。
- DeepSeek live provider gate 在本地密钥有效时通过，且不污染离线测试。
- 文档、状态页和桌面开发日志同步更新。
- 编译中间产物和 session smoke 产物清理完成。

## 下一步执行指令

下一阶段直接按本报告实施 Stage 4L。执行时应先整体迁移和接线这 12 个构建面，中途
不做频繁测试；所有源码迁移完成后再统一运行最终验证门。遇到单点阻塞时先落 facade、
fixture 或兼容层并继续推进，保证 YunXi Agent 向 Codex CLI headless Agent 深层 parity
持续收敛。
