# YunXi Stage 4J Child Runtime And End-To-End Parity Harness Development Report

生成时间：2026-07-10 19:22:43 +08:00

## 核心结论

Stage 4I 已经把 YunXi Agent 推进到“默认自主 headless runtime 可运行，并且具备一批行为级
parity 基础事件和工具边界”的状态。当前默认 `yunxi-agent-cli` 依赖树仍然不包含
`vendor/codex-rs`、`codex-*` 或 `yunxi-agent-codex`，也就是说默认运行路径已经保持自主。

但 Stage 4I 结束后仍不能宣称“完全复刻 Codex CLI 端 Agent 核心能力”。目前最值得优先推进的缺口是：

```text
multi-agent child runtime 真执行
-> 父子 session/storage graph
-> child event 汇入 parent JSONL stream
-> 端到端 parity harness
```

原因是：multi-agent 能力是 Codex CLI headless Agent 的关键核心能力之一，也是把 provider loop、tool loop、
storage、context、event protocol、JSONL 输出串成完整自主系统的枢纽。Stage 4I 第二切片已经建立了
`spawn_run`、`ChildAgentRuntime` 和 `FixtureChildAgentRuntime`，下一版 Stage 4J 必须把这条 fixture 边界推进到
真实 YunXi runtime child turn。

## 硬性约束

- 默认 YunXi runtime 不得依赖 `vendor/codex-rs`。
- 默认依赖图不得包含 `codex-*` crate。
- 默认 `yunxi-agent-cli` 不得依赖 `yunxi-agent-codex`。
- `vendor/codex-rs` 和 `extracted/codex-core-agent-sources` 只能作为行为参考、源码迁移输入和 fixture 对照，不能重新进入默认运行时依赖。
- 模型/provider 层必须继续保持可替换接口，不能把 runtime 绑定死到 OpenAI。
- 不引入 TUI、desktop app、cloud tasks、doctor、update、completion、marketplace、installer 等非 headless core surfaces。
- 构建阶段先整体迁移和接线，不做中途频繁测试；全部构建完成后统一运行最终验证门。
- 遇到复杂单点时先落 facade、fixture 或兼容层并继续推进，不在单点上消耗过多时间。
- 每次任务结束必须追加桌面开发日志：`C:\Users\admin\Desktop\YunXi Agent开发日志.md`。
- 最终验证后必须清理编译中间产物，避免再次占用大量硬盘空间。

## 当前基线

Stage 4I 第二切片后已经具备：

- `yunxi-agent-provider`：增量 SSE decoder、provider stream accumulator、streamed tool name/arguments 聚合。
- `yunxi-agent-runtime`：provider stream loop、tool loop、tool runtime event 映射、context/storage state event、MCP session 和 multi-agent event 输出。
- `yunxi-agent-tools`：`ToolRuntimeEvent` 穿透 sandbox decision、MCP session、multi-agent state、patch diagnostic。
- `yunxi-agent-multi-agent`：agent graph、registry、`ChildAgentRuntime` trait、`FixtureChildAgentRuntime`、`SpawnRun` command。
- `yunxi-agent-storage`：file-backed session、history restore、session graph、runtime rollout/state snapshot。
- `yunxi-agent-patch`：Codex-style add/update/delete/move 主路径，以及 Add File 不覆盖、重复路径、move target exists、NonUtf8 等诊断边界。
- `yunxi-agent-mcp`：workspace MCP config loader、session manager、stdio/http JSON-RPC 边界。
- `yunxi-agent-cli`：默认 `yunxi` backend、JSONL 输出、sessions list/show/history/resume/graph 等 headless CLI surface。

Stage 4I 第二切片统一验证已通过：

- `cargo fmt`
- `cargo fmt -- --check`
- `cargo test`
- `cargo check --workspace`
- `cargo build -p yunxi-agent-cli`
- `cargo run -p yunxi-agent-cli -- parity map`
- `cargo run -p yunxi-agent-cli -- --backend yunxi --jsonl "run stage 4i second slice fixture"`
- `cargo tree -p yunxi-agent-cli` 依赖关键词扫描
- `git diff --check`
- `cargo clean`

## Stage 4J 总目标

Stage 4J 的目标是把 multi-agent child runtime 从 fixture 推进到真实自主执行，并建立第一条覆盖核心 Agent 回合的端到端 parity harness。

完成后，YunXi Agent 应当可以：

1. 由 parent runtime 接收 provider 发出的 `multi_agent spawn_run` tool call。
2. 创建 child session/thread metadata，并写入 YunXi-owned storage。
3. 用默认 YunXi runtime backend 执行 child turn，而不是只返回 fixture response。
4. child turn 复用 YunXi provider/tool/runtime facade，不进入 Codex backend。
5. child run 的关键事件汇入 parent event stream，并保留 child session id / parent session id。
6. `sessions graph` 能重建 parent/child session graph。
7. JSONL fixture 能逐行解析，并体现 child runtime lifecycle。
8. 默认依赖图继续不包含上游 Codex 运行时依赖。

Stage 4J 不是为了做 UI 或产品面扩张；它只补 headless Agent core。

## 上游源码参考范围

继续参考这些 Codex CLI 上游源码，但迁移结果必须落入 YunXi-owned crates：

- `vendor/codex-rs/core/src/agent/**`
- `vendor/codex-rs/core/src/agent_communication.rs`
- `vendor/codex-rs/core/src/tools/handlers/multi_agents/**`
- `vendor/codex-rs/core/src/tools/handlers/multi_agents_v2/**`
- `vendor/codex-rs/agent-graph-store/**`
- `vendor/codex-rs/thread-store/**`
- `vendor/codex-rs/core/src/thread_manager.rs`
- `vendor/codex-rs/core/src/codex_thread.rs`
- `vendor/codex-rs/core/src/session/**`
- `vendor/codex-rs/core/src/rollout.rs`
- `vendor/codex-rs/core/src/thread_rollout_truncation.rs`
- `vendor/codex-rs/core/src/event_mapping.rs`

## 构建面 1：Child Runtime Executor Boundary

目标 crate：

- `crates/yunxi-agent-runtime/src/lib.rs`
- `crates/yunxi-agent-tools/src/lib.rs`
- `crates/yunxi-agent-multi-agent/src/lib.rs`

构建内容：

- 在 runtime crate 内新增真实 child runtime executor，而不是让 `yunxi-agent-multi-agent` 依赖 runtime crate。
- 保持 `yunxi-agent-multi-agent::ChildAgentRuntime` 作为抽象边界。
- 在 `yunxi-agent-runtime` 中实现一个 adapter，消费 `ChildAgentRunRequest` 并执行 `YunXiRuntimeBackend::run_turn`。
- child executor 使用受控 recursion depth，避免 parent/child 无限递归。
- child executor 支持 fixture provider fallback，保证最终验证不依赖真实 API key。
- child executor 需要能继承 parent config 的 cwd/model/provider/sandbox/approval/context 设置，同时覆盖 child session title 和 parent session id。

完成口径：

- `spawn_run` 不再只能走 `FixtureChildAgentRuntime`。
- 默认 composite/runtime 路径能触发真实 child turn。
- child runtime 不引入 `yunxi-agent-codex` 或 `codex-*` 依赖。

## 构建面 2：Parent/Child Session Storage Graph

目标 crate：

- `crates/yunxi-agent-storage/src/lib.rs`
- `crates/yunxi-agent-runtime/src/lib.rs`
- `crates/yunxi-agent-cli/src/main.rs`

构建内容：

- child run 创建独立 `SessionRecord`，并设置 `parent_id`。
- parent session 保存时保留 child session id 事件或 metadata。
- `RuntimeStateSnapshot` 扩展 child session count / child session ids。
- `sessions graph` 能从 file-backed storage 重建 parent/child 图。
- `sessions history` 和 `sessions resume` 对 parent/child 图保持可解释语义：resume parent 不应错误拼接 child prompt；child history 只从自己的 parent chain 恢复。

完成口径：

- 一次 parent prompt 触发 child run 后，`.yunxi/sessions` 中能看到 parent 和 child 两条记录。
- `sessions graph` 输出能显示 parent -> child。
- storage 不依赖上游 thread-store。

## 构建面 3：Child Event 汇入 Parent Stream

目标 crate：

- `crates/yunxi-agent-core/src/event.rs`
- `crates/yunxi-agent-protocol/src/lib.rs`
- `crates/yunxi-agent-runtime/src/lib.rs`
- `crates/yunxi-agent-cli/src/main.rs`

构建内容：

- 明确 child event envelope：至少携带 `agent_id`、`child_session_id`、`parent_session_id`、`status`、`message`。
- parent stream 需要输出 child started / child event / child completed。
- child 内部 tool events 不应丢失；至少以 scoped event 或 summarized event 进入 parent stream。
- CLI JSONL 一行一个 event，不把 child event 合并成不可解析文本。
- 保持已有 `MultiAgentEvent` 兼容，并在需要时增加更稳定的 protocol event shape。

完成口径：

- JSONL fixture 能看到 child runtime lifecycle。
- child event 不只存在于 tool output JSON 字符串里。
- parent final response 能使用 child tool result 继续 provider loop。

## 构建面 4：End-To-End Parity Harness

目标文件：

- `crates/yunxi-agent-runtime/tests/runtime_tests.rs`
- `crates/yunxi-agent-tools/tests/tool_tests.rs`
- `crates/yunxi-agent-cli/tests/jsonl_tests.rs`
- `docs/extraction-status.md`

构建内容：

- 建立一个离线 fixture provider：parent provider 先发 `multi_agent spawn_run`，child provider 完成一轮 assistant response，parent provider 再基于 tool message 完成最终回答。
- fixture 覆盖：provider stream -> tool loop -> child runtime -> storage -> parent stream -> JSONL。
- 增加 JSONL round-trip 检查，确保新增 child events 逐行可解析。
- 增加 dependency guard，继续确认默认 CLI 依赖树没有 `codex`、`vendor`、`yunxi-agent-codex`。
- 保留 live provider / real MCP / real network 为可选 smoke，不作为最终通过条件。

完成口径：

- 一条端到端 fixture 能证明 child runtime 真执行。
- 端到端 fixture 不依赖真实 OpenAI API key。
- 端到端 fixture 不依赖真实 MCP server。
- 端到端 fixture 不依赖上游 Codex runtime。

## 构建面 5：Failure, Cancellation, And Recursion Safety

目标 crate：

- `crates/yunxi-agent-runtime/src/lib.rs`
- `crates/yunxi-agent-tools/src/lib.rs`
- `crates/yunxi-agent-multi-agent/src/lib.rs`

构建内容：

- child run 失败时，parent tool response 应为 failed，并保留结构化错误。
- child run 超过 recursion depth 时，返回明确 declined/failed，不进入死循环。
- parent interruption/cancellation 先落 facade/event 边界，真实异步 cancellation 可后续深化。
- child final response 缺失时，不让 parent runtime panic；返回结构化 failed result。
- multi-agent wait/list 对真实 child session 状态有可解释输出。

完成口径：

- child failure 不破坏 parent runtime loop。
- recursion guard 有 fixture 覆盖。
- multi-agent output 对模型可读，对 JSONL 消费者也可解析。

## 最终统一验证门

Stage 4J 构建完成后一次性运行，构建中途不反复运行：

```powershell
cargo fmt
cargo fmt -- --check
cargo test
cargo check --workspace
cargo build -p yunxi-agent-cli
cargo run -p yunxi-agent-cli -- parity map
cargo run -p yunxi-agent-cli -- --backend yunxi --jsonl "run stage 4j child runtime fixture"
cargo tree -p yunxi-agent-cli
git diff --check
cargo clean
```

额外检查：

- `cargo tree -p yunxi-agent-cli` 输出中不得出现 `codex`、`vendor`、`yunxi-agent-codex`。
- JSONL fixture 必须逐行可解析。
- 端到端 child runtime fixture 必须不依赖真实 API key。
- 验证结束后必须删除 CLI smoke 产生的 `.yunxi` 本地 session 运行痕迹，不能提交临时 session 文件。

## 完成定义

Stage 4J 完成时必须同时满足：

- `multi_agent spawn_run` 触发真实 YunXi child runtime turn。
- child session 写入 YunXi-owned storage，并可由 `sessions graph` 重建 parent/child 关系。
- child events 汇入 parent event stream，并可从 CLI JSONL 看到稳定结构。
- parent runtime 能消费 child tool result 并继续 provider loop。
- 默认 YunXi CLI 依赖树仍然不含上游 Codex runtime 依赖。
- 所有新增能力有 fixture 或集成测试覆盖，并在最终统一验证门中通过。
- 编译中间产物已清理，桌面开发日志已追加，Git 提交和推送已完成。

## 风险处理

- 如果真实 child runtime 引发 crate 循环依赖，保持 `ChildAgentRuntime` trait 在 `yunxi-agent-multi-agent`，实现放在 `yunxi-agent-runtime` 或 `yunxi-agent-tools` 的 adapter 中，不让 multi-agent crate 依赖 runtime crate。
- 如果 child runtime storage 写入与 parent storage 共享困难，先用共享 `Arc<dyn SessionStore>` adapter；不要复制上游 thread-store。
- 如果 child events 过多导致 parent JSONL 噪声过大，先采用 summarized child event envelope，后续再扩展 full scoped stream。
- 如果 recursion/cancellation 太复杂，先落 depth guard 和 structured failed result，真实 interrupt propagation 放入后续 Stage。
- 如果 GitHub 普通 push 继续失败，沿用 GitHub Git Data API 推送，并校验远端 tree/blob 与本地提交一致。

## 下一步执行指令

下一阶段开发直接按本报告执行：

```text
先整体构建 Stage 4J 的 child runtime 真执行、storage graph、child event stream、端到端 parity harness；
构建过程中不做中途测试和验证；
遇到复杂单点先落 facade / fixture / 兼容层继续推进；
全部构建完成后统一运行最终验证门；
验证后清理编译中间产物、清理本地 .yunxi 运行痕迹、写桌面日志、提交并推送。
```
