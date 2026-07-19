# YunXi Agent v2.0.2 流式状态机与消息幂等化开发报告

撰写时间：2026-07-19 07:18:26 +08:00
审核报告：`C:\Users\24763\Desktop\YunXi Agent审核报告\2026-07-18-213015-YunXi-Agent-v2.0.1-源码与TUI视觉审核报告.md`
开发目录：`D:\YunXi Agent`
项目内报告位置：`D:\YunXi Agent\docs\reports\2026-07-19-071826-yunxi-agent-v2-0-2-streaming-state-machine-idempotency-development-report.md`
桌面报告位置：`C:\Users\24763\Desktop\YunXi Agent开发报告\2026-07-19-071826-yunxi-agent-v2-0-2-streaming-state-machine-idempotency-development-report.md`

## 一、硬性约束

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

## 二、审核结论转开发目标

本次审核报告判定 YunXi Agent v2.0.1 审核通过，可以进入 v2.0.2 开发。当前发布基线如下：

- v2.0.1 为 annotated tag，指向唯一发布提交 `2b07a13ae49380b63aa966801e9a52c628107708`。
- workspace 与已安装 CLI 均为 `2.0.1`，`yunxi --version` 输出 `yunxi 2.0.1`。
- 当前 `master...origin/master` 工作树干净。
- `D:\YunXi Agent\target` 不存在。
- v2.0.1 符合新总纲“一次发布提交 + 一个不可移动 annotated tag”的要求。

v2.0.2 的唯一开发目标是：建立流式状态机与消息幂等化能力，解决 live provider 下 active streaming 行与 final assistant 行重复的问题。完成后，单轮流式回答结束时 transcript 必须只保留一个 canonical assistant cell，历史滚动位置不得被强制抢回尾部。

## 三、v2.0.2 必须解决的问题

审核报告已明确记录 live provider 复核发现：

- 普通 live 会话完成后出现同一助手回答的 active streaming 行和 final assistant 行。
- header cell 计数从用户单元后的 1 变为 3。
- 该现象与用户截图中的“重复回复”属于同类表象。

开发者必须在 v2.0.2 内解决该问题。若 v2.0.2 未以真实 live provider 证明单轮结束后只有一个 canonical assistant cell、无重复内容、历史位置不被抢回尾部，则不得宣称完成，不得进入 v2.0.3。

## 四、源码接入点

### `D:\YunXi Agent\crates\yunxi-agent-tui\src\streaming.rs`

当前 `MarkdownStreamController` 已具备 stable source / live tail 基础，但还不是完整流式状态机。v2.0.2 应在此基础上补齐：

- `TurnId` 或等价 turn 关联。
- `StreamSession` 或等价 stream 生命周期。
- delta、retry、cancel、final 的状态转移。
- 最终 final 到达时对 active cell 的 canonical 回写，而不是新增重复 assistant cell。
- 对 CJK、日文假名、Emoji ZWJ、Markdown fence、超长 token 跨 delta 的稳定处理。

禁止用“文本相等”作为主要去重策略。重复文本可能是合法输出，例如 “哈哈”“OK OK”“test test”。去重必须基于可靠的 event id、source sequence、turn id、stream session 或同等结构化来源。

### `D:\YunXi Agent\crates\yunxi-agent-tui\src\chat.rs`

`Transcript` 和 `HistoryCell` 应接入 timeline/store 的 canonical cell 更新：

- active assistant cell 与 final assistant cell 必须指向同一个 canonical id。
- final 到达后更新原 active cell，而不是 push 新 cell。
- cancel 后应冻结或标记当前 active cell，不得让后续 retry 错绑到旧 cell。
- 历史回看状态下，final 更新不能强制 follow-tail。

### `D:\YunXi Agent\crates\yunxi-agent-tui\src\timeline_store.rs`

建议新增该文件，集中管理 turn、stream、source sequence 与 cell 关系。建议职责：

- 保存 `TurnId -> StreamSession`。
- 保存 `StreamSession -> canonical TuiCellId`。
- 保存 source sequence / event id 去重记录。
- 处理 delta、retry、final、cancel 的幂等更新。
- 向 `Transcript` 输出“插入、更新、完成、取消”的明确操作。

建议模型：

```rust
pub struct StreamSession {
    pub turn_id: String,
    pub stream_id: String,
    pub canonical_cell_id: TuiCellId,
    pub last_sequence: Option<u64>,
    pub state: StreamState,
}

pub enum StreamState {
    Active,
    Finalized,
    Cancelled,
}

pub enum TimelineUpdate {
    InsertAssistant { id: TuiCellId, text: String },
    UpdateAssistant { id: TuiCellId, text: String },
    FinalizeAssistant { id: TuiCellId, text: String },
    CancelAssistant { id: TuiCellId },
}
```

### `D:\YunXi Agent\crates\yunxi-agent-tui\src\presentation.rs`

v2.0.1 已建立 `AgentEvent -> TuiEvent` 映射，v2.0.2 应继续复用它，但要补齐流式身份：

- `TuiEvent` 应携带足够的 turn/stream/source 信息，供 timeline store 进行幂等处理。
- 不能让 `presentation.rs` 和 `chat.rs` 各自生成不同 assistant id。
- Debug/details 继续保留完整细节，不影响普通 transcript 的 canonical cell。

### `D:\YunXi Agent\crates\yunxi-agent-tui\src\host.rs`

`YunxiTui` 应把流式事件交给 timeline store，而不是简单 push 新 assistant cell：

- `push_agent_event` 继续是外部入口。
- 内部应通过 presentation + timeline store 得出 insert/update/finalize/cancel。
- resize、tick、flush、details 展示不能破坏 active stream 状态。

### `D:\YunXi Agent\crates\yunxi-agent-cli\src\tui\mod.rs`

CLI TUI bridge 不能改变 runtime event 结构，也不能把重复修复放在 CLI renderer 的文本层：

- `TuiInteractiveRenderer` 继续只桥接事件。
- 去重和回写在 TUI timeline/store 层完成。
- plain CLI、`--json`、`--jsonl` 输出契约不变。

### `D:\YunXi Agent\crates\yunxi-agent-core\src\stream.rs`

`AgentRunControl` 的 event/approval/user-input/cancel 通道语义保持不变。若现有 `AgentEvent` 缺少足够的 turn/source identity，可增加向后兼容字段或在 TUI 层构建可靠 session id，但不得破坏 JSON/JSONL 结构化输出契约。

## 五、必须覆盖的流式场景

v2.0.2 至少覆盖以下场景：

- delta + final。
- delta + retry + final。
- delta + cancel。
- final 重复到达。
- 相同 delta payload 合法重复。
- CJK 跨 delta。
- 日文假名跨 delta。
- Emoji ZWJ 跨 delta。
- Markdown fence 跨 delta。
- 超长 token 跨 delta。
- live provider 单轮普通对话。
- 用户滚动回看时 final 到达。

验收重点不是“看起来少一行”，而是 transcript 数据结构中只存在一个 canonical assistant cell。

## 六、参考源码抽取建议

参考源码仍为本机 Codex CLI TUI 的 active streaming cell 与 history cell 分离思想：

- `D:\源码\codex\codex-rs\tui\src\chatwidget\rendering.rs`
- `D:\源码\codex\codex-rs\tui\src\render\renderable.rs`
- `D:\源码\codex\codex-rs\tui\src\bottom_pane\mod.rs`
- `D:\源码\codex\codex-rs\tui\src\wrapping.rs`
- `D:\源码\codex\codex-rs\tui\src\width.rs`

抽取原则：

- 只参考 active streaming cell、history cell、turn/message 回写、测试组织思想。
- 不导入或复制上游私有 UI 实现。
- 不引入上游 TUI crate、Python、Go 或 Node 运行时。
- 所有逻辑必须在 YunXi 自有 Rust crate 中复刻。

## 七、实现顺序

1. 将 workspace version 升级为 `2.0.2`，同步 CLI/TUI/README/docs 版本显示。
2. 新增或等价实现 `timeline_store.rs`，定义 turn、stream、sequence、canonical cell 的关系。
3. 调整 `streaming.rs`，把 Markdown stream controller 纳入 StreamSession 生命周期。
4. 调整 `presentation.rs`，为 assistant stream/final 事件提供可靠身份信息。
5. 调整 `chat.rs` 和 `host.rs`，把新增 assistant cell 改为 canonical cell insert/update/finalize。
6. 保持 `event_filter.rs` 的 v2.0.1 呈现边界不退化。
7. 增加 delta/final/retry/cancel/重复 final/合法重复文本/CJK/Emoji/Markdown fence 的单元测试。
8. 增加 TestBackend 或等价 TUI fixture，断言完成后 transcript 只有一个 canonical assistant cell。
9. 使用可用 live provider 做普通流式对话视觉复核，记录是否只有一个 canonical assistant cell。
10. 完成一批构建后统一验证，验证通过前不得宣称完成。
11. 阶段结束后清理编译中间产物；若涉及 `cargo clean`、递归删除或等价高风险命令，必须先得到用户确认。
12. v2.0.2 只能形成一个发布提交，随后立即创建不可移动的 annotated `v2.0.2` tag；发布后不得追加同版本 docs-only 或 hotfix 提交。

## 八、验收标准

v2.0.2 完成时至少满足以下验收项：

- 存在 `timeline_store.rs` 或等价集中状态机。
- 存在 TurnId/StreamSession/source sequence 或等价结构化身份。
- 不以文本相等作为主要去重策略。
- delta + final 后只保留一个 canonical assistant cell。
- delta + retry + final 后只保留一个 canonical assistant cell。
- final 重复到达不会新增重复 assistant cell。
- cancel 后不会把后续流式事件错绑到旧 cell。
- 合法重复文本不会被误删。
- CJK、日文假名、Emoji ZWJ、Markdown fence、超长 token 跨 delta 均有测试覆盖。
- 用户历史回看时 final 到达不会强制抢回尾部。
- plain CLI、`--json`、`--jsonl` 输出契约不变。
- live provider 普通流式对话复核通过：单轮结束后主 transcript 无重复回答。
- 发布后只有一个 v2.0.2 发布提交和一个 annotated `v2.0.2` tag，不追加同版本 docs-only 或 hotfix 提交。

## 九、统一验证要求

开发者应在完成一批构建后统一执行验证，建议顺序如下：

```powershell
cargo fmt --all
cargo fmt --all -- --check
cargo check --workspace
cargo test --workspace
cargo build --workspace
cargo build -p yunxi-agent-cli --release --bins
.\target\release\yunxi.exe --version
.\target\release\yunxi.exe eval companion --json
.\target\release\yunxi.exe eval companion --jsonl
git diff --check
git status --short
git diff --stat
```

补充检查：

- 检查 workspace version 是否为 `2.0.2`。
- 检查 release 输出是否为 `yunxi 2.0.2`。
- 检查 live provider 下普通流式对话是否只有一个 canonical assistant cell。
- 检查历史回看状态下 final 到达是否不抢回尾部。
- 检查 JSON/JSONL 输出是否未受 TUI 幂等化影响。
- 检查旧 tag 未删除、未移动、未重写。
- 检查 v2.0.2 是否只有一个发布提交和一个 annotated tag。

注意：`cargo clean`、递归删除 `target`、强制移动目录、清空目录等清理动作必须遵守用户 shell 安全要求；需要确认时先征得用户同意。

## 十、文档同步要求

v2.0.2 开发过程中，以下文档需要与代码状态保持一致：

- `D:\YunXi Agent\docs\reports\2026-07-19-071826-yunxi-agent-v2-0-2-streaming-state-machine-idempotency-development-report.md`
- `D:\YunXi Agent\docs\development-log.md`
- `D:\YunXi Agent\docs\extraction-status.md`
- `D:\YunXi Agent\README.md`
- 如新增 TUI streaming/timeline 设计说明，应放入 `D:\YunXi Agent\docs`，并与实际代码保持一致。

## 十一、本报告生成状态

- 本轮工作仅撰写开发报告，没有修改 Rust 源码。
- 本轮未运行 `cargo fmt`、`cargo check`、`cargo test` 或 `cargo build`。
- 本轮未执行编译产物清理。
- 本轮未提交、未推送、未创建 Git tag。
- 后续开发者必须以实际代码实现、统一验证结果、清理状态、提交状态、推送状态和新 tag 更新对应文档。

署名：开发报告撰写者

## 十二、实际开发、验证与发布准备记录

时间戳：`2026-07-19 09:18:11 +08:00`

### 12.1 基线与参考范围

- 开发基线：`master` 与 `origin/master` 均位于 `2b07a13ae49380b63aa966801e9a52c628107708`。
- 开发前 `v2.0.1` 为 annotated tag，tag object 为 `0cdcfced0564f12ef118ad342be4604ec0e158b3`，解析到上述基线提交；旧 tag 未删除、未移动、未重写。
- 开发前 `target` 不存在；保留了报告生成阶段已有的 `docs/development-log.md` 修改与本报告文件。
- 使用仓库 `.codegraph` 索引定位 core、runtime、TUI、CLI 与测试边界，并完成索引同步核验。
- 只读参考了本机 Codex TUI 的 `rendering.rs`、`renderable.rs`、`bottom_pane/mod.rs`、`wrapping.rs` 与 `width.rs`；未复制上游私有 UI 实现，未引入上游 TUI crate、Python、Go 或 Node 运行时。

### 12.2 实际实现

- workspace、CLI、TUI、persona、evaluation harness 及相关文档版本统一升级为 `2.0.2`。
- 在 `yunxi-agent-core` 中为消息事件增加不参与序列化的流式元数据，公开 `AgentMessageStream` 与 `AgentMessageStreamPhase`；JSON/JSONL 既有字段形状保持不变，并增加序列化回归测试。
- 在 `yunxi-agent-runtime` 中增加持久的协议流事件映射器，以 provider thread/turn/item/source sequence/phase 建立稳定流身份；fallback 身份包含 turn 与 response generation，legacy final 复用同一流身份。
- provider 响应收集优先使用 completed snapshot，避免把 started/delta/final 文本重复拼接到最终响应。
- 新增 `D:\YunXi Agent\crates\yunxi-agent-tui\src\timeline_store.rs`，集中实现 `TurnId`、`StreamSessionId`、`SourceSequence`、`StreamSession`、canonical cell 与流状态机。
- 流状态覆盖 Active、Retrying、Finalized、Cancelled、Superseded；Started、Delta、Retry、Final、Finish、Cancel 按结构化身份与 source sequence 幂等更新，不以文本相等作为主要去重依据。
- `MarkdownStreamController` 纳入 `StreamSession` 生命周期；final 原位更新同一 canonical assistant cell，重复 final 被忽略，合法的相同文本 delta 仍被保留。
- cancel 后旧流被冻结，迟到 delta 不会写入旧 cell；后续新流获得新的 canonical cell。tool/debug 等非消息事件不再拆分当前流，终端边界会完成所有 active session。
- transcript 只有在实际数据变更时才触发 viewport 处理；用户历史回看期间保留 `view_start`，并显示 `new output below`，final 不抢回尾部。
- `TuiPresentation` 仅携带流身份并负责事件呈现映射，不再拥有 Markdown 流控制器；CLI TUI bridge 保持仅转发事件。
- README、`docs/extraction-status.md`、`docs/persona-memory.md` 与 `docs/tui-presentation.md` 已同步实现边界、状态机、兼容性和验证说明。

### 12.3 场景覆盖与统一验证

- 自动化场景覆盖 delta + final、delta + retry + final、delta + cancel/迟到事件、重复 final、合法重复 payload，以及 CJK、日文假名、Emoji ZWJ、Markdown fence、4096 字符长 token 跨 delta。
- provider-shaped TUI 集成测试断言单轮完成后只有一个 canonical assistant cell；历史回看回归测试断言 final 到达后 `view_start` 不变且保留新输出提示。
- `cargo fmt --all`：通过。
- `cargo fmt --all -- --check`：通过。
- `cargo check --workspace`：通过。
- `cargo test --workspace`：通过。首次统一测试发现 storage 与 detached Codex 测试 fixture 缺少新增向后兼容字段，补齐 fixture 后重新执行 fmt/check/test，全部通过。
- `cargo build --workspace`：通过。
- `cargo build -p yunxi-agent-cli --release --bins`：通过。
- release 版本冒烟：输出 `yunxi 2.0.2`。
- Evaluation Harness：31/31 场景通过，golden 判定通过；JSON 输出完整，JSONL 恰好一行，harness version 为 `2.0.2`。
- `git diff --check`：通过，仅出现 Windows 工作副本 LF/CRLF 转换提示，无空白错误。

### 12.4 隔离 live provider 复核

- 在 `%TEMP%\yunxi-v202-live-smoke` 创建独立 `YUNXI_HOME` 与空工作目录，关闭 memory，未向 provider 暴露仓库源码或工作区上下文。
- 使用 `YunXi Agent v2.0.2 | deepseek live | model=deepseek-v4-flash`，固定提示为“请只回复‘流式幂等复核通过’，不要使用工具。”。
- 输出从同一 active assistant row 原位更新为“流式幂等复核通过”；等待 15 秒后 transcript 仍为 1 个 user cell 与恰好 1 个 canonical assistant cell，没有第三个 cell 或重复回答。
- 历史回看不抢尾部由自动化 TUI 回归测试验证通过。

### 12.5 清理与发布约束

- 已在用户明确授权后核对目标绝对路径，执行 `cargo clean`，清理 `D:\YunXi Agent\target`：移除 15,211 个文件，共 4.2 GiB。
- 已清理测试状态目录 `D:\YunXi Agent\crates\yunxi-agent-cli\.yunxi` 与隔离 live 临时目录 `%TEMP%\yunxi-v202-live-smoke`。
- 清理后上述目录均不存在；不会把编译中间产物、测试状态或 API key 写入仓库。
- 本次全部代码、测试、版本与文档变更将形成唯一一个 v2.0.2 发布提交，并立即创建 annotated `v2.0.2` tag；随后使用指定 GitHub API key 通过临时认证头 non-force 推送 `master` 与新 tag。
- 发布后不追加同版本 docs-only 或 hotfix 提交；最终远程 `master`、新 tag object、peeled commit 与旧 tag 状态以 Git 历史和远程 refs 为准。

署名：开发者
