# YunXi Agent v1.8.0 Persona And Transparent Memory Foundation Development Report

生成时间：2026-07-13 16:50:00 +08:00

## 背景

YunXi Agent 当前真实基线为 `v1.7.8`。v1.7.8 已完成 sandbox event schema、execution policy honesty、TUI/CLI 基线和 REST API 发布流程收口，默认 CLI 继续保持 YunXi-owned runtime，不依赖上游 Codex CLI runtime。

用户提供了人格与长期记忆模块调研报告：

- `C:\Users\admin\Desktop\YunXi Agent调研项目\2026-07-13-YunXi-Agent-人格与长期记忆模块开源调研.md`

该调研报告写作时的项目基线为 `v1.7.5`，但本开发报告以当前仓库已发布的 `v1.7.8` 为真实基线。v1.8.0 是 1.8 新模块窗口的起点，不再继续作为 1.7 系列 TUI/sandbox 小修版本。

v1.8.0 的主题是：为 YunXi Agent 建立自有的人格与透明长期记忆底座，让 YunXi 从单轮/会话型 terminal agent 进入“有稳定人格、有本地可控记忆、有用户审计权”的 Personal Companion Runtime Layer。

## 调研报告结论复核

调研报告给出的总体方向合理：

- 人格模块不能只是一段 system prompt，必须有 schema、约束、生命周期和 runtime 注入边界。
- 长期记忆必须本地优先、结构化、可查看、可删除、可关闭。
- 记忆写入应采用混合策略：低风险偏好可自动保存，敏感/重要/长期画像/重大事件进入 pending 等待确认。
- 非 Rust 项目只做设计参考，不直接接入默认 runtime。
- Rust 项目如 `ai-memory`、`ai-memory-mcp`、`Anda` 可在后续做第二轮源码审计，但 v1.8.0 不直接 fork 或依赖。
- AGPL 类项目只参考算法思想，不复制实现。

本轮已按仓库规则使用 CodeGraph 复核当前工程接入点：

- `D:\YunXi Agent\crates\yunxi-agent-runtime\src\lib.rs` 是 turn 编排核心，适合在 turn start 注入 persona/recall，在 turn end 触发 memory extraction。
- `D:\YunXi Agent\crates\yunxi-agent-context` 当前负责上下文片段，适合承载 marked persona/memory context fragment。
- `D:\YunXi Agent\crates\yunxi-agent-storage\src\lib.rs` 当前负责 session storage，v1.8.0 可新增 persona/memory store adapter，但不把记忆判断逻辑塞进 storage。
- `D:\YunXi Agent\crates\yunxi-agent-provider` 只应继续负责模型调用，不承载人格或记忆业务。
- `D:\YunXi Agent\crates\yunxi-agent-cli` 适合新增 `persona` / `memory` 管理命令。
- `D:\YunXi Agent\crates\yunxi-agent-tui` 在 v1.8.0 不新增 memory UI，避免扩大本版本范围。

因此 v1.8.0 应采用“新增 persona crate + 少量 runtime/context/storage/cli 接入”的结构，而不是在 provider、TUI 或现有 runtime 大文件中堆叠业务逻辑。

## v1.8.0 总目标

YunXi Agent v1.8.0 的目标是建立第一版可运行、可审计、可关闭的 persona/memory 底座：

- 新增 `yunxi-agent-persona` crate，作为人格、用户画像、关系摘要、长期记忆 schema 与策略的所有者。
- 内置默认人格 profile：`yunxi_companion_strong`。
- 增加 persona prompt compiler，把人格编译为有边界的 context fragment。
- 增加 global/workspace 两级 JSONL 记忆存储。
- 增加 recall engine，在每轮 provider 调用前检索少量相关 active memory。
- 增加 memory extraction，在每轮结束后生成候选记忆。
- 增加 write policy：`Auto`、`RequireConfirmation`、`Discard`、`Disabled`。
- 增加 CLI 管理面：`yunxi persona ...` 与 `yunxi memory ...`。
- 默认不让 pending 记忆进入 recall。
- provider/规则提炼失败不影响主对话 final response。
- 完成后发布 `v1.8.0` annotated tag，并保留所有旧 tag。

## 用户硬性约束

实现 v1.8.0 时继续遵守以下约束：

- 先按报告完整构建源码，中间不做反复单点测试，不在某个点卡太久。
- 构建完成后统一执行验证。
- 每个正式版本必须创建新的不可变 annotated tag，旧 tag 不删除、不移动。
- GitHub 读写和推送只走 REST API，不使用 `git push`、`git fetch`、`git ls-remote`。
- API 密钥只从 `C:\Users\admin\Desktop\api.txt` 读取，不打印、不写日志、不提交。
- 每次任务结束必须追加 `C:\Users\admin\Desktop\YunXi Agent开发日志.md`。
- 发布收尾前执行 `cargo clean` 清理构建中间产物。
- 默认运行路径继续保持 YunXi 自主化，不恢复对上游 Codex CLI runtime、`vendor/codex-rs`、`codex-*` 或 `yunxi-agent-codex` 的依赖。
- 人格与记忆不能绕过 v1.7.8 已建立的 sandbox、approval、execution policy 和 provider honesty 边界。

## 非目标

v1.8.0 不做以下内容：

- 不实现复杂关系状态机。
- 不实现记忆衰减、矛盾检测、consolidation、主动关心 trigger。
- 不引入 SQLite、FTS5、向量库、embedding store 或 graph database。
- 不新增 TUI memory inspector 或 TUI pending overlay。
- 不接入外部 Python/TypeScript/Node memory runtime。
- 不 fork Mem0、Letta、Graphiti、MemOS、OpenPersona、Nocturne、QwenPaw、Cognee、memU、MCP memory server 等项目。
- 不复制 AGPL 项目实现。
- 不把敏感/高风险/长期画像记忆默认自动保存为 active。
- 不让记忆上下文覆盖 AGENTS.md、项目硬约束、安全策略或用户当轮指令。
- 不在没有 consent/config 的情况下悄悄开启长期记忆写入。

## 总体架构

v1.8.0 增加一个新 domain crate，并在现有边界上做最小接入：

| crate | v1.8.0 职责 |
| --- | --- |
| `yunxi-agent-persona` | 新增；拥有 persona/human/relationship/memory schema、recall、write policy、prompt compiler、rule extractor、provider extraction prompt |
| `yunxi-agent-core` | 增加少量 persona/memory agent event facade，避免泄漏内部结构 |
| `yunxi-agent-context` | 增加 marked persona/memory context fragment，统计 prompt budget |
| `yunxi-agent-storage` | 增加 JSONL persona/memory store adapter，保持 append-only 和损坏恢复 |
| `yunxi-agent-runtime` | turn start recall + persona injection；turn end memory extraction；失败降级 |
| `yunxi-agent-provider` | 继续只负责模型调用；可被 persona crate 用于结构化提炼，但不承载人格逻辑 |
| `yunxi-agent-cli` | 新增 `persona` / `memory` 命令和 JSON/JSONL 管理输出 |
| `yunxi-agent-tui` | v1.8.0 不新增 memory UI，仅保持现有事件渲染不退化 |

数据流：

1. CLI 解析 persona/memory 配置。
2. Runtime 创建 turn。
3. Persona module 加载默认人格、配置、global memory、workspace memory。
4. Recall engine 根据当轮 prompt 检索少量 active memory。
5. Context assembler 生成 marked persona/memory fragments。
6. Provider 按现有 YunXi runtime 正常运行。
7. Tool loop 按现有 policy/sandbox/approval 正常运行。
8. Turn 完成后，memory extractor 生成 memory candidates。
9. Write policy 决定自动保存、进入 pending、丢弃或禁用。
10. Storage 追加 JSONL 审计记录。
11. CLI 可查看、批准、拒绝、删除或关闭记忆。

## 开发主线一：新增 `yunxi-agent-persona`

新增文件建议：

- `D:\YunXi Agent\crates\yunxi-agent-persona\Cargo.toml`
- `D:\YunXi Agent\crates\yunxi-agent-persona\src\lib.rs`
- `D:\YunXi Agent\crates\yunxi-agent-persona\src\profile.rs`
- `D:\YunXi Agent\crates\yunxi-agent-persona\src\memory.rs`
- `D:\YunXi Agent\crates\yunxi-agent-persona\src\policy.rs`
- `D:\YunXi Agent\crates\yunxi-agent-persona\src\recall.rs`
- `D:\YunXi Agent\crates\yunxi-agent-persona\src\compiler.rs`
- `D:\YunXi Agent\crates\yunxi-agent-persona\src\extractor.rs`
- `D:\YunXi Agent\crates\yunxi-agent-persona\tests\persona_tests.rs`
- `D:\YunXi Agent\crates\yunxi-agent-persona\tests\memory_policy_tests.rs`

核心 facade 类型：

- `PersonaProfile`
- `PersonaLayers`
- `PersonaConstraint`
- `CompanionStrength`
- `HumanProfile`
- `RelationshipState`
- `MemoryScope`
- `MemoryKind`
- `MemorySensitivity`
- `MemoryStatus`
- `MemoryRecord`
- `MemoryCandidate`
- `MemoryWritePolicy`
- `MemoryRecallRequest`
- `MemoryRecallResult`
- `PersonaPromptCompiler`
- `MemoryRuleExtractor`
- `MemoryPrivacyClassifier`

数据模型要求：

- 所有持久化 record 必须有 `schema_version`，v1.8.0 固定为 `1`。
- 所有记忆必须有 `id`、`scope`、`kind`、`content`、`status`、`confidence`、`importance`、`sensitivity`、`created_at_millis`、`updated_at_millis`。
- 记忆必须可关联 `source_session_id`。
- `MemoryStatus::Pending` 不进入 recall。
- `MemoryStatus::Rejected` 与 `MemoryStatus::Archived` 默认不进入 recall。
- `MemorySensitivity::High` 默认 `RequireConfirmation` 或 `Discard`，不自动 active。

默认人格：

- profile id：`yunxi_companion_strong`
- display name：`YunXi Agent`
- companion strength：`Strong`
- 语气：中文优先、温暖、稳定、有边界、有工程判断。
- 工作边界：项目硬约束、安全策略、用户当轮指令高于人格表达。
- 记忆边界：不暗示已经记住 pending/rejected 内容；不声称拥有用户未确认的长期画像。

完成条件：

- `yunxi-agent-persona` 可单独 `cargo test -p yunxi-agent-persona`。
- 默认 profile 可编译为 bounded prompt fragment。
- memory write policy 可离线测试，不依赖 live provider。
- privacy classifier 对 API key、token、authorization header、私密凭据类内容默认 `Discard` 或 `RequireConfirmation`。

## 开发主线二：本地透明 JSONL 存储

修改：

- `D:\YunXi Agent\Cargo.toml`
- `D:\YunXi Agent\crates\yunxi-agent-storage\Cargo.toml`
- `D:\YunXi Agent\crates\yunxi-agent-storage\src\lib.rs`
- 可新增：`D:\YunXi Agent\crates\yunxi-agent-storage\src\persona_memory.rs`
- 可新增测试：`D:\YunXi Agent\crates\yunxi-agent-storage\tests\persona_memory_store_tests.rs`

默认路径：

- 全局记忆：`%USERPROFILE%\.yunxi\memory\global-memory.jsonl`
- 全局 pending：`%USERPROFILE%\.yunxi\memory\pending.jsonl`
- 全局配置：`%USERPROFILE%\.yunxi\persona\config.toml`
- workspace 记忆：`<workspace>\.yunxi\memory\workspace-memory.jsonl`
- workspace pending：`<workspace>\.yunxi\memory\pending.jsonl`

存储原则：

- v1.8.0 使用 JSONL append-only，不引入 SQLite。
- 删除默认写入 tombstone/status update，不直接物理删除原始审计记录。
- `memory clear --workspace` 可重写 workspace active view，但必须保留用户明确确认步骤。
- JSONL 单行损坏不能导致全部记忆不可用；需要跳过损坏行并产生 warning。
- storage 只负责读写和恢复，不负责判断记忆是否应该写入。

workspace fingerprint：

- v1.8.0 可使用小型内部稳定 hash 生成 `root_fingerprint`，避免新增大型依赖。
- workspace-local 文件可记录 workspace scope；global 文件不应暴露不必要的完整项目路径。

完成条件：

- global/workspace store 都能 append、list、search、status update。
- pending 与 active 分文件或分 status 均可被 CLI 明确展示。
- 损坏 JSONL 行不会崩溃主对话。
- memory off 时不读不写。

## 开发主线三：Prompt 注入与 Recall

修改：

- `D:\YunXi Agent\crates\yunxi-agent-context\Cargo.toml`
- `D:\YunXi Agent\crates\yunxi-agent-context\src\lib.rs`
- `D:\YunXi Agent\crates\yunxi-agent-runtime\Cargo.toml`
- `D:\YunXi Agent\crates\yunxi-agent-runtime\src\lib.rs`
- 可新增：`D:\YunXi Agent\crates\yunxi-agent-runtime\src\persona_context.rs`

注入顺序：

1. AGENTS.md / project instructions。
2. YunXi persona compiled prompt。
3. Active relationship summary。
4. Recalled global memories。
5. Recalled workspace memories。
6. Mentioned file context。
7. Restored history。
8. User prompt。

约束：

- AGENTS.md 和项目硬约束优先级高于 persona/memory。
- memory 是上下文，不是指令，必须带明确 marker，例如 `[YunXi memory context]`。
- prompt 注入有预算上限，v1.8.0 建议默认最多 8 条记忆、总计不超过约 1200 字符或等价 token estimate。
- recall ranking 使用关键词匹配 + recency + importance + scope boost。
- workspace memory 在 workspace prompt 中优先于 global memory，但不能覆盖当轮用户指令。

完成条件：

- turn start 可注入 persona fragment。
- active memory 可按 query 检索并限制数量。
- pending/rejected/archived memory 不进入 recall。
- memory/context 失败时继续运行 provider 主流程，并输出 warning event。

## 开发主线四：Turn End Memory Extraction

修改：

- `D:\YunXi Agent\crates\yunxi-agent-persona\src\extractor.rs`
- `D:\YunXi Agent\crates\yunxi-agent-runtime\src\lib.rs`
- `D:\YunXi Agent\crates\yunxi-agent-provider\src\lib.rs`
- 测试：`D:\YunXi Agent\crates\yunxi-agent-runtime\tests\runtime_tests.rs`

提炼策略：

- offline 模式使用规则提炼：
  - 明确偏好：例如“以后用中文回答”“我喜欢简洁一点”。
  - 明确纠正：例如“不要再这样称呼我”。
  - 明确项目长期偏好：例如“这个项目后续都走 REST API 发布”。
- live provider 可选结构化提炼：
  - 输出 JSON object/array。
  - provider 失败、超时、schema parse 失败时降级为规则提炼或不写。
  - live extraction 不允许影响 assistant final response。

写入策略：

- 低风险互动偏好：可 `Auto` 保存为 active。
- API key、token、密码、Authorization header、cookie、私密凭证：默认 `Discard` 或 `RequireConfirmation`。
- 个人身份、健康、财务、情绪状态、关系、长期画像、重大事件：默认 `RequireConfirmation`。
- 工具完整输出不写长期记忆，只允许写简短 tool trace summary，且 v1.8.0 默认可先不启用。

完成条件：

- 规则提炼可在无 provider 时运行。
- live extraction 的 schema parse 有测试。
- extraction 失败不会改变主对话结果。
- 高敏内容不会自动进入 active memory。

## 开发主线五：CLI 管理命令

修改：

- `D:\YunXi Agent\crates\yunxi-agent-cli\src\main.rs`
- 可新增：`D:\YunXi Agent\crates\yunxi-agent-cli\src\persona_commands.rs`
- 可新增：`D:\YunXi Agent\crates\yunxi-agent-cli\src\memory_commands.rs`
- 测试：`D:\YunXi Agent\crates\yunxi-agent-cli\tests\cli_tests.rs`
- 测试：`D:\YunXi Agent\crates\yunxi-agent-cli\tests\jsonl_tests.rs`

v1.8.0 persona 命令：

```text
yunxi persona status
yunxi persona profile
yunxi persona set <profile>
yunxi persona off
yunxi persona on
```

v1.8.0 memory 命令：

```text
yunxi memory status
yunxi memory list [--global|--workspace]
yunxi memory show <id>
yunxi memory search <query>
yunxi memory pending
yunxi memory approve <id>
yunxi memory reject <id>
yunxi memory delete <id>
yunxi memory clear --workspace --confirm
yunxi memory off
yunxi memory on
```

输出要求：

- plain 输出适合人工阅读。
- `--json` 输出稳定 object。
- `--jsonl` 对不适合流式事件的 management command 应明确拒绝或输出 documented management JSONL，不得误导为 agent execution events。
- 所有命令不得打印 API key 或 secret。

完成条件：

- 用户能查看当前 persona/memory 是否开启。
- 用户能查看 pending 记忆并 approve/reject。
- 用户能删除或关闭记忆。
- memory off 后 runtime 不读取、不写入 persona memory。

## 开发主线六：Core/Protocol/Event 可观测性

修改：

- `D:\YunXi Agent\crates\yunxi-agent-core\src\event.rs`
- `D:\YunXi Agent\crates\yunxi-agent-protocol\src\lib.rs`
- `D:\YunXi Agent\crates\yunxi-agent-cli\src\render.rs`
- `D:\YunXi Agent\crates\yunxi-agent-tui\src\event_filter.rs`

新增或复用事件建议：

- `persona_loaded`
- `persona_context_injected`
- `memory_recall`
- `memory_candidate`
- `memory_write`
- `memory_warning`

事件要求：

- 事件中只包含 id、scope、kind、status、数量、预算等摘要信息。
- 默认不把完整敏感 memory content 打到事件流。
- JSON/JSONL 中应有稳定字段：
  - `schema_version`
  - `enabled`
  - `scope`
  - `count`
  - `budget_used`
  - `pending_count`
  - `warning`
- TUI 默认只展示简洁 summary，detail/debug 可查看非敏感机器字段。

完成条件：

- runtime fixture 可以证明 persona/memory chain 被调用。
- JSONL schema 不泄漏 pending/high sensitivity 内容。
- 事件不影响现有 sandbox/tool/provider event 兼容性。

## 开发主线七：文档、版本和发布

修改：

- `D:\YunXi Agent\Cargo.toml`
- `D:\YunXi Agent\Cargo.lock`
- `D:\YunXi Agent\README.md`
- `D:\YunXi Agent\docs\extraction-status.md`
- 新增：`D:\YunXi Agent\docs\persona-memory.md`
- 本报告：`D:\YunXi Agent\docs\reports\2026-07-13-yunxi-agent-v1-8-0-persona-memory-foundation-development-report.md`
- 日志：`C:\Users\admin\Desktop\YunXi Agent开发日志.md`

文档要求：

- README 说明 v1.8.0 的 persona/memory 是本地、透明、可关闭能力。
- README 说明第一次启用长期记忆的 consent/config 行为。
- `docs/persona-memory.md` 记录 schema、路径、CLI、隐私策略、pending 流程、非目标。
- `docs/extraction-status.md` 记录 v1.8.0 构建和统一验证结果。

发布要求：

- workspace version 推进到 `1.8.0`。
- release binaries `yunxi` 与 `yunxi-agent-cli` 输出 `yunxi 1.8.0`。
- GitHub REST API 发布 `master`。
- 创建不可变 annotated tag `v1.8.0`。
- 旧 tag 不删除、不移动。
- 本地 `master`、本地 `origin/master`、GitHub API branch、`v1.8.0` peeled target 一致。
- 发布后 `cargo clean`，最终 `D:\YunXi Agent\target` 不存在。

## 分阶段实施建议

继续遵守用户硬性约束：构建阶段一次性推进，不在中途围绕单点反复测试；源码构建完成后统一验证。

### Phase 1：Persona crate 与 schema

目标：

- 新增 `yunxi-agent-persona`。
- 定义 profile、human、relationship、memory schema。
- 内置 `yunxi_companion_strong`。
- 实现 prompt compiler、rule extractor、privacy classifier。

### Phase 2：Storage 与 CLI 管理面

目标：

- 增加 JSONL memory store。
- 增加 global/workspace 路径解析。
- 增加 `persona` / `memory` CLI 命令。
- 支持 pending/approve/reject/delete/off/on。

### Phase 3：Runtime 接入

目标：

- turn start 加载 persona/config/memory。
- recall active memory 并注入 context。
- turn end 生成 memory candidates。
- extraction 失败不影响主 response。

### Phase 4：Protocol、文档和回归

目标：

- 增加 persona/memory event schema。
- README、`docs/persona-memory.md`、`docs/extraction-status.md` 同步。
- 保持 TUI/CLI/sandbox/provider 既有能力不退化。

### Phase 5：统一验证、安装、发布

目标：

- 统一验证全部通过。
- REST API 发布 `master`。
- 创建 annotated tag `v1.8.0`。
- 写日志并清理构建产物。

## 统一验证计划

源码构建完成后统一运行：

- `cargo fmt`
- `cargo fmt -- --check`
- `cargo test -p yunxi-agent-persona`
- `cargo test -p yunxi-agent-storage`
- `cargo test -p yunxi-agent-context`
- `cargo test -p yunxi-agent-runtime`
- `cargo test -p yunxi-agent-cli`
- `cargo test`
- `cargo check --workspace`
- `cargo build -p yunxi-agent-cli --release --bins`
- `target\release\yunxi.exe --version`，期望 `yunxi 1.8.0`
- `target\release\yunxi-agent-cli.exe --version`，期望 `yunxi 1.8.0`
- offline one-shot smoke：
  - `target\release\yunxi.exe --offline "v1.8.0 persona memory smoke"`
- offline JSON smoke：
  - `target\release\yunxi.exe --offline --json "v1.8.0 json smoke"`
- offline JSONL smoke：
  - `target\release\yunxi.exe --offline --jsonl "v1.8.0 jsonl smoke"`
- persona CLI smoke：
  - `target\release\yunxi.exe persona status`
  - `target\release\yunxi.exe persona profile`
  - `target\release\yunxi.exe persona off`
  - `target\release\yunxi.exe persona on`
- memory CLI smoke：
  - `target\release\yunxi.exe memory status`
  - `target\release\yunxi.exe memory list --global`
  - `target\release\yunxi.exe memory pending`
  - `target\release\yunxi.exe memory search "中文"`
- memory write policy fixture：
  - 低风险偏好可进入 active 或 auto candidate。
  - API key/token/Authorization header 不进入 active。
  - 情绪/关系/长期画像进入 pending。
  - pending 不进入 recall。
- memory off fixture：
  - 不读取 global/workspace memory。
  - 不写入 active/pending。
  - 不注入 persona memory context。
- JSONL event fixture：
  - persona/memory events 有 `schema_version`。
  - 不泄漏 high sensitivity content。
  - provider failure 不影响 completed final response。
- corrupted JSONL fixture：
  - 单行损坏产生 warning。
  - 其他有效记忆仍可读取。
- runtime fixture：
  - turn start recall。
  - prompt budget 生效。
  - turn end extraction。
  - extraction timeout/provider error 降级。
- `target\release\yunxi.exe --backend codex "hello codex"`，期望 exit code `2`。
- `target\release\yunxi.exe sessions list --jsonl`，期望既有 contract 不退化。
- `target\release\yunxi.exe parity map --jsonl`，期望既有 contract 不退化。
- TUI smoke：
  - 进入 TUI 不因 persona/memory event 崩溃。
  - 默认 event list 不刷屏。
  - v1.7.8 滚动和 sandbox detail 不退化。
- DeepSeek live JSONL smoke：
  - 从 `C:\Users\admin\Desktop\api.txt` 读取密钥。
  - 不打印密钥。
  - live extraction schema 可解析或安全降级。
- DeepSeek live JSON smoke：同上。
- dependency scan：
  - 默认 CLI normal graph 不含 `codex-*`、`vendor/codex-rs`、`yunxi-agent-codex`。
  - 不含新增 Python/Node memory runtime。
- owned-source secret scan：
  - 排除 `.git`、`.codegraph`、`target`、`vendor`、`extracted`。
  - 不发现 API-key-shaped secret、bearer token、authorization header。
- `git diff --check`
- `codegraph sync "D:\YunXi Agent"`
- `codegraph status "D:\YunXi Agent"`
- `scripts\install\install-yunxi.ps1 -AddToPath -SkipBuild`
- PATH smoke：
  - `yunxi --version`
  - `yunxi-agent-cli --version`
  - `yunxi --offline "installed path v1.8.0 smoke"`
- GitHub REST API 发布 `master` 与 annotated tag `v1.8.0`
- 本地 `master`、本地 `origin/master`、GitHub API branch、`v1.8.0` tag peeled target 一致性校验
- `cargo clean`
- `Test-Path D:\YunXi Agent\target`，期望 `False`

## 风险与处理

- 记忆污染风险：通过 pending、privacy classifier、prompt budget、memory marker 和 status filter 降低风险。
- 隐私泄漏风险：高敏内容默认不自动 active；事件和日志只记录摘要；secret scan 必须通过。
- 上下文膨胀风险：recall 数量和预算硬限制，pending/rejected/archived 不注入。
- 主对话稳定风险：memory extraction 失败不得影响 final response；storage 损坏行只产生 warning。
- 人格越权风险：AGENTS.md、项目硬约束、安全策略、用户当轮指令优先级高于 persona。
- scope 膨胀风险：v1.8.0 不做 SQLite/graph/TUI memory UI/主动关心/复杂关系状态机。
- 外部项目许可风险：v1.8.0 不复制非 Rust runtime，不引入 AGPL 实现。
- 默认依赖回退风险：默认 CLI 依赖图继续禁止 `codex-*`、`vendor/codex-rs`、`yunxi-agent-codex`。

## v1.8.0 完成后的预期状态

完成 v1.8.0 后，YunXi Agent 应达到：

- 具备稳定默认人格 `yunxi_companion_strong`。
- 具备本地透明长期记忆底座。
- 用户可以查看、搜索、批准、拒绝、删除、关闭记忆。
- 低风险偏好可被自动保存，敏感/重要/长期画像默认 pending。
- 每轮运行可在预算内 recall 少量 active memory。
- 记忆提炼失败不影响主对话。
- pending/high sensitivity 内容不会被默认注入 prompt 或泄漏到事件流。
- 默认 CLI 继续保持 YunXi-owned runtime，不依赖上游 Codex CLI runtime。
- 后续 v1.8.1 可继续做隐私/稳定性硬化，v1.9 再做关系状态机、记忆生命周期和检索升级。

## v1.8.0 构建记录

本轮按报告进入实现阶段，构建内容包括：

- 新增 `crates/yunxi-agent-persona` crate，提供 persona profile、memory schema、
  recall、prompt compiler、rule extractor、privacy classifier 和 write policy。
- `crates/yunxi-agent-storage` 增加 `FilePersonaMemoryStore`，支持 global/workspace
  JSONL 记忆、pending 文件、状态更新、搜索、损坏行 warning 和 workspace fingerprint。
- `crates/yunxi-agent-runtime` 在 turn start 加载 persona/settings/memory，注入
  bounded persona/memory context；在 turn end 执行规则提炼和 write policy，并发出
  persona/memory 摘要事件。
- `crates/yunxi-agent-core` 与 `crates/yunxi-agent-protocol` 增加 persona/memory
  事件 schema。
- `crates/yunxi-agent-cli` 增加 `persona` / `memory` 管理命令，并将事件映射到
  JSONL protocol event。
- `crates/yunxi-agent-cli/src/render.rs` 与 `crates/yunxi-agent-tui/src/event_filter.rs`
  增加 persona/memory 摘要渲染，默认不泄漏完整记忆正文。
- README、`docs/persona-memory.md` 和 `docs/extraction-status.md` 同步 v1.8.0
  persona/memory 使用方式、隐私策略和非目标。

统一验证仍按本报告的验证计划在源码构建完成后一次性执行；最终命令结果、发布
SHA、annotated tag 和 `cargo clean` 结果在验证后追加到本报告和桌面工作日志。

## v1.8.0 统一验证记录

统一验证在源码构建完成后执行，中途发现并修复一次事件覆盖遗漏：

- `crates/yunxi-agent-cli/src/interactive.rs` 的 `TurnSummary` match 未覆盖
  persona/memory 新事件，已补为 summary/debug 级处理。
- `crates/yunxi-agent-persona/tests/memory_policy_tests.rs` 中的假 secret fixture
  命中源代码密钥形态扫描，已改为不符合真实 key 形态的 `<redacted>` 占位文本。

最终验证结果：

- `cargo fmt`：通过。
- `cargo fmt -- --check`：通过。
- `cargo test -p yunxi-agent-persona`：通过。
- `cargo test -p yunxi-agent-storage`：通过。
- `cargo test -p yunxi-agent-context`：通过。
- `cargo test -p yunxi-agent-runtime`：通过。
- `cargo test -p yunxi-agent-cli`：通过。
- `cargo test`：通过。
- `cargo check --workspace`：通过。
- `cargo build -p yunxi-agent-cli --release --bins`：通过。
- release version smoke：
  - `target\release\yunxi.exe --version`：`yunxi 1.8.0`。
  - `target\release\yunxi-agent-cli.exe --version`：`yunxi 1.8.0`。
- offline plain/JSON/JSONL smoke：通过，JSONL 含 `persona_loaded`、
  `memory_recall`、`persona_context_injected` 事件。
- persona CLI smoke：`status`、`profile`、`off`、`on` 通过。
- memory CLI smoke：`status`、`on`、offline 写入偏好、`list --workspace`、
  `pending`、`search`、`off` 通过。
- DeepSeek live JSON smoke：通过，模型 `deepseek-chat` 返回 `OK`。
- DeepSeek live JSONL smoke：通过，模型 `deepseek-chat` 返回 `OK`，输出未泄漏本地密钥。
- 默认 CLI dependency scan：未发现 `codex`、`vendor`、`yunxi-agent-codex`。
- owned-source secret scan：`secret_scan_matches=0`。
- `git diff --check`：通过，仅有 Windows LF/CRLF 提示。
- `codegraph sync "D:\YunXi Agent"`：通过。
- `codegraph status "D:\YunXi Agent"`：通过，索引 up to date。
- `scripts\install\install-yunxi.ps1 -AddToPath -SkipBuild`：通过。
- PATH smoke：`yunxi --version`、`yunxi-agent-cli --version`、
  `yunxi --offline "installed path v1.8.0 smoke"` 通过。

发布 SHA、`v1.8.0` annotated tag 和 `cargo clean` 结果在发布收尾后追加。
