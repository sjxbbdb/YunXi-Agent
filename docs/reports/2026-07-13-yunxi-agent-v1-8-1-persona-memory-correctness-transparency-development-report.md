# YunXi Agent v1.8.1 Persona Memory Correctness And Transparency Development Report

生成时间：2026-07-13 19:59:38 +08:00

## 背景

YunXi Agent 当前正式基线为 `v1.8.0`，已完成 persona / memory 第一版骨架：独立 `yunxi-agent-persona` crate、本地 JSONL 记忆、默认 `memory off`、persona 默认启用、CLI 记忆管理命令、runtime turn start recall、turn end 规则提炼、TUI 基础呈现和 `v1.8.0` 不可变 tag 发布。

用户提供了新的源码审核报告：

- `C:\Users\admin\Desktop\YunXi Agent调研项目\2026-07-13-YunXi-Agent-1.8.0-源码审核报告.md`

本报告已经按外部审核意见接收流程复核源码，不直接照单实现；当前源码与审核报告指出的核心问题基本一致。v1.8.1 的目标不是扩展新模块，而是把 v1.8.0 的人格与长期记忆底座从“可运行骨架”推进到“语义正确、透明可控、可回归验证”的基础版本。

## 已复核的关键问题

本轮使用 CodeGraph 与源码定位确认以下问题成立：

- `D:\YunXi Agent\crates\yunxi-agent-persona\src\extractor.rs:26` 到 `30`：`MemoryRuleExtractor` 只要收到 `workspace_fingerprint`，所有候选都会进入 `MemoryScope::Workspace`。
- `D:\YunXi Agent\crates\yunxi-agent-runtime\src\lib.rs:1858` 到 `1865`：runtime 每轮 extraction 都传入当前 workspace fingerprint。
- `D:\YunXi Agent\crates\yunxi-agent-persona\src\recall.rs:63` 到 `75`：recall 初始分数来自 `importance * 3.0 + confidence`，active 记录天然大于 0。
- `D:\YunXi Agent\crates\yunxi-agent-persona\src\recall.rs:19`：过滤条件允许任意正分 active 记忆进入候选。
- `D:\YunXi Agent\crates\yunxi-agent-storage\src\lib.rs:794` 到 `805`：`update_status` 把更新后的记录追加到 active/global/workspace 文件。
- `D:\YunXi Agent\crates\yunxi-agent-storage\src\lib.rs:862` 到 `865`：`PersonaMemoryScope::Pending` 只读取 pending JSONL，导致 approve/reject/archive 后旧 pending 仍可被 `memory pending` 看到。
- `D:\YunXi Agent\crates\yunxi-agent-cli\src\main.rs:1637` 到 `1642`：`memory on` 只输出状态，没有首次启用长期记忆的 disclosure / consent 文案。
- `D:\YunXi Agent\crates\yunxi-agent-runtime\src\lib.rs:1621` 到 `1655`：当前 prompt 顺序为 AGENTS/persona/restored history/mentioned file/user，与 v1.8.0 设计建议不一致。
- `D:\YunXi Agent\crates\yunxi-agent-storage\src\lib.rs:808` 到 `819`：`memory clear --workspace` 只归档 active workspace records，不处理 pending。
- `D:\YunXi Agent\crates\yunxi-agent-tui\src\event_filter.rs:303` 到 `340`：memory recall/candidate/write 默认全部归为 debug-only，普通 TUI 用户看不到自动保存或 pending 提醒。
- `D:\YunXi Agent\crates\yunxi-agent-tui\src\render.rs:163` 到 `205` 与 `D:\YunXi Agent\crates\yunxi-agent-tui\src\bottom_pane.rs:284` 到 `289`：composer 高度和光标位置仍主要按逻辑行和 `chars().count()` 计算，长中文输入会有错位风险。

## v1.8.1 总目标

v1.8.1 要把 v1.8.0 的 persona / memory 能力补到可信基础线：

- 全局用户记忆与 workspace 记忆按语义正确分层。
- recall 只注入相关 active 记忆，避免无关长期记忆污染 prompt。
- pending 工作流状态一致，approve/reject/delete/archive 后 CLI 视图不再显示旧 pending。
- live provider 可用时支持结构化候选提炼，失败时降级规则提炼，不影响主对话。
- `memory on` 首次启用时明确说明本地存储、pending、关闭和删除方式。
- prompt 注入顺序与设计文档一致，并用测试锁住。
- `memory clear --workspace` 的行为与输出语义一致。
- TUI 默认展示 memory write 透明性事件，但不泄露记忆正文。
- TUI composer 对中文宽度、长 token、多行输入的高度和光标位置更稳定。

## 用户硬性约束

执行 v1.8.1 开发时继续遵守以下约束：

- 按本报告先整体构建源码，中间不做反复单点测试，不在某个点卡过多时间。
- 构建过程中可以新增/修改测试源码，但不执行中途验证；全部构建完成后统一执行验证。
- 每个正式版本必须创建新的不可变 annotated tag；v1.8.1 发布时创建 `v1.8.1`，旧 tag 不删除、不移动。
- GitHub 读写和推送只走 REST API，不使用 `git push`、`git fetch`、`git ls-remote`。
- API key 只从用户指定的本地 txt 文件读取，不打印、不写日志、不提交。
- 每次任务结束必须追加 `C:\Users\admin\Desktop\YunXi Agent开发日志.md`。
- 发布收尾前执行 `cargo clean` 清理构建中间产物。
- 默认运行路径继续保持 YunXi 自主化，不恢复对上游 Codex CLI runtime、`vendor/codex-rs`、`codex-*` 或 `yunxi-agent-codex` 的运行依赖。
- 人格与记忆不能绕过既有 sandbox、approval、execution policy、provider honesty 和 secret redaction 边界。

## 非目标

v1.8.1 不做以下内容：

- 不引入 SQLite、向量库、embedding store、graph database 或外部 memory service。
- 不实现完整关系状态机、记忆衰减、矛盾检测、主动关心 trigger。
- 不复制 AGPL 或外部项目实现。
- 不新增 TUI memory inspector 大面板，只做必要透明 notice。
- 不改变用户指定的 DeepSeek / provider 模型接口替换方向。
- 不把 provider-backed extraction 变成主对话阻塞路径。

## 开发主线一：记忆 scope 语义路由

目标：修复“所有候选因为 workspace fingerprint 存在而进入 workspace”的问题。

修改文件：

- `D:\YunXi Agent\crates\yunxi-agent-persona\src\extractor.rs`
- `D:\YunXi Agent\crates\yunxi-agent-persona\src\memory.rs`
- `D:\YunXi Agent\crates\yunxi-agent-persona\src\lib.rs`
- `D:\YunXi Agent\crates\yunxi-agent-persona\tests\extractor_tests.rs`
- `D:\YunXi Agent\crates\yunxi-agent-runtime\src\lib.rs`

设计要求：

- 新增 `MemoryScopeRouter` 或等价内部函数，由 `MemoryKind`、规则 reason、内容语义和当前 workspace fingerprint 决定 scope。
- `Preference` 中的语言、表达风格、通用互动偏好默认进入 `GlobalUser`。
- `PersonalFact` 默认进入 `GlobalUser`，敏感时仍按 policy 进入 pending。
- `ProjectContext` 和“当前项目/当前仓库/硬性约束/开发流程”进入 `Workspace`。
- `Correction` 按语义分流：通用表达纠正进入 `GlobalUser`，项目约束进入 `Workspace`。
- `RelationshipNote`、`EmotionalState` 默认进入 `Relationship` 或全局关系语义，不再默认 workspace。
- runtime 仍可传 workspace fingerprint，但 extractor 不能把它当成唯一 scope 决策。

必须新增回归覆盖：

- `以后请用中文回答` 生成 `Preference + GlobalUser`。
- `当前项目的硬性要求是...` 生成 `ProjectContext + Workspace`。
- global 语言偏好在另一个 workspace 可被 recall。
- workspace 记忆只在相同 fingerprint 下 recall。

## 开发主线二：recall 相关性门槛与 always-on profile 分离

目标：修复无关 active 记忆被注入 prompt 的问题。

修改文件：

- `D:\YunXi Agent\crates\yunxi-agent-persona\src\recall.rs`
- `D:\YunXi Agent\crates\yunxi-agent-persona\src\compiler.rs`
- `D:\YunXi Agent\crates\yunxi-agent-persona\src\memory.rs`
- `D:\YunXi Agent\crates\yunxi-agent-persona\tests\recall_tests.rs`
- `D:\YunXi Agent\crates\yunxi-agent-runtime\src\lib.rs`

设计要求：

- `score_record` 不再让 `importance/confidence` 单独决定入选；非空 query 必须至少满足 query term、kind trigger、scope scene trigger 或 always-on profile 策略之一。
- 引入最低相关性阈值，例如 `RECALL_RELEVANCE_THRESHOLD`，阈值命中后才进入普通 recall。
- 把“语言偏好/称呼偏好/输出风格偏好”作为小预算 always-on profile memory，独立于 query-relevant recall。
- always-on profile 要去重、限量、限字符预算，不能演变为全量 active 注入。
- `MemoryRecallResult` 保留 `truncated`，必要时增加可解释字段，如 `dropped_unrelated`、`dropped_by_budget`、`always_on_count`。
- pending/rejected/archived 继续不进入 recall。

必须新增回归覆盖：

- 已有中文偏好时，询问 `请计算 2+2` 不召回无关项目记忆。
- 空 query 的策略明确：只允许 always-on profile 或零召回，不能全量 active。
- 超过 `max_records` 或 `budget_chars` 时 `truncated = true` 且输出可解释。
- pending/rejected/archived 不召回。

## 开发主线三：pending 工作流一致性

目标：修复 approve/reject/delete/archive 后 `memory pending` 仍显示旧候选的问题。

修改文件：

- `D:\YunXi Agent\crates\yunxi-agent-storage\src\lib.rs`
- `D:\YunXi Agent\crates\yunxi-agent-storage\tests\storage_tests.rs`
- `D:\YunXi Agent\crates\yunxi-agent-cli\src\main.rs`
- `D:\YunXi Agent\crates\yunxi-agent-cli\tests\memory_cli_tests.rs`

设计要求：

- `memory pending` 必须基于 `PersonaMemoryScope::All` 的 latest 状态，再过滤 `status == Pending`。
- 可以新增 `FilePersonaMemoryStore::pending_records()`，避免 CLI 重复状态规则。
- `update_status` 更新 pending 记录后，latest 视图必须压过 pending ledger 中的旧状态。
- `memory status` 的 pending 数量与 `memory pending` 输出数量保持一致。
- 保留 JSONL append-only 模型，不做破坏性重写。

必须新增回归覆盖：

- approve 后 pending 列表为空。
- reject 后 pending 列表为空。
- delete/archive 后 pending 列表为空。
- pending 记录从 global/workspace 两处来源都能被正确更新。

## 开发主线四：live provider 结构化候选提炼

目标：live provider 可用时使用模型提炼结构化 memory candidates，规则提炼作为 fallback。

修改文件：

- `D:\YunXi Agent\crates\yunxi-agent-persona\src\extractor.rs`
- `D:\YunXi Agent\crates\yunxi-agent-persona\src\provider_extractor.rs`
- `D:\YunXi Agent\crates\yunxi-agent-persona\src\lib.rs`
- `D:\YunXi Agent\crates\yunxi-agent-provider\src\lib.rs`
- `D:\YunXi Agent\crates\yunxi-agent-runtime\src\lib.rs`
- `D:\YunXi Agent\crates\yunxi-agent-runtime\tests\runtime_tests.rs`

设计要求：

- 定义 provider extraction schema：`kind`、`content`、`scope_hint`、`sensitivity_hint`、`confidence`、`importance`、`reason`。
- live provider 可用且 memory enabled 时，turn end 触发低优先级结构化提炼。
- provider extraction 必须设置超时和失败降级；失败只发 warning/debug 事件，不影响 final response。
- provider extraction 输出必须再次经过 `MemoryWritePolicyEngine` 和 `MemoryScopeRouter`，不能绕过 secret discard / pending 规则。
- offline/no provider 时保留 `MemoryRuleExtractor` fallback。
- 对 DeepSeek / OpenAI-style provider 兼容当前 provider transport，不把具体 provider key 写入源码或日志。

必须新增回归覆盖：

- mock provider 返回结构化候选时能入库或进入 pending。
- provider extraction 超时/失败时主对话正常完成，并降级规则提炼。
- provider 返回 secret-like candidate 时仍被 discard。
- provider 返回 workspace hint 但内容是通用语言偏好时，scope router 仍写入 global。

## 开发主线五：首次启用 memory 的 disclosure

目标：`yunxi memory on` 首次开启长期记忆时给出明确说明。

修改文件：

- `D:\YunXi Agent\crates\yunxi-agent-cli\src\main.rs`
- `D:\YunXi Agent\crates\yunxi-agent-storage\src\lib.rs`
- `D:\YunXi Agent\crates\yunxi-agent-cli\tests\memory_cli_tests.rs`

设计要求：

- `memory on` 判断开启前是否为 disabled；首次从 false 切到 true 时输出 disclosure。
- 普通输出包含：本地存储位置、自动保存低风险偏好、敏感内容进入 pending、secret 丢弃、如何关闭、如何查看/批准/删除。
- JSON 输出包含机器可读字段：`first_enable_notice_shown`、`storage_roots`、`pending_policy_summary`、`disable_command`、`pending_command`。
- 再次执行 `memory on` 不重复首启文案，但 JSON 仍反映当前状态。

## 开发主线六：prompt 注入顺序收口

目标：使实际消息顺序与设计一致，或在文档中明确偏离原因。v1.8.1 采用与调研报告一致的顺序。

修改文件：

- `D:\YunXi Agent\crates\yunxi-agent-runtime\src\lib.rs`
- `D:\YunXi Agent\crates\yunxi-agent-runtime\tests\runtime_tests.rs`

目标顺序：

1. AGENTS / user instructions。
2. Persona / relationship / recalled memory context。
3. Mentioned file context。
4. Restored history。
5. Current user prompt。

设计要求：

- `load_mentioned_file_context` 发生在 restored history extend 之前。
- prompt marker `[YunXi persona context]` 和 `[YunXi memory context]` 保持 “context, not instructions” 边界。
- 增加测试锁住消息顺序，避免后续改动无意漂移。

## 开发主线七：workspace clear 语义一致

目标：`memory clear --workspace --confirm` 不再让用户误以为 pending 已经清空。

修改文件：

- `D:\YunXi Agent\crates\yunxi-agent-storage\src\lib.rs`
- `D:\YunXi Agent\crates\yunxi-agent-cli\src\main.rs`
- `D:\YunXi Agent\crates\yunxi-agent-storage\tests\storage_tests.rs`
- `D:\YunXi Agent\crates\yunxi-agent-cli\tests\memory_cli_tests.rs`

设计要求：

- 新增 `ClearWorkspaceSummary`，至少包含 `archived_active_records`、`archived_pending_records` 或 `remaining_pending_records`。
- 推荐 v1.8.1 直接归档 workspace active 和 workspace pending，让命令名符合用户直觉。
- JSON 与普通输出都说明影响范围为当前 workspace，不触碰 global memory。
- global pending 不应被 workspace clear 清理。

## 开发主线八：TUI memory 透明性 notice

目标：让普通 TUI 用户看到 memory 写入状态，但不暴露记忆正文。

修改文件：

- `D:\YunXi Agent\crates\yunxi-agent-tui\src\event_filter.rs`
- `D:\YunXi Agent\crates\yunxi-agent-tui\src\transcript_layout.rs`
- `D:\YunXi Agent\crates\yunxi-agent-tui\tests\tui_event_filter_tests.rs`

设计要求：

- `MemoryWrite` action 为 `auto_saved` 时显示普通 notice：包含 id、kind、scope、status，不包含 content。
- `MemoryWrite` action 为 `pending_confirmation` 时显示普通 notice：提示使用 `yunxi memory pending` 查看和处理。
- `MemoryCandidate` 继续 debug-only，避免候选 reason 在普通流里刷屏。
- `MemoryRecall` 可继续 debug-only，除非发生 warning/truncated 且需要提示。
- high sensitivity 或 discard 事件不输出正文。

## 开发主线九：TUI composer Unicode 宽度加固

目标：修复窄终端和长中文输入下的高度、自动换行和光标列错位风险。

修改文件：

- `D:\YunXi Agent\crates\yunxi-agent-tui\src\render.rs`
- `D:\YunXi Agent\crates\yunxi-agent-tui\src\bottom_pane.rs`
- `D:\YunXi Agent\crates\yunxi-agent-tui\src\layout.rs`
- `D:\YunXi Agent\crates\yunxi-agent-tui\tests\tui_layout_tests.rs`

设计要求：

- composer prompt display width 使用 `unicode-width`，不再使用 `prompt.chars().count()`。
- cursor x 使用当前逻辑行的 display width，并考虑 prompt 宽度、pane inner width 和自动 wrap。
- desired height 根据实际显示行估算，而不是只按 `buffer.lines().count()`。
- 长未断词 token、混合 CJK/English、多行输入都不能挤压 footer 或越过 pane 边界。
- 保留现有 transcript CJK wrap 能力，不引入大规模 TUI 重构。

必须新增回归覆盖：

- 58 列宽终端长中文输入不溢出。
- 混合中文 English 输入光标列正确。
- 长未断词 token 高度可预测。
- 多行输入的 footer 不被覆盖。

## 开发主线十：HumanProfile / RelationshipState 的边界说明

目标：不在 v1.8.1 扩大到完整画像状态机，但要消除“只有 schema 没使用”的歧义。

修改文件：

- `D:\YunXi Agent\crates\yunxi-agent-persona\src\profile.rs`
- `D:\YunXi Agent\crates\yunxi-agent-persona\src\compiler.rs`
- `D:\YunXi Agent\docs\reports\2026-07-13-yunxi-agent-v1-8-1-persona-memory-correctness-transparency-development-report.md`

设计要求：

- 明确 `HumanProfile` / `RelationshipState` 仍是 v1.8.x 后续层，不作为 v1.8.1 完整交付目标。
- 如果 compiler 继续使用 default profile，要在代码注释或文档中说明目前长期画像仍通过 memory records 表达。
- 不把未成熟的 relationship state 自动写入 prompt，避免伪个性化。

## 统一验证计划

开发过程中不执行中途验证；所有源码构建完成后一次性执行以下验证：

```powershell
cargo fmt --check
cargo test -p yunxi-agent-persona -p yunxi-agent-storage -p yunxi-agent-runtime -p yunxi-agent-cli -p yunxi-agent-tui
cargo test
codegraph sync .
codegraph status .
target\debug\yunxi.exe --version
```

黑盒验证使用临时 `YUNXI_HOME` 和临时 `ws1/ws2`：

- 默认 memory off 不读 active records、不 extraction、不创建 memory 文件。
- `memory on` 首次开启显示 disclosure，JSON 字段完整。
- `以后请用中文回答` 写入 global。
- `当前项目硬性要求...` 写入 workspace。
- ws2 可 recall global 语言偏好，不能 recall ws1 workspace 记忆。
- 无关 query 不注入 unrelated active memory。
- medium sensitivity 进入 pending，secret-like 内容 discard。
- approve/reject/delete 后 `memory pending` 与 `memory status` 一致。
- `memory clear --workspace --confirm` 输出 active/pending 处理摘要。
- TUI 普通模式显示 memory write notice，不显示 memory content。
- 58 列中文长输入 composer 不错位。

## 发布计划

v1.8.1 完成并验证通过后：

- 更新 workspace crate/package version 到 `1.8.1`。
- 更新 CLI/TUI banner/version 输出到 `1.8.1`。
- 更新 `CHANGELOG` 或 release notes。
- 追加 `C:\Users\admin\Desktop\YunXi Agent开发日志.md`。
- 执行 `cargo clean` 清理中间产物。
- 通过 GitHub REST API 提交 master 更新。
- 通过 GitHub REST API 创建不可变 annotated tag `v1.8.1`，不删除、不移动旧 tag。

## 完成定义

v1.8.1 只有同时满足以下条件才算完成：

- 审核报告列出的 P1 问题全部修复。
- P2 中 memory disclosure、prompt 顺序、workspace clear、TUI memory notice、composer CJK 稳定性均有源码改动和回归测试。
- provider-backed structured extraction 存在可测试实现，并且不会绕过 write policy。
- 全量验证通过。
- CodeGraph 已同步。
- 工作日志已更新。
- GitHub REST API 发布完成，`v1.8.1` tag 保留可回滚点。
