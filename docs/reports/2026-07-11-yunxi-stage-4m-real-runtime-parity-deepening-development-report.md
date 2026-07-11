# YunXi Stage 4M Real Runtime Parity Deepening Development Report

生成时间：2026-07-11 08:55:44 +08:00

## 核心结论

Stage 4L 已经完成 Codex CLI headless Agent 深层 parity 的 YunXi-owned
承载层：thread/turn 状态、provider feature matrix、unified exec、sandbox、
approval、MCP、skills、context、storage、multi-agent、protocol/JSONL 和 parity
harness 都已经有 facade、事件和离线 mega fixture。

Stage 4M 的目标不是继续扩大 facade 数量，而是把 Stage 4L 的 facade
逐步接到真实 YunXi runtime 行为上。下一阶段要把“可观察语义”推进为“真实执行语义”：
shell/exec 要真的拥有 handle/poll/cancel/stdin；approval cache 要真的影响 tool
router；MCP session 要真的长生命周期复用；context manager 要真的参与 prompt
assembly 和 auto compact；storage/rollout 要真的保存并恢复 runtime events；multi-agent
要真的通过 mailbox 和 action routing 协作。

本阶段继续遵守用户硬性约束：先整体构建和迁移，中途不做测试和验证，不在单点卡住；
全部构建完成后再统一运行最终验证门。遇到平台真实隔离、OAuth、复杂 provider 生态等深点时，
先落 YunXi-owned 真实可运行子集和兼容层，不把主线阻塞在单个平台或单个 provider 上。

## 当前基线

Stage 4L 后已经具备：

- 默认 `yunxi-agent-cli` 不依赖 `vendor/codex-rs`、`codex-*` 或
  `yunxi-agent-codex`。
- `yunxi-agent-runtime` 默认 backend 可独立运行 provider loop、tool loop、
  child runtime、storage 和 JSONL。
- DeepSeek `deepseek-v4-flash` stream 与 non-stream live smoke 已通过。
- Stage 4L offline fixture 可输出 45 行 JSONL，其中 `deep_parity_state=12`。
- CodeGraph 已同步 Stage 4L 变更。
- `target` 与根目录 `.yunxi` 已在上一阶段清理。

Stage 4L 仍然留下的关键现实差距：

- 多数 deep parity 类型仍是 facade 或 synthetic fixture，尚未成为真实运行主路径。
- `stage 4l deep parity fixture` 证明事件 shape 和状态语义存在，但不是每个深层能力的真实端到端执行。
- `yunxi-agent-runtime/src/lib.rs` 继续变大，下一阶段如果继续堆叠逻辑，维护风险会上升。
- 平台 sandbox runner、MCP auth、provider Responses-style mapping、多 agent v2 routing
  仍需要更真实的执行模型。

## 硬性约束

- 默认 YunXi runtime 不得依赖 `vendor/codex-rs`。
- 默认依赖图不得包含 `codex-*` crate。
- 默认 `yunxi-agent-cli` 不得依赖 `yunxi-agent-codex`。
- `vendor/codex-rs` 和 `extracted/codex-core-agent-sources` 只能作为参考源和对照输入。
- 模型/provider 层必须继续保持可替换接口，不得绑定到 OpenAI 或 DeepSeek。
- 真实 API key 只能从 `C:\Users\admin\Desktop\api.txt` 临时读取到环境变量。
- 命令输出、日志、报告、提交和 JSONL 都不得包含 API key 或 bearer token 原文。
- 构建阶段先整体迁移和接线，中途不运行测试；全部构建完成后统一验证。
- 遇到复杂单点时先落真实可运行子集、YunXi-owned 兼容层或 deterministic fixture，不在单点消耗过多时间。
- 任务结束必须同步桌面开发日志：
  `C:\Users\admin\Desktop\YunXi Agent开发日志.md`。
- 最终验证后必须清理 `target`、根目录 `.yunxi` 和临时 smoke 产物。
- 不引入 TUI、desktop app、cloud tasks、doctor、update、completion、marketplace、
  installer 等非 headless core surfaces。

## Stage 4M 总目标

Stage 4M 要把 Stage 4L 的 12 个 facade 面尽量转成真实自主能力。完成后应满足：

1. runtime 不只发状态事件，而是有可复用的 thread/session/turn driver。
2. provider feature matrix 真实参与请求构造、stream 聚合、错误分类和 retry 策略。
3. shell/exec 具备真实 unified exec handle、stdin、poll、cancel、output limit 和 snapshot。
4. sandbox runner 真实影响工具执行决策，并输出平台 runner attempt record。
5. approval cache 真实影响 shell、patch、MCP、skill 和 multi-agent 工具请求。
6. MCP session manager 支持 workspace 级长生命周期复用、tools/resources cache、auth/elicitation 事件。
7. skills/plugins runtime catalog 真实进入 provider tool schema 和 dynamic dispatch。
8. context manager 真实组装 prompt、估算 token、触发 auto compact、保存 prompt debug。
9. storage/rollout 真实记录 runtime events、state snapshot、history filter、resume context。
10. multi-agent v2 真实支持 mailbox、wait/message/follow-up/interrupt/list、child scoped stream。
11. protocol/JSONL 真实输出 exec/patch/MCP/approval/context/storage/multi-agent payload。
12. parity harness 从 synthetic mega fixture 升级为“真实链路 mega fixture + synthetic fallback”。

## 构建面 1：Runtime Driver 拆分与真实状态机

目标文件：

- `crates/yunxi-agent-runtime/src/lib.rs`
- 新建 `crates/yunxi-agent-runtime/src/turn_driver.rs`
- 新建 `crates/yunxi-agent-runtime/src/session_driver.rs`
- 新建 `crates/yunxi-agent-runtime/src/runtime_state.rs`
- `crates/yunxi-agent-core/src/event.rs`
- `crates/yunxi-agent-protocol/src/lib.rs`

构建内容：

- 从 `YunXiRuntimeBackend::run_turn` 中抽出 thread/session/turn driver。
- 建立 `RuntimeTurnDriver`，显式管理 turn phases：
  `started -> context_assembled -> provider_started -> provider_completed ->
  tool_loop_started -> tool_loop_completed -> storage_saved -> completed`。
- 建立 `RuntimeSessionDriver`，统一处理 session id、parent id、title、child session ids、
  rollout items 和 save failure。
- `ThreadRuntimeState`、`TurnRuntimeMetadata`、`TurnRuntimeState` 不再只由 fixture 发出，
  主路径每轮都要发出。
- provider failure、tool failure、cancelled、depth limit、storage failure 都进入同一状态机。

完成口径：

- 普通 provider-only run 可看到 thread state、turn metadata、turn state。
- tool loop run 可看到 provider/tool/storage phase 转换。
- child runtime run 可恢复 parent/child state metadata。
- Stage 4L synthetic fixture 保留，但真实 runtime fixture 优先使用新 driver。

## 构建面 2：Provider Feature Matrix 真实接线

目标文件：

- `crates/yunxi-agent-provider/src/lib.rs`
- `crates/yunxi-agent-runtime/src/turn_driver.rs`
- `crates/yunxi-agent-protocol/src/lib.rs`
- `crates/yunxi-agent-provider/tests/provider_tests.rs`

构建内容：

- `ProviderFeatureMatrix` 由配置真实生成，并进入 request build。
- 根据 matrix 控制：
  - tool schema 是否输出
  - parallel tool call flag
  - stream usage
  - reasoning delta
  - request metadata
  - unsupported schema fallback
- 增加 provider-neutral item mapper：
  assistant text、reasoning、function tool call、dynamic tool call、MCP tool call、
  usage、error。
- retry bucket 从错误分类真实接入：
  auth、rate_limit、server、network、timeout、bad_request、unsupported_schema、
  unsupported_model。
- DeepSeek profile 保持 OpenAI-compatible chat path，不绑定到 DeepSeek 专有逻辑。

完成口径：

- provider tests 证明 feature matrix 会改变请求 JSON。
- stream/non-stream 都可以产出 provider-neutral items。
- provider error JSONL 不包含 key、header 或 request body。

## 构建面 3：Unified Exec Handle 真实执行

目标文件：

- `crates/yunxi-agent-exec/src/lib.rs`
- `crates/yunxi-agent-tools/src/lib.rs`
- `crates/yunxi-agent-runtime/src/turn_driver.rs`
- `crates/yunxi-agent-protocol/src/lib.rs`

构建内容：

- 增加 `ExecHandleRegistry`，保存长生命周期进程 handle。
- `UnifiedExecRequest` 接入 `ExecManager` 主路径。
- 支持：
  - `exec start`
  - `stdin write`
  - `poll`
  - `cancel`
  - timeout cancellation
  - stdout/stderr delta
  - output limit truncation
  - shell snapshot
- 当前直接 shell 执行保留为 fallback。
- CLI JSONL 输出更接近 Codex exec begin/delta/end shape。

完成口径：

- fixture 覆盖 stdout、stderr、stdin、poll、cancel、timeout、non-zero exit。
- handle registry 可在同一 runtime 内 poll/cancel。
- 缺少平台 helper 时返回 structured diagnostic，不 panic。

## 构建面 4：Sandbox Runner 真实决策与平台 attempt record

目标文件：

- `crates/yunxi-agent-sandbox/src/lib.rs`
- `crates/yunxi-agent-tools/src/lib.rs`
- `crates/yunxi-agent-runtime/src/turn_driver.rs`
- `crates/yunxi-agent-protocol/src/lib.rs`

构建内容：

- `SandboxAttemptRecord` 接入工具执行主路径。
- read-only、workspace-write、danger-full-access、network-disabled 都必须产生 attempt record。
- Windows 先实现真实 workspace/cwd/network 决策与 diagnostic；OS 级隔离 helper 作为 hook，
  不阻塞主线。
- Linux 先实现 bwrap/landlock facade 与 diagnostic；缺 helper 时 structured fallback。
- sandbox 失败可以走 approval/escalation fallback，而不是直接吞掉工具结果。

完成口径：

- shell 和 patch 工具均能输出 sandbox attempt record。
- network-disabled 命令会被识别并生成 escalation request。
- 默认依赖图不引入上游 sandbox crate。

## 构建面 5：Granular Approval Cache 主路径接入

目标文件：

- `crates/yunxi-agent-sandbox/src/lib.rs`
- `crates/yunxi-agent-tools/src/lib.rs`
- `crates/yunxi-agent-runtime/src/session_driver.rs`
- `crates/yunxi-agent-cli/src/main.rs`

构建内容：

- `SessionApprovalCache` 进入 `ToolRouter` 或 `CompositeToolRuntime`。
- 支持 shell command key、patch touched paths key、MCP server/tool key、
  skill/plugin key、child agent key。
- 同 session 内 approved 结果复用。
- declined、ask_again、unavailable 明确区分。
- 非交互 CLI 按 policy 自动决定；后续交互 host 可以通过事件接入。
- approval request payload 包含 command、cwd、sandbox、network、tool name、target paths。

完成口径：

- 同一 session 内重复命令不重复请求 approval。
- shell、patch、MCP、skill、child agent 能区分 key。
- JSONL 不泄露环境变量值。

## 构建面 6：MCP Long-Lived Runtime

目标文件：

- `crates/yunxi-agent-mcp/src/lib.rs`
- `crates/yunxi-agent-tools/src/lib.rs`
- `crates/yunxi-agent-runtime/src/session_driver.rs`
- `crates/yunxi-agent-runtime/src/turn_driver.rs`

构建内容：

- `McpLongLivedSession` 接入 workspace-scoped manager。
- parent/child runtime 复用同一个 workspace MCP session manager。
- tools/resources cache 从 facade 变成真实 cache。
- 增加 lifecycle：
  configured、initialized、capability_negotiated、auth_required、
  elicitation_requested、tool_started、tool_completed、shutdown、reused。
- bearer-token-env-var 只解析环境变量名，不记录 token value。
- auth required、transport error、tool error、schema error、timeout、cancelled
  使用独立 failure bucket。

完成口径：

- fixture MCP server 覆盖 list tools、read resource、call tool、elicitation、auth required、
  reused session。
- child run 可复用 parent MCP manager。
- JSONL 可观察 lifecycle 细粒度事件。

## 构建面 7：Skills / Plugins Runtime Catalog 接入

目标文件：

- `crates/yunxi-agent-skills/src/lib.rs`
- `crates/yunxi-agent-tools/src/lib.rs`
- `crates/yunxi-agent-provider/src/lib.rs`
- `crates/yunxi-agent-runtime/src/turn_driver.rs`

构建内容：

- `SkillRuntimeCatalog` 在 provider request 构造前加载。
- core assets、workspace skills、plugin skills、plugin MCP seeds 合并成 provider-visible tools。
- extension tool executor 支持 model-visible schema 和 structured dispatch result。
- skill invocation 支持 instruction injection、resource load、tool call boundary、structured failure。
- tool_search 返回 source、kind、score、deferred/loadable metadata。

完成口径：

- workspace skill、plugin skill、plugin MCP、extension tool 都有真实 fixture。
- provider 请求可以看到 dynamic skill tool schema。
- manifest 错误返回 structured diagnostic。

## 构建面 8：Context Manager / Auto Compact 主路径接入

目标文件：

- `crates/yunxi-agent-context/src/lib.rs`
- `crates/yunxi-agent-runtime/src/turn_driver.rs`
- `crates/yunxi-agent-storage/src/lib.rs`
- `crates/yunxi-agent-protocol/src/lib.rs`

构建内容：

- `ContextManagerState` 从 facade 变成 prompt assembly 的真实输出。
- AGENTS.md hierarchy、file mentions、history fragments、skill injections、
  MCP/tool summaries 都进入同一 assembly。
- token budget 记录 input estimate、output reserve、threshold、used ratio。
- 预算超限时触发 deterministic compact summary，并保存到 session metadata。
- prompt debug snapshot 进入 JSONL 和 rollout。

完成口径：

- resume 后 prompt 顺序稳定。
- compact summary 可从 session history 重建。
- parent/child metadata 不因 compact 丢失。

## 构建面 9：Storage / Rollout / Thread Store 深化

目标文件：

- `crates/yunxi-agent-storage/src/lib.rs`
- `crates/yunxi-agent-runtime/src/session_driver.rs`
- `crates/yunxi-agent-cli/src/main.rs`
- `crates/yunxi-agent-protocol/src/lib.rs`

构建内容：

- `RolloutParityRecord` 接入真实 session save。
- 保存 input item、response item、tool event、runtime event、state snapshot。
- session file 写入 thread metadata、turn metadata、parent/child graph、fork lineage、
  compact summary。
- `sessions history` 支持按 turn、tool、child、runtime event 过滤。
- `sessions resume` 从 rollout 重建 context，而不是只拼旧 prompt 文本。

完成口径：

- parent -> child -> fork -> resume fixture 可重建 graph 和 history。
- 过长 rollout 会稳定 truncation。
- 老 session 文件仍可读取。

## 构建面 10：Multi-Agent v2 Action Routing

目标文件：

- `crates/yunxi-agent-multi-agent/src/lib.rs`
- `crates/yunxi-agent-tools/src/lib.rs`
- `crates/yunxi-agent-runtime/src/turn_driver.rs`
- `crates/yunxi-agent-storage/src/lib.rs`

构建内容：

- `AgentMailboxMessage` 接入 registry。
- wait/message/follow-up/interrupt/list 不再只是 registry facade，而是真实影响 child state。
- child run 继承 provider/tools/storage/context/MCP manager。
- scoped cancellation 从 parent 传播到 child。
- sub-agent activity events 覆盖 spawn begin/end、interaction begin/end、waiting begin/end、
  close begin/end。
- budget sharing 和 depth guard 进入 rollout metadata。

完成口径：

- multi-agent fixture 覆盖 spawn_run、wait、message、follow-up、interrupt、list。
- parent JSONL 能看到 child scoped stream 和 activity events。
- storage graph 可恢复 child session 和 message lineage。

## 构建面 11：Protocol / JSONL Full Shape 实装

目标文件：

- `crates/yunxi-agent-protocol/src/lib.rs`
- `crates/yunxi-agent-core/src/event.rs`
- `crates/yunxi-agent-cli/src/main.rs`
- `crates/yunxi-agent-runtime/src/turn_driver.rs`

构建内容：

- 为 exec begin/end、patch begin/end、MCP begin/end、approval、sandbox、
  context、storage、multi-agent、sub-agent activity 增加稳定 runtime payload。
- 保留旧 `item`、`tool_started`、`tool_completed` 兼容输出。
- 对新事件使用 stable snake_case type。
- CLI JSONL 错误输出继续走 structured error。
- 增加 versioned envelope smoke，但默认 JSONL 仍保持兼容。

完成口径：

- 每行 JSONL 都可被下游解析器按 type 消费。
- protocol round-trip 覆盖新增 payload。
- 旧 CLI JSONL 测试不破。

## 构建面 12：Real Parity Harness

目标文件：

- `crates/yunxi-agent-runtime/tests/runtime_tests.rs`
- `crates/yunxi-agent-cli/tests/jsonl_tests.rs`
- `crates/yunxi-agent-provider/tests/provider_tests.rs`
- `crates/yunxi-agent-tools/tests/tool_tests.rs`
- `scripts/provider/deepseek-live-smoke.ps1`
- `docs/extraction-status.md`

构建内容：

- 保留 Stage 4L synthetic mega fixture 作为 shape fallback。
- 新增 Stage 4M real mega fixture，实际执行：
  provider -> context -> approval cache -> sandbox decision -> exec handle ->
  patch -> MCP cache -> skill catalog -> multi-agent child -> storage rollout -> JSONL。
- 新增 disabled-vendor verification 命令或脚本文档。
- DeepSeek live smoke 仍独立运行，不作为 cargo test 前置条件。
- dependency keyword scan 和 owned-source secret scan 继续作为 final gate。

完成口径：

- real mega fixture 通过后，才能宣称 Stage 4M 主链路完成。
- synthetic fixture 只作为协议形状保护，不作为真实能力完成证据。
- DeepSeek live gate 通过证明真实 provider 可用。

## 最终统一验证门

Stage 4M 构建完成后一次性运行：

```powershell
cargo fmt
cargo fmt -- --check
cargo test
cargo check --workspace
cargo build -p yunxi-agent-cli
cargo run -p yunxi-agent-cli -- parity map
cargo run -p yunxi-agent-cli -- --backend yunxi --jsonl "run stage 4m real parity fixture"
cargo run -p yunxi-agent-cli -- --backend yunxi --jsonl "run stage 4l deep parity fixture"
cargo tree -p yunxi-agent-cli
# Run dependency keyword scan for codex/vendor/yunxi-agent-codex in default CLI tree.
# Run owned-source secret-pattern scan without recording sensitive regex text in docs/logs.
git diff --check
.\scripts\provider\deepseek-live-smoke.ps1 -Model deepseek-v4-flash
.\scripts\provider\deepseek-live-smoke.ps1 -Model deepseek-v4-flash -NoStream
codegraph sync "D:\YunXi Agent"
cargo clean
Remove-Item -Recurse -Force .\.yunxi -ErrorAction SilentlyContinue
```

验收时必须记录：

- 每条命令退出码。
- Stage 4M real fixture JSONL 行数和事件计数。
- Stage 4L synthetic fixture 是否仍通过。
- DeepSeek stream/non-stream 是否通过；失败时记录 classification，不记录密钥。
- 默认 CLI dependency tree 是否命中上游依赖关键词。
- owned-source secret scan 是否无匹配。
- `target` 和 `.yunxi` 是否已清理。

## 完成定义

Stage 4M 完成时必须同时满足：

- Stage 4L 的 12 个 facade 面至少有一条真实 runtime 主路径接线。
- `stage 4m real parity fixture` 不是纯 synthetic 事件，而是真实执行 provider/context/tool/storage/multi-agent 链路。
- 默认 CLI 仍不依赖上游 Codex runtime。
- DeepSeek live provider gate 在本地密钥有效时通过，且不污染离线测试。
- 文档、状态页、CodeGraph、桌面开发日志全部同步。
- 编译产物和 session smoke 产物清理完成。

## 下一步执行指令

下一阶段直接按本报告实施 Stage 4M。执行时应先整体迁移和接线这 12 个构建面，
中途不做频繁测试；全部构建完成后再统一运行最终验证门。遇到平台 sandbox、
OAuth、exec-server helper 等复杂单点时，先落真实可运行子集和 structured diagnostic，
继续推进主链路，避免在一个点上浪费大量时间。
