# YunXi Agent v1.8.3 Memory Merge Fidelity Development Report

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this report task-by-task. The project owner has a hard constraint that construction happens first, with unified verification only after all planned source migration/construction is complete.

## 目标

YunXi Agent v1.8.3 的目标是修复 v1.8.2 审核报告指出的 P1 问题：长期记忆在同一 `dedup_key` 下合并时，不能让泛化的新记忆覆盖更丰富的旧记忆，必须保证持久化内容保真。

本版本不引入 SQLite、向量库、图记忆、关系状态机、TUI 记忆管理器或新 UI 大模块。v1.8.3 只做记忆合并保真、冲突保护、provider/rule 混合场景回归和必要的文档/发布收尾。

## 背景

v1.8.2 已经解决重复记忆无限追加、schema v2 migration、recall diagnostics、provider extraction 保护和 TUI memory notice 文案问题，可以作为 1.8 系长期记忆继续开发的基线。

新的审核报告确认 v1.8.2 仍存在一个必须优先处理的问题：

- `D:\YunXi Agent\crates\yunxi-agent-storage\src\lib.rs:821` 当前使用 `let mut merged = incoming;` 作为合并记录基础。
- 这会导致已有丰富记忆被后续规则抽取出的泛化记忆覆盖。
- `D:\YunXi Agent\crates\yunxi-agent-persona\src\dedup.rs:31` 将中文偏好统一归一为 `language:zh`，这是正确的槽位归一，但它意味着合并策略必须能区分“同槽位重复”和“同槽位有信息增量”。
- 当前 batch dedup 只合并 `confidence/importance/sensitivity/policy/evidence/reason`，不会保护或升级 `content`。

本轮源码核对结论：审核报告的 P1 判断成立，需要进入 v1.8.3。

## 全局硬性约束

- 构建过程中不执行中途测试或验证。
- 不在单个点反复卡住；先整体构建，再统一验证。
- 每次任务结束必须追加 `C:\Users\admin\Desktop\YunXi Agent开发日志.md`。
- 发布前必须执行 `cargo clean` 清理构建中间产物。
- 每个正式版本必须创建新的不可变 annotated tag，本版 tag 为 `v1.8.3`。
- 旧 tag 不删除、不移动。
- GitHub 读写、推送、发布全部走 GitHub REST API。
- 不使用 `git push`、`git fetch`、`git ls-remote`。
- API key 不打印、不写日志、不提交。
- 最终发布后必须通过 GitHub REST API 核验远端 `master`、`refs/tags/v1.8.3` 和 tag target。
- 如 GitHub API commit SHA 与本地 commit SHA 不一致，必须把本地 `master`、`refs/remotes/origin/master` 和本地 tag 对齐到远端发布对象，避免后续分叉。

## 版本定位

- 当前基线：YunXi Agent v1.8.2
- 下一版本：YunXi Agent v1.8.3
- 版本主题：memory merge fidelity
- 核心指标：同槽位合并不丢失更丰富的长期记忆内容，provider 细粒度记忆不会被 rule 泛化记忆抹掉。

## 文件结构规划

新增文件：

- `D:\YunXi Agent\crates\yunxi-agent-persona\src\merge.rs`

修改文件：

- `D:\YunXi Agent\crates\yunxi-agent-persona\src\lib.rs`
- `D:\YunXi Agent\crates\yunxi-agent-persona\src\dedup.rs`
- `D:\YunXi Agent\crates\yunxi-agent-persona\tests\extractor_tests.rs`
- `D:\YunXi Agent\crates\yunxi-agent-persona\tests\memory_policy_tests.rs`
- `D:\YunXi Agent\crates\yunxi-agent-storage\src\lib.rs`
- `D:\YunXi Agent\crates\yunxi-agent-storage\tests\storage_tests.rs`
- `D:\YunXi Agent\crates\yunxi-agent-runtime\src\lib.rs`
- `D:\YunXi Agent\crates\yunxi-agent-runtime\tests\runtime_tests.rs`
- `D:\YunXi Agent\crates\yunxi-agent-core\src\event.rs`
- `D:\YunXi Agent\crates\yunxi-agent-protocol\src\lib.rs`
- `D:\YunXi Agent\crates\yunxi-agent-cli\src\render.rs`
- `D:\YunXi Agent\crates\yunxi-agent-tui\src\event_filter.rs`
- `D:\YunXi Agent\Cargo.toml`
- `D:\YunXi Agent\Cargo.lock`
- `D:\YunXi Agent\crates\yunxi-agent-persona\src\profile.rs`
- `D:\YunXi Agent\docs\persona-memory.md`
- `D:\YunXi Agent\docs\extraction-status.md`
- `D:\YunXi Agent\docs\reports\2026-07-14-yunxi-agent-v1-8-3-memory-merge-fidelity-development-report.md`
- `C:\Users\admin\Desktop\YunXi Agent开发日志.md`

## 开发主线一：记忆合并保真模块

目标：把“如何合并两个等价记忆”的规则放到 persona crate，storage 只负责落盘与 ledger，不直接用 incoming 覆盖旧内容。

新增文件：

- `D:\YunXi Agent\crates\yunxi-agent-persona\src\merge.rs`

修改文件：

- `D:\YunXi Agent\crates\yunxi-agent-persona\src\lib.rs`
- `D:\YunXi Agent\crates\yunxi-agent-persona\src\dedup.rs`

设计要求：

- 新增 `MemoryMergeStrategy`：
  - `PreserveExisting`
  - `PromoteIncoming`
  - `CombineNonConflicting`
  - `ConflictRequiresConfirmation`
- 新增 `MemoryMergeResult`：
  - `record: MemoryRecord`
  - `strategy: MemoryMergeStrategy`
  - `summary: String`
- 新增 `merge_equivalent_memory_records(existing: &MemoryRecord, incoming: &MemoryRecord, now_millis: u128) -> MemoryMergeResult`。
- 新增 `merge_memory_candidates(existing: &mut MemoryCandidate, incoming: MemoryCandidate)`，由 `deduplicate_candidates` 调用。
- 新增内容信息量评分：
  - CJK 字符、ASCII 单词、标点不直接决定胜负，但用于判断泛化短句是否低信息量。
  - 如果 `existing.content` 包含更多约束词，例如“简洁”“关键细节”“保留细节”“不要太长”“默认”，而 `incoming.content` 只是“使用中文回答”，保留 existing。
  - 如果 incoming 明显比 existing 更丰富，并且不冲突，提升 incoming。
  - 如果双方各有非冲突细节，合成为一条内容，格式为 `旧内容；新补充。`，并限制最大长度，避免无限膨胀。
- 新增语言偏好槽位解析：
  - `language:zh`
  - `language:en`
  - 其他内容继续使用原有 normalized text。
- 合并不改变 `id` 和 `created_at_millis`，这些仍由 storage 层保留旧记录。
- 合并必须保留更高的 `confidence`、`importance`、`sensitivity`，并合并 `source_session_id`：
  - 如果 existing 有 `source_session_id`，且 incoming 没有，保留 existing。
  - 如果 incoming 有而 existing 没有，采用 incoming。
  - 如果双方都有不同 session，保留 existing，避免单字段伪装成多来源链路。

回归覆盖：

- provider/rule 同 batch 中，provider 产出“用户偏好使用中文回答，并且回答要简洁、保留关键细节。”，rule 产出“用户偏好使用中文回答。”，最终候选保留 provider 丰富 content。
- rule 泛化先出现、provider 丰富后出现，最终候选升级为丰富 content。
- 两条同键非冲突偏好合并时，content 保留双方信息，不只合并 evidence/reason。
- secret-like candidate 仍然保持 discard 优先级，不因为 content 更丰富而降级为 auto。

## 开发主线二：storage append_or_merge 改为保真合并

目标：修复 `append_or_merge` 用 incoming 作为 merged 基础导致的持久化内容退化。

修改文件：

- `D:\YunXi Agent\crates\yunxi-agent-storage\src\lib.rs`
- `D:\YunXi Agent\crates\yunxi-agent-storage\tests\storage_tests.rs`

设计要求：

- `append_or_merge` 找到 same `dedup_key` 的可合并旧记录后，调用 `merge_equivalent_memory_records(&existing, &incoming, memory_now_millis())`。
- merged record 必须：
  - `id = existing.id`
  - `created_at_millis = existing.created_at_millis`
  - `updated_at_millis = now`
  - `revision = existing.revision.saturating_add(1).max(2)`
  - `merged_count = existing.merged_count.saturating_add(1).max(2)`
  - `dedup_key` 不为空，且仍与原槽位一致
  - `content` 由 merge fidelity 策略决定
- `MemoryPersistOutcome::Merged` 增加 `strategy: String` 或新增 `merge_strategy: Option<String>`；如果为避免事件面破坏，也可以先只在 debug warning/details 中使用内部策略，但测试必须能通过 persisted record 证明策略生效。
- 不能再出现无条件 `let mut merged = incoming;`。

必须新增 storage 测试：

- `file_persona_memory_store_preserves_rich_existing_when_generic_language_preference_repeats`
  - 先写入 content：`用户偏好使用中文回答，并且回答要简洁、保留关键细节。`
  - 再写入 content：`用户偏好后续默认使用中文交流。`
  - 断言最终只有一条记录，id 保留第一条，revision 增加，content 保留“简洁、保留关键细节”。
- `file_persona_memory_store_promotes_rich_incoming_over_generic_existing`
  - 先写泛化中文偏好。
  - 再写丰富中文偏好。
  - 断言最终 content 升级为丰富版本。
- `file_persona_memory_store_combines_non_conflicting_same_slot_details`
  - 两条同 `dedup_key`、不同非冲突细节。
  - 断言最终 content 同时包含两边关键信息。

## 开发主线三：冲突记忆进入待确认而不是自动并存或覆盖

目标：处理审核报告中“中文偏好与英文偏好应走冲突/待确认策略”的约束。

修改文件：

- `D:\YunXi Agent\crates\yunxi-agent-persona\src\merge.rs`
- `D:\YunXi Agent\crates\yunxi-agent-storage\src\lib.rs`
- `D:\YunXi Agent\crates\yunxi-agent-storage\tests\storage_tests.rs`
- `D:\YunXi Agent\crates\yunxi-agent-runtime\tests\runtime_tests.rs`

设计要求：

- 新增 `memory_conflict_family(record: &MemoryRecord) -> Option<String>`：
  - global/workspace + preference + language preference 都归入同一个 language family。
  - `language:zh` 与 `language:en` 不是 same dedup slot，但属于 same conflict family。
- `append_or_merge` 在没有 same `dedup_key` 可合并记录时，检查 active/pending 记录里是否存在 same conflict family 的不同 dedup key。
- 如果存在冲突：
  - 不覆盖旧 active 记忆。
  - incoming 记录写入 pending。
  - incoming content 必须保留原文，方便用户确认。
  - outcome 使用 `Inserted` 也可以，但记录 status 必须是 `Pending`；更好是新增 `MemoryPersistOutcome::ConflictPending`，事件层可以给出 `memory pending review`。
- `memory pending --json` 能看到冲突候选，用户仍可 approve/reject。

必须新增测试：

- 已有 active 中文偏好，再写 active 英文偏好，最终中文 active 仍存在，英文进入 pending。
- 已有 pending 中文偏好，再写英文偏好，英文也不能自动 active。
- recall 不应同时把 active 中文和 pending 英文注入对话上下文。

## 开发主线四：provider/rule 混合场景端到端保护

目标：provider extraction 更可能产出细粒度偏好，rule extraction 更可能产出泛化偏好；v1.8.3 必须保证两者混合不会丢细节。

修改文件：

- `D:\YunXi Agent\crates\yunxi-agent-runtime\src\lib.rs`
- `D:\YunXi Agent\crates\yunxi-agent-runtime\tests\runtime_tests.rs`
- `D:\YunXi Agent\crates\yunxi-agent-cli\tests\cli_tests.rs`

设计要求：

- `emit_memory_extraction_events` 保持先统一 dedup，再落盘。
- provider 产出丰富偏好、rule 同时产出泛化偏好时，batch dedup 阶段保留丰富 content。
- 已落盘 provider 丰富偏好后，后续 rule 泛化输入只更新 revision/merged_count/updated_at，不覆盖 content。
- JSONL `memory_write` 可继续只展示短字段，不泄露 content；测试通过 memory list 验证 persisted content。

必须新增 runtime 测试：

- `provider_rich_memory_is_not_degraded_by_rule_generic_followup`
  - 第一轮 fixture provider 返回丰富中文偏好。
  - 第二轮 offline/rule 输入泛化中文偏好。
  - 断言最终 global active preference 只有一条，content 仍含“简洁、保留关键细节”，revision 增加。
- `rule_generic_and_provider_rich_same_turn_keeps_rich_candidate`
  - 同一轮 provider + rule 都产出同槽位候选。
  - 断言最终写入 content 为丰富版本。

## 开发主线五：事件、协议与显示层最小跟进

目标：合并保真策略需要可诊断，但不能把长期记忆正文泄露到普通输出或 TUI 普通 notice。

修改文件：

- `D:\YunXi Agent\crates\yunxi-agent-core\src\event.rs`
- `D:\YunXi Agent\crates\yunxi-agent-protocol\src\lib.rs`
- `D:\YunXi Agent\crates\yunxi-agent-cli\src\render.rs`
- `D:\YunXi Agent\crates\yunxi-agent-tui\src\event_filter.rs`

设计要求：

- `MemoryWrite` 事件可以新增：
  - `merge_strategy: Option<String>`
  - `conflict_family: Option<String>`
- JSONL 可以暴露 `merge_strategy` 与 `conflict_family`，但不能暴露 `content`。
- 普通 CLI/TUI 文案保持短提示：
  - preserved rich existing：`memory updated: preference`
  - promoted rich incoming：`memory updated: preference`
  - conflict pending：`memory pending review; run yunxi memory pending...`
- TUI debug/details 可显示 `merge_strategy`、`revision`、`merged_count`、`conflict_family`。

必须新增测试：

- protocol JSONL round-trip 包含 `merge_strategy`。
- TUI 普通 notice 不包含 rich content。
- TUI debug/details 包含 `merge_strategy`，但不泄露 secret-like query。

## 开发主线六：版本、文档、状态与发布

目标：把 v1.8.3 的边界、验证结果和发布对象写入项目文档。

修改文件：

- `D:\YunXi Agent\Cargo.toml`
- `D:\YunXi Agent\Cargo.lock`
- `D:\YunXi Agent\crates\yunxi-agent-cli\src\main.rs`
- `D:\YunXi Agent\crates\yunxi-agent-cli\src\render.rs`
- `D:\YunXi Agent\crates\yunxi-agent-tui\src\app.rs`
- `D:\YunXi Agent\crates\yunxi-agent-persona\src\profile.rs`
- `D:\YunXi Agent\docs\persona-memory.md`
- `D:\YunXi Agent\docs\extraction-status.md`
- `D:\YunXi Agent\docs\reports\2026-07-14-yunxi-agent-v1-8-3-memory-merge-fidelity-development-report.md`
- `C:\Users\admin\Desktop\YunXi Agent开发日志.md`

设计要求：

- workspace/package version 更新到 `1.8.3`。
- CLI/TUI banner/version 输出更新到 `1.8.3`。
- persona profile version 更新到 `1.8.3`。
- `docs/persona-memory.md` 增加 merge fidelity、content preservation、conflict pending、provider/rule mixed scenarios。
- `docs/extraction-status.md` 增加 v1.8.3 construction 与最终统一验证记录。
- 发布时通过 GitHub REST API 更新远端 `master` 并创建 annotated tag `v1.8.3`。

## 统一验证计划

开发过程中不执行中途验证；所有源码构建完成后一次性执行：

```powershell
cargo fmt
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

黑盒验证使用临时 `YUNXI_HOME`：

- 预置丰富中文偏好后运行 `yunxi --offline --jsonl "以后请用中文回答"`，`memory list --global --json` 中 content 保留丰富细节。
- 先写泛化中文偏好，再写丰富中文偏好，最终 content 升级为丰富版本。
- provider fixture 产出丰富中文偏好后，后续 rule 泛化输入不会覆盖 content。
- 已有 active 中文偏好后输入英文偏好，英文进入 pending，active 中文不被覆盖。
- `--jsonl` 输出包含 merge strategy diagnostics，但不泄露 memory content。
- TUI event filter 测试确认普通 notice 不泄露 content，debug/details 可看 merge strategy。
- owned-source secret scan 排除 `vendor`、`extracted`、`target`、`.git`、`.codegraph` 后无真实 API key、bearer token 或 authorization bearer header。

安装刷新与本机入口验证：

```powershell
powershell -ExecutionPolicy Bypass -File .\scripts\install\install-yunxi.ps1 -AddToPath -SkipBuild
yunxi --version
yunxi-agent-cli --version
```

清理：

```powershell
cargo clean
```

## 发布计划

v1.8.3 完成并验证通过后：

- 追加 `C:\Users\admin\Desktop\YunXi Agent开发日志.md`，记录改动文件、验证结果、清理结果、API 发布结果。
- 本地创建 release commit。
- 通过 GitHub REST API 创建远端 blob/tree/commit，快进 `master`。
- 通过 GitHub REST API 创建不可变 annotated tag `v1.8.3`。
- 通过 GitHub REST API 查询远端 `master`、`refs/tags/v1.8.3` 和 tag target，确认发布对象一致。
- 如果 API commit/tag SHA 与本地对象不一致，物化远端对象并更新本地 refs。
- 旧版本 tag 不删除、不移动。

## 完成定义

v1.8.3 只有同时满足以下条件才算完成：

- rich existing + generic incoming 不会退化 content。
- generic existing + rich incoming 会升级 content。
- 同 batch provider rich + rule generic 保留 rich content。
- 已落盘 provider rich 后，rule generic follow-up 不覆盖 rich content。
- 中文/英文偏好冲突不自动覆盖，冲突候选进入 pending。
- persisted JSONL revision ledger 继续 append-only。
- recall 只注入 active 且非冲突 pending 不进入上下文。
- CLI/TUI 普通 notice 不泄露 memory content。
- JSONL/debug diagnostics 能看出 merge strategy。
- 全量统一验证通过。
- CodeGraph 已同步。
- 桌面开发日志已更新。
- `cargo clean` 已清理中间产物。
- GitHub REST API 发布完成，`v1.8.3` tag 成为新的回滚点。

## 自检结论

- 审核报告 P1 已映射到记忆合并保真、storage 落盘策略、provider/rule 混合保护和冲突 pending 四条开发主线。
- 审核报告 P2 的 TUI 截图级视觉核查不作为 v1.8.3 阻断源码项；本版只补事件过滤测试和文档记录，真实截图核查可在后续有稳定终端截图能力时执行。
- 本报告不扩大到向量库、关系状态机、TUI 记忆管理器或新 UI 模块。
- 本报告继续遵守用户硬性约束：先整体构建、最后统一验证、日志、清理、REST API 发布、不可变 tag。

## v1.8.3 构建记录

构建时间：2026-07-14

本轮按报告先完成整体源码构建，构建过程中不执行中途测试。

已构建内容：

- 新增 `yunxi-agent-persona::merge` 模块。
- 增加 `MemoryMergeStrategy`、`MemoryMergeResult`、`merge_equivalent_memory_records`、`merge_memory_candidates` 和 `memory_conflict_family`。
- batch dedup 改为调用 merge fidelity 策略，避免 provider/rule 同批候选丢失丰富 content。
- storage `append_or_merge` 改为基于 existing + incoming 的保真合并，不再无条件以 incoming 作为 merged 基础。
- storage 增加 language conflict family 识别，中文/英文偏好冲突进入 pending。
- runtime Auto extraction 改为 provider candidates + rule candidates 统一 dedup。
- `MemoryWrite` / `RuntimeEvent::MemoryWrite` 增加 `merge_strategy` 和 `conflict_family` 诊断字段。
- CLI plain render 与 TUI debug/details 显示 merge diagnostics，但普通 notice 不暴露 memory content。
- 新增 persona/storage/runtime/protocol/TUI 回归测试源码。
- 更新 workspace/package version、CLI/TUI banner、persona profile 到 `1.8.3`。
- 更新 `docs/persona-memory.md` 与 `docs/extraction-status.md`。

## v1.8.3 统一验证记录

验证时间：2026-07-14

统一验证按“先整体构建、最后统一验证”的硬约束执行。构建阶段未做中途测试；源码构建完成后统一执行以下验证。

已通过验证：

- `cargo fmt`: pass
- `cargo fmt --check`: pass
- `cargo test -p yunxi-agent-persona -p yunxi-agent-storage -p yunxi-agent-runtime -p yunxi-agent-cli -p yunxi-agent-tui`: pass
- `cargo test`: pass，workspace unit tests、integration tests、doc tests 全部通过
- `cargo check --workspace`: pass
- `cargo build -p yunxi-agent-cli --release --bins`: pass
- `target\release\yunxi.exe --version`: pass，输出 `yunxi 1.8.3`
- `target\release\yunxi-agent-cli.exe --version`: pass，输出 `yunxi 1.8.3`
- `git diff --check`: pass，仅有 Windows LF-to-CRLF 工作区提示
- 隔离 `YUNXI_HOME` 黑盒验证：pass
  - rich existing + CLI generic follow-up 保留“简洁、保留关键细节”
  - JSONL `memory_write` 输出包含 `merge_strategy` 且不泄露 memory content
  - pending 记录可通过 `memory pending --json` 列出，且不进入 runtime recall 输出
- owned-source secret scan：pass，仅命中文档占位符/fixture，无真实 API key、Bearer token 或 GitHub token
- `codegraph sync .`: pass，同步 18 个变更文件
- `codegraph status .`: pass，index is up to date，Files 1,163，Nodes 44,917，Edges 145,370
- `powershell -ExecutionPolicy Bypass -File .\scripts\install\install-yunxi.ps1 -AddToPath -SkipBuild`: pass
- PATH 入口验证：`yunxi --version` 与 `yunxi-agent-cli --version` 均输出 `yunxi 1.8.3`
- `cargo clean`: pass，Removed 13154 files, 3.4GiB total

验证过程中的非产品问题：

- 第一轮黑盒脚本使用 `$home` 变量触发 PowerShell 内置只读 `$HOME` 冲突，已确认是脚本变量名问题。
- 使用自然语言提示制造 rich memory 时，offline rule extractor 会归一成泛化中文偏好，不能作为 rich memory 黑盒夹具。最终改为在隔离存储中用 CLI 生成合法 JSONL 模板后预置 rich record，再通过 CLI generic follow-up 验证真实 merge/read 路径。
- 当前 offline rule extractor 只识别中文语言偏好，不从“英文回答”自然语言生成英文候选；中文/英文 conflict pending 写入能力由 storage/runtime 回归测试覆盖，黑盒层验证 pending list 与 recall 过滤路径。
