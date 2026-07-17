# YunXi Agent v1.9.1 Relationship Graph Lite 开发报告

撰写时间：2026-07-17 18:59:13 +08:00  
审核报告：`C:\Users\24763\Desktop\YunXi Agent审核报告\2026-07-17-185401-YunXi-Agent-v1.9.0-源码审核报告.md`  
开发目录：`D:\YunXi Agent`  
项目内报告位置：`D:\YunXi Agent\docs\reports\2026-07-17-185913-yunxi-agent-v1-9-1-relationship-graph-lite-development-report.md`  
桌面报告位置：`C:\Users\24763\Desktop\YunXi Agent开发报告\2026-07-17-185913-yunxi-agent-v1-9-1-relationship-graph-lite-development-report.md`

本报告面向后续开发者，用于把 v1.9.0 审核通过后的下一阶段要求转化为可执行的 v1.9.1 开发任务。v1.9.1 的目标是 Relationship Graph Lite，即在不引入重型图数据库的前提下，支持实体、关系、有效期、失效链和按时间召回的轻图谱。本报告不是完成证明；验证通过、文档写回、工作区收敛、提交/tag 状态明确之前，不得宣称 v1.9.1 完成。

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

审核报告结论为：YunXi Agent v1.9.0 审核通过，可以进入下一版本开发。

v1.9.0 已闭环的能力包括：

- Boot Context 与 Dynamic Recall 已区分职责。
- `MemoryRecallRouter` 已支持 boot/dynamic 两类召回和解释信息。
- `MemoryRecallExplanation` 已提供 id、route、score、source、layer、scope、kind、reason，不输出 raw memory content。
- Persona Context Blocks 已能渲染 boot memory context 与 dynamic memory context。
- 当前 `master/origin/master` 位于 `v1.9.0` tag 之后一个 docs-only 审计提交；后续开发以当前 `master` 为基线，旧 tag 不得移动或删除。

因此 v1.9.1 的开发目标进入总纲图下一节点：

```text
v1.9.1 Relationship Graph Lite
关系/时间/实体轻图谱
```

本轮核心目标是在不引入重型图数据库的前提下，支持实体、关系、有效期、失效链。系统必须能表达“用户喜欢 X 但后来改了”这类随时间变化的事实，关系事件必须可按时间召回，旧事实不应覆盖新事实。

## 范围与非目标

### 必须完成

- 基于 v1.8.8 schema v3 的 `entities`、`temporal`、`invalidation` 字段，构建轻量关系图能力。
- 能表达实体：user、agent、workspace、project、tool、person、relationship 等。
- 能表达关系边：preference、correction、supersedes、conflicts_with、relationship_note、project_context、goal 等。
- 能表达有效期：valid_from、expires_at、event_at、observed_at。
- 能表达失效链：supersedes、superseded_by、conflicts_with、invalidated_at、expires_reason。
- 能处理“旧事实不覆盖新事实”：新事实应通过失效链或时间优先级覆盖旧事实，而不是直接删除历史。
- 关系事件必须可按时间召回。
- 保持 append-only JSONL，不引入破坏性重写。
- 保持 v1.8.7 Persona Context Blocks、v1.8.8 Memory Schema v3、v1.8.9 L0-L3 Memory Pipeline、v1.9.0 Boot Context & Recall Router 不退化。
- 更新 `docs/persona-memory.md`、`docs/extraction-status.md` 和本版本 development report。
- 阶段结束后统一验证、清理构建产物、提交、推送并创建新的 `v1.9.1` Git tag。

### 不应提前实现

- Proactive Companion Loop。
- TUI memory inspector。
- Companion Evaluation Harness。
- SQLite、vector search、外部 graph database 或外部 memory runtime。
- 云任务、marketplace、SDK packaging 或非当前阶段产品面。

## 当前源码接入点

v1.9.1 的主要接入点是 memory schema、recall 和 recall router：

- `D:\YunXi Agent\crates\yunxi-agent-persona\src\memory.rs`
  - 当前 `MemoryRecord` 已有 `entities: Vec<MemoryEntityRef>`。
  - 当前 `MemoryEntityRef` 已有 `entity_type`、`id`、`label`。
  - 当前 `MemoryTemporal` 已有 `observed_at_millis`、`event_at_millis`、`valid_from_millis`、`expires_at_millis`。
  - 当前 `MemoryInvalidation` 已有 `supersedes`、`superseded_by`、`conflicts_with`、`expires_reason`、`invalidated_at_millis`。
  - 当前这些字段是 schema v3 元数据，还没有明确组织为 graph-lite 查询能力。
- `D:\YunXi Agent\crates\yunxi-agent-persona\src\recall.rs`
  - 当前 recall 主要按 query relevance、kind trigger、importance、confidence 排序。
  - 当前 `collapse_duplicate_records` 会选择 newer/revision/importance 更高的记录。
  - v1.9.1 需要在 recall 层识别关系事件和 temporal ordering。
- `D:\YunXi Agent\crates\yunxi-agent-persona\src\recall_router.rs`
  - 当前 router 已有 boot/dynamic route 和 explanations。
  - v1.9.1 应将 graph-lite selection reason 写入 explanations，例如 `temporal_relationship_event_match`、`superseded_by_newer_fact`、`active_relation_edge`。
- 还需同步核对：
  - `crates\yunxi-agent-persona\src\merge.rs`
  - `crates\yunxi-agent-persona\src\dedup.rs`
  - `crates\yunxi-agent-persona\src\pipeline.rs`
  - `crates\yunxi-agent-storage\src\lib.rs`
  - `crates\yunxi-agent-runtime\src\lib.rs`

## 参考源码建议

审核报告为 v1.9.1 指定的参考源码：

- `D:\源码\graphiti\graphiti_core\graphiti.py`
- `D:\源码\graphiti\graphiti_core\driver`
- `D:\源码\nocturne_memory\backend\db\models.py`

参考方式必须遵守以下原则：

- 这些路径在 `D:\YunXi Agent` 项目外。读取前要明确告知用户；不得向这些目录写入内容。
- Graphiti 如果是 Python 实现，只抽取 temporal entity/relation、episode、edge invalidation、time-aware retrieval 等逻辑，不迁入 Python runtime。
- nocturne_memory 如果是 Python/DB 模型，只抽取实体、关系、版本链、有效期、失效语义，不引入其数据库或服务。
- 不要引入 Neo4j、SQLite、vector search、外部 graph database 或外部 memory runtime。
- 不要引入 `vendor/codex-rs`、`codex-*` crate 或 `yunxi-agent-codex` 到默认 YunXi 运行路径。

## 推荐技术设计

### 1. 新增轻图谱视图，而非重型数据库

建议在 `yunxi-agent-persona` 中新增 `relationship_graph.rs`，只基于已加载的 active memory records 构建内存视图：

```rust
pub struct RelationshipGraphLite {
    pub nodes: Vec<MemoryGraphNode>,
    pub edges: Vec<MemoryGraphEdge>,
}

pub struct MemoryGraphNode {
    pub entity_type: MemoryEntityType,
    pub id: String,
    pub label: Option<String>,
}

pub struct MemoryGraphEdge {
    pub id: String,
    pub from: String,
    pub to: String,
    pub relation: MemoryGraphRelation,
    pub memory_id: String,
    pub event_at_millis: Option<u128>,
    pub valid_from_millis: Option<u128>,
    pub expires_at_millis: Option<u128>,
    pub supersedes: Vec<String>,
    pub superseded_by: Option<String>,
}
```

图谱视图应从 `MemoryRecord` 派生，不应成为新的持久化数据库。持久层仍是 append-only JSONL memory records。

### 2. 关系类型

建议新增轻量 enum：

```rust
pub enum MemoryGraphRelation {
    Prefers,
    Corrects,
    RelatedTo,
    FeelsAbout,
    WorkingOn,
    GoalFor,
    Supersedes,
    ConflictsWith,
}
```

映射建议：

- `MemoryKind::Preference` -> `Prefers`。
- `MemoryKind::Correction` -> `Corrects` 或 `Supersedes`。
- `RelationshipNote` / `EmotionalState` -> `RelatedTo` / `FeelsAbout`。
- `ProjectContext` / `ToolTraceSummary` -> `WorkingOn`。
- `Goal` -> `GoalFor`。
- `MemoryInvalidation.supersedes` -> `Supersedes` edge。
- `MemoryInvalidation.conflicts_with` -> `ConflictsWith` edge。

### 3. 时间与有效期

v1.9.1 必须让关系事件可按时间召回：

- 优先使用 `temporal.event_at_millis`。
- 没有 event time 时使用 `observed_at_millis`。
- 再退到 `updated_at_millis` 或 `created_at_millis`。
- recall 和 graph query 应能按时间倒序返回 relationship events。

有效期规则：

- `expires_at_millis` 早于当前时间的记录不能作为 active fact 召回。
- `invalidated_at_millis` 存在的记录不能作为 active fact 召回，但可作为历史链路展示。
- `valid_from_millis` 未来时间的记录不能提前成为 active fact。

### 4. 旧事实不覆盖新事实

不要删除旧事实。建议策略：

- 新 correction/preference 与旧 dedup slot 冲突时，保留旧记录，并在新记录的 `invalidation.supersedes` 指向旧记录。
- 旧记录可通过 `superseded_by` 指向新记录。
- recall active fact 时选择最新未失效记录。
- 历史查询或 relationship event timeline 可返回旧记录，但要标注 superseded/invalidated。

### 5. Recall Router 接入

`MemoryRecallRouter` 应利用 graph-lite：

- boot context 选择 active current facts，不选择 superseded old facts。
- dynamic recall 对“关系/情绪/改过/以前/后来”等 query 能走 relationship event timeline。
- explanations 应包含 graph reason：
  - `active_relation_edge`
  - `temporal_event_match`
  - `superseded_by_newer_fact`
  - `expired_relation_edge`
  - `conflict_pending_confirmation`

解释仍不能输出 raw sensitive memory content。

### 6. Pipeline 和 Merge 接入

v1.9.1 应让 pipeline/merge 在生成冲突或修正候选时维护失效链：

- 语言偏好从中文改成英文：新英文偏好 supersedes 旧中文偏好，旧中文偏好保留历史但不作为 current active fact。
- 用户说“我现在不喜欢 X 了”：新 correction 或 preference supersedes 旧 preference。
- 关系事件可不覆盖旧事件，但进入 timeline。
- 高敏关系/情绪记忆仍 pending，不能自动 active。

## 测试要求

必须新增或更新以下测试：

- `relationship_graph_builds_nodes_and_edges_from_memory_entities`
- `relationship_graph_orders_events_by_time`
- `new_preference_supersedes_old_preference_without_deleting_history`
- `old_superseded_fact_is_not_recalled_as_active_boot_context`
- `relationship_events_are_recalled_by_time_for_dynamic_query`
- `expired_relationship_edge_is_excluded_from_active_recall`
- `invalidated_memory_remains_in_history_but_not_active_recall`
- `graph_explanations_include_relation_and_temporal_reason`
- `graph_explanations_do_not_include_raw_sensitive_content`
- `pipeline_correction_creates_supersession_chain`
- `schema_v3_migration_still_preserves_invalidation_fields`
- `boot_context_and_l0_l3_pipeline_regressions_still_pass`

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

- Relationship Graph Lite timeline 黑盒检查。
- Supersession chain 检查。
- old fact not active recall 检查。
- Boot Context & Dynamic Recall regression。
- Memory Schema v3 和 L0-L3 pipeline regression。
- owned-source secret scan。

如果执行安装 helper、PATH smoke、系统 PATH 修改或 `cargo clean`，必须先得到用户确认。`cargo clean` 会递归清理 `target`，也必须先确认。

## 文档与状态同步

验证通过后必须更新：

- `D:\YunXi Agent\docs\extraction-status.md`
- `D:\YunXi Agent\docs\persona-memory.md`
- `D:\YunXi Agent\docs\reports\2026-07-17-185913-yunxi-agent-v1-9-1-relationship-graph-lite-development-report.md`
- 如有版本显示变更，还需同步 `README.md`、CLI/TUI 文案和测试期望。

写回内容必须包含：

- 实际修改文件。
- 实际执行命令。
- 每条验证结果。
- release binary version 输出。
- relationship graph tests 结果。
- supersession/time-aware recall 检查结果。
- Boot Context、Schema v3、L0-L3 pipeline regression 结果。
- secret scan 结果。
- CodeGraph sync/status 结果。
- 构建产物清理结果。
- 提交、推送和 `v1.9.1` tag 状态。

## 推荐执行顺序

1. 读取本报告和 v1.9.0 审核报告，确认 v1.9.1 范围。
2. 运行只读状态检查：`git status --short --branch`、`git log -1 --oneline --decorate`。
3. 注意当前 `master` 位于 `v1.9.0` tag 之后一个 docs-only 审计提交；以当前 `master` 为开发基线，不移动旧 tag。
4. 明确告知用户后，只读参考 `D:\源码\graphiti` 和 `D:\源码\nocturne_memory` 中与 temporal graph/entity relation 相关的源码。
5. 设计 `RelationshipGraphLite`、node、edge、relation 类型。
6. 从 `MemoryRecord` 构建轻图谱视图，不改变 append-only JSONL 持久边界。
7. 扩展 merge/pipeline，让 correction/preference conflict 形成 supersession chain。
8. 扩展 recall/router，让关系事件可按时间召回，并避免旧事实作为 active fact。
9. 新增 graph、recall、pipeline、runtime regression 测试。
10. 同步版本号、文档和 development report。
11. 完成统一验证。
12. 经用户确认后清理编译中间产物。
13. 收敛工作区，提交、推送并创建 annotated `v1.9.1` tag，不删除、不移动旧 tag。
14. 追加开发日志，日志结尾署名为开发者。
15. 再提交审核。

## 实施状态（2026-07-17 开发中）

已完成的源码构造：

- 新增 `crates/yunxi-agent-persona/src/relationship_graph.rs`，从 Schema v3
  记录派生 typed nodes、relation edges、时间顺序和 active/history 视图。
- 扩展 `recall_router.rs`，让关系/情绪/变化/前后时间线查询按事件时间动态
  召回，并增加不含 raw content 的 relation/temporal explanation。
- 扩展 `yunxi-agent-storage::append_or_merge`，让明确 active 的偏好变化和
  correction 追加双向 supersession chain；旧 JSONL 行不删除。
- 扩展 runtime memory-write outcome，并新增 12 项指定命名测试。
- 同步 v1.9.1 版本显示、README、persona-memory、extraction index/status。

已执行的阶段性验证：

- `cargo fmt`：通过。
- `cargo test -p yunxi-agent-persona --test relationship_graph_tests`：10/10
  通过。
- `cargo test -p yunxi-agent-storage --test storage_tests`：22/22 通过，包含
  append-only supersession history 与 pipeline correction chain。

统一验证于 `2026-07-17 20:19:18 +08:00` 完成：

- `cargo fmt`、`cargo fmt --check`：通过。
- `cargo test -p yunxi-agent-persona`：通过；72 个 integration tests，其中
  Relationship Graph suite 11/11，通过全部 12 项硬性命名覆盖点及额外 pending
  conflict explanation 回归。
- `cargo test -p yunxi-agent-storage`：通过；5 个 unit tests、22 个 integration
  tests。
- `cargo test -p yunxi-agent-runtime`：通过；1 个 unit test、41 个 integration
  tests。
- `cargo test -p yunxi-agent-cli`：通过；22 个 binary unit tests、40 个 CLI
  integration tests、10 个 JSONL tests。
- `cargo test`、`cargo check --workspace`：通过。
- `cargo build -p yunxi-agent-cli --release --bins`：通过。
- `target\release\yunxi.exe --version`：`yunxi 1.9.1`。
- `target\release\yunxi-agent-cli.exe --version`：`yunxi 1.9.1`。
- public-facade timeline 黑盒：按 event/observed/updated/created 时间倒序通过；
  superseded/expired 历史带失效 reason，pending conflict 不进入历史召回。
- CLI supersession 黑盒：中→英 active 偏好输出 `supersession_chain`，旧记录保留、
  新记录 active、双向链成立、无错误 pending。
- Memory Schema v3、L0-L3、Boot Context、Dynamic Recall 回归：通过。
- 12 个硬性测试名检查：12/12 存在。
- owned-source secret-shape scan：0 个命中文件；测试 fake secret 运行时拼接。
- stale crate version scan：0 个 v1.9.0 命中。
- `git diff --check`：通过，仅 Windows LF/CRLF 提示。
- `codegraph sync .`、`codegraph status .`：通过；索引为 up to date。
- 经用户明确确认执行安装 helper：两个 release 二进制安装到
  `C:\Users\24763\AppData\Local\YunXi Agent\bin`，均输出 `yunxi 1.9.1`。
- 用户 PATH 已包含上述安装目录，`path_updated=False`，未重复写入 PATH。
- 经用户明确确认执行 `cargo clean`：删除 13,133 个文件、约 3.6 GiB；
  `D:\YunXi Agent\target` 已不存在。

统一验证中曾有一个旧 CLI 测试仍期待语言变化进入 pending；实际实现已按本报告
要求正确产生 supersession chain。测试更新为校验旧事实保留、新事实 active 和
双向失效链后，完整验证门重跑通过。

提交、推送和 annotated `v1.9.1` tag 尚未完成；这些结果必须在实际发布后继续
写回，因此本节当前仍不是最终发布完成证明。

报告撰写者：开发报告撰写者
