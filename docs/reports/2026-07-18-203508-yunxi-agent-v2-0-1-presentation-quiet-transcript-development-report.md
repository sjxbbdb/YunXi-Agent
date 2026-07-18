# YunXi Agent v2.0.1 呈现边界与安静对话基线开发报告

撰写时间：2026-07-18 20:35:08 +08:00
审核报告：`C:\Users\24763\Desktop\YunXi Agent审核报告\2026-07-18-203016-YunXi-Agent-v2.0.0-基线源码审核报告.md`
开发目录：`D:\YunXi Agent`
项目内报告位置：`D:\YunXi Agent\docs\reports\2026-07-18-203508-yunxi-agent-v2-0-1-presentation-quiet-transcript-development-report.md`
桌面报告位置：`C:\Users\24763\Desktop\YunXi Agent开发报告\2026-07-18-203508-yunxi-agent-v2-0-1-presentation-quiet-transcript-development-report.md`

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

本次基线审核确认 YunXi Agent v2.0.0 审核通过，可以进入 `v2.0.1` 开发。当前基线状态如下：

- Workspace 版本为 `2.0.0`。
- 本地 annotated tag `v2.0.0` 存在，指向实现提交 `4cf890b86889e72c47f0e56881152053c47d76ae`。
- 当前 HEAD 为 `6143237`，位于 `v2.0.0` tag 之后。
- 当前 `master...origin/master` 工作树干净。
- tag 后差异仅涉及 `docs/development-log.md` 和 v2.0.0 开发报告，无源码改动。
- `D:\YunXi Agent\target` 不存在。

v2.0.1 的唯一开发目标是：建立 TUI 呈现边界与安静对话基线，开始对 TUI、事件呈现、流式输出和 details/debug 分层进行重构。该版本不做新的 agent 能力扩展，重点是让用户默认看到干净、稳定、可回看的对话 transcript，同时把原始事件、底层异常、thinking、provider wire、完整 stdout、`arguments_json` 等内容收敛到 debug/details。

## 三、v2.0.1 必须明确的重构方向

开发者必须把接下来的工作重心放在 TUI 与流式输出重构上：

- TUI 不再直接猜测 `AgentEvent` 的语义。
- 新增 `crates/yunxi-agent-tui/src/presentation.rs`，作为 `AgentEvent -> TuiEvent` 的唯一映射层。
- `event_filter.rs` 只保留纯过滤规则或委托给 presentation 层，不能继续承担混杂的显示语义。
- `render.rs` 只负责渲染已经分类好的 cell，不再自行判断哪些 runtime event 应出现在普通 transcript。
- `streaming.rs` 需要继续区分 stable source 与 live tail，并为后续 v2.0.2 的流式幂等回写做好 cell id 与 message id 基础。
- 普通 transcript 默认安静，只显示用户消息、助手消息、明确非敏感 progress event 的短摘要、工具状态摘要和需要用户决定的审批。
- debug/details 承载完整事件细节，不能污染默认对话流。

## 四、源码接入点

### `D:\YunXi Agent\crates\yunxi-agent-tui\src\presentation.rs`

本版本应新增该文件。建议职责：

- 定义 `TuiEvent`、`TuiCellKind`、`TuiCellId`、`PresentationDetail`。
- 将 `AgentEvent` 映射为普通 transcript cell、工具状态摘要、审批提示、debug detail 或 hidden event。
- 对每个可呈现 cell 生成稳定 id，为 v2.0.2 turn/message 回写与 details 索引做准备。
- 明确敏感字段默认不进入普通 transcript。

建议模型：

```rust
pub enum TuiCellKind {
    UserMessage,
    AssistantMessage,
    ProgressSummary,
    ToolStatusSummary,
    ApprovalRequest,
    Notice,
    ErrorSummary,
    DebugDetail,
}

pub struct TuiEvent {
    pub id: String,
    pub kind: TuiCellKind,
    pub visible_text: String,
    pub detail: Option<PresentationDetail>,
}
```

### `D:\YunXi Agent\crates\yunxi-agent-tui\src\event_filter.rs`

调整方向：

- 不再作为“显示逻辑中心”。
- 只保留纯过滤规则，例如 debug disabled 时隐藏 debug-only event。
- 复杂语义必须迁移到 `presentation.rs`。

### `D:\YunXi Agent\crates\yunxi-agent-tui\src\chat.rs`

调整方向：

- `HistoryCell` 和 `Transcript` 应使用 presentation 层产出的稳定 id。
- 支持普通 cell 与 details 索引关联。
- 不允许把底层完整 stdout、provider wire、hidden prompt 或 `arguments_json` 直接加入默认 transcript。

### `D:\YunXi Agent\crates\yunxi-agent-tui\src\streaming.rs`

调整方向：

- 保留 `MarkdownStreamController` 的 stable source / live tail 思路。
- 增加对重复 delta、最终 message 回写、newline boundary 和 live tail 清理的测试准备。
- v2.0.1 至少要让流式输出进入 presentation 管线，避免 renderer 与 streaming controller 双方各自决定显示内容。

### `D:\YunXi Agent\crates\yunxi-agent-tui\src\render.rs`

调整方向：

- `render.rs` 只渲染已分类的 `TuiEvent` / `HistoryCell`。
- header、transcript、scrollbar、bottom pane 的布局继续保留。
- 审批仍走 bottom pane，不得混入普通助手消息。
- 默认 transcript 应以安静对话为目标，减少内部进度、memory/context 判断和底层异常噪声。

### `D:\YunXi Agent\crates\yunxi-agent-tui\src\host.rs`

调整方向：

- `YunxiTui` 对外提供 `push_tui_event` 或等价入口。
- `push_agent_event` 应被收窄为调用 presentation 层，而不是直接写 transcript。
- tick、flush、resize、follow-tail、details 展示保持可控。

### `D:\YunXi Agent\crates\yunxi-agent-cli\src\tui\mod.rs`

调整方向：

- `TuiInteractiveRenderer` 继续桥接 `InteractiveRenderer` 与 `YunxiTui`。
- `event(&AgentEvent, RenderState)` 中不要再直接决定消息可见性；应委托 TUI presentation。
- plain CLI、`--json`、`--jsonl` 与审批策略不变。

### `D:\YunXi Agent\crates\yunxi-agent-core\src\stream.rs`

调整方向：

- 不改变 runtime event 的真实顺序和结构化输出契约。
- `AgentRunControl`、approval channel、user input channel、cancel token 的语义保持不变。
- v2.0.1 的过滤只发生在 TUI presentation 层，不得影响 JSON/JSONL 或 runtime event stream。

## 五、普通 transcript 与 details 边界

普通 transcript 默认允许：

- 用户消息。
- 助手最终消息和安全的流式助手消息。
- 明确非敏感 progress event 的短摘要。
- 工具状态摘要，例如“工具请求待审批”“工具执行完成/失败摘要”。
- 审批和用户输入请求的可操作提示。

普通 transcript 默认禁止：

- 原始 thinking。
- memory/context 内部判断。
- hidden prompt。
- provider wire。
- `arguments_json`。
- 完整 stdout/stderr。
- 底层异常堆栈。
- 未脱敏的工具参数。

被禁止内容可以进入 debug/details，但必须满足：

- 默认不展示。
- 有稳定 details id。
- 不改变 runtime 原始事件。
- 不影响 plain CLI、JSON、JSONL 输出契约。

## 六、参考源码抽取建议

本版本可继续参考本机 Codex CLI TUI 架构：

- `D:\源码\codex\codex-rs\tui\src\render\renderable.rs`
- `D:\源码\codex\codex-rs\tui\src\chatwidget\rendering.rs`
- `D:\源码\codex\codex-rs\tui\src\bottom_pane\mod.rs`
- `D:\源码\codex\codex-rs\tui\src\bottom_pane\approval_overlay.rs`
- `D:\源码\codex\codex-rs\tui\src\wrapping.rs`
- `D:\源码\codex\codex-rs\tui\src\width.rs`

抽取原则：

- 只参考架构、边界和测试思想。
- 不引入上游 TUI crate。
- 不引入 Python、Go、Node 运行时。
- 不把上游 Codex CLI 源码放入 YunXi 默认运行路径。
- 所有逻辑必须在 YunXi 自有 Rust crate 中复刻。

## 七、实现顺序

1. 将 workspace version 升级为 `2.0.1`，同步 CLI/TUI/README/docs 版本显示。
2. 新增 `presentation.rs`，定义 `AgentEvent -> TuiEvent` 映射模型和稳定 cell id。
3. 迁移 `event_filter.rs` 中的显示语义，保留纯过滤或委托逻辑。
4. 调整 `chat.rs`、`host.rs`、`render.rs`，使 transcript 只消费 presentation 结果。
5. 调整 `streaming.rs` 与 TUI 写入路径，为后续流式幂等和 message 回写建立基础。
6. 确认 plain CLI、`--json`、`--jsonl`、approval/user input/cancel 行为不变。
7. 增加 TUI presentation 单元测试和渲染回归测试。
8. 同步文档、开发报告、状态文档、索引和日志。
9. 完成一批构建后统一验证，验证通过前不得宣称完成。
10. 阶段结束后清理编译中间产物；若涉及 `cargo clean`、递归删除或等价高风险命令，必须先得到用户确认。
11. `v2.0.1` 只能形成一个发布提交，随后立即创建不可移动的 annotated `v2.0.1` tag；发布后不得追加同版本 docs-only 或 hotfix 提交。

## 八、验收标准

v2.0.1 完成时至少满足以下验收项：

- 存在 `crates/yunxi-agent-tui/src/presentation.rs`。
- `AgentEvent -> TuiEvent` 映射有单一入口。
- renderer 不再自行猜测 runtime event 语义。
- 默认 transcript 不显示 thinking、memory/context 内部判断、hidden prompt、provider wire、`arguments_json`、完整 stdout/stderr 或底层异常堆栈。
- 可呈现 cell 具备稳定 id。
- debug/details 可查看被收敛的底层细节。
- 流式输出进入 presentation 管线，stable source 与 live tail 边界不退化。
- plain CLI、`--json`、`--jsonl` 输出契约不变。
- approval、user input、cancel 通道行为不变。
- TUI presentation、streaming、rendering 有覆盖测试。
- 发布后只有一个 v2.0.1 发布提交和一个 annotated `v2.0.1` tag，不追加同版本 docs-only 或 hotfix 提交。

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

- 检查 workspace version 是否为 `2.0.1`。
- 检查 release 输出是否为 `yunxi 2.0.1`。
- 检查 TUI 默认 transcript 是否安静。
- 检查 details/debug 是否能承载完整底层细节。
- 检查 JSON/JSONL 输出是否未受 TUI filtering 影响。
- 检查旧 tag 未删除、未移动、未重写。
- 检查 `v2.0.1` 是否只有一个发布提交和一个 annotated tag。

注意：`cargo clean`、递归删除 `target`、强制移动目录、清空目录等清理动作必须遵守用户 shell 安全要求；需要确认时先征得用户同意。

## 十、文档同步要求

v2.0.1 开发过程中，以下文档需要与代码状态保持一致：

- `D:\YunXi Agent\docs\reports\2026-07-18-203508-yunxi-agent-v2-0-1-presentation-quiet-transcript-development-report.md`
- `D:\YunXi Agent\docs\development-log.md`
- `D:\YunXi Agent\docs\extraction-status.md`
- `D:\YunXi Agent\README.md`
- 如新增 TUI/presentation 设计说明，应放入 `D:\YunXi Agent\docs`，并与实际代码保持一致。

## 十一、本报告生成状态

- 本轮工作仅撰写开发报告，没有修改 Rust 源码。
- 本轮未运行 `cargo fmt`、`cargo check`、`cargo test` 或 `cargo build`。
- 本轮未执行编译产物清理。
- 本轮未提交、未推送、未创建 Git tag。
- 后续开发者必须以实际代码实现、统一验证结果、清理状态、提交状态、推送状态和新 tag 更新对应文档。

署名：开发报告撰写者

## 十二、实际开发结果

开发完成时间：2026-07-18 21:09:56 +08:00

本次已按本报告完成 v2.0.1 呈现边界与安静 transcript 开发，未扩展新的
agent 能力，未改变 core/runtime 的原始事件结构与顺序，也未引入上游 TUI
crate 或 Python、Go、Node 等非 Rust 运行时。

实际实现如下：

1. 新增 `crates/yunxi-agent-tui/src/presentation.rs`，集中定义
   `TuiCellId`、`TuiCellKind`、`PresentationDetail`、`TuiStreamState`、
   `TuiEvent` 和唯一的 `AgentEvent -> TuiEvent` 语义映射。
2. 将 `event_filter.rs` 收窄为纯 visibility gate，仅依据已分类事件的
   transcript/debug-only/hidden 属性和 debug 开关决定是否显示。
3. `Transcript` 与 `HistoryCell` 改为按稳定 cell ID 更新；详情进入带稳定
   detail ID 的 `DebugBuffer`，相同工具生命周期不会制造协议噪声 cell。
4. 默认 transcript 只保留用户、助手、安全进度、工具状态/输出计数、审批、
   notice 和脱敏错误摘要。raw thinking、memory/context、hidden prompt、provider
   wire、`arguments_json`、完整 stdout/stderr、stack 和未脱敏参数仅进入
   debug/details。
5. TUI assistant delta 统一进入 presentation 内的
   `MarkdownStreamController`；连续 delta 共用一个 cell ID，stable source 与
   live tail 独立，非 message 边界结束当前 stream 并为下一段生成新 ID。
6. `YunxiTui` 新增 `push_tui_event`，`push_agent_event` 仅负责委托
   presentation；CLI TUI bridge 删除 assistant 可见性旁路。plain CLI、JSON、
   JSONL、approval、user input、cancel、tick、flush 和 bottom pane 契约未改。
7. 新增 presentation、quiet transcript、redacted details、stable detail update、
   streaming boundary、重复 payload、stream finalize 和 renderer 回归测试。
8. workspace、CLI、TUI、persona context、evaluation harness 和当前文档版本统一
   升级为 `2.0.1`；新增 `docs/tui-presentation.md`。

## 十三、修改文件与路径

- Workspace/版本：`D:\YunXi Agent\Cargo.toml`、`D:\YunXi Agent\Cargo.lock`
- 总览文档：`D:\YunXi Agent\README.md`
- CLI：`D:\YunXi Agent\crates\yunxi-agent-cli\src\main.rs`、
  `src\render.rs`、`src\tui\mod.rs`、`tests\cli_tests.rs`
- Evaluation：`D:\YunXi Agent\crates\yunxi-agent-eval\src\lib.rs`、
  `D:\YunXi Agent\evals\companion\README.md`
- Persona/runtime 版本回归：
  `D:\YunXi Agent\crates\yunxi-agent-persona\src\compiler.rs`、`src\profile.rs`、
  `tests\evaluation_regression_tests.rs`、`tests\persona_tests.rs`、
  `D:\YunXi Agent\crates\yunxi-agent-runtime\tests\general_companion_tests.rs`
- TUI presentation：`D:\YunXi Agent\crates\yunxi-agent-tui\src\presentation.rs`、
  `app.rs`、`chat.rs`、`debug.rs`、`event_filter.rs`、`host.rs`、`lib.rs`、
  `output_summary.rs`、`render.rs`、`streaming.rs`、`timeline.rs`、
  `transcript_layout.rs`
- 状态与设计文档：`D:\YunXi Agent\docs\extraction-status.md`、
  `docs\persona-memory.md`、`docs\tui-presentation.md`
- 报告与日志：本报告及 `D:\YunXi Agent\docs\development-log.md`

## 十四、统一验证与清理结果

以下命令全部成功：

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
```

验证证据：

- release 二进制返回 `yunxi 2.0.1`。
- workspace 全部测试通过；TUI 60 项测试通过，CLI integration 44 项和 JSONL
  10 项测试通过。
- Evaluation Harness 版本为 `2.0.1`，31/31 场景通过，
  `golden_passed=true`，persona consistency、memory precision/recall、
  relationship continuity 和 control regression 均为 `1.0`；memory false
  positive/missed/forbidden、proactive violation 和 tool approval bypass 均为
  `0`。
- JSON 输出可解析；JSONL 恰好一行且可解析，未受 TUI filtering 影响。
- TUI 回归确认默认 transcript 不泄漏 raw thinking、memory/context query、
  tool arguments、完整 stdout、provider wire 或 stack；显式 details 可读取脱敏
  底层内容。
- 经用户清理授权及绝对路径核验，`cargo clean` 删除 9,556 个文件、2.9 GiB；
  同时删除本次冒烟生成的 `crates\yunxi-agent-cli\.yunxi` 临时状态。最终
  `D:\YunXi Agent\target` 与该临时状态目录均不存在。

## 十五、发布状态

源码、测试、版本、文档、报告、状态与日志已组成唯一的 v2.0.1 发布候选。
本报告随该唯一发布提交进入 Git，提交后立即创建不可移动的 annotated
`v2.0.1` tag，并以 API key 临时认证 non-force 推送 `master` 和新 tag；不会
删除、移动或重写 `v2.0.0` 及任何旧 tag，也不会追加同版本 docs-only 或
hotfix 提交。最终远程 master、提交和 tag 对象以 Git 历史及远程 refs 为准。

署名：开发者
