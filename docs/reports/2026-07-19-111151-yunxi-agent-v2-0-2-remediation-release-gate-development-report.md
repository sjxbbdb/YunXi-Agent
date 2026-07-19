# YunXi Agent v2.0.2 整改候选发布门禁开发报告

- 撰写时间：2026-07-19 11:11:51 +08:00
- 审核报告：`C:\Users\24763\Desktop\YunXi Agent审核报告\2026-07-19-110509-YunXi-Agent-v2.0.2-整改候选源码预审报告.md`
- 整改依据：`D:\YunXi Agent\docs\reports\2026-07-19-100611-yunxi-agent-v2-0-2-streaming-remediation-development-report.md`
- 开发目录：`D:\YunXi Agent`
- 项目内报告：`D:\YunXi Agent\docs\reports\2026-07-19-111151-yunxi-agent-v2-0-2-remediation-release-gate-development-report.md`
- 桌面报告：`C:\Users\24763\Desktop\YunXi Agent开发报告\2026-07-19-111151-yunxi-agent-v2-0-2-remediation-release-gate-development-report.md`
- 报告类型：整改候选发布门禁与正式复审准备开发报告
- 审核结论转化：v2.0.2 整改候选源码预审通过，但不构成正式版本审核通过，不允许进入 v2.0.3 开发。

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

本阶段不是 v2.0.3 新功能开发，而是把 v2.0.2 失败后的整改候选源码推进到可正式复审、可发布的状态。预审报告确认静态源码层面已经覆盖四项缺口，但仍缺少正式发布所需的版本号确认、发布提交、新 annotated tag、non-force 推送结果和审核者独立 live/TUI 复核。

开发者的下一阶段目标如下：

1. 保持当前整改候选已经通过预审的源码方向，不回退稳定 `event_id`、可靠 sequence、Markdown/Unicode 提交边界、session 回收和 active-stream `Ctrl+C` 取消链路。
2. 在用户确认修复版本号和 tag 方案前，不得创建发布提交、不得打 tag、不得推送、不得安装候选版本。
3. 用户确认版本号后，一次性同步 `Cargo.toml`、`Cargo.lock`、README、状态文档、开发报告、测试说明和发布计划。
4. 统一验证通过后，以一个发布提交承载该修复版本，并创建新的不可移动 annotated tag。
5. 完成 non-force 推送后，再通知审核者进入正式源码、自动化证据、真实 live provider 和 TUI 视觉/交互复审。

## 三、当前预审结论与边界

预审报告结论为：整改候选源码预审通过。通过范围只覆盖静态源码层面对 v2.0.2 审核失败时四项硬性缺口的整改情况：

- 稳定事件身份与可靠序号语义。
- Markdown/Unicode 增量提交边界。
- 完成 session 回收。
- active-stream `Ctrl+C` 取消链路。

必须强调以下边界：

- 该结论不等于 v2.0.2 正式审核通过。
- 该结论不允许进入 v2.0.3 开发。
- 当前 HEAD 仍为已发布的 `ef7f43f`。
- 既有 annotated `v2.0.2` tag 指向旧发布提交，不得移动、删除或覆盖。
- 当前候选版本号仍为 `2.0.2`，不能在没有用户确认的情况下发布为同名版本。
- 当前工作树仍有未提交整改候选源码改动，不是干净发布态。

## 四、已预审通过的整改能力

### 1. 稳定 event_id 与 sequence 语义

预审确认以下方向已经覆盖 v2.0.2 整改要求：

- `crates\yunxi-agent-core\src\event.rs` 新增 `AgentMessageSequence::{ProviderReliable, LocalFallback}`。
- `AgentMessageStream` 携带稳定 `event_id`。
- `crates\yunxi-agent-protocol\src\lib.rs` 通过 `StreamEventMetadata { event_id, sequence }` 传递事件元数据。
- `crates\yunxi-agent-provider\src\lib.rs` 优先采用 provider `sequence_number`，缺失时生成显式 `LocalFallback`。
- `crates\yunxi-agent-runtime\src\lib.rs` 完整映射 reliable/fallback sequence。
- `crates\yunxi-agent-tui\src\timeline_store.rs` 以 `event_id` 去重并记录 duplicate counter，仅 `ProviderReliable` 可拒绝迟到事件。

后续不得重新把本地抵达顺序当作 provider 可靠顺序，也不得移除 duplicate counter。

### 2. canonical cell 与 Markdown/Unicode 提交边界

预审确认：

- `timeline_store.rs` 能将 delta、retry、final、finish、cancel 聚合为一个 canonical assistant cell。
- final 原位冻结，不追加第二份最终回答。
- `streaming.rs` 已从仅换行提交改为完整行、段落、闭合 Markdown fence 与安全 grapheme 边界。
- `unicode-segmentation` 与相关测试覆盖 CJK、日文假名、Emoji ZWJ、组合字符、partial fence 与长 token。

后续验证必须继续覆盖这些路径，不能只看最终文本是否拼接正确。

### 3. final/cancel 后 session 回收

预审确认：

- `TimelineStore::finalize`、`finish`、`finish_all` 会从 active `sessions` 移除完整 `StreamSession`。
- 完成后释放 content 和流式 buffer。
- 归档只保存最小元数据，最多 256 条。
- seen event ID 集最多 8192 条。
- 测试覆盖 final 后 active map 清空、cancel 后 late delta 不可重新绑定、长会话 retained content 为零。

后续不得为了调试便利长期保留完整 assistant 内容或无限增长的 event/session map。

### 4. active live stream Ctrl+C

预审确认静态链路已打通：

- `crates\yunxi-agent-tui\src\host.rs` 在 raw TUI tick 中识别 `Ctrl+C` 并返回 `CancelCurrentTurn`。
- `crates\yunxi-agent-cli\src\interactive.rs` 以 33ms tick 调用共享 `AgentRunControl::cancel()`。
- `crates\yunxi-agent-runtime\src\lib.rs` 通过 `tokio::select!` 让 in-flight `stream_with_sink` 与取消通知竞争。
- `crates\yunxi-agent-tui\src\app.rs` 取消后冻结 partial assistant cell，随后 REPL 可继续接受下一次输入。

该链路仍必须在正式审核中由审核者用真实 live provider 独立复核。开发者自己的 live 记录只能作为验证证据，不能替代正式审核。

## 五、发布前硬门禁

正式发布前必须满足以下门禁：

1. 用户明确确认修复版本号与新 tag 方案。
2. 旧 `v2.0.2` annotated tag 不移动、不删除、不覆盖。
3. 工作树整理为单一发布提交所需的实际改动，不混入无关改动。
4. 版本号、README、状态文档、开发报告、发布计划和测试说明与实际代码一致。
5. 统一验证全部通过，并记录具体命令、结果和时间戳。
6. 经用户确认后清理编译中间产物。
7. 创建新的 annotated tag，并 non-force 推送 master 与 tag。
8. 推送后用远程 refs 核验发布提交和 tag object。
9. 通知审核者时必须说明：候选已发布、版本号、提交、tag、推送状态、验证结果和清理状态。

## 六、正式复审必须覆盖的视觉与交互场景

正式审核不得只做 offline fixture。至少要覆盖：

1. live 短流只保留一个 canonical assistant cell。
2. `[assistant*]` 输出中 `Ctrl+C` 立即停止当前 turn。
3. 取消后 partial text 冻结，程序不退出。
4. 取消后可以继续输入并获得下一次回答。
5. 滚动、resize、history anchor、approval/error 不出现视觉重叠或尾部抢焦点。
6. 普通视图不暴露 thinking、memory/context、协议 JSON、工具参数或内部错误栈。

## 七、源码参考与 Rust 化边界

可继续参考 Codex TUI 的 active streaming cell、history cell、状态机与渲染分层、wrapping/width 独立处理思想：

- `D:\源码\codex\codex-rs\tui\src\chatwidget\rendering.rs`
- `D:\源码\codex\codex-rs\tui\src\render\renderable.rs`
- `D:\源码\codex\codex-rs\tui\src\wrapping.rs`
- `D:\源码\codex\codex-rs\tui\src\width.rs`

参考边界不变：

- 只抽取架构和算法思想。
- 不引入上游 Codex runtime 依赖。
- 不让默认 YunXi 运行路径依赖 `vendor/codex-rs`、`codex-*` crate 或 `yunxi-agent-codex`。
- 非 Rust 参考源码只参考逻辑，必须用 YunXi 自身 Rust 模块复刻。

## 八、建议执行顺序

1. 等待用户确认修复版本号与新 tag 命名。
2. 同步 workspace 版本号、锁文件、README、状态文档和发布计划。
3. 复核当前 27 项未提交改动，剔除与整改无关的内容。
4. 统一执行 fmt、check、test、build、release build、evaluation harness、TUI offline/live 验证。
5. 根据验证结果修正文档和日志，不得在验证通过前宣称完成。
6. 经用户确认后清理 `target` 与测试/临时状态目录。
7. 创建单一发布提交和新的 annotated tag。
8. non-force 推送，并核验远程 master 与 tag。
9. 追加最终日志，通知正式复审。

## 九、统一验证要求

候选发布前至少执行：

1. `cargo fmt --all`
2. `cargo fmt --all -- --check`
3. `cargo check --workspace`
4. `cargo test --workspace`
5. `cargo build --workspace`
6. release build 与 `yunxi --version`
7. Evaluation Harness、golden、JSON、JSONL 检查
8. TUI offline smoke
9. TUI live 短流 canonical cell 复核
10. TUI live active-stream `Ctrl+C` 取消复核
11. `git diff --check`
12. `git status --short --branch`
13. 远程 refs 核验

涉及 `cargo clean`、递归删除临时目录、强制移动、清空目录、系统级安装/卸载、修改 PATH/注册表/系统配置等操作前，必须先取得用户确认。

## 十、文档同步要求

下列文档必须与实际代码、版本号和发布状态保持一致：

- `D:\YunXi Agent\README.md`
- `D:\YunXi Agent\docs\extraction-status.md`
- `D:\YunXi Agent\docs\tui-presentation.md`
- `D:\YunXi Agent\docs\development-log.md`
- `D:\YunXi Agent\docs\reports\2026-07-19-100611-yunxi-agent-v2-0-2-streaming-remediation-development-report.md`
- `D:\YunXi Agent\docs\reports\2026-07-19-111151-yunxi-agent-v2-0-2-remediation-release-gate-development-report.md`
- 如用户确认新的修复版本号，所有版本显示、发布计划、索引和状态文档必须同步更新。

## 十一、本报告生成状态

本次任务只根据整改候选源码预审报告生成开发者发布门禁报告，并同步日志。未修改 Rust 源码，未运行构建、测试、清理、提交、推送或创建 Git tag。

后续开发者必须先等待用户确认修复版本号与 tag 方案，再执行发布前统一验证、清理、提交、tag、推送和正式复审准备。正式审核通过前不得宣称完成，也不得进入 v2.0.3。

署名：开发报告撰写者

## 十二、2026-07-19 11:34:41 +08:00 发布门禁执行记录

### 用户确认与不可变基线

- 用户确认版本：`2.0.2-hotfix.1`。
- 用户确认新 annotated tag：`v2.0.2-hotfix.1`。
- 旧 `v2.0.2` tag object 为
  `3f60445680211c27e1f6fe4e3b5c85a471fe5513`，peeled commit 为
  `ef7f43f97a783b3ee37d47ab034b709180d0c82e`；本次发布不移动、不删除、不覆盖旧 tag。
- 本阶段仍是 v2.0.2 审核失败后的修复候选发布，不进入 v2.0.3。

### 实际执行

1. 将 workspace、锁文件、CLI/TUI、persona context、Evaluation Harness、
   自动化测试、README 和状态文档统一为 `2.0.2-hotfix.1`。
2. 完整运行 fmt、check、workspace tests、workspace/release build、版本冒烟、
   Evaluation Harness、JSON/JSONL、offline TUI、真实 DeepSeek live 短流和
   active-stream Ctrl+C 取消门禁。
3. live 短流得到唯一 `[assistant] LIVE_OK`；正常长篇技术内容在
   `[assistant*]` 活跃时被 Ctrl+C 取消并保留 partial，随后同一进程成功得到
   `[assistant] NEXT_OK`，空闲退出码为 0。
4. `git diff --check` 通过；只有 Windows 行尾提示，没有 whitespace error。
5. 经用户授权并校验绝对路径后执行清理：`cargo clean` 移除 9,593 个文件、
   2.9 GiB；`target`、CLI `.yunxi` 与本次隔离 smoke 目录最终均不存在。

### 验证结果

- 全工作区测试全部通过；关键计数为 CLI 集成 44、provider 44、runtime 45、
  TUI 76。
- release 版本输出为 `yunxi 2.0.2-hotfix.1`。
- Evaluation Harness 为 31/31，golden 通过，JSON 正常，JSONL 恰好一行；
  所有质量率为 1.0，违规与审批绕过计数为 0。
- 新 tag 在发布前不存在，避免覆盖；旧 tag refs 已再次核验并保持不变。

### 发布与复审状态说明

本记录与单一 hotfix 发布提交一同入库；提交后立即创建 annotated
`v2.0.2-hotfix.1` tag，并通过 API key 临时认证 non-force 推送 `master` 和新 tag。
最终本地/远程提交与 tag object 由 Git refs 和桌面最终开发日志记录，避免在同一
发布提交中写入自引用哈希。推送完成只表示整改候选已固定并可供审核，正式审核者
仍需对已发布对象独立执行 live/TUI 复核；正式审核通过前不得宣称整改完成。

署名：开发者
