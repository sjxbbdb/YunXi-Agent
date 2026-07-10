# YunXi Stage 4H Upstream Core Gap Closure Development Report

生成时间：2026-07-10 15:58:00 +08:00

## 核心结论

YunXi Agent 现在已经具备独立默认运行路径：默认 `yunxi-agent-cli`
不依赖 `yunxi-agent-codex`，默认依赖树不包含 `codex-*` crate，
`vendor/codex-rs` 和 `extracted/codex-core-agent-sources` 只作为源码参考和
迁移输入。

Stage 4H 的目标不是继续证明“能否摆脱上游依赖”，而是直接照着 Codex CLI
headless Agent 的上游源码，把剩余核心缺口一次性补齐到 YunXi-owned crates。
本阶段完成后，YunXi 默认 runtime 应能在不恢复上游运行时依赖的前提下完成
一个完整 headless Agent 回合：

```text
provider streaming -> protocol events -> tool loop ->
shell/patch/MCP/skill/multi-agent -> approval/sandbox/escalation ->
context/history/compact/resume/rollout -> CLI JSONL output
```

## 硬性约束

- 默认 YunXi runtime 不得依赖 `vendor/codex-rs`。
- 默认依赖图不得包含 `codex-*` crate。
- 默认 `yunxi-agent-cli` 不得依赖 `yunxi-agent-codex`。
- `vendor/codex-rs` 和 `extracted/codex-core-agent-sources` 只能作为行为参考和
  源码迁移输入。
- 直接照着上游源码补齐行为，优先保持行为一致，后续再逐步去 Codex 命名和
  产品痕迹。
- 模型/provider 层必须保持可替换，不能把 runtime 绑定死到 OpenAI。
- 不引入 TUI、desktop app、cloud tasks、doctor、update、completion、
  marketplace、release installer 等非 headless core surfaces。
- 构建过程中不做频繁测试和验证，不在单点问题上反复卡住；先整体迁移和构建，
  等缺口整体补齐后统一验证。
- 每个迁移能力必须落到 YunXi-owned crate。
- 复制实质性上游源码时保留许可、来源说明和行为参考路径。
- 阶段结束必须统一验证、清理编译中间产物、追加桌面开发日志。

## 当前基线

已经完成并验证的自主能力：

- `yunxi-agent-runtime`：默认 YunXi backend、provider/tool loop、AGENTS.md 注入、
  history restore、compact entry、approval/escalation event emission。
- `yunxi-agent-provider`：OpenAI-compatible request/response/stream fixture 基础、
  provider capability、timeout、stream usage 边界。
- `yunxi-agent-protocol`：基础 runtime event、stream event、tool output delta、
  response cancelled、approval/escalation JSONL event。
- `yunxi-agent-exec`：`ExecManager`、spawn、stdin、stdout/stderr capture、timeout、
  cancel lifecycle、`ExecTrace`。
- `yunxi-agent-sandbox`：统一 `PolicyEvaluation`、command risk、approval request、
  escalation request、sandbox backend selection、network policy decision。
- `yunxi-agent-tools`：tool registry/router、`CompositeToolRuntime`、shell、patch、
  tool_search、view_image、request_user_input 边界、MCP/skill/multi-agent 调度。
- `yunxi-agent-patch`：Codex-style apply_patch 语法基础和 JSON patch 基础。
- `yunxi-agent-mcp`：in-memory runtime 和 workspace `.yunxi/mcp-runtime.json` seed。
- `yunxi-agent-skills`：skill discovery、metadata、injection、workspace skill 调用。
- `yunxi-agent-multi-agent`：in-memory registry 和 spawn/wait/message/follow-up/
  interrupt/list 基础生命周期。
- `yunxi-agent-storage`：session、history、rollout、resume、archive、fork、pin 基础。
- `yunxi-agent-cli`：默认 YunXi backend、JSON/JSONL 输出、session 管理、parity map。

## Stage 4H 总目标

Stage 4H 要把剩余缺口直接压成一次性构建：

1. Provider live HTTP/SSE streaming。
2. Runtime streaming turn loop。
3. Protocol/event mapping full parity。
4. Exec long-running handle、output limit、file snapshot。
5. Sandbox runner facade 和真实 escalation execution boundary。
6. Patch diagnostics 和 full grammar parity。
7. Context manager、prompt assets、file context、compact provider flow。
8. MCP stdio/http client。
9. Skills/plugins/dynamic tool injection。
10. Multi-agent 子 runtime 和持久化 agent graph。
11. Storage rollout/thread state/resume 深度对齐。
12. Headless CLI 参数、配置、退出码、JSONL parity。

## 上游源码迁移原则

本阶段允许直接读取并机械迁移下列源码，但迁移结果必须进入 YunXi-owned crates：

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
- `vendor/codex-rs/core/src/shell.rs`
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
- `vendor/codex-rs/core/src/plugins/**`
- `vendor/codex-rs/message-history/**`
- `vendor/codex-rs/thread-store/**`
- `vendor/codex-rs/state/**`
- `vendor/codex-rs/agent-graph-store/**`
- `vendor/codex-rs/core/src/agent/**`
- `vendor/codex-rs/core/src/agent_communication.rs`

## 构建顺序

本阶段不按“写一点测一点”的节奏推进，而按下面顺序连续构建。每个步骤遇到
非阻塞问题时先落 facade、fixture 或兼容层，不在一个点上耗尽时间；所有缺口
构建完成后再统一验证。

### 1. Provider Live Transport

修改文件：

- `crates/yunxi-agent-provider/src/lib.rs`
- `crates/yunxi-agent-runtime/src/lib.rs`
- `crates/yunxi-agent-cli/src/main.rs`

构建内容：

- 新增 provider config loader：base_url、api_key、model、timeout、stream、
  capabilities。
- 新增 HTTP JSON transport trait 和默认实现。
- 新增 SSE parser，把 provider event 转成 `yunxi_agent_protocol::StreamEvent`。
- 新增 retry/backoff：429、5xx、timeout、connection reset。
- 新增错误分类：auth、rate_limit、server、network、invalid_response、
  unsupported_model。
- 保留 fixture/mock transport，使最终验证不依赖真实 API key。

### 2. Runtime Streaming Turn Loop

修改文件：

- `crates/yunxi-agent-runtime/src/lib.rs`
- `crates/yunxi-agent-provider/src/lib.rs`
- `crates/yunxi-agent-protocol/src/lib.rs`

构建内容：

- 在 `YunXiRuntimeBackend` 中新增 streaming provider path。
- 支持 provider delta、reasoning delta、tool argument delta、tool call completed。
- 将 stream event 增量映射为 `AgentEvent` 和 CLI JSONL。
- 支持 tool call aggregation：分片 arguments 合并后再路由工具。
- 支持 response cancelled/failed 的统一收束。

### 3. Protocol / Event Mapping Full Parity

修改文件：

- `crates/yunxi-agent-protocol/src/lib.rs`
- `crates/yunxi-agent-core/src/event.rs`
- `crates/yunxi-agent-cli/src/main.rs`

构建内容：

- 补齐 input item、response item、content item、reasoning item、tool call item。
- 补齐 thread id、turn id、item id、call id、turn metadata、workspace metadata。
- 补齐 tool started、tool delta、tool completed、approval、escalation、mcp、
  patch、todo、warning、error、usage event。
- 增加协议版本字段，稳定 JSONL shape。
- CLI JSONL 输出只从 protocol/runtime event 映射生成。

### 4. Exec Deep Parity

修改文件：

- `crates/yunxi-agent-exec/src/lib.rs`
- `crates/yunxi-agent-tools/src/lib.rs`
- `crates/yunxi-agent-runtime/src/lib.rs`

构建内容：

- 新增 `ExecHandle`，支持 long-running command 的 cancel/status/output polling。
- 引入 output limit：max bytes、max lines、head/tail truncation、truncation marker。
- 增强 platform shell argv、cwd/env 合并、duration、termination reason。
- 引入 shell snapshot/file-change detector，统一 shell 和 patch 的 file changes。
- 将 exec lifecycle event 全量映射到 protocol event。

### 5. Sandbox Runner And Escalation Boundary

修改文件：

- `crates/yunxi-agent-sandbox/src/lib.rs`
- `crates/yunxi-agent-exec/src/lib.rs`
- `crates/yunxi-agent-tools/src/lib.rs`
- `crates/yunxi-agent-runtime/src/lib.rs`

构建内容：

- 保留当前 `PolicyEvaluation`，补齐真实 runner facade。
- 新增 Windows/Linux sandbox backend boundary。
- 新增 network policy enforcement facade。
- 新增 escalation execution request/response boundary。
- non-interactive host 下 escalation 默认返回可解释 declined result。
- danger-full-access bypass 必须产生明确 trace/event。

### 6. Patch Full Parity

修改文件：

- `crates/yunxi-agent-patch/src/lib.rs`
- `crates/yunxi-agent-patch/assets/apply_patch.lark`
- `crates/yunxi-agent-tools/src/lib.rs`

构建内容：

- 补齐 add、delete、update、move、update+move、多 hunk、EOF marker。
- 补齐上下文匹配、缩进保持、重复路径、非法路径、路径逃逸 diagnostics。
- 统一 JSON patch 和 Codex patch 的 change report。
- patch started/file changed/patch completed/failed 进入 protocol event。

### 7. Context / Prompt / Compact Manager

修改文件：

- `crates/yunxi-agent-context/src/lib.rs`
- `crates/yunxi-agent-runtime/src/lib.rs`
- `crates/yunxi-agent-storage/src/lib.rs`
- `crates/yunxi-agent-cli/src/main.rs`

构建内容：

- 引入 prompt asset registry。
- 迁移 Codex context fragments：workspace、instructions、tools、files、
  history、compact summary。
- 补齐 file mention parser 和 workspace file search。
- 补齐 context window state：normal、pressure、needs_compaction、compacted。
- compact flow 支持 provider summary，provider 不可用时用 fixture fallback。
- 增加 prompt debug 输出。

### 8. MCP Stdio / HTTP Client

修改文件：

- `crates/yunxi-agent-mcp/src/lib.rs`
- `crates/yunxi-agent-tools/src/lib.rs`
- `crates/yunxi-agent-runtime/src/lib.rs`
- `crates/yunxi-agent-cli/src/main.rs`

构建内容：

- 加载 workspace/global MCP config。
- 实现 stdio client：spawn、initialize、list tools/resources、call tool。
- 实现 HTTP client facade。
- tool/resource discovery 注入 dynamic tool metadata。
- MCP tool call 走统一 approval/sandbox/escalation policy。
- 保留 `.yunxi/mcp-runtime.json` seed 作为离线 fixture。

### 9. Skills / Plugins / Dynamic Tools

修改文件：

- `crates/yunxi-agent-skills/src/lib.rs`
- `crates/yunxi-agent-tools/src/lib.rs`
- `crates/yunxi-agent-runtime/src/lib.rs`
- 可选新增 `crates/yunxi-agent-plugins`

构建内容：

- 补齐 skill manifest、metadata、routing、instruction injection。
- 补齐 plugin manifest、plugin skill roots、dynamic tool registration。
- 迁移 core skills/core plugins 的 headless 可用部分。
- tool_search、request_user_input、view_image 继续保持 host boundary。
- dynamic tools metadata 注入 provider request。

### 10. Multi-Agent Runtime

修改文件：

- `crates/yunxi-agent-multi-agent/src/lib.rs`
- `crates/yunxi-agent-runtime/src/lib.rs`
- `crates/yunxi-agent-tools/src/lib.rs`
- `crates/yunxi-agent-storage/src/lib.rs`

构建内容：

- 从 in-memory registry 推进到 child runtime boundary。
- spawn 子 agent 时创建 session/thread metadata。
- wait/message/follow-up/interrupt/list 进入 storage-backed agent graph。
- 子 agent 复用 provider/tool/runtime facade，不依赖 Codex。
- multi-agent event 映射到 JSONL。

### 11. Storage / Rollout / Resume Full Parity

修改文件：

- `crates/yunxi-agent-storage/src/lib.rs`
- `crates/yunxi-agent-runtime/src/lib.rs`
- `crates/yunxi-agent-cli/src/main.rs`

构建内容：

- 迁移 rollout budget、thread rollout truncation、state record。
- 补齐 session import/export、resume/fork/archive/pin/unpin lifecycle。
- 补齐 parent/child history reconstruction 和 cycle detection。
- storage event 和 protocol event 可重建完整 headless run。
- CLI sessions 命令输出与 headless runtime 行为对齐。

### 12. Headless CLI Parity

修改文件：

- `crates/yunxi-agent-cli/src/main.rs`
- `crates/yunxi-agent-core/src/config.rs`
- `crates/yunxi-agent-provider/src/lib.rs`

构建内容：

- 补齐 headless 配置加载：cwd、model、provider、approval、sandbox、context、
  stream、json/jsonl、session。
- 补齐退出码：success、invalid_input、provider_error、tool_error、cancelled。
- 补齐 stderr/stdout 分离规则。
- `--jsonl` 输出严格一行一个 runtime event。
- `--backend codex` 继续作为显式兼容路径或清晰错误，不进入默认依赖。

## 最终统一验证门

Stage 4H 构建完成后一次性运行以下验证，不在中间阶段反复运行：

- `cargo fmt`
- `cargo fmt -- --check`
- `cargo test`
- `cargo check --workspace`
- `cargo build -p yunxi-agent-cli`
- `cargo run -p yunxi-agent-cli -- parity map`
- fixture streaming provider 完整回合：
  - assistant reasoning delta
  - shell tool call
  - patch tool call
  - MCP tool call
  - skill tool call
  - multi-agent tool call
  - approval/sandbox/escalation blocked path
  - final assistant response
- CLI JSONL fixture：确认一行一个 event，且 event shape 稳定。
- `cargo tree -p yunxi-agent-cli`：确认无 `codex`、`vendor`、
  `yunxi-agent-codex` 命中。
- `git diff --check`
- `cargo clean`

## 完成定义

Stage 4H 完成时必须同时满足：

- 默认 YunXi runtime 不依赖上游 Codex 源码或 crate。
- 模型/provider 层仍是可替换接口。
- 一个完整 headless Agent 回合可以经过 streaming provider、工具调用、
  approval/sandbox/escalation、context/history/resume/rollout，并输出稳定 JSONL。
- shell、patch、MCP、skill、multi-agent 都不是 schema-only，而是有 YunXi 自主
  runtime 或明确 host boundary。
- 所有新增行为都有 fixture、单元测试或集成测试在最终统一验证中覆盖。
- 编译中间产物已清理，桌面开发日志已追加。

## 风险与处理

- 如果某个上游模块牵涉非核心产品面，先迁移 headless core 所需最小行为，记录
  未迁移原因，不扩张到 TUI/cloud/update/marketplace。
- 如果真实平台 sandbox runner 在当前环境无法完整验证，先落 facade、policy、
  event、fixture runner，真实平台差异留后续专门切片。
- 如果真实 provider 网络不稳定，最终验证使用 fixture/mock transport，真实网络
  只作为可选 smoke。
- 如果 GitHub 普通 push 仍不可用，继续使用 GitHub Git Data API 推送内容等价
  提交，并在日志中记录远端 ref 和 tree。

## 下一步执行指令

下一阶段开发应直接按本报告执行：

```text
先整体迁移并构建 Stage 4H 的 12 个剩余能力面；
中间不做频繁测试和验证；
遇到局部复杂点先落 facade/fixture/兼容层并继续向后推进；
全部构建完成后统一运行最终验证门；
验证完成后清理中间产物、写桌面日志、提交并推送。
```
