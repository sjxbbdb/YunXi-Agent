# YunXi Agent v1.8.2 Memory Deduplication, Migration, And Diagnostics Development Report

生成时间：2026-07-14 07:02:21 +08:00

> **For agentic workers:** REQUIRED SUB-SKILL: Use `superpowers:executing-plans` to implement this report task-by-task. The user hard constraint overrides the default test cadence: write and modify tests during construction, but do not execute verification until all construction tasks are complete.

## 背景

YunXi Agent 当前正式基线为 `v1.8.1`。该版本已经完成 persona / transparent memory 的第一轮正确性与透明性加固：scope 语义路由、相关性 recall、pending 状态一致性、workspace clear 统计、首次开启 memory disclosure、provider-backed extraction 初步接入、TUI memory write notice、CJK composer 宽度修正和 `v1.8.1` 不可变 tag 发布。

用户提供了新的源码审核报告：

- `C:\Users\admin\Desktop\YunXi Agent调研项目\2026-07-13-YunXi-Agent-1.8.1-源码审核报告.md`

本报告按外部审核意见接收流程处理：先完整读取审核报告，再使用 CodeGraph 对 persona / storage / runtime / protocol / TUI 相关路径进行核对。审核报告指出的 P1 重复记忆问题成立，P2 schema migration、recall 诊断暴露、provider extraction 端到端保护和 TUI 记忆提示可读性问题也成立。v1.8.2 的目标是把 v1.8.1 的长期记忆从“可用且透明”推进到“可去重、可升级、可诊断、可解释”。

## 已复核的关键问题

本轮使用 CodeGraph 与源码定位确认以下问题成立：

- `D:\YunXi Agent\crates\yunxi-agent-persona\src\extractor.rs` 的语言偏好规则会让同一句“以后请用中文回答”产生两条等价候选。
- `D:\YunXi Agent\crates\yunxi-agent-storage\src\lib.rs:757` 的 `FilePersonaMemoryStore::append()` 仍然只追加记录，不按 `scope + kind + normalized_content` 查重或合并。
- `D:\YunXi Agent\crates\yunxi-agent-storage\src\lib.rs:934` 的 `latest_records()` 只按 `id` 收敛；重复内容每次生成新 id，无法被 latest view 合并。
- `D:\YunXi Agent\crates\yunxi-agent-persona\src\memory.rs:154` 已有 `always_on_count`、`dropped_unrelated`、`dropped_by_budget`，但 runtime/core/protocol 的 `memory_recall` 事件仍只暴露 `count/budget/truncated`。
- `D:\YunXi Agent\crates\yunxi-agent-storage\src\lib.rs` 的 JSONL 读取能跳过损坏行，但没有旧 schema envelope、fixture migration 或字段补全路径。
- `D:\YunXi Agent\crates\yunxi-agent-runtime\src\lib.rs` 已接入 live provider structured extraction，但缺少 runtime fixture 级合法 JSON、非法 JSON、空 candidates、超时、fallback 和 warning 保护测试。
- `D:\YunXi Agent\crates\yunxi-agent-tui\src\event_filter.rs` 已让部分 `MemoryWrite` 可见，但普通 TUI notice 仍偏工程化，`MemoryRecall` / discard / disabled 等透明反馈还不够清晰。

## v1.8.2 总目标

v1.8.2 要完成长期记忆稳定性第二轮加固：

- 同一轮 memory candidates 先做 batch 内语义去重。
- 落盘前与已有 active/pending 记录做等价合并，不再为等价语言偏好生成新 id。
- 旧脏数据在 recall 前被防御性合并，不能继续挤占 prompt 预算。
- JSONL memory schema 具备 v1 到 v2 的显式 migration，避免后续升级静默丢记忆。
- JSONL / TUI 暴露 recall 诊断计数，不泄露记忆正文。
- provider-backed extraction 具备 runtime 级端到端保护和用户可解释配置/状态。
- TUI memory notice 面向普通用户短句展示，details/debug 保留工程细节。

## 用户硬性约束

执行 v1.8.2 开发时继续遵守以下约束：

- 按本报告先整体构建源码，中间不做反复单点测试，不在某个点卡过多时间。
- 构建过程中可以新增/修改测试源码，但不执行中途验证；全部构建完成后统一执行验证。
- 每个正式版本必须创建新的不可变 annotated tag；v1.8.2 发布时创建 `v1.8.2`，旧 tag 不删除、不移动。
- GitHub 读写和推送只走 REST API，不使用 `git push`、`git fetch`、`git ls-remote`。
- API key 只从用户指定的本地 txt 文件读取，不打印、不写日志、不提交。
- 每次任务结束必须追加 `C:\Users\admin\Desktop\YunXi Agent开发日志.md`。
- 发布收尾前执行 `cargo clean` 清理构建中间产物。
- 默认运行路径继续保持 YunXi 自主化，不恢复对上游 Codex CLI runtime、`vendor/codex-rs`、`codex-*` 或 `yunxi-agent-codex` 的运行依赖。
- 人格与记忆不能绕过既有 sandbox、approval、execution policy、provider honesty 和 secret redaction 边界。

## 非目标

v1.8.2 不做以下内容：

- 不引入 SQLite、向量数据库、embedding store、graph database 或外部 memory service。
- 不实现完整关系状态机、主动关心 trigger、复杂矛盾仲裁或长期记忆衰减模型。
- 不新增大型 TUI memory inspector 页面。
- 不改变 DeepSeek / OpenAI-compatible provider 的凭据读取方式。
- 不把 provider-backed extraction 变成主对话强阻塞路径。
- 不删除或重写用户已有 memory JSONL；本版继续使用 append-only ledger，通过 migrated latest view 与追加 revision 实现合并。

## 开发主线一：记忆规范化与 batch 内去重

目标：同一 prompt 内不再产生两条等价语言偏好候选。

修改文件：

- `D:\YunXi Agent\crates\yunxi-agent-persona\src\dedup.rs`
- `D:\YunXi Agent\crates\yunxi-agent-persona\src\extractor.rs`
- `D:\YunXi Agent\crates\yunxi-agent-persona\src\provider_extractor.rs`
- `D:\YunXi Agent\crates\yunxi-agent-persona\src\lib.rs`
- `D:\YunXi Agent\crates\yunxi-agent-persona\tests\extractor_tests.rs`

设计要求：

- 新增 `dedup.rs`，提供 `MemoryDedupKey`、`normalized_memory_content()`、`dedup_key_for_record()`、`deduplicate_candidates()`。
- `MemoryDedupKey` 至少包含 `scope_label`、`kind`、`normalized_content`。
- 语言偏好需要语义归一化：`以后请用中文回答`、`默认用中文交流`、`用中文回复我` 统一归为 `language:zh`。
- 英文偏好归一化为 `language:en`；不明确语言的偏好使用去标点、压缩空白、统一大小写后的内容。
- batch 内遇到等价候选时保留更高 `confidence`、更高 `importance`、更严格 `sensitivity` 和更严格 `write_policy`；证据与 reason 合并为短字符串，不能泄露 secret。
- `MemoryRuleExtractor` 和 `ProviderMemoryExtractor` 都必须在返回前调用同一个 batch 去重入口。

必须新增回归覆盖：

- `以后请用中文回答` 只产生 1 个等价语言偏好候选。
- provider 返回 `默认中文交流` 与规则返回 `用中文回答` 时，经统一入口后只保留 1 个等价候选。
- 同一 prompt 中项目硬约束和语言偏好不能互相合并，因为 scope/kind 不同。
- secret-like candidate 仍被 policy 标为 discard，不能因合并被降级。

## 开发主线二：落盘前合并与 append-only revision

目标：重复运行同一句偏好后，global active 等价偏好仍只有 1 条。

修改文件：

- `D:\YunXi Agent\crates\yunxi-agent-persona\src\memory.rs`
- `D:\YunXi Agent\crates\yunxi-agent-persona\src\dedup.rs`
- `D:\YunXi Agent\crates\yunxi-agent-storage\src\lib.rs`
- `D:\YunXi Agent\crates\yunxi-agent-storage\tests\storage_tests.rs`
- `D:\YunXi Agent\crates\yunxi-agent-runtime\src\lib.rs`
- `D:\YunXi Agent\crates\yunxi-agent-runtime\tests\runtime_tests.rs`
- `D:\YunXi Agent\crates\yunxi-agent-cli\tests\cli_tests.rs`

设计要求：

- 将 memory schema 提升到 `SCHEMA_VERSION = 2`。
- `MemoryRecord` 新增 `dedup_key: String`、`revision: u32`、`merged_count: u32`。
- 新增 `MemoryRecord::with_dedup_metadata()` 或等价构造路径，所有新记录写入前必须具备 dedup key。
- `FilePersonaMemoryStore` 新增 `append_or_merge(record: &MemoryRecord) -> AgentResult<MemoryPersistOutcome>`。
- `MemoryPersistOutcome` 包含 `Inserted { id }`、`Merged { id, revision, merged_count }`、`Skipped { id, reason }`。
- 合并时复用已有等价 active/pending 记录的 `id` 与 `created_at_millis`，追加一条同 id 的新 revision；不破坏 append-only JSONL。
- 合并时 `updated_at_millis` 使用当前时间，`confidence` / `importance` 取较高值，`sensitivity` 取更严格值。
- status 合并规则必须保守：
  - existing `Pending` 与 incoming `Active` 合并后仍保持 `Pending`。
  - existing `Active` 与 incoming `Active` 合并后保持 `Active`。
  - incoming `RequireConfirmation` 不能把 existing `Active` 静默降级为更多可见正文；若内容等价但敏感性更高，合并后进入 `Pending`。
  - existing `Archived` / `Rejected` 不作为自动复活目标；新候选可生成新记录。
- runtime 写入路径从 `append()` 改为 `append_or_merge()`，并在 `MemoryWrite` 事件中用 action 表示 `auto_saved`、`merged`、`pending_confirmation`、`discarded`、`disabled`。

必须新增回归覆盖：

- 连续两次写入 `以后请用中文回答` 后 global active 等价偏好为 1 条，revision 大于 1。
- 写入 `默认用中文交流` 后再写入 `用中文回答`，仍合并到同一 id。
- pending 个人事实重复出现时仍只有 1 条 pending 等价记录。
- archived/rejected 的旧等价记录不会被自动复活。
- CLI 黑盒使用临时 `YUNXI_HOME` 连续运行两次同句语言偏好后，`memory list --global --json` 只返回 1 条等价 active preference。

## 开发主线三：旧 schema migration 与 JSONL 读取解释

目标：让 v1 JSONL 长期记忆可升级到 v2，不因新增字段静默丢失。

修改文件：

- `D:\YunXi Agent\crates\yunxi-agent-persona\src\migration.rs`
- `D:\YunXi Agent\crates\yunxi-agent-persona\src\memory.rs`
- `D:\YunXi Agent\crates\yunxi-agent-persona\src\lib.rs`
- `D:\YunXi Agent\crates\yunxi-agent-storage\src\lib.rs`
- `D:\YunXi Agent\crates\yunxi-agent-storage\tests\fixtures\memory-v1-global-preference.jsonl`
- `D:\YunXi Agent\crates\yunxi-agent-storage\tests\fixtures\memory-v1-workspace-project-context.jsonl`
- `D:\YunXi Agent\crates\yunxi-agent-storage\tests\storage_tests.rs`
- `D:\YunXi Agent\docs\persona-memory.md`

设计要求：

- 新增 `MemoryRecordV1` 和 `migrate_memory_record_value(value: serde_json::Value) -> Result<MemoryRecord, MemoryMigrationWarning>`。
- `read_memory_jsonl()` 先按 `serde_json::Value` 读取每行，再根据 `schema_version` 分流。
- v1 记录迁移到 v2 时补齐：
  - `dedup_key = dedup_key_for_record(v1_record)`
  - `revision = 1`
  - `merged_count = 1`
  - 其余字段保持原值
- 缺少 `schema_version` 但字段形状等同 v1 时按 legacy v1 迁移，并输出 warning。
- 不支持的未来 schema 不能 panic；该行跳过并输出包含行号和 schema_version 的 warning。
- 损坏 JSON 行继续保持现有 warning 行为，不影响其他记录。

必须新增回归覆盖：

- v1 fixture 能被读取为 v2 `MemoryRecord`，且 dedup metadata 完整。
- legacy no-version fixture 能迁移，并产生可解释 warning。
- future schema fixture 被跳过，并产生可解释 warning。
- corrupt line + valid v1 line 的混合文件能保留 valid 记录。

## 开发主线四：latest view 与 recall 防御性去重

目标：即使历史 JSONL 已经存在重复脏数据，recall 也不能让重复偏好挤占预算。

修改文件：

- `D:\YunXi Agent\crates\yunxi-agent-storage\src\lib.rs`
- `D:\YunXi Agent\crates\yunxi-agent-persona\src\recall.rs`
- `D:\YunXi Agent\crates\yunxi-agent-persona\tests\memory_policy_tests.rs`
- `D:\YunXi Agent\crates\yunxi-agent-storage\tests\storage_tests.rs`

设计要求：

- `latest_records()` 先按 `id` 取最新 revision，再按 `dedup_key + live status group` 合并等价记录。
- active/pending 的合并不能把 pending 隐藏成 active；pending 仍需用户确认。
- recall 在 scoring 前再次按 dedup key collapse，作为防御层。
- always-on profile 预算先 dedup，再取前 3 条。
- `MemoryRecallResult` 新增或复用 `dropped_duplicates`，记录 recall 层丢弃的重复数量。

必须新增回归覆盖：

- 手工构造 4 条旧重复语言偏好 active 记录，recall 后只注入 1 条 always-on preference。
- 无关 query 的 `dropped_unrelated` 与 `dropped_duplicates` 都能正确计数。
- `budget_chars` 小于全部候选时，dedup 先发生，再计算 `dropped_by_budget`。

## 开发主线五：recall 诊断事件扩展

目标：JSONL / TUI 能解释本轮 memory recall 做了什么，但不泄露记忆正文。

修改文件：

- `D:\YunXi Agent\crates\yunxi-agent-core\src\event.rs`
- `D:\YunXi Agent\crates\yunxi-agent-protocol\src\lib.rs`
- `D:\YunXi Agent\crates\yunxi-agent-runtime\src\lib.rs`
- `D:\YunXi Agent\crates\yunxi-agent-cli\src\render.rs`
- `D:\YunXi Agent\crates\yunxi-agent-tui\src\event_filter.rs`
- `D:\YunXi Agent\crates\yunxi-agent-tui\src\debug.rs`
- `D:\YunXi Agent\crates\yunxi-agent-protocol\tests\protocol_tests.rs`
- `D:\YunXi Agent\crates\yunxi-agent-cli\tests\jsonl_tests.rs`

设计要求：

- `AgentEvent::MemoryRecall` 增加 `always_on_count`、`dropped_unrelated`、`dropped_by_budget`、`dropped_duplicates`。
- protocol JSONL `memory_recall` 输出同名字段。
- plain render 使用短摘要：`[memory] recalled=1 always_on=1 dropped=...`。
- TUI 普通视图默认不显示 recall 内容；debug/details 可见计数。
- query 包含 secret-like 内容时，继续输出 `[redacted-sensitive-query]`，不能为了诊断泄露原 query。

必须新增回归覆盖：

- `memory_recall` JSONL 包含新增字段。
- 无关 workspace 记忆被 drop、global 中文偏好被 always-on 保留时，JSONL 计数可解释。
- secret query redaction 与新增诊断字段同时存在。

## 开发主线六：provider-backed extraction 端到端保护与开关文案

目标：live provider memory extraction 不影响主对话，并且用户知道开启 memory 后可能多一次结构化提炼调用。

修改文件：

- `D:\YunXi Agent\crates\yunxi-agent-core\src\config.rs`
- `D:\YunXi Agent\crates\yunxi-agent-provider\src\lib.rs`
- `D:\YunXi Agent\crates\yunxi-agent-runtime\src\lib.rs`
- `D:\YunXi Agent\crates\yunxi-agent-runtime\tests\runtime_tests.rs`
- `D:\YunXi Agent\crates\yunxi-agent-cli\src\main.rs`
- `D:\YunXi Agent\crates\yunxi-agent-cli\tests\cli_tests.rs`
- `D:\YunXi Agent\docs\persona-memory.md`

设计要求：

- 新增 `MemoryExtractionMode`，取值为 `auto`、`rule_only`、`provider`。
- 默认 `auto`：live provider 可用时尝试 provider extraction，失败降级规则；offline 或无 provider 时规则提炼。
- CLI 增加 `--memory-extraction <auto|rule-only|provider>` 或配置等价入口；`memory on` disclosure 必须说明 auto 模式下 live provider 可能执行一次结构化提炼调用。
- runtime fixture provider 增加四类 fixture：
  - 合法 JSON candidates：应写入或 pending。
  - 非法 JSON：应 warning 并 fallback rule extraction。
  - 空 candidates：不写入，不报错。
  - 慢响应/超时：应 warning 并不影响 final response。
- provider 返回 secret-like candidate 时仍必须 discard。
- provider scope hint 不能越过 `MemoryScopeRouter`。

必须新增回归覆盖：

- live fixture 合法 JSON 端到端写入。
- invalid JSON 产生 warning，主对话 completed。
- timeout 产生 warning，主对话 completed。
- `rule_only` 模式不触发 provider extraction。
- `provider` 模式无可用 provider 时产生可解释 warning 并不崩溃。

## 开发主线七：TUI memory notice 文案与 details 分层

目标：普通 TUI 只显示用户能理解的短提示，细节进入 details/debug。

修改文件：

- `D:\YunXi Agent\crates\yunxi-agent-tui\src\event_filter.rs`
- `D:\YunXi Agent\crates\yunxi-agent-tui\src\chat.rs`
- `D:\YunXi Agent\crates\yunxi-agent-tui\src\debug.rs`
- `D:\YunXi Agent\crates\yunxi-agent-tui\tests\tui_event_filter_tests.rs`
- `D:\YunXi Agent\crates\yunxi-agent-tui\src\app.rs`

设计要求：

- 普通视图短文案：
  - `memory saved: preference`
  - `memory updated: preference`
  - `memory pending review`
  - `memory discarded by privacy policy`
  - `memory disabled`
- pending notice 必须继续提示 `yunxi memory pending`。
- 普通视图不显示 id、scope、dedup_key、content、evidence。
- details/debug 显示 id、scope、kind、status、policy、reason、revision、merged_count、recall 诊断计数。
- 窄终端下 memory notice 必须可换行，不挤占 composer。

必须新增回归覆盖：

- `auto_saved` 显示短 notice，不包含 content。
- `merged` 显示 `memory updated: preference`。
- `discarded` 显示隐私策略丢弃，不包含 secret。
- pending notice 包含 `yunxi memory pending`。
- debug/details 包含工程字段。

## 开发主线八：版本、文档、状态与发布

目标：把 v1.8.2 的行为边界写入项目文档并完成正式版本发布准备。

修改文件：

- `D:\YunXi Agent\Cargo.toml`
- `D:\YunXi Agent\Cargo.lock`
- `D:\YunXi Agent\crates\yunxi-agent-cli\src\main.rs`
- `D:\YunXi Agent\crates\yunxi-agent-cli\src\render.rs`
- `D:\YunXi Agent\crates\yunxi-agent-tui\src\app.rs`
- `D:\YunXi Agent\crates\yunxi-agent-persona\src\profile.rs`
- `D:\YunXi Agent\docs\persona-memory.md`
- `D:\YunXi Agent\docs\extraction-status.md`
- `D:\YunXi Agent\docs\reports\2026-07-14-yunxi-agent-v1-8-2-memory-dedup-migration-diagnostics-development-report.md`
- `C:\Users\admin\Desktop\YunXi Agent开发日志.md`

设计要求：

- workspace/package version 更新到 `1.8.2`。
- CLI/TUI banner/version 输出更新到 `1.8.2`。
- `docs/persona-memory.md` 记录 v2 schema、dedup key、revision、migration、provider extraction mode、recall diagnostics、TUI notice 文案。
- `docs/extraction-status.md` 增加 v1.8.2 construction 与最终统一验证记录。
- 发布时通过 GitHub REST API 更新远端 `master` 并创建 annotated tag `v1.8.2`。
- 如果 GitHub API 创建的 commit SHA 与本地 commit SHA 因时间戳/消息尾换行不同而不同，必须像 v1.8.1 一样把本地 `master` 和本地 tag ref 对齐到远端 API 发布对象，避免后续从同内容不同 SHA 分叉。

## 统一验证计划

开发过程中不执行中途验证；所有源码构建完成后一次性执行以下验证：

```powershell
cargo fmt --check
cargo test -p yunxi-agent-persona -p yunxi-agent-storage -p yunxi-agent-runtime -p yunxi-agent-cli -p yunxi-agent-tui
cargo test
cargo check --workspace
cargo build -p yunxi-agent-cli --release --bins
target\release\yunxi.exe --version
target\release\yunxi-agent-cli.exe --version
git diff --check
codegraph sync .
codegraph status .
```

黑盒验证使用临时 `YUNXI_HOME` 和临时 workspace：

- `yunxi memory on --json` 输出 provider extraction disclosure 与 storage roots。
- 连续两次运行 `yunxi --offline --jsonl "以后请用中文回答"` 后，global active 等价语言偏好仍为 1 条。
- `yunxi --offline --jsonl "默认用中文交流"` 与 `yunxi --offline --jsonl "用中文回答"` 合并到同一 id。
- `yunxi --offline --jsonl "请计算 2+2"` 的 `memory_recall` JSONL 包含 `always_on_count`、`dropped_unrelated`、`dropped_by_budget`、`dropped_duplicates`。
- 手工放入 v1 memory fixture 后，`memory list --global --json` 能读出迁移后的 v2 记录。
- provider extraction fixture 的合法 JSON、非法 JSON、空 candidates、超时都不会影响主对话完成。
- TUI event filter 测试确认普通 notice 不泄露 content/secret，details/debug 保留诊断字段。
- owned-source secret scan 排除 `vendor`、`extracted`、`target`、`.git`、`.codegraph` 后无真实 API key、bearer token 或 authorization bearer header。

## 发布计划

v1.8.2 完成并验证通过后：

- 追加 `C:\Users\admin\Desktop\YunXi Agent开发日志.md`，记录改动文件、验证结果、清理结果、API 发布结果。
- 执行 `scripts\install\install-yunxi.ps1 -AddToPath -SkipBuild` 刷新本机 PATH 安装位。
- 执行 `cargo clean` 清理中间产物。
- 本地创建 release commit。
- 通过 GitHub REST API 创建远端 blob/tree/commit，快进 `master`。
- 通过 GitHub REST API 创建不可变 annotated tag `v1.8.2`。
- 通过 GitHub REST API 查询远端 `master`、`refs/tags/v1.8.2` 和 tag target，确认发布对象一致。
- 旧版本 tag 不删除、不移动。

## 完成定义

v1.8.2 只有同时满足以下条件才算完成：

- 审核报告的 P1 重复记忆问题已修复，连续重复语言偏好只保留 1 条等价 active preference。
- batch dedup、storage merge、latest view dedup、recall defensive dedup 均有测试覆盖。
- memory schema migration 具备 v1 fixture、legacy no-version fixture、future schema warning 和 corrupt line 混合读取测试。
- JSONL / TUI recall diagnostics 可见且不泄露记忆正文或 secret query。
- provider-backed extraction runtime 级合法 JSON、非法 JSON、空 candidates、超时、rule-only/provider 模式均有测试覆盖。
- TUI 普通 memory notice 已改为短文案，details/debug 保留工程细节。
- 全量统一验证通过。
- CodeGraph 已同步。
- 桌面开发日志已更新。
- GitHub REST API 发布完成，`v1.8.2` tag 成为新的回滚点。

## 自检结论

- 审核报告 P1 / P2 均已映射到明确开发主线。
- 每条主线包含目标、修改文件、设计要求和回归覆盖。
- 本报告不扩大到向量库、关系状态机或新 TUI 大模块，保持 v1.8.x 记忆稳定性加固范围。
- 本报告保留用户硬性约束：先整体构建、最后统一验证、日志、清理、REST API 发布、不可变 tag。

## v1.8.2 构建记录

构建时间：2026-07-14

本轮按报告先完成整体源码构建，未在构建过程中执行中途测试。

已构建内容：

- 新增 persona memory dedup 与 migration 模块。
- 将 memory schema 推进到 v2，增加 dedup key、revision、merged count。
- 将 rule/provider memory candidates 接入统一 batch dedup。
- 将 storage append 路径升级为 append-or-merge revision ledger。
- 将 latest view 与 recall 增加防御性重复折叠。
- 扩展 runtime/core/protocol/CLI/TUI 的 memory recall/write diagnostics。
- 增加 `--memory-extraction <auto|rule-only|provider>`。
- 增加 provider-backed extraction 的 invalid JSON、empty candidates、timeout、rule-only、provider-required 保护路径。
- 将 TUI memory 普通 notice 改为短文案，并把工程字段放入 debug/details。
- 新增 persona/storage/runtime/CLI/TUI/protocol 回归测试源码。
- 更新 workspace/package version、CLI/TUI banner、persona profile 到 `1.8.2`。
- 更新 `docs/persona-memory.md` 与 `docs/extraction-status.md`。

## v1.8.2 统一验证记录

验证时间：2026-07-14

本轮遵守硬性约束：构建阶段完成后统一验证，未在构建过程中围绕单点反复测试。

已通过验证：

- `cargo fmt`
- `cargo fmt --check`
- `cargo test -p yunxi-agent-persona -p yunxi-agent-storage -p yunxi-agent-runtime -p yunxi-agent-cli -p yunxi-agent-tui`
- `cargo test`
- `cargo check --workspace`
- `cargo build -p yunxi-agent-cli --release --bins`
- `target\release\yunxi.exe --version`，输出 `yunxi 1.8.2`
- `target\release\yunxi-agent-cli.exe --version`，输出 `yunxi 1.8.2`
- `git diff --check`
- `codegraph sync .`，同步 26 个变更文件
- `codegraph status .`，索引处于 up to date 状态
- `scripts\install\install-yunxi.ps1 -AddToPath -SkipBuild`
- `yunxi --version`，输出 `yunxi 1.8.2`
- `yunxi-agent-cli --version`，输出 `yunxi 1.8.2`

黑盒验证结果：

- 使用隔离 `YUNXI_HOME` 开启 memory，`memory on --json` disclosure 存在。
- 连续写入“以后请用中文回答”和“默认用中文交流”后，全局 active preference 保持 1 条，`revision=2`。
- `--jsonl "请计算 2+2"` 输出包含 memory recall diagnostics。
- `--memory-extraction provider --offline --jsonl` 输出 no-live-runtime warning，没有静默降级。

本轮 CodeGraph 状态：

- Files: 1,162
- Nodes: 44,881
- Edges: 145,121
- DB Size: 176.27 MB
- Status: index is up to date

待发布收尾：

- 追加桌面开发日志。
- 执行 `cargo clean` 清理构建中间产物。
- 创建本地 release commit。
- 通过 GitHub REST API 快进远端 `master` 并创建不可变 annotated tag `v1.8.2`。
