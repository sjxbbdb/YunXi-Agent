# YunXi Agent v1.8.9 L0-L3 Memory Pipeline 开发报告

撰写时间：2026-07-17 16:51:18 +08:00  
审核报告：`C:\Users\24763\Desktop\YunXi Agent审核报告\2026-07-17-163137-YunXi-Agent-v1.8.8-源码审核报告.md`  
开发目录：`D:\YunXi Agent`  
项目内报告位置：`D:\YunXi Agent\docs\reports\2026-07-17-165118-yunxi-agent-v1-8-9-l0-l3-memory-pipeline-development-report.md`  
桌面报告位置：`C:\Users\24763\Desktop\YunXi Agent开发报告\2026-07-17-165118-yunxi-agent-v1-8-9-l0-l3-memory-pipeline-development-report.md`

本报告面向后续开发者，用于把 v1.8.8 审核通过后的下一阶段要求转化为可执行的 v1.8.9 开发任务。v1.8.9 的目标是 L0-L3 Memory Pipeline，即从“记几条事实”升级为“形成长期关系记忆”的分层抽取与写入策略。本报告不是完成证明；验证通过、文档写回、工作区收敛、提交/tag 状态明确之前，不得宣称 v1.8.9 完成。

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

审核报告结论为：YunXi Agent v1.8.8 审核通过，可以进入下一版本开发。

v1.8.8 已闭环的能力包括：

- `MemoryRecord` 已升级到 schema v3。
- 记忆字段已覆盖 layer、entities、temporal、evidence、source、confidence、invalidation。
- v1/v2 旧 JSONL 记忆可迁移到 v3。
- append-only JSONL 存储不因加载迁移而被破坏性重写。
- Persona Context Blocks 在 v1.8.8 中保持不退化。
- v1.8.8 已有统一验证、文档记录、干净工作区、annotated `v1.8.8` tag 和构建产物清理记录。

因此 v1.8.9 的开发目标进入总纲图下一节点：

```text
v1.8.9 L0-L3 Memory Pipeline
分层抽取与写入策略
```

本轮核心目标是把现有 rule/provider memory extraction 从“候选列表”升级为分层 pipeline：原始回合、结构事实、关系事件、画像摘要逐层产生、筛选、去重、写入和审查。每轮应能产生候选记忆，Pending/Active/Rejected 流程必须可测，误写敏感记忆必须降级或待确认。

## 范围与非目标

### 必须完成

- 建立 L0-L3 记忆分层概念：
  - L0 raw turn：原始回合摘要或安全压缩后的观察输入。
  - L1 structured facts：偏好、事实、目标、项目上下文、纠正等结构事实候选。
  - L2 relationship events：关系、情绪、长期互动事件和信任语义候选。
  - L3 profile summaries：可长期稳定注入 persona context 的画像摘要候选。
- 每轮运行后应能形成可审查的候选记忆集合，即使最终策略决定 pending、rejected 或 discard。
- Pending/Active/Rejected/Archived 流程必须保持可测试、可解释。
- 敏感、模糊、高风险或可能误写的记忆必须降级为 pending 或 rejected，不得静默 active。
- secret-like 内容必须 discard 或 redacted，不能落入 durable memory。
- provider extractor、rule extractor、policy、dedup、merge、storage 要通过统一 pipeline 串起来。
- 保持 v1.8.7 Persona Context Blocks 和 v1.8.8 Memory Schema v3 不退化。
- 更新 `docs/persona-memory.md`、`docs/extraction-status.md` 和本版本 development report。
- 阶段结束后统一验证、清理构建产物、提交、推送并创建新的 `v1.8.9` Git tag。

### 不应提前实现

- Boot Context & Recall Router。
- Relationship Graph Lite。
- Proactive Companion Loop。
- TUI memory inspector。
- Companion Evaluation Harness。
- SQLite、vector search、graph memory 或外部 memory runtime。
- 云任务、marketplace、SDK packaging 或非当前阶段产品面。

## 当前源码接入点

v1.8.9 的主要接入点是抽取、策略、去重和 runtime 写入链路：

- `D:\YunXi Agent\crates\yunxi-agent-persona\src\provider_extractor.rs`
  - 当前 `ProviderMemoryExtractor::extract_from_response_checked` 从 provider JSON candidates 生成 `MemoryCandidate`。
  - 当前 provider candidate 支持 kind、content、scope_hint、sensitivity_hint、confidence、importance、reason。
  - 当前写入状态由 `MemoryWritePolicyEngine::policy_for` 决定，再映射为 Active/Pending/Rejected。
- `D:\YunXi Agent\crates\yunxi-agent-persona\src\extractor.rs`
  - 当前 `MemoryRuleExtractor::extract` 基于 prompt/assistant_response 规则生成候选。
  - 当前 rule extractor 能识别语言偏好、correction、项目硬性约束、自述事实等。
  - 当前规则候选也走 policy、scope router 和 dedup。
- `D:\YunXi Agent\crates\yunxi-agent-persona\src\policy.rs`
  - 当前 `MemoryWritePolicyEngine` 支持 Auto、RequireConfirmation、Discard、Disabled。
  - 当前 privacy classifier 对 secret marker 返回 High/Discard，对 health/finance/emotion/relationship 等敏感画像返回 Medium/RequireConfirmation。
- `D:\YunXi Agent\crates\yunxi-agent-persona\src\dedup.rs`
  - 当前 `deduplicate_candidates` 会先 `ensure_v3_provenance`，再按 dedup_key merge。
  - 当前 `status_for_policy` 将 Auto 映射 Active，RequireConfirmation 映射 Pending，Discard/Disabled 映射 Rejected。
- 还需核对：
  - `crates\yunxi-agent-persona\src\merge.rs`
  - `crates\yunxi-agent-persona\src\recall.rs`
  - `crates\yunxi-agent-persona\src\memory.rs`
  - `crates\yunxi-agent-storage\src\lib.rs`
  - `crates\yunxi-agent-runtime\src\lib.rs`
  - `crates\yunxi-agent-cli\src\main.rs`

## 参考源码建议

审核报告为 v1.8.9 指定的参考源码：

- `D:\源码\TencentDB-Agent-Memory\src`
- `D:\源码\TencentDB-Agent-Memory\hermes-plugin\memory\memory_tencentdb\client.py`
- `D:\源码\mem0\mem0\memory\main.py`

参考方式必须遵守以下原则：

- 这些路径在 `D:\YunXi Agent` 项目外。读取前要明确告知用户；不得向这些目录写入内容。
- TencentDB-Agent-Memory 和 mem0 如果是 Python 实现，只抽取 memory pipeline、extract/update/delete、conflict resolution、history summarization、candidate decision 等逻辑，不迁入 Python runtime。
- 参考项目里的数据库、云服务、向量检索或外部服务不可成为 YunXi 默认运行路径依赖。
- 只在 YunXi Rust workspace 内复刻必要逻辑，优先落在 `yunxi-agent-persona`、`yunxi-agent-storage`、`yunxi-agent-runtime` 边界。
- 不要引入 `vendor/codex-rs`、`codex-*` crate 或 `yunxi-agent-codex` 到默认 YunXi 运行路径。

## 推荐技术设计

### 1. 引入 Pipeline 类型

建议在 `yunxi-agent-persona` 中新增轻量 pipeline 模块，例如：

```rust
pub struct MemoryPipeline {
    rule_extractor: MemoryRuleExtractor,
    provider_extractor: ProviderMemoryExtractor,
    policy: MemoryWritePolicyEngine,
}

pub struct MemoryPipelineInput {
    pub prompt: String,
    pub assistant_response: Option<String>,
    pub provider_response: Option<String>,
    pub source_session_id: Option<String>,
    pub workspace_fingerprint: Option<String>,
    pub memory_enabled: bool,
}

pub struct MemoryPipelineOutput {
    pub stages: Vec<MemoryPipelineStageResult>,
    pub candidates: Vec<MemoryCandidate>,
}
```

如果不想新增大模块，也可以先在现有 extractor 上增加内部 stage builder。但建议显式命名 pipeline，因为 v1.8.9 的验收目标是分层抽取与写入策略，隐藏在散落函数里会难以测试和审计。

### 2. 定义 L0-L3 层级

建议新增：

```rust
pub enum MemoryPipelineLayer {
    L0RawTurn,
    L1StructuredFact,
    L2RelationshipEvent,
    L3ProfileSummary,
}
```

映射建议：

- L0 raw turn：不直接长期注入，作为证据或摘要来源；如保存，必须压缩、脱敏、低重要度、默认 pending 或 rejected。
- L1 structured facts：Preference、PersonalFact、Goal、ProjectContext、Correction。
- L2 relationship events：RelationshipNote、EmotionalState、Event。
- L3 profile summaries：长期稳定偏好、边界、互动风格、关系摘要；必须高置信、低风险、可解释。

可以复用 v1.8.8 的 `MemoryLayer` 字段，把 pipeline 层级落到 durable record。不要在 v1.8.9 做完整 Recall Router，但要保证字段足够支撑后续版本。

### 3. 每轮候选生成

runtime 中每轮 memory extraction 应统一调用 pipeline，而不是 rule/provider 两条路各自散落：

- 收集 prompt、assistant final response、provider extraction payload、workspace fingerprint、session id。
- L0 生成本轮安全摘要或 evidence reference。
- L1 从 rule/provider 生成结构事实候选。
- L2 从敏感关系/情绪/互动内容生成关系事件候选，默认 pending。
- L3 只对稳定、重复、低风险信息生成 profile summary 候选。
- 所有候选统一进入 dedup、merge、policy、status routing。

如果 provider 不可用或离线，pipeline 仍应能通过 rule extractor 产生候选；不能让 provider failure 阻断整轮 agent。

### 4. 写入策略与敏感降级

`MemoryWritePolicyEngine` 应扩展为同时考虑：

- kind。
- sensitivity。
- pipeline layer。
- confidence。
- importance。
- source extractor。
- content 风险。
- 用户是否显式要求记住。
- memory_enabled 状态。

建议策略：

- Secret-like：Discard/Rejected，证据 redacted，不持久化原文。
- 高敏或关系/情绪画像：RequireConfirmation/Pending。
- 模糊自述事实：Pending。
- 明确稳定偏好：可 Auto/Active，但仍要可审计。
- L0 raw turn：默认不 active，最多作为 redacted evidence 或 pending summary。
- L3 profile summary：必须高置信、低敏、来源清楚，否则 pending。

### 5. Dedup 与 Merge

v1.8.9 应保持现有 dedup 稳定性，同时纳入 pipeline 层级：

- 不要让同一事实在 L1/L3 两层无限重复。
- L3 summary 可以引用或 merge 多条 L1/L2 来源，但要保留 source attribution。
- 冲突记忆应 pending，不应静默覆盖 active。
- `merged_count`、`revision`、evidence、source lineage 必须继续保存。

### 6. Storage 与事件

存储仍保持 append-only JSONL：

- pipeline output 写入前应能产生 diagnostics。
- 写入事件应能说明 action：auto_saved、pending、discard、disabled、merged、rejected。
- 不要打印完整私密 memory content 到普通事件流；继续遵守 v1.8.6 输出脱敏边界。
- CLI memory list/show/search/pending/approve/reject/delete 仍应可用。

### 7. 不退化约束

开发时必须证明：

- v1.8.7 Persona Context Blocks 仍然结构化。
- v1.8.8 schema v3 字段仍存在，v1/v2 迁移仍通过。
- Pending/Active/Rejected/Archived 语义不变。
- memory 是 context，不是 instruction。
- secret-like 内容不会写入 durable memory，也不会从 JSON/JSONL 输出泄漏。

## 测试要求

必须新增或更新以下测试：

- `pipeline_generates_l1_candidate_from_language_preference`
- `pipeline_generates_l2_relationship_candidate_as_pending`
- `pipeline_generates_l3_profile_summary_only_for_stable_low_risk_memory`
- `pipeline_records_l0_summary_as_evidence_not_instruction`
- `pipeline_discards_secret_like_memory`
- `pipeline_downgrades_sensitive_personal_memory_to_pending`
- `pending_active_rejected_flow_is_testable`
- `dedup_merges_rule_and_provider_candidates_across_layers`
- `merge_preserves_source_lineage_across_l1_l3`
- `provider_failure_does_not_block_rule_candidates`
- `persona_context_blocks_do_not_regress_after_pipeline_write`
- `schema_v3_migration_tests_still_pass`

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

- Memory pipeline 每轮候选黑盒检查。
- Pending/Active/Rejected CLI smoke。
- secret-like memory discard 黑盒检查。
- Persona Context Blocks regression。
- Memory Schema v3 migration regression。
- owned-source secret scan。

如果执行安装 helper、PATH smoke、系统 PATH 修改或 `cargo clean`，必须先得到用户确认。`cargo clean` 会递归清理 `target`，也必须先确认。

## 文档与状态同步

验证通过后必须更新：

- `D:\YunXi Agent\docs\extraction-status.md`
- `D:\YunXi Agent\docs\persona-memory.md`
- `D:\YunXi Agent\docs\reports\2026-07-17-165118-yunxi-agent-v1-8-9-l0-l3-memory-pipeline-development-report.md`
- 如有版本显示变更，还需同步 `README.md`、CLI/TUI 文案和测试期望。

写回内容必须包含：

- 实际修改文件。
- 实际执行命令。
- 每条验证结果。
- release binary version 输出。
- L0-L3 pipeline 测试结果。
- Pending/Active/Rejected flow 检查结果。
- sensitive/secret-like memory 降级或丢弃检查结果。
- Persona Context Blocks 与 schema v3 regression 结果。
- secret scan 结果。
- CodeGraph sync/status 结果。
- 构建产物清理结果。
- 提交、推送和 `v1.8.9` tag 状态。

## 推荐执行顺序

1. 读取本报告和 v1.8.8 审核报告，确认 v1.8.9 范围。
2. 运行只读状态检查：`git status --short --branch`、`git log -1 --oneline --decorate`。
3. 明确告知用户后，只读参考 `D:\源码\TencentDB-Agent-Memory` 和 `D:\源码\mem0` 中与 memory pipeline 相关的源码。
4. 设计 `MemoryPipelineLayer`、`MemoryPipelineInput`、`MemoryPipelineOutput` 和 stage result。
5. 将 rule/provider extraction 接入统一 pipeline。
6. 扩展 policy，让 layer、confidence、sensitivity、source 共同决定 write policy。
7. 调整 dedup/merge，让跨层候选可合并并保留 source lineage。
8. 保持 storage append-only JSONL 和 CLI memory 管理入口稳定。
9. 新增 pipeline、policy、dedup、runtime、CLI smoke 测试。
10. 同步版本号、文档和 development report。
11. 完成统一验证。
12. 经用户确认后清理编译中间产物。
13. 收敛工作区，提交、推送并创建 annotated `v1.8.9` tag，不删除、不移动旧 tag。
14. 追加开发日志，日志结尾署名为开发者。
15. 再提交审核。

## 本报告生成状态

本次仅完成 v1.8.9 开发报告撰写和落盘，没有运行构建、测试、安装、清理、提交、推送或 tag 操作。因此本报告不能作为 v1.8.9 完成证明。

报告撰写者：开发报告撰写者
