# YunXi Agent v1.9.0 Boot Context & Recall Router 开发报告

撰写时间：2026-07-17 18:09:11 +08:00
审核报告：`C:\Users\24763\Desktop\YunXi Agent审核报告\2026-07-17-180431-YunXi-Agent-v1.8.9-源码审核报告.md`
开发目录：`D:\YunXi Agent`
项目内报告位置：`D:\YunXi Agent\docs\reports\2026-07-17-180911-yunxi-agent-v1-9-0-boot-context-recall-router-development-report.md`
桌面报告位置：`C:\Users\24763\Desktop\YunXi Agent开发报告\2026-07-17-180911-yunxi-agent-v1-9-0-boot-context-recall-router-development-report.md`

本报告面向后续开发者，用于把 v1.8.9 审核通过后的下一阶段要求转化为可执行的 v1.9.0 开发任务。v1.9.0 的目标是 Boot Context & Recall Router，即让新会话无需用户重复说明基础关系和长期偏好，同时保留每轮动态召回。本报告不是完成证明；验证通过、文档写回、工作区收敛、提交/tag 状态明确之前，不得宣称 v1.9.0 完成。

## 硬性约束

1. 开发目录固定在 `D:\YunXi Agent` 及其工作树内。
2. 核心目标是开发一个通用型陪伴 agent。
3. 涉及到源码参考需要先抽取、迁移和构建整体能力，不要在单个点上反复纠结；如果参考的源码不为 Rust 语言，则需参考其逻辑，进行 Rust 复刻。
4. 中间无需频繁验证，完成一批构建后再统一测试和验证。
5. 验证通过前不能宣称完成。
6. 每次阶段结束后要清理编译中间产物，避免占用大量硬盘空间。
7. 每次任务结束后，都要在日志中追加本次工作的详细记录。
8. 记录必须包含时间戳、工作目标、执行流程、修改文件、文件路径、验证结果、提交和推送状态，以及日志结尾处要进行署名，署名为开发者。
9. 后续开发报告、设计文档、索引和状态文档要与实际代码状态保持一致。
10. 回复用户使用中文。
11. 之后每个版本更迭都必须提交为新的 Git tag；以前的版本 tag 不得删除，方便出错后回滚。
12. 涉及到 shell 命令时要万分小心，例如递归删除、强制移动、清空目录、系统级安装/卸载、修改 PATH/注册表/系统配置等操作必须得到用户确认。
13. 不可因为过长的上下文忽略硬性要求，如果察觉到硬性要求的效力已经减弱，则必须汇报当前情况。
14. 工作尽量固定在项目文件夹，涉及到其他文件夹的操作，要明确告知用户，尤其是 C 盘的用户目录。

补充说明：开发报告和开发日志是用户明确指定的例外输出位置，可写入 `C:\Users\24763\Desktop\YunXi Agent开发报告\` 与 `C:\Users\24763\Desktop\YunXi Agent开发日志.md`；除此之外，开发操作应固定在 `D:\YunXi Agent` 及其工作树内。若需要读取 `D:\源码` 下的参考项目，也必须先明确告知用户这是项目外参考读取，不得向其中写入内容。

## 审核结论转开发目标

审核报告结论为：YunXi Agent v1.8.9 审核通过，可以进入下一版本开发。

v1.8.9 已闭环的能力包括：

- Rust-native L0-L3 Memory Pipeline 已接入每轮 memory extraction 主链路。
- L0 raw turn、L1 structured fact、L2 relationship event、L3 profile summary 已有分层候选策略。
- Pending/Active/Rejected 流程可测，敏感记忆可降级或待确认。
- provider failure fail-soft，不阻断 rule candidates。
- v1.8.7 Persona Context Blocks 与 v1.8.8 Memory Schema v3 保持不退化。
- v1.8.9 已有统一验证、文档记录、干净工作区、annotated `v1.8.9` tag 和构建产物清理记录。

因此 v1.9.0 的开发目标进入总纲图下一节点：

```text
v1.9.0 Boot Context & Recall Router
启动上下文与召回路由
```

本轮核心目标是让新会话自动加载高价值长期上下文，即 boot context；每轮仍执行 dynamic recall，并明确二者职责差异。boot context 必须有 token/char 预算，不能无限注入；recall 结果必须可解释，至少能说明召回来源、数量和选择原因。

## 范围与非目标

### 必须完成

- 新增 boot context 构建路径：会话启动或首轮 turn 前加载高价值长期上下文。
- 保留 dynamic recall：每轮按用户当前 prompt 和工作区上下文动态召回相关记忆。
- 区分 boot context 与 dynamic recall：
  - boot context 负责长期稳定偏好、关系基线、工作区长期事实、常驻安全边界。
  - dynamic recall 负责当前 query 相关、近期或任务相关记忆。
- 为 boot context 设置独立预算，不能无限注入。
- 为 dynamic recall 继续设置预算，并避免与 boot context 重复注入。
- recall 结果必须可解释：记录来源、层级、scope、kind、分数、选择原因、丢弃原因和数量。
- 继续保证只召回 active、未过期、未失效、未被 privacy policy 禁止的 memory。
- 保持 v1.8.7 Persona Context Blocks、v1.8.8 Memory Schema v3、v1.8.9 L0-L3 Memory Pipeline 不退化。
- 更新 `docs/persona-memory.md`、`docs/extraction-status.md` 和本版本 development report。
- 阶段结束后统一验证、清理构建产物、提交、推送并创建新的 `v1.9.0` Git tag。

### 不应提前实现

- Relationship Graph Lite。
- Proactive Companion Loop。
- TUI memory inspector。
- Companion Evaluation Harness。
- SQLite、vector search、graph memory 或外部 memory runtime。
- 云任务、marketplace、SDK packaging 或非当前阶段产品面。

## 当前源码接入点

v1.9.0 的主要接入点是 runtime persona context 构建和 memory recall：

- `D:\YunXi Agent\crates\yunxi-agent-runtime\src\lib.rs`
  - 当前 `build_persona_turn_context(config, prompt)` 只执行每轮 dynamic recall。
  - 当前逻辑在 memory enabled 时读取 `store.active_records()`，构造 `MemoryRecallRequest::new(prompt)`。
  - 当前 dynamic recall 参数为 `max_records = 8`、`budget_chars = 1200`。
  - 当前 `PersonaPromptCompiler::compile` 只接收 `memory_recall.records`，尚未区分 boot context 和 dynamic recall。
  - `emit_persona_context_events` 当前能发出 `MemoryRecall` 事件，但缺少解释性 recall route 明细。
- `D:\YunXi Agent\crates\yunxi-agent-persona\src\recall.rs`
  - 当前 `MemoryRecallEngine::recall` 会 collapse duplicates、过滤 workspace scope、过滤不可召回记录、按 query relevance 和 importance/confidence 排序。
  - 当前 `MemoryRecallResult` 包含 records、budget_used_chars、truncated、always_on_count、dropped_unrelated、dropped_by_budget、dropped_duplicates。
  - 当前 recall 没有显式返回每条记录的选择原因、分数、来源、层级或 dropped reason。
- `D:\YunXi Agent\crates\yunxi-agent-persona\src\memory.rs`
  - v1.8.8 已有 schema v3 字段，包括 layer、entities、temporal、evidence、source、invalidation。
  - v1.9.0 应利用这些字段做 boot/dynamic route，而不是新增平行私有字段。
- `D:\YunXi Agent\crates\yunxi-agent-persona\src\pipeline.rs`
  - v1.8.9 pipeline 写入的 layer/source/evidence 可作为 recall explanation 的依据。

## 参考源码建议

审核报告为 v1.9.0 指定的参考源码：

- `D:\源码\ai-memory-mcp\src\cli\boot.rs`
- `D:\源码\ai-memory-mcp\src\store\mod.rs`
- `D:\源码\letta\letta\schemas\memory.py`

参考方式必须遵守以下原则：

- 这些路径在 `D:\YunXi Agent` 项目外。读取前要明确告知用户；不得向这些目录写入内容。
- ai-memory-mcp 如果是 Rust 实现，只参考 boot context 选择、store 读取、预算和 CLI/diagnostic 思路，不直接迁入其 crate。
- Letta 如果是 Python 实现，只抽取 memory blocks、stable memory sections、recall/explanation 的逻辑，复刻到 YunXi Rust persona/runtime 中。
- 不要引入 MCP 服务、外部 store、SQLite/vector search/graph memory 到 YunXi 默认路径。
- 不要引入 `vendor/codex-rs`、`codex-*` crate 或 `yunxi-agent-codex` 到默认 YunXi 运行路径。

## 推荐技术设计

### 1. 新增 Recall Router

建议在 `yunxi-agent-persona` 中新增 `recall_router.rs`，或在 `recall.rs` 内先实现轻量 router：

```rust
pub struct MemoryRecallRouter {
    engine: MemoryRecallEngine,
}

pub struct MemoryRecallRouterRequest {
    pub query: String,
    pub workspace_fingerprint: Option<String>,
    pub boot_budget_chars: usize,
    pub dynamic_budget_chars: usize,
    pub boot_max_records: usize,
    pub dynamic_max_records: usize,
}

pub struct MemoryRecallRouterResult {
    pub boot_context: MemoryRecallResult,
    pub dynamic_recall: MemoryRecallResult,
    pub explanations: Vec<MemoryRecallExplanation>,
}
```

如果不新增文件，也必须让 boot context 与 dynamic recall 的职责在类型层面可见，不要只靠两个局部变量隐藏在 runtime 中。

### 2. 扩展 Recall Result Explanation

建议新增：

```rust
pub struct MemoryRecallExplanation {
    pub memory_id: String,
    pub route: MemoryRecallRoute,
    pub score: f32,
    pub selected: bool,
    pub reason: String,
    pub source: String,
    pub layer: String,
    pub scope: String,
    pub kind: String,
}

pub enum MemoryRecallRoute {
    Boot,
    Dynamic,
    DroppedDuplicate,
    DroppedUnrelated,
    DroppedBudget,
    DroppedInvalid,
}
```

解释信息不得泄露完整私密 memory content。日志和事件应优先输出 id、kind、scope、source、route、reason、计数和预算，不直接打印内容。

### 3. Boot Context 选择规则

boot context 负责“新会话基础关系和长期偏好”，建议选择：

- 全局语言/称呼/语气偏好。
- 长期稳定 companion interaction preference。
- 关系 familiarity、长期边界、长期目标摘要。
- 当前 workspace 的稳定项目事实和项目约束。
- L3 profile summary。

boot context 不应选择：

- pending/rejected/archived。
- expired/invalidated。
- secret-like 或 high sensitivity。
- 低置信、低重要度、纯事件噪声。
- 与当前 workspace 不匹配的 workspace memory。

boot context 应有独立预算，例如 `boot_budget_chars = 1000`、`boot_max_records = 6`。具体数值可按现有 1200 dynamic budget 保守设计。

### 4. Dynamic Recall 选择规则

dynamic recall 继续使用 prompt/query 相关性，但要避开 boot context 已注入记录：

- 优先当前 prompt 命中的 workspace/project/tool/task memory。
- 允许召回近期 L1/L2 事件、goal、correction、relationship note。
- 与 boot context 重复的 dedup_key 不再重复注入。
- dropped reason 要可解释。

dynamic recall 应继续有独立预算，例如 `dynamic_budget_chars = 1200`、`dynamic_max_records = 8`。

### 5. Runtime 接入

`build_persona_turn_context` 应升级为：

- 加载 active records。
- 调用 recall router 构造 boot context 和 dynamic recall。
- 将两部分交给 `PersonaPromptCompiler`。
- 如果 compiler 暂时只接收一个 memory slice，可先在 runtime 或 persona crate 内合并，但必须保留 route/explanation 数据。
- `emit_persona_context_events` 应发出 boot context 与 dynamic recall 的统计事件，至少包括 count、budget_used_chars、dropped_by_budget、dropped_duplicates 和 route explanation summary。

建议新增或扩展 `PersonaTurnContext` 字段：

```rust
pub boot_context: MemoryRecallResult,
pub dynamic_recall: MemoryRecallResult,
pub recall_explanations: Vec<MemoryRecallExplanation>,
```

若为了控制变更面不直接暴露全部字段，也必须在 runtime event 中保留可审计 summary。

### 6. Persona Context Blocks 不退化

Persona compiler 应明确渲染：

- boot memory context。
- dynamic memory context。
- 两者都必须声明 memory 是 context，不是 instruction。
- boot context 和 dynamic recall 的 block 标签稳定、可测试。
- 不要让 boot context 覆盖 AGENTS.md、用户当轮请求、sandbox policy、privacy policy 或工具边界。

## 测试要求

必须新增或更新以下测试：

- `boot_context_selects_stable_global_preferences`
- `boot_context_selects_workspace_project_context_with_matching_fingerprint`
- `boot_context_respects_budget_and_max_records`
- `boot_context_excludes_pending_rejected_archived_expired_invalidated`
- `dynamic_recall_uses_prompt_relevance`
- `dynamic_recall_deduplicates_against_boot_context`
- `recall_explanations_include_route_source_score_and_reason`
- `recall_explanations_do_not_include_raw_sensitive_content`
- `runtime_emits_boot_and_dynamic_recall_summaries`
- `persona_context_blocks_render_boot_and_dynamic_memory_as_context_not_instruction`
- `memory_pipeline_v1_8_9_tests_still_pass`
- `memory_schema_v3_migration_tests_still_pass`

测试中不得硬编码真实 secret、token 或用户私密内容。需要模拟敏感文本时，应使用运行时拼接的 fake secret 片段，避免仓库 secret scan 误报。

## 统一验证要求

完成一批源码和文档修改后，再统一执行验证。建议至少包括：

```powershell
cargo fmt
cargo fmt --check
cargo test -p yunxi-agent-persona
cargo test -p yunxi-agent-storage
cargo test -p yunxi-agent-runtime
cargo test -p yunxi-agent-cli
cargo test
cargo check --workspace
cargo build -p yunxi-agent-cli --release --bins
target\release\yunxi.exe --version
target\release\yunxi-agent-cli.exe --version
git diff --check
codegraph sync .
codegraph status .
```

还应补充：

- 新会话 boot context 黑盒检查。
- 每轮 dynamic recall 黑盒检查。
- recall explanation JSON/JSONL smoke。
- boot/dynamic budget 检查。
- Persona Context Blocks regression。
- Memory Schema v3 和 L0-L3 pipeline regression。
- owned-source secret scan。

如果执行安装 helper、PATH smoke、系统 PATH 修改或 `cargo clean`，必须先得到用户确认。`cargo clean` 会递归清理 `target`，也必须先确认。

## 文档与状态同步

验证通过后必须更新：

- `D:\YunXi Agent\docs\extraction-status.md`
- `D:\YunXi Agent\docs\persona-memory.md`
- `D:\YunXi Agent\docs\reports\2026-07-17-180911-yunxi-agent-v1-9-0-boot-context-recall-router-development-report.md`
- 如有版本显示变更，还需同步 `README.md`、CLI/TUI 文案和测试期望。

写回内容必须包含：

- 实际修改文件。
- 实际执行命令。
- 每条验证结果。
- release binary version 输出。
- boot context 测试结果。
- dynamic recall 测试结果。
- recall explanation 检查结果。
- Persona Context Blocks、Schema v3、L0-L3 pipeline regression 结果。
- secret scan 结果。
- CodeGraph sync/status 结果。
- 构建产物清理结果。
- 提交、推送和 `v1.9.0` tag 状态。

## 推荐执行顺序

1. 读取本报告和 v1.8.9 审核报告，确认 v1.9.0 范围。
2. 运行只读状态检查：`git status --short --branch`、`git log -1 --oneline --decorate`。
3. 明确告知用户后，只读参考 `D:\源码\ai-memory-mcp` 和 `D:\源码\letta` 中与 boot context/recall router 相关的源码。
4. 设计 `MemoryRecallRouter`、route 类型和 explanation 类型。
5. 扩展 recall engine，让选择与丢弃原因可解释。
6. 在 runtime 中接入 boot context 与 dynamic recall。
7. 调整 Persona Context Blocks，稳定渲染 boot/dynamic memory blocks。
8. 新增 persona/runtime/CLI smoke 测试。
9. 同步版本号、文档和 development report。
10. 完成统一验证。
11. 经用户确认后清理编译中间产物。
12. 收敛工作区，提交、推送并创建 annotated `v1.9.0` tag，不删除、不移动旧 tag。
13. 追加开发日志，日志结尾署名为开发者。
14. 再提交审核。

## 开发执行记录（2026-07-17）

本轮已在 `D:\YunXi Agent` 完成集中开发和统一验证，硬性要求未因上下文压缩而弱化。实现内容如下：

- 新增 `crates/yunxi-agent-persona/src/recall_router.rs`：提供 `MemoryRecallRouter`、独立 boot/dynamic 预算、跨路由去重、稳定性/作用域/隐私过滤和不含正文的 `MemoryRecallExplanation`。
- 扩展 `crates/yunxi-agent-persona/src/recall.rs`：保留 v1.8.9 兼容召回入口，并为 Dynamic Recall 增加纯 prompt relevance 内部入口。
- 扩展 `crates/yunxi-agent-persona/src/compiler.rs`：新增稳定的 `boot_memory_context` 与 `dynamic_memory_context`；恢复会话跳过 Boot block；两个 block 均保留 context-not-instruction 和高优先级边界。
- 扩展 `crates/yunxi-agent-runtime/src/lib.rs`：新会话首轮路由 Boot Context，恢复会话仅执行 Dynamic Recall；Dynamic 查询纳入独立受限的 recent user/assistant turns；分别发出 `scope=boot` 与 `scope=dynamic` 的摘要事件。
- 新增 `crates/yunxi-agent-persona/tests/recall_router_tests.rs` 并扩展 runtime/CLI 测试，覆盖报告指定的 12 个测试名称和 JSON/JSONL 摘要行为。
- 版本、README、Persona/Memory 设计说明、提取状态、CLI/TUI 展示与测试期望已同步到 `1.9.0`。

已通过的统一验证：

- `cargo fmt` 与 `cargo fmt --check`：通过。
- `cargo test -p yunxi-agent-persona`：通过；新增 11 个 Recall Router 测试，既有 Persona、Schema v3、L0-L3 Pipeline、Policy 测试全部通过。
- `cargo test -p yunxi-agent-storage`：通过；5 个单元测试和 21 个集成测试。
- `cargo test -p yunxi-agent-runtime`：通过；1 个 recent-turn query 单元测试和 41 个集成测试，包含 `runtime_emits_boot_and_dynamic_recall_summaries`。
- `cargo test -p yunxi-agent-cli`：通过；22 个二进制单元测试、40 个 CLI 集成测试和 10 个 JSONL 测试。
- `cargo test` 与 `cargo check --workspace`：通过。
- `cargo build -p yunxi-agent-cli --release --bins`：通过；两个 release 二进制均输出 `yunxi 1.9.0`。
- 隔离黑盒：新会话 Boot 召回 2 条、78 字符，Dynamic 0 条；恢复会话 Boot 0 条、Dynamic 1 条、63 字符；boot query 为空且 dynamic query 安全。
- explanation JSON 序列化、敏感正文不泄漏、预算/上限、Persona Context Blocks、Schema v3、L0-L3 regression：通过。
- owned-source key-shape scan：0 命中；`git diff --check`：通过。
- `codegraph sync .` 与 `codegraph status .`：通过；索引为 1,169 files、45,164 nodes、146,675 edges，状态 current。

经用户明确确认后，已把最终双二进制安装到 `C:\Users\24763\AppData\Local\YunXi Agent\bin`；两个安装后二进制均输出 `yunxi 1.9.0`。用户 PATH 已包含该目录，installer 报告 `path_updated=False`，未进行重复写入。隔离黑盒目录 `D:\YunXi Agent\.tmp\v190-blackbox` 已递归删除。补齐 recent-turn 审计并重建 release 后，最终一次 `cargo clean` 已清理 9,195 个文件、约 2.7 GiB，仓库不保留构建产物。

发布已通过 GitHub Git Data REST API 完成，全程 `force=false`：release commit 为 `4e014314df9c9296b5fb843b13bf390c71f6d4e0`，release tree 为 `2580cc625ff5cc640af438af9250333677c876c3`，annotated `v1.9.0` tag object 为 `2625358b36861811912e4c2be2671b0db04ff5db`，tag 解引用到同一 release commit。GitHub 标签总数由 27 增加为 28；旧 `v1.8.9` tag object 保持 `3ab5c70dc3edc69583fa863412c1b4d454bd6f29`，未删除、未移动。API 首次创建 commit 因 PowerShell 将消息序列化为数组而返回 422，远端引用未变；修正为单字符串后发布成功。Git fetch 首次使用 Bearer 头被拒绝，改用 GitHub 支持的 `x-access-token` Basic 头后完成只读 fetch，并在 tree 完全一致后把本地 `master` 与远端对齐。

本报告随发布后审计提交写回；审计提交自身的最终对象 id 与推送状态记录在外部时间戳开发日志中，避免要求一个 Git commit 在自身内容中预先包含其内容派生 SHA。v1.9.0 开发、验证、安装、清理和 release tag 闭环已完成。

执行记录维护者：开发者
