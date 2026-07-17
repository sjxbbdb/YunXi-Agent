# YunXi Agent v1.8.8 Memory Schema v3 开发报告

撰写时间：2026-07-17 15:37:15 +08:00
审核报告：`C:\Users\24763\Desktop\YunXi Agent审核报告\2026-07-17-153224-YunXi-Agent-v1.8.7-源码审核报告.md`
开发目录：`D:\YunXi Agent`
项目内报告位置：`D:\YunXi Agent\docs\reports\2026-07-17-153715-yunxi-agent-v1-8-8-memory-schema-v3-development-report.md`
桌面报告位置：`C:\Users\24763\Desktop\YunXi Agent开发报告\2026-07-17-153715-yunxi-agent-v1-8-8-memory-schema-v3-development-report.md`

本报告面向后续开发者，用于把 v1.8.7 审核通过后的下一阶段要求转化为可执行的 v1.8.8 开发任务。v1.8.8 的目标是 Memory Schema v3，即长期记忆模型升级。本报告不是完成证明；验证通过、文档写回、工作区收敛、提交/tag 状态明确之前，不得宣称 v1.8.8 完成。

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

审核报告结论为：YunXi Agent v1.8.7 审核通过，可以进入下一版本开发。

v1.8.7 已闭环的能力包括：

- `Persona Context Blocks` 已将 persona、human、relationship、memory、boundaries 渲染为稳定结构化块。
- `PersonaPromptCompiler` 已有 block 构造、转义、预算、截断和 memory-only 结构化路径。
- runtime 仍保持窄接入，只消费 `CompiledPersonaContext.content`。
- v1.8.7 已有统一验证、文档记录、干净工作区、annotated `v1.8.7` tag 和构建产物清理记录。

因此 v1.8.8 的开发目标进入总纲图下一节点：

```text
v1.8.8 Memory Schema v3
记忆模型升级
```

本轮核心目标是扩展长期记忆模型，为通用型陪伴 agent 的长期关系记忆打地基。开发者必须扩展记忆字段：层级、实体、时间、证据、来源、置信度、失效关系，并保证 v1/v2 记忆可迁移、旧 JSONL 不丢失、v1.8.7 Persona Context Blocks 不退化。

## 范围与非目标

### 必须完成

- 将 `MemoryRecord::schema_version` 当前值从 v2 升级到 v3。
- 为长期记忆新增结构化字段，覆盖：
  - 层级：记忆所属层级或未来 L0-L3 路由的基础标记。
  - 实体：记忆涉及的人、项目、工具、工作区或关系对象。
  - 时间：事件时间、观察时间、有效时间、过期时间或失效时间。
  - 证据：候选记忆来源片段、摘要、证据类型或证据引用。
  - 来源：来源 session、turn、extractor、rule/provider、workspace fingerprint 等。
  - 置信度：保留现有 `confidence`，并明确其 schema v3 语义与范围。
  - 失效关系：被哪条记忆替代、修正、撤销、过期或冲突。
- 保证 v1 和 v2 JSONL 记录可迁移到 v3。
- 保证现有 append-only JSONL 存储不丢失旧记录；迁移应发生在读取/加载边界，不要破坏历史文件。
- 新增 schema migration 测试。
- 保持 v1.8.7 Persona Context Blocks 输出稳定，不因记忆字段扩展退回松散文本。
- 更新 `docs/persona-memory.md`、`docs/extraction-status.md` 和本版本 development report。
- 阶段结束后统一验证、清理构建产物、提交、推送并创建新的 `v1.8.8` Git tag。

### 不应提前实现

- 完整 L0-L3 Memory Pipeline。
- Boot Context & Recall Router。
- Relationship Graph Lite。
- Proactive Companion Loop。
- TUI memory inspector。
- Companion Evaluation Harness。
- SQLite、vector search、graph memory 或外部 memory runtime。
- 云任务、marketplace、SDK packaging 或非当前阶段产品面。

## 当前源码接入点

v1.8.8 的主要 blast radius 在 persona memory 模型、迁移、存储、召回、去重、合并和抽取链路。

- `D:\YunXi Agent\crates\yunxi-agent-persona\src\memory.rs`
  - 当前 `SCHEMA_VERSION: u32 = 2`。
  - `MemoryRecord` 当前字段包括 `id`、`schema_version`、`scope`、`kind`、`content`、`source_session_id`、`confidence`、`importance`、`sensitivity`、`status`、`created_at_millis`、`updated_at_millis`、`dedup_key`、`revision`、`merged_count`。
  - `MemoryCandidate` 当前有 `proposed_record`、`evidence`、`write_policy`、`reason`；v1.8.8 应考虑把 evidence/source 从候选期语义稳定映射到 durable record。
- `D:\YunXi Agent\crates\yunxi-agent-persona\src\migration.rs`
  - 当前 `migrate_memory_record_value` 支持当前 schema、v1 和 missing schema。
  - 当前 `migrate_v1` 将 v1 记录升级到 `SCHEMA_VERSION`，并填充 dedup/revision/merged_count。
  - v1.8.8 必须新增 v2 迁移路径，且保留 v1 迁移路径。
- `D:\YunXi Agent\crates\yunxi-agent-storage\src\lib.rs`
  - `FilePersonaMemoryStore` 负责本地 append-only JSONL、active records、dedup collapse、merge、status update 等存储边界。
  - v1.8.8 不应改成破坏性重写存储；迁移应保持可审查、可追溯。
- 需要同步核对的影响面：
  - `crates\yunxi-agent-persona\src\recall.rs`
  - `crates\yunxi-agent-persona\src\dedup.rs`
  - `crates\yunxi-agent-persona\src\merge.rs`
  - `crates\yunxi-agent-persona\src\policy.rs`
  - `crates\yunxi-agent-persona\src\provider_extractor.rs`
  - `crates\yunxi-agent-persona\src\extractor.rs`
  - `crates\yunxi-agent-runtime\src\lib.rs`
  - `crates\yunxi-agent-cli\src\main.rs`

## 参考源码建议

审核报告为 v1.8.8 指定的参考源码：

- `D:\源码\memU\src\memu\database\models.py`
- `D:\源码\nocturne_memory\backend\db\models.py`
- `D:\源码\yantrikdb\crates\yantrikdb-core\src\engine\lifecycle.rs`

参考方式必须遵守以下原则：

- 这些路径在 `D:\YunXi Agent` 项目外。读取前要明确告知用户；不得向这些目录写入内容。
- memU 和 nocturne_memory 如果是 Python 实现，只抽取长期记忆模型字段、实体关系、时间语义、证据/source 记录方式，不迁入 Python runtime。
- yantrikdb 如果是 Rust 实现，也只能参考 lifecycle、失效、替换、版本演进等设计逻辑，不把其 crate 直接引入 YunXi 默认运行路径。
- 不要把参考项目作为默认运行时依赖，不要引入数据库服务或外部 memory runtime。
- 不要引入 `vendor/codex-rs`、`codex-*` crate 或 `yunxi-agent-codex` 到默认 YunXi 运行路径。

## 推荐技术设计

### 1. 扩展 MemoryRecord 为 v3

建议在 `memory.rs` 中将 `SCHEMA_VERSION` 提升为 3，并为 v3 新增小型结构化类型，优先保持 serde 兼容和默认值：

```rust
pub const SCHEMA_VERSION: u32 = 3;

pub struct MemoryRecord {
    // existing fields...
    #[serde(default)]
    pub layer: MemoryLayer,
    #[serde(default)]
    pub entities: Vec<MemoryEntityRef>,
    #[serde(default)]
    pub temporal: MemoryTemporal,
    #[serde(default)]
    pub evidence: Vec<MemoryEvidence>,
    #[serde(default)]
    pub source: MemorySource,
    #[serde(default)]
    pub invalidation: MemoryInvalidation,
}
```

建议新增类型：

- `MemoryLayer`
  - 初期可用 `Profile`、`Preference`、`Relationship`、`Workspace`、`Episode`、`ToolTrace`、`Unknown`。
  - 不要在 v1.8.8 中实现完整 L0-L3 pipeline，但字段要为后续路由留下位置。
- `MemoryEntityRef`
  - 字段建议包括 `entity_type`、`id`、`label`。
  - 用于表达 user、agent、workspace、project、tool、person、relationship 等对象。
- `MemoryTemporal`
  - 字段建议包括 `observed_at_millis`、`event_at_millis`、`valid_from_millis`、`expires_at_millis`。
  - 默认使用 `created_at_millis` / `updated_at_millis` 填充。
- `MemoryEvidence`
  - 字段建议包括 `kind`、`summary`、`source_session_id`、`source_turn_id` 或 `source_event_id`。
  - 注意不要持久化高风险原文；必要时只存摘要或 redacted excerpt。
- `MemorySource`
  - 字段建议包括 `extractor`、`session_id`、`workspace_fingerprint`、`provider`、`rule_id`。
  - 兼容现有 `source_session_id`，不要马上删除旧字段。
- `MemoryInvalidation`
  - 字段建议包括 `supersedes`、`superseded_by`、`conflicts_with`、`expires_reason`、`invalidated_at_millis`。

### 2. 保持向后兼容

不要删除 v2 字段。`source_session_id`、`confidence`、`importance`、`dedup_key`、`revision`、`merged_count` 仍要保留，避免 CLI、存储、merge、recall 立即大范围破裂。

v3 新字段必须有 `#[serde(default)]` 或 `skip_serializing_if` 策略，确保旧 JSONL 在读取时不因缺字段失败。默认值要语义明确，例如：

- `layer = MemoryLayer::Unknown` 或按 `MemoryKind` 推导。
- `entities = []`。
- `temporal.observed_at_millis = created_at_millis`。
- `source.session_id = source_session_id.clone()`。
- `evidence = []`，如果候选里有 evidence，则新写入记录要填充。
- `invalidation = MemoryInvalidation::default()`。

### 3. 迁移策略

`migration.rs` 必须保留：

- missing schema 或 v1 迁移。
- v2 迁移。
- 当前 schema 直接解析并补齐 dedup metadata。
- future schema skip warning。

建议结构：

```rust
match schema_version {
    Some(SCHEMA_VERSION) => parse_current(value),
    Some(2) => migrate_v2(value, None),
    Some(1) => migrate_v1(value, None),
    None => migrate_v1(value, Some(...)),
    Some(version) => skip_future(version),
}
```

v2 迁移建议新增 `MemoryRecordV2`，字段应匹配当前 `MemoryRecord` v2。迁移到 v3 时：

- `schema_version = 3`。
- `source.session_id` 从 `source_session_id` 转换。
- `temporal.observed_at_millis` 从 `created_at_millis` 转换。
- `layer` 可从 `kind` 推导。
- `evidence` 默认为空，或从旧候选不可恢复时留空。
- `invalidation` 默认为空。
- 调用 `ensure_dedup_metadata()`。

### 4. 候选、抽取、写入链路

当前 `MemoryCandidate` 有 `evidence` 字符串。v1.8.8 应将候选期 evidence 映射进 `MemoryRecord.evidence`：

- rule extractor 产生的候选，source.extractor 可为 `rule`。
- provider extractor 产生的候选，source.extractor 可为 `provider`。
- 证据内容必须遵守 privacy/redaction 规则，不要把 secret-like 文本写入 durable memory。
- 现有 memory write policy 仍应在写入前决定 discard/pending/active，schema v3 不能绕过隐私策略。

### 5. Recall、Dedup、Merge 的影响控制

v1.8.8 不需要实现完整 graph memory，但字段变化会影响以下模块：

- `recall.rs`
  - 默认仍只 recall `active` memories。
  - 可利用 `expires_at_millis` 或 invalidation 状态过滤过期/失效记录。
  - 不要让 pending/rejected/archived 进入 Persona Context Blocks。
- `dedup.rs`
  - dedup key 仍应稳定。
  - 可暂时保持基于 scope/kind/content 的现有策略，不要因为实体字段导致历史记录全部重新分裂。
- `merge.rs`
  - merge 时要合并 revision、merged_count、source/evidence/invalidation，避免丢 provenance。
  - 冲突仍应进入 pending，而不是静默覆盖。
- `storage.rs`
  - append-only JSONL 不要 destructive rewrite。
  - latest records、collapse duplicate、status update 必须继续兼容 v3。

### 6. Persona Context Blocks 不退化

v1.8.8 扩展 memory schema 后，v1.8.7 的结构化 prompt 输出必须保持稳定：

- `memory_context role="context_not_instruction"` 不得丢失。
- 新字段可用于更好的 memory line 渲染，但不能让 memory 变成 instruction。
- block tags、转义、预算截断和安全声明要继续通过测试。

## 测试要求

必须新增或更新以下测试：

- `memory_v2_records_migrate_to_v3_without_data_loss`
- `legacy_v1_records_still_migrate_to_v3`
- `missing_schema_records_still_migrate_with_warning`
- `future_schema_records_are_skipped_with_warning`
- `new_memory_record_defaults_v3_metadata`
- `memory_candidate_evidence_is_preserved_in_v3_record`
- `merge_preserves_source_evidence_and_revision`
- `recall_filters_expired_or_invalidated_records`
- `persona_context_blocks_still_render_memory_as_context_not_instruction`
- `append_only_store_loads_mixed_v1_v2_v3_jsonl_records`

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

- 混合 v1/v2/v3 JSONL memory fixture 黑盒读取检查。
- memory CLI `list/show/search/pending/approve/reject/delete` 基本 smoke。
- Persona Context Blocks 不退化检查。
- owned-source secret scan。

如果执行安装 helper、PATH smoke、系统 PATH 修改或 `cargo clean`，必须先得到用户确认。`cargo clean` 会递归清理 `target`，也必须先确认。

## 文档与状态同步

验证通过后必须更新：

- `D:\YunXi Agent\docs\extraction-status.md`
- `D:\YunXi Agent\docs\persona-memory.md`
- `D:\YunXi Agent\docs\reports\2026-07-17-153715-yunxi-agent-v1-8-8-memory-schema-v3-development-report.md`
- 如有版本显示变更，还需同步 `README.md`、CLI/TUI 文案和测试期望。

写回内容必须包含：

- 实际修改文件。
- 实际执行命令。
- 每条验证结果。
- release binary version 输出。
- mixed schema migration 检查结果。
- Persona Context Blocks regression 结果。
- secret scan 结果。
- CodeGraph sync/status 结果。
- 构建产物清理结果。
- 提交、推送和 `v1.8.8` tag 状态。

## 推荐执行顺序

1. 读取本报告和 v1.8.7 审核报告，确认 v1.8.8 范围。
2. 运行只读状态检查：`git status --short --branch`、`git log -1 --oneline --decorate`。
3. 明确告知用户后，只读参考 `D:\源码\memU`、`D:\源码\nocturne_memory`、`D:\源码\yantrikdb` 中与 memory schema/lifecycle 相关的源码。
4. 在 `yunxi-agent-persona` 中设计 v3 字段和默认值。
5. 修改 `MemoryRecord`、`MemoryCandidate` 和相关 helper，保持 serde 兼容。
6. 修改 `migration.rs`，新增 v2 -> v3 迁移并保留 v1/missing schema 路径。
7. 调整 storage、recall、dedup、merge、extractor 的最小必要逻辑。
8. 新增 migration、storage、recall、persona block regression 测试。
9. 同步版本号、文档和 development report。
10. 完成统一验证。
11. 经用户确认后清理编译中间产物。
12. 收敛工作区，提交、推送并创建 annotated `v1.8.8` tag，不删除、不移动旧 tag。
13. 追加开发日志，日志结尾署名为开发者。
14. 再提交审核。

## 开发执行记录

### 已完成的构造批次

- `crates/yunxi-agent-persona/src/memory.rs`：`SCHEMA_VERSION` 提升为 3；新增
  layer、typed entities、temporal、evidence、source attribution lineage 和
  invalidation 结构；保留全部 v2 兼容字段。
- `crates/yunxi-agent-persona/src/migration.rs`：新增显式 v2 -> v3 迁移，保留
  v1、missing schema warning 和 future schema warning-and-skip 路径。
- `crates/yunxi-agent-persona/src/dedup.rs`、`extractor.rs`、
  `provider_extractor.rs`：候选在共享 dedup 入口映射 provenance；疑似 secret
  的原始 evidence 仅保留固定脱敏说明。
- `crates/yunxi-agent-persona/src/merge.rs`：合并双方的 evidence、entities、
  source attributions、temporal/invalidation、revision 和 merged_count；旧
  `source_session_id` 继续跟随内容策略。
- `crates/yunxi-agent-persona/src/recall.rs`、`compiler.rs` 和
  `crates/yunxi-agent-storage/src/lib.rs`：召回、active view 和 Persona Context
  Blocks 过滤过期、失效或被替代记录；存储仍为 append-only JSONL。
- 新增报告要求的十个具名回归测试，并额外覆盖 secret-like evidence
  持久化前脱敏。
- 版本、CLI/TUI/persona 文案、README、`docs/persona-memory.md` 和
  `docs/extraction-status.md` 同步到 v1.8.8 / Memory Schema v3。

### 外部参考使用记录

开发前已明确告知用户，并以只读方式查看报告指定的三个 `D:\源码` 路径。
实际只提取稳定内容去重、版本链、tombstone/修订审计语义；没有向外部源码
目录写入，没有复制数据库实现，没有引入 Python/JavaScript/数据库或外部
memory runtime 依赖。

### 当前状态

构造批次、统一验证、安装/PATH 和构建产物清理已经完成。发布前 GitHub
REST API 检查确认远端 `master` 与本地基线一致且 `v1.8.8` 不存在；提交、
推送和 tag 结果见下方发布记录，并在外部时间戳日志中记录精确对象 id。

### 统一验证结果

- `cargo fmt`：通过。
- `cargo fmt --check`：通过。
- `cargo test -p yunxi-agent-persona`：通过；38 个集成测试通过，其中
  Memory Schema v3 新增测试 10/10、Persona Context Blocks 4/4。
- `cargo test -p yunxi-agent-storage`：通过；5 个 unit、21 个 integration
  测试通过，混合 v1/v2/v3 JSONL 在读取后保持原文件字节不变。
- `cargo test -p yunxi-agent-runtime`：通过；40 个 integration 测试通过。
- `cargo test -p yunxi-agent-cli`：通过；两个 binary 各 11 个 unit、CLI
  integration 40 个、JSONL integration 10 个全部通过。
- `cargo test`：通过；全 workspace unit/integration/doc tests 零失败。
- `cargo check --workspace`：通过。
- `cargo build -p yunxi-agent-cli --release --bins`：通过。
- `target\release\yunxi.exe --version`：`yunxi 1.8.8`。
- `target\release\yunxi-agent-cli.exe --version`：`yunxi 1.8.8`。
- 隔离 `target\v1.8.8-smoke` memory CLI 黑盒：list/show/search/pending
  通过；approve -> active、reject -> rejected、delete -> archived；记录均为
  schema v3。
- Persona Context Blocks regression：通过；保留
  `memory_context role="context_not_instruction"` 和安全声明。
- owned-source secret scan：通过；0 个 live key-shaped match files。
- `git diff --check`：通过；仅有 Windows LF-to-CRLF 提示。
- `codegraph sync .`：通过；already up to date。
- `codegraph status .`：通过；1,165 files、45,052 nodes、146,173 edges。
- `scripts\install\install-yunxi.ps1 -AddToPath -SkipBuild`：经确认后通过；
  两个 binary 复制到
  `C:\Users\24763\AppData\Local\YunXi Agent\bin`，direct/PATH 版本均为
  `yunxi 1.8.8`；用户 PATH 中该目录恰好一条，未重复写入。
- `cargo clean`：经单独确认后通过；删除 19,120 个文件、约 3.2 GiB
  （清理前计量 3,451,338,167 bytes），`D:\YunXi Agent\target` 已不存在。

验证过程中发现并修复两类回归：旧 storage 测试仍期待 schema v2；候选期
dedup 错误消耗 durable revision。前者同步为 v3 期望，后者调整为只有真实
存储 merge 才递增 revision/merged_count；persona、storage、runtime 和
workspace 全部重跑通过。

### 提交与发布记录

- 发布提交说明：`Release YunXi Agent v1.8.8 Memory Schema v3`。
- 仓库管理方式：使用用户提供、仅保存在本机的 GitHub API key，通过
  GitHub Git Data REST API 创建 blobs/tree/commit；密钥值没有打印、写入
  项目、提交或日志。
- 分支更新：从已核验的
  `270e836b983141e2654568e4b07aed006a16bb27` 对 `master` 做 non-force 更新。
- 版本标签：创建新的 annotated `v1.8.8` 并指向同一 release commit。
- 回滚标签：发布前已有 25 个旧标签；本轮不删除、不移动、不重写任何旧
  tag，`v1.8.7` 继续指向原提交。
- release commit/tag 的精确对象 id 在发布成功后写入仓库外的
  `C:\Users\24763\Desktop\YunXi Agent开发日志.md`；Git commit 无法在自身
  内容中可靠嵌入其内容派生的最终 SHA。

记录者：开发者
