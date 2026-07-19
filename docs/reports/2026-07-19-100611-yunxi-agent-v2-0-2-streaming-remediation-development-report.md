# YunXi Agent v2.0.2 流式状态机整改开发报告

- 撰写时间：2026-07-19 10:06:11 +08:00
- 审核报告：`C:\Users\24763\Desktop\YunXi Agent审核报告\2026-07-19-094830-YunXi-Agent-v2.0.2-源码与TUI视觉审核报告.md`
- 开发目录：`D:\YunXi Agent`
- 项目内报告：`D:\YunXi Agent\docs\reports\2026-07-19-100611-yunxi-agent-v2-0-2-streaming-remediation-development-report.md`
- 桌面报告：`C:\Users\24763\Desktop\YunXi Agent开发报告\2026-07-19-100611-yunxi-agent-v2-0-2-streaming-remediation-development-report.md`
- 报告类型：当前版本整改开发报告
- 审核结论转化：v2.0.2 审核不通过，不可进入 v2.0.3 开发。

## 一、硬性约束

以下约束是本项目后续开发、验证、发布和文档同步的前置要求，不得因为上下文过长、任务拆分、实现难度或局部修复而忽略、弱化或绕过。

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
13. 不可因为过长的上下文忽略硬性要求；如果察觉到硬性要求的效力已经减弱，则必须汇报当前情况。
14. 工作尽量固定在项目文件夹，涉及到其他文件夹的操作，要明确告知用户，尤其是 C 盘的用户目录。

## 二、开发目标

v2.0.2 已完成发布纪律、短 live 流式 canonical assistant cell、重复 final、retry/cancel fixture、宽字符与历史回看等部分目标，但审核报告明确判定仍有四项硬性缺口。因此下一阶段不得进入 v2.0.3，而是先围绕 v2.0.2 流式状态机进行整改、复测和重新审核。

本阶段核心目标是把当前“可避免重复最终回答”的实现推进为真正可审计、可取消、可回收的流式事件状态机：

1. 规范化流事件必须具备稳定 `event_id`。
2. `source_sequence` 必须区分 provider 可靠顺序与本地 fallback 顺序，不得把本地抵达顺序伪装成 provider 可靠顺序。
3. TUI timeline store 必须以 `event_id` 做幂等去重，并保留重复事件 debug counter。
4. Markdown 流式提交边界不得只依赖换行，必须扩展到完整行、完整 Markdown fence、完整段落或安全 grapheme 边界。
5. final/cancel 后必须释放或有界保留已完成 `StreamSession` 的完整内容与 buffer，只归档 canonical cell 所需的最小元数据。
6. 真实 live provider 下，active stream 期间 `Ctrl+C` 必须立即取消当前 turn、保留未完成文本状态，并允许用户继续下一次输入。

## 三、发布与版本边界

当前 `v2.0.2` annotated tag 已存在并指向发布提交 `ef7f43f97a783b3ee37d47ab034b709180d0c82e`，提交说明为 `fix: make streaming assistant messages idempotent`。该 tag 不得移动、删除或覆盖。

审核报告同时指出，总纲禁止同版本追加 hotfix。因此整改实现完成后的修复发布版本号和 tag 名称必须由用户确认，并同步更新总纲、发布计划和状态文档。开发者不得自行假定进入 `v2.0.3`，也不得自行指定新的修复版本号。

本报告只定义 v2.0.2 审核失败后的整改工作范围；在重新审核通过前，不得宣称 v2.0.2 流式状态机整改完成。

## 四、审核通过项保留要求

整改不能破坏当前已经通过的能力：

1. delta + final 必须仍然只保留一个 canonical assistant cell。
2. retry、cancel、重复 final 的结构化 fixture 必须继续通过。
3. CJK、日文、Emoji ZWJ、Markdown fence、长 token 合并必须保持文本精确性。
4. 完成后历史回看不得被 final 或 late event 强行抢回尾部。
5. 普通主视图仍只显示用户与助手内容，不得暴露 thinking、memory/context、工具参数、协议 JSON 或 provider wire。

## 五、必须整改的问题

### 1. 稳定 event_id 与可靠 sequence

审核发现 `crates\yunxi-agent-core\src\event.rs` 中 `AgentMessageStream` 只有 `thread_id`、`turn_id`、`stream_id`、`source_sequence` 和 `phase`，缺少稳定 `event_id`。`crates\yunxi-agent-tui\src\presentation.rs` 的 `TuiStreamIdentity` 同样缺少 `event_id`，而 `TimelineStore` 当前按 `source_sequence <= last_sequence` 忽略事件。

开发要求：

- 在核心事件契约中新增稳定 `event_id`，并保证协议映射、TUI presentation 与 timeline store 都能传递该字段。
- 明确区分 provider reliable sequence 与 local fallback sequence。只有 provider 明确提供可靠单调顺序时，才允许用 sequence 判断迟到事件。
- 缺少可靠 provider sequence 时，不得仅凭本地抵达顺序丢弃事件。
- timeline store 必须以 `event_id` 做幂等去重；重复 `event_id` 应忽略并记录 debug counter。
- 本地 fallback id 可由 thread/turn/stream/source/phase/local counter 等组成，但必须明确标记为 fallback，不能影响 provider reliable sequence 语义。

### 2. 流式提交边界

审核发现 `crates\yunxi-agent-tui\src\streaming.rs` 中 `MarkdownStreamCollector::commit_complete_source` 仍以 `buffer.rfind('\n')` 作为唯一提交边界。

开发要求：

- 将提交策略扩展为状态化边界识别：完整行、完整 Markdown fence、完整段落、安全 grapheme 边界。
- fence 内内容不得在代码块结构未闭合时被错误截断或重复提交。
- 段落边界应允许在连续自然语言输出中形成稳定增量，避免 UI 长时间不刷新。
- grapheme 边界必须保护 Emoji ZWJ、组合字符、CJK 与假名，不得产生半字符、错宽或丢字。
- 为每种边界补充状态级测试，不能只依赖最终拼接文本测试。

### 3. 完成 session 回收

审核发现 `crates\yunxi-agent-tui\src\timeline_store.rs` 的 `TimelineStore.sessions` 保存 `StreamSession` 完整 `content`，finalize/finish/cancel 只改变状态，未释放或归档已完成会话，`last_by_turn` 也未见完成后清理。

开发要求：

- final/cancel 后将 transcript 所需 canonical cell 归档为最小元数据。
- `sessions` 中已完成 turn 的完整 `content` 与 stream buffer 必须释放，或设置明确的有界保留策略。
- `last_by_turn` 等索引必须与 session 生命周期一致，避免完成会话长期滞留。
- 增加长会话内存/会话回收回归测试，覆盖多轮大文本流式输出后 `sessions` 与 buffer 的上限行为。

### 4. 真实 live Ctrl+C 取消

审核报告记录真实 DeepSeek live TUI 中，assistant 仍为 `[assistant*]` 且输出持续增长时，两次 `Ctrl+C` 均未取消当前 stream；回答自然结束后再次 `Ctrl+C` 才退出。

开发要求：

- 修复 active stream 期间输入轮询和取消通道，使 `Ctrl+C` 在流式输出中立即作用于当前 turn。
- 取消后应保留未完成 assistant 文本状态，明确进入 cancelled/inactive 状态。
- 取消当前 turn 后，TUI 不应退出整个程序，除非用户在非 active stream 状态再次执行退出操作。
- 取消后必须允许用户继续输入下一次请求。
- 必须在真实 live provider 下重新复核，不能只用离线 fixture 替代。

## 六、源码接入点

### `crates\yunxi-agent-core\src\event.rs`

- 为规范化流事件新增 `event_id`。
- 将 sequence 表示拆分为可靠 provider sequence 与本地 fallback sequence，避免语义混用。
- 保持兼容性时要明确 serde 默认值和旧字段处理策略，不能破坏已有 event JSON 契约。

### `crates\yunxi-agent-runtime\src\lib.rs`

- 调整 `ProtocolStreamEventMapper`，优先传播 provider 提供的稳定事件 id 与可靠 sequence。
- provider 未提供事件 id 时，生成稳定 fallback event id，并标记来源。
- 不得再把 `next_source_sequence` 这种本地抵达序号作为 provider reliable sequence 使用。
- 保持现有工具审批与普通 turn 主链路不被流式映射改动破坏。

### `crates\yunxi-agent-tui\src\presentation.rs`

- `TuiStreamIdentity` 必须携带 `event_id` 和 sequence 来源信息。
- legacy identity 生成逻辑需要显式标记 fallback，避免被 timeline store 当作可靠 provider sequence。
- 输出给普通视图的数据仍保持安静，不暴露 provider wire。

### `crates\yunxi-agent-tui\src\timeline_store.rs`

- 以 `event_id` 建立已见事件集合和重复事件 debug counter。
- 只有 reliable sequence 才参与 late event 判定。
- final/cancel 后归档 canonical cell 最小元数据，并释放或有界保留 session content/buffer。
- 覆盖 retry、cancel、重复 final、late delta、合法重复文本和多 session 回收。

### `crates\yunxi-agent-tui\src\streaming.rs`

- 重构 `MarkdownStreamCollector::commit_complete_source`，从换行提交改为状态化 Markdown/grapheme 安全提交。
- 不要把完整内容重写、重复提交或在复杂 Unicode 中切断字符簇。

### `crates\yunxi-agent-tui\src\app.rs`、`host.rs` 与 CLI TUI 入口

- 修复 active stream 下输入事件优先级与取消信号路径。
- `Ctrl+C` 在流式中应取消当前 turn；非流式状态下再按退出。
- 保持 transcript、composer、scroll anchor 与 redraw 的一致性。

## 七、源码参考建议

可参考审核报告列出的 Codex TUI 源码：

- `D:\源码\codex\codex-rs\tui\src\chatwidget\rendering.rs`
- `D:\源码\codex\codex-rs\tui\src\render\renderable.rs`
- `D:\源码\codex\codex-rs\tui\src\wrapping.rs`
- `D:\源码\codex\codex-rs\tui\src\width.rs`

参考边界：

- 只抽取 active streaming cell、history cell、wrapping、width 与 renderable 分层的设计思想。
- 不得让 YunXi 默认运行路径依赖 `vendor/codex-rs`、`codex-*` crate 或上游 Codex TUI runtime。
- 不直接复制上游 UI 代码替代 YunXi 自身状态机。
- 若参考非 Rust 实现，只参考逻辑，使用 YunXi 自身 Rust 模块复刻。

## 八、推荐实现顺序

1. 先固定事件契约：新增 `event_id`，拆分 reliable/fallback sequence，并同步 runtime/presentation/timeline store 类型。
2. 再改 timeline store 幂等语义：以 `event_id` 去重，可靠 sequence 才判迟到，补 debug counter。
3. 接着处理 final/cancel 生命周期：归档 canonical cell 最小元数据，释放或有界保留 session content/buffer。
4. 然后重构 `MarkdownStreamCollector` 提交边界，补齐 Markdown 与 Unicode 状态测试。
5. 最后修复 TUI active stream 取消路径，确保 live provider 下 `Ctrl+C` 能立即取消当前 turn。
6. 完成一批构建后统一测试、真实 live 复核、清理中间产物，并等待用户确认修复版本号/tag 后再进入发布动作。

## 九、最低测试要求

必须新增或调整以下测试，且不能删除已有 v2.0.2 通过项测试：

1. 重复 `event_id` 被忽略，debug counter 增加。
2. provider reliable sequence 的迟到事件被拒绝；local fallback sequence 不得仅按本地抵达顺序丢事件。
3. 缺少 provider event id 时，fallback event id 在同一事件重放中保持稳定。
4. 完整行、完整 Markdown fence、完整段落、安全 grapheme 边界分别有状态级测试。
5. Emoji ZWJ、组合字符、CJK、假名、长 token 在增量提交和最终文本中都保持精确。
6. final/cancel 后 completed session 从 active map 释放或进入有界保留；多轮长文本不会无限滞留。
7. active stream 期间 cancel 事件能中止当前 turn，late delta 不得重新绑定已取消 session。
8. 实际 TUI 路径下，取消后 transcript 保留未完成文本，composer 可继续输入。
9. v2.0.1 与 v2.0.2 已有回归测试继续通过。

## 十、统一验证要求

完成整改后统一执行验证，不在单个小点上反复停顿：

1. `cargo fmt --all`
2. `cargo fmt --all -- --check`
3. `cargo check --workspace`
4. `cargo test --workspace`
5. `cargo build --workspace`
6. release build 与 `yunxi --version`
7. TUI offline smoke：普通输入、scroll、退出
8. TUI live smoke：短流式 canonical assistant cell
9. TUI live cancel：active stream 期间 `Ctrl+C` 立即取消当前 turn
10. Evaluation Harness 与 golden/JSON/JSONL 检查
11. `git diff --check`
12. `git status --short --branch`

涉及 `cargo clean`、递归清理临时目录、强制移动、清空目录、系统级安装/卸载、修改 PATH/注册表/系统配置等操作前，必须先取得用户确认。阶段结束后经确认清理编译中间产物，避免硬盘占用。

## 十一、文档同步要求

整改实现完成后，以下文档必须与实际代码状态一致：

- `D:\YunXi Agent\docs\reports\2026-07-19-100611-yunxi-agent-v2-0-2-streaming-remediation-development-report.md`
- `D:\YunXi Agent\docs\development-log.md`
- `D:\YunXi Agent\docs\extraction-status.md`
- `D:\YunXi Agent\docs\tui-presentation.md`
- `D:\YunXi Agent\README.md`
- 如版本号、发布计划或总纲发生变化，必须同步对应总纲、索引和状态文档。

## 十二、本报告生成状态

本次任务只根据审核报告生成开发者整改报告，并同步日志。未修改 Rust 源码，未运行构建、测试、清理、提交、推送或创建 Git tag。

后续开发者必须依据本报告先完成 v2.0.2 整改和重新审核；审核通过前不得宣称完成，也不得进入 v2.0.3。

署名：开发报告撰写者

## 十三、2026-07-19 10:45:34 +08:00 整改候选实施记录

当前状态：源码整改候选与本地统一验证已完成，但真实 DeepSeek live
取消验证、编译产物清理、重新审核、修复版本号/tag 确认、提交和推送均未完成。
因此本记录不宣称整改完成，也不进入 v2.0.3。

### 已实施内容

1. 在 protocol/core/runtime/presentation 全链路增加稳定 `event_id`，并将
   sequence 明确拆分为 provider reliable 与 local fallback。
2. provider parser 优先传播 Responses API 的 `sequence_number` 与事件标识；
   Chat Completions 等缺少可靠序列的来源生成显式 fallback 身份。
3. timeline store 改为按 `event_id` 幂等去重，增加 duplicate debug counter；
   只有 provider reliable sequence 才参与迟到判定。
4. final/cancel 后释放 active `StreamSession` 的内容与
   `MarkdownStreamController` buffer，只保留至多 256 条最小归档元数据；
   seen event ID 集合上限为 8192。
5. Markdown 增量提交支持完整行、段落、闭合 fence 与安全 grapheme 边界，
   保护 CJK、假名、Emoji ZWJ、组合字符和长 token 的精确性。
6. raw TUI tick 识别 active turn 中的 Ctrl+C；CLI 触发共享取消 token，
   runtime 通过 `tokio::select!` 立即丢弃 in-flight provider future，取消后
   transcript 冻结部分文本且 REPL 可继续下一次输入。

### 统一验证结果

- `cargo fmt --all`：通过。
- `cargo fmt --all -- --check`：通过。
- `cargo check --workspace`：通过。
- `cargo test --workspace`：通过；其中 TUI 76 条、runtime 45 条、provider
  44 条测试全部通过，其余 workspace 与 doc tests 通过。
- `cargo build --workspace`：通过。
- `cargo build -p yunxi-agent-cli --release --bins`：通过。
- release `yunxi --version`：`yunxi 2.0.2`。
- Evaluation Harness：31/31、golden 通过；JSON 可解析；JSONL 恰好一行；
  persona/memory/relationship/control 指标均为 1.0，违规和绕过计数均为 0。
- offline release TUI：普通输入、PageUp/PageDown 与空闲 Ctrl+C 退出通过。
- `git diff --check`：通过，仅显示 Windows 行尾转换提示。

### 真实 live 与清理复核（2026-07-19 10:53:21 +08:00）

- 经用户明确授权，在隔离空白目录启动 release TUI，provider 为
  `deepseek-v4-flash`；固定短提示返回唯一 `[assistant] LIVE_OK` canonical cell。
- 提交长流式提示，在 `[assistant*]` 持续增长时发送 Ctrl+C；TUI 显示
  cancellation requested/current turn cancelled/turn cancelled，停止 provider
  输出并保留已生成的部分数字文本，程序未退出。
- 同一 TUI 随后成功接受下一次输入并返回 `[assistant] NEXT_OK`；空闲状态
  Ctrl+C 正常退出，进程退出码为 0。
- 清理前核验 `D:\YunXi Agent\target` 为 12,319 项、约 3.50 GB，CLI
  `.yunxi` 状态为 3 项、1,248 字节。经用户授权执行 `cargo clean`，实际移除
  11,373 个文件、3.3 GiB，并递归删除 CLI 测试状态目录。最终 `target`、
  CLI `.yunxi` 与 C 盘 live 隔离目录均不存在。

### 尚未通过的硬门禁

- 重新审核尚未完成，修复版本号和新 tag 未由用户确认；现有 annotated
  `v2.0.2` tag 未移动、未删除、未覆盖。
- 未提交、未推送、未创建新 tag，未安装此整改候选。

署名：开发者

## 十四、2026-07-19 11:34:41 +08:00 hotfix 发布候选门禁记录

用户已明确确认整改发布版本为 `2.0.2-hotfix.1`，新 annotated tag 为
`v2.0.2-hotfix.1`。现有 `v2.0.2` tag object
`3f60445680211c27e1f6fe4e3b5c85a471fe5513` 及其 peeled commit
`ef7f43f97a783b3ee37d47ab034b709180d0c82e` 保持不变，不移动、不删除、不覆盖。

### 版本与文档同步

- workspace 及 18 个 YunXi crate 的 Cargo 版本统一为 `2.0.2-hotfix.1`，
  `Cargo.lock` 已同步。
- CLI/TUI 版本显示、persona context block、Evaluation Harness 与对应测试期望
  已统一为 `2.0.2-hotfix.1`。
- README、extraction status、TUI presentation 与发布门禁报告已明确：
  `v2.0.2-hotfix.1` 是新的正式复审候选；独立正式审核通过前不宣称整改完成，
  不进入 v2.0.3。

### 发布门禁验证

- `cargo fmt --all`：通过。
- `cargo fmt --all -- --check`：通过。
- `cargo check --workspace`：通过，18 个 YunXi crate 均解析为
  `2.0.2-hotfix.1`。
- `cargo test --workspace`：通过；CLI 集成 44 条、provider 44 条、runtime
  45 条、TUI 76 条及其余 workspace/doc tests 全部通过。
- `cargo build --workspace`：通过。
- `cargo build -p yunxi-agent-cli --release --bins`：通过。
- release `yunxi --version`：`yunxi 2.0.2-hotfix.1`。
- Evaluation Harness：31/31、golden 通过；JSON 可解析；JSONL 恰好一行；
  persona/memory/relationship/control 指标均为 1.0，违规与绕过计数均为 0。
- offline release TUI：普通输入、PageUp/PageDown 与空闲 Ctrl+C 退出通过，
  退出码为 0。
- 隔离 live TUI：固定短提示只产生一个 `[assistant] LIVE_OK` canonical cell；
  正常长篇 Rust 教程在 `[assistant*]` 活跃输出时 Ctrl+C 成功取消当前 turn，
  保留 partial text 并显示 cancellation requested/current turn cancelled/turn
  cancelled；同一进程随后返回 `[assistant] NEXT_OK`，空闲 Ctrl+C 退出码为 0。
- `git diff --check`：通过，仅有 Windows 行尾转换提示。

### 清理与发布边界

- 清理前 `D:\YunXi Agent\target` 有 10,503 项、文件合计
  3,133,098,236 字节。经用户授权和绝对路径校验后执行 `cargo clean`，
  移除 9,593 个文件、2.9 GiB。
- `D:\YunXi Agent\target`、`D:\YunXi Agent\crates\yunxi-agent-cli\.yunxi`
  与本次 C 盘隔离 smoke 目录均已清理，不纳入发布提交。
- 本记录随单一 hotfix 发布提交入库；随后创建 annotated
  `v2.0.2-hotfix.1` tag，并以 API key 临时认证 non-force 推送 `master` 与新 tag。
  最终提交、tag object 和远程 refs 以 Git 历史及桌面最终开发日志为准。
- 候选发布只为正式独立复审提供固定对象，不代表正式审核已经通过。

署名：开发者
