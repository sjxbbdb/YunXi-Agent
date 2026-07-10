# YunXi Stage 4I Behavior-Level Core Parity Closure Development Report

生成时间：2026-07-10 17:44:25 +08:00

## 核心结论

Stage 4H 已经把 YunXi Agent 推进到“默认自主运行 + 主干 headless agent loop
可用”的状态：默认 `yunxi-agent-cli` 依赖树不包含 `vendor/codex-rs`、
`codex-*` 或 `yunxi-agent-codex`，并且 provider、runtime、tool loop、patch、
MCP facade、skills/plugins、multi-agent graph、storage graph、CLI JSONL 等能力都已经
落进 YunXi-owned crates。

但 Stage 4H 仍不能宣称“完全复刻 Codex CLI headless Agent 核心能力”。下一版
Stage 4I 的目标是把剩余 25%-35% 的深水区压成一次行为级 parity 收敛构建：

```text
真实增量 SSE
-> 长生命周期 MCP session
-> 平台 sandbox / escalation runner
-> 子 agent runtime
-> apply_patch 全边界
-> context / compact / rollout / state 深度对齐
-> protocol / CLI JSONL 全量稳定
-> 禁用上游依赖后的端到端 fixture 验证
```

Stage 4I 完成后，YunXi Agent 应当不只是“能自主跑”，而是可以用默认 YunXi runtime
完成一个接近 Codex CLI headless core 的完整 Agent 回合，并且该回合不依赖上游源码或
上游 crate。

## 硬性约束

- 默认 YunXi runtime 不得依赖 `vendor/codex-rs`。
- 默认依赖图不得包含 `codex-*` crate。
- 默认 `yunxi-agent-cli` 不得依赖 `yunxi-agent-codex`。
- `vendor/codex-rs` 和 `extracted/codex-core-agent-sources` 只能作为行为参考、
  源码迁移输入和 fixture 对照，不能重新进入默认运行时依赖。
- 直接照着 Codex CLI 上游源码补齐 headless core 行为；优先保持行为一致，
  后续再做命名和产品痕迹去 Codex 化。
- 模型/provider 层必须继续保持可替换接口，不能把 runtime 绑定死到 OpenAI。
- 不引入 TUI、desktop app、cloud tasks、doctor、update、completion、marketplace、
  installer 等非 headless core surfaces。
- 构建阶段先整体迁移和接线，不做中途频繁测试；全部构建完成后统一验证。
- 遇到单点复杂问题时先落 facade、fixture 或兼容层并继续推进，不在一个点上消耗过多时间。
- 每次任务结束必须追加桌面开发日志，并在最终验证后清理编译中间产物。

## 当前基线

Stage 4H 后已经具备的 YunXi-owned 能力：

- `yunxi-agent-provider`：workspace-aware tool registry、OpenAI-compatible
  request、fixture/live transport 边界、SSE/tool call 聚合、动态函数名解析。
- `yunxi-agent-runtime`：provider stream loop、tool loop、动态函数桥、file mention
  context 注入、live provider backend。
- `yunxi-agent-tools`：fixed/dynamic tool registry、workspace registry、
  `tool_search` 文件和动态工具元数据返回、patch failure diagnostics。
- `yunxi-agent-skills`：workspace plugin discovery、plugin skill roots、plugin MCP seed、
  dynamic tool metadata。
- `yunxi-agent-patch`：`PatchDiagnostic`、`PatchApplyError`、
  `apply_patch_detailed`。
- `yunxi-agent-mcp`：stdio JSON-RPC facade、HTTP JSON-RPC client facade、reqwest
  transport、fixture transport。
- `yunxi-agent-multi-agent`：agent graph session metadata、cycle-safe graph
  validation/insertion、graph reconstruction。
- `yunxi-agent-storage`：session graph view、session/thread metadata 到 multi-agent
  graph metadata 的桥接。
- `yunxi-agent-context`：AGENTS.md、history/compact 基础、prompt 中 `@path` 文件上下文注入。
- `yunxi-agent-cli`：`sessions graph`、exit-code 分类、provider-live 路径、JSONL 基础输出。

## Stage 4I 总目标

Stage 4I 要把 Stage 4H 的“能力面接通”推进为“行为级完整度收敛”。完成标准不是再加几个
schema 或 facade，而是让一个默认 YunXi headless run 能覆盖：

1. provider 真正边收边处理的 SSE 增量输出；
2. tool call argument delta 聚合和多轮 tool loop；
3. shell/patch/MCP/skill/multi-agent 的真实 runtime 调度；
4. approval/sandbox/escalation 的真实执行边界；
5. context/history/compact/resume/rollout/state 的可重建状态链；
6. Codex-style JSONL protocol event shape；
7. 禁用上游依赖后仍可构建、测试和运行 fixture。

## 上游源码参考范围

Stage 4I 继续以这些上游源码为行为参考，但迁移结果必须落入 YunXi-owned crates：

- `vendor/codex-rs/core/src/client.rs`
- `vendor/codex-rs/core/src/client_common.rs`
- `vendor/codex-rs/core/src/event_mapping.rs`
- `vendor/codex-rs/core/src/codex_thread.rs`
- `vendor/codex-rs/core/src/thread_manager.rs`
- `vendor/codex-rs/core/src/session/**`
- `vendor/codex-rs/core/src/context/**`
- `vendor/codex-rs/core/src/context_manager/**`
- `vendor/codex-rs/core/src/compact*.rs`
- `vendor/codex-rs/core/src/tools/**`
- `vendor/codex-rs/core/src/exec.rs`
- `vendor/codex-rs/core/src/unified_exec/**`
- `vendor/codex-rs/core/src/shell_snapshot.rs`
- `vendor/codex-rs/core/src/exec_policy.rs`
- `vendor/codex-rs/exec/**`
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

## 构建面 1：真实增量 Provider SSE

目标 crate：

- `crates/yunxi-agent-provider/src/lib.rs`
- `crates/yunxi-agent-runtime/src/lib.rs`
- `crates/yunxi-agent-protocol/src/lib.rs`
- `crates/yunxi-agent-cli/src/main.rs`

构建内容：

- 把 Stage 4H 的 stream 聚合从完整 body/fixture 聚合推进到真正的逐 chunk 解析。
- provider transport 输出 `ProviderStreamChunk`，runtime 不等待最终完整响应即可消费 delta。
- 支持 assistant text delta、reasoning delta、tool call id delta、tool name delta、
  tool arguments delta、completed、failed、cancelled。
- tool argument delta 按 call id 聚合，聚合完成后再进入 tool router。
- provider retry/backoff 分清 auth、rate_limit、server、network、timeout、
  invalid_response、unsupported_model。
- fixture transport 保留，最终验证不依赖真实 API key。

完成口径：

- fixture 可以模拟分片 SSE；
- runtime 可以在收到 tool call completed 后立即进入 tool loop；
- CLI JSONL 可以逐条输出 delta event，而不是只输出最终消息。

## 构建面 2：MCP 长生命周期 Session Runtime

目标 crate：

- `crates/yunxi-agent-mcp/src/lib.rs`
- `crates/yunxi-agent-tools/src/lib.rs`
- `crates/yunxi-agent-runtime/src/lib.rs`
- `crates/yunxi-agent-cli/src/main.rs`

构建内容：

- 增加 `McpSessionManager`，统一管理 stdio/http MCP server lifecycle。
- stdio path 支持 spawn、initialize、capabilities、tools/list、resources/list、
  tools/call、shutdown。
- HTTP path 支持 JSON-RPC request/response、session id、transport error 分类。
- MCP tool discovery 注入 dynamic tool registry，供 provider request schema 使用。
- MCP tool call 走统一 approval/sandbox/escalation policy。
- `.yunxi/mcp-runtime.json` 继续作为离线 fixture seed，不替代真实 session manager。

完成口径：

- YunXi runtime 能从 workspace MCP config 建立 session；
- MCP tools 可以被 provider 动态发现并调用；
- MCP session 失败能进入 protocol error/warning event，而不是静默吞掉。

## 构建面 3：Exec / Sandbox / Escalation 真实执行边界

目标 crate：

- `crates/yunxi-agent-exec/src/lib.rs`
- `crates/yunxi-agent-sandbox/src/lib.rs`
- `crates/yunxi-agent-tools/src/lib.rs`
- `crates/yunxi-agent-runtime/src/lib.rs`

构建内容：

- `ExecHandle` 支持 long-running command 的 status、cancel、output polling。
- 输出限制支持 max bytes、max lines、head/tail truncation、truncation marker。
- shell snapshot/file-change detector 对 shell 和 patch 统一产生 file-change report。
- sandbox runner boundary 区分 no sandbox、read-only、workspace-write、danger-full-access。
- Windows/Linux 后端先落统一 trait 和 fixture runner；能安全接入平台实现时再接平台实现。
- escalation execution request/response 明确区分 requested、approved、declined、not_available。
- non-interactive 场景默认产生可解释 declined result，不阻塞 runtime loop。

完成口径：

- shell tool 不再只是一次性 output facade；
- sandbox/escalation 不只是 policy event，而有清晰 runner boundary；
- long-running/cancel/timeout/truncated/file-change 都能映射到 JSONL。

## 构建面 4：Multi-Agent Child Runtime

目标 crate：

- `crates/yunxi-agent-multi-agent/src/lib.rs`
- `crates/yunxi-agent-runtime/src/lib.rs`
- `crates/yunxi-agent-tools/src/lib.rs`
- `crates/yunxi-agent-storage/src/lib.rs`

构建内容：

- 把 Stage 4H 的 graph/lifecycle/registry 推进到 child runtime boundary。
- spawn 子 agent 时创建 session/thread metadata，并写入 storage-backed graph。
- 子 agent 复用 YunXi provider/tool/runtime facade，不进入 Codex backend。
- wait/message/follow-up/interrupt/list 读取 storage-backed graph state。
- 子 agent event 汇入父 session 的 protocol stream，并保留 child session id。
- 防止 graph cycle、重复 child id、孤儿 child session。

完成口径：

- multi-agent 不再只是 graph metadata；
- 子 agent 可以执行一个默认 YunXi runtime turn；
- 父子 session 可以通过 `sessions graph` 和 JSONL event 重建。

## 构建面 5：Apply Patch 全边界 Parity

目标 crate：

- `crates/yunxi-agent-patch/src/lib.rs`
- `crates/yunxi-agent-patch/assets/apply_patch.lark`
- `crates/yunxi-agent-tools/src/lib.rs`
- `crates/yunxi-agent-protocol/src/lib.rs`

构建内容：

- 对齐 add、delete、update、move、update+move、多 hunk、EOF marker。
- 对齐上下文匹配、缩进保留、重复路径、路径逃逸、空文件、缺失文件。
- 对齐 binary/非 UTF-8 文件拒绝策略和 diagnostic shape。
- patch started、file changed、patch completed、patch failed 进入 protocol event。
- JSON patch 和 Codex-style patch 统一 change report。

完成口径：

- patch 失败不会破坏 runtime loop；
- 所有失败都有结构化 diagnostic；
- Codex-style patch 的主要语法边界有 fixture 覆盖。

## 构建面 6：Context / Compact / Rollout / State 深度对齐

目标 crate：

- `crates/yunxi-agent-context/src/lib.rs`
- `crates/yunxi-agent-storage/src/lib.rs`
- `crates/yunxi-agent-runtime/src/lib.rs`
- `crates/yunxi-agent-cli/src/main.rs`

构建内容：

- 迁移 Codex context fragments：workspace、instructions、tools、files、history、
  compact summary。
- prompt asset registry 接入 provider messages。
- context window state 区分 normal、pressure、needs_compaction、compacted。
- compact flow 支持 provider summary；provider 不可用时使用 fixture fallback。
- rollout budget、thread rollout truncation、state record 写入 storage。
- resume/fork/archive/pin/unpin 的状态变化可以重建完整 headless run。
- prompt debug 输出用于定位最终发送给 provider 的 messages。

完成口径：

- history/compact/resume 不只是简单拼接；
- storage 可以重建 turn/context/rollout 状态；
- CLI 能输出关键 session state，不依赖上游 thread-store。

## 构建面 7：Protocol / Event / CLI JSONL Full Shape

目标 crate：

- `crates/yunxi-agent-protocol/src/lib.rs`
- `crates/yunxi-agent-core/src/event.rs`
- `crates/yunxi-agent-runtime/src/lib.rs`
- `crates/yunxi-agent-cli/src/main.rs`

构建内容：

- 补齐 input item、response item、content item、reasoning item、tool call item。
- 所有 event 携带 thread id、turn id、item id、call id、workspace metadata。
- tool lifecycle、approval、escalation、MCP、patch、multi-agent、context、storage、
  warning、error、usage 都有稳定 event shape。
- CLI `--jsonl` 严格保持一行一个 runtime event。
- stdout/stderr 分离：结构化 JSONL 只走 stdout，诊断和不可恢复错误走 stderr。
- exit code 覆盖 success、invalid_input、provider_error、tool_error、
  cancelled、internal_error。

完成口径：

- JSONL fixture 可以逐行 round trip；
- runtime event shape 不依赖 CLI 临时拼接；
- CLI 是 protocol 的薄输出层，而不是另一个事件定义源。

## 构建面 8：默认依赖禁用与 Parity Harness

目标文件：

- `Cargo.toml`
- `crates/*/Cargo.toml`
- `crates/yunxi-agent-runtime/tests/runtime_tests.rs`
- `crates/yunxi-agent-provider/tests/provider_tests.rs`
- `crates/yunxi-agent-tools/tests/tool_tests.rs`
- `crates/yunxi-agent-cli/tests/jsonl_tests.rs`
- `docs/extraction-status.md`

构建内容：

- 建立端到端 fixture：provider SSE -> tool loop -> shell -> patch -> MCP -> skill
  -> child agent -> compact -> resume -> JSONL。
- 建立 dependency guard：默认 `yunxi-agent-cli` 依赖树不得命中 `codex`、`vendor`、
  `yunxi-agent-codex`。
- 建立 disabled-vendor guard：临时隐藏 `vendor/codex-rs` 后默认 workspace 仍能
  check/build/test。
- 所有 live 网络、真实 API key、真实 MCP server 都是可选 smoke，不作为最终通过条件。

完成口径：

- Stage 4I 的最终验证能证明默认运行路径完全自主；
- fixture 覆盖核心行为链；
- 上游源码只保留为参考和刷新输入。

## 最终统一验证门

Stage 4I 构建完成后一次性运行，不在中途反复运行：

```powershell
cargo fmt
cargo fmt -- --check
cargo test
cargo check --workspace
cargo build -p yunxi-agent-cli
cargo run -p yunxi-agent-cli -- parity map
cargo run -p yunxi-agent-cli -- --backend yunxi --jsonl "run stage 4i fixture"
cargo tree -p yunxi-agent-cli
git diff --check
cargo clean
```

额外验证口径：

- `cargo tree -p yunxi-agent-cli` 输出中不得出现 `codex`、`vendor`、
  `yunxi-agent-codex`。
- disabled-vendor 检查必须证明默认 workspace 不需要 `vendor/codex-rs`。
- JSONL fixture 必须逐行可解析。
- provider/MCP/live 网络相关能力必须有 fixture fallback，不能依赖外部网络才能通过。

## 完成定义

Stage 4I 完成时必须同时满足：

- 默认 YunXi runtime 不依赖上游 Codex 源码或 crate。
- 一个完整 headless run 能经过 provider streaming、tool loop、shell、patch、MCP、
  skill、multi-agent、approval/sandbox/escalation、context/compact/resume/rollout，
  并输出稳定 JSONL。
- 模型/provider 层保持可替换，后续可以接入其他 Agent 模型接口。
- 上游源码只作为行为参考，不进入默认运行时。
- 所有新增行为有 fixture、单元测试或集成测试在最终统一验证中覆盖。
- 编译中间产物已清理，桌面开发日志已追加，Git 提交和推送已完成。

## 风险处理

- 如果真实平台 sandbox runner 无法在当前 Windows 环境完整验证，先完成统一 runner
  trait、fixture runner、policy/event/trace，再把平台细节作为后续专门切片。
- 如果真实 MCP server 生命周期牵涉外部环境，最终验证使用 fixture server/session；
  真实 server 只作为可选 smoke。
- 如果 provider 网络不稳定，最终验证使用 fixture transport；真实 API 只作为可选 smoke。
- 如果某个 Codex 上游模块牵涉非 headless 产品面，只迁移 headless core 必需行为。
- 如果 GitHub 普通 push 不可用，继续使用 GitHub Git Data API 推送内容等价提交，并记录远端 ref。

## 下一步执行指令

下一阶段开发直接按本报告执行：

```text
先整体构建 Stage 4I 的 8 个行为级收敛面；
中间不做频繁测试和验证；
遇到复杂单点先落 facade / fixture / 兼容层并继续向后推进；
全部构建完成后统一运行最终验证门；
验证后清理编译中间产物、写桌面日志、提交并推送。
```
