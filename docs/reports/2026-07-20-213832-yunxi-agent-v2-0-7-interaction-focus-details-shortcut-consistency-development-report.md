# YunXi Agent v2.0.7 交互焦点、详情层与快捷键一致性开发报告

- 撰写时间：2026-07-20 21:38:32 +08:00
- 审核依据：`C:\Users\24763\Desktop\YunXi Agent审核报告\2026-07-20-204147-YunXi-Agent-v2.0.6-Composer输入恢复审核报告.md`
- 项目内审核报告归档：`D:\YunXi Agent\docs\reports\2026-07-20-204147-yunxi-agent-v2-0-6-composer-input-recovery-audit-report.md`
- 审核结论转化：`v2.0.6` 审核通过，可进入 `v2.0.7` 开发。
- 发布基线：`v2.0.6`，发布提交 `30842cf3bae3053d36a3f9229c90eb75597d8eac`，annotated tag object `12e64e0cd31e612046c24f4a073b95c6bc887dff`。
- 当前 `HEAD`：`2b25d4a0bef4f05e0c7d37b6818eec34d8cbcb5b`，审核报告明确其为 tag 后的 `docs-only` 记录，不构成阻塞。
- 开发目录：`D:\YunXi Agent`
- 报告类型：下一阶段开发指令报告，主要面向开发者。

## 一、硬性约束

以下约束是本项目后续开发、验证、发布和文档同步的前置要求，不得因为上下文过长、任务拆分、实现困难或局部修复而忽略、弱化或绕过。

1. 开发目录固定在 `D:\YunXi Agent` 及其工作树内。
2. 核心目标是开发一个通用型陪伴 agent，并且不得默认依赖 `vendor/codex-rs`、`codex-*` crate 或 `yunxi-agent-codex`。
3. 涉及到源码参考需要先抽取、迁移和构建整体能力，不要在单个点上反复纠结；如果参考的源码不为 Rust 语言，则需参考其逻辑，进行 Rust 复刻。
4. 中间无需频繁验证，完成一批构建后再统一测试和验证。
5. 验证通过前不能宣称完成。
6. 每次阶段结束后要清理编译中间产物，避免占用大量硬盘空间。
7. 每次任务结束后，都要在日志中追加本次工作的详细记录。
8. 记录必须包含时间戳、工作目标、执行流程、修改文件、文件路径、验证结果、提交和推送状态，以及日志结尾处要进行署名，署名为开发报告撰写者。
9. 后续开发报告、设计文档、索引和状态文档要与实际代码状态保持一致。
10. 回复用户使用中文。
11. 之后每个版本更迭都必须提交为新的 Git tag；以前的版本 tag 不得删除，方便出错后回滚。
12. 涉及到 shell 命令时要万分小心，例如递归删除、强制移动、清空目录、系统级安装/卸载、修改 PATH/注册表/系统配置等操作必须得到用户确认。
13. 不可因为过长的上下文忽略硬性要求；如果察觉到硬性要求的效力已经减弱，则必须汇报当前情况。
14. 工作尽量固定在项目文件夹，涉及到其他文件夹的操作，要明确告知用户，尤其是 C 盘的用户目录。

## 二、阶段目标

`v2.0.7` 的主题是交互焦点、详情层与快捷键一致性。开发者必须把本阶段视为 TUI 焦点路由与详情呈现的一次整体整理，而不是某个按键或某个面板的局部修补。

本阶段必须交付：

1. 建立明确的 `FocusTarget` 与 `TuiAction` 语义，统一 Composer、History、Approval 和 Details 的焦点切换。
2. 把快捷键解析从分散事件分支中抽离出来，形成纯 Rust 的 keymap resolver；若需要新文件，建议新增 `crates/yunxi-agent-tui/src/input_map.rs` 或等价模块。
3. 保证 Esc、Enter、PgUp/PgDown、滚轮、Ctrl+C、Tab 及其组合在不同 active view 下不冲突。
4. 详情层打开/关闭时保留 Composer 文本、光标、审批选择和 transcript anchor，不得因切换焦点而丢失草稿。
5. `/help`、footer 和窄屏提示只显示当前焦点可用的核心动作，避免把不可执行动作暴露给用户。
6. 继续保留 `v2.0.6` 已通过的 EditBuffer、Compose 输入恢复、Grapheme 光标、流式 assistant cell 一致性、真实 Provider 和 ConPTY 证据能力。
7. 在真实 Windows ConPTY 中补齐 `v2.0.7` 交互焦点与快捷键一致性证据，验证通过前不得宣称完成。

## 三、版本与发布边界

当前有效结论是：`v2.0.6` 审核通过，可进入 `v2.0.7`。开发者不得再把本阶段描述为 `v2.0.6` 收尾，也不得移动、删除或覆盖既有 tag。

必须遵守：

- `v2.0.6` 及更早历史 tag 全部保留。
- `v2.0.7` 代码、文档、验证和发布记录必须最终收敛到新的提交和新的 annotated Git tag。
- 创建 `v2.0.7` tag 前必须完成统一验证、证据归档、清理中间产物和日志记录。
- tag 后若仅追加仓库内 docs-only 状态、日志或报告提交，按用户已确认规则不再构成版本审核阻塞；但不得在 tag 后追加功能代码、测试代码或运行时行为变更。
- 如果开发中发现总纲图、审核报告和实际代码状态冲突，以当前代码和用户最新确认规则为准，并在日志中写明差异。

## 四、必须保持的既有能力

以下能力已在 `v2.0.6` 审核中被视为通过，`v2.0.7` 开发不得破坏：

1. `EditBuffer` 已完成 grapheme 级编辑、CRLF 归一化、快照和恢复，不能回退到 byte 光标。
2. `BottomPane` 已具备 Composer 草稿挂起/恢复、Approval/UserInput 复用和安全默认拒绝，不得让审批切断草稿生命周期。
3. `host.rs` 已负责真实终端事件路由、Composer/Approval/UserInput 事件循环和 Windows 输入突发处理，不得再拆散成彼此不一致的局部状态机。
4. `render.rs` 已基于 `EditBuffer` 渲染 Composer 与 UserInput，不得重新引入和编辑核心不一致的显示层。
5. `debug.rs` 已负责脱敏详情存储、统一摘要和可选 debug 展示，不得把它误用为独立焦点系统。
6. `app.rs` 已具备 `control_snapshot` 和底部面板状态切换入口，当前详情层建议优先复用这个路径，而不是另起一套完全不同的 view 栈。
7. 真实 ConPTY 证据、CLI no-TUI/JSON/JSONL 兼容性、审核 tag 纪律和历史 tag 保留规则继续有效。

## 五、源码接入点

### 1. `crates/yunxi-agent-tui/src/app.rs`

`YunxiTuiApp` 当前已经持有 `presentation`、`timeline`、`transcript`、`viewport`、`bottom_pane` 和 `control_snapshot`。这说明当前项目已经有“主视图 / 控制快照 / 底部交互区”三层结构，但还没有显式的 `FocusTarget` 与 `TuiAction` 语义。

本阶段建议：

- 在 app 层或相邻状态层新增显式焦点枚举，至少覆盖 `Composer`、`History`、`Approval`、`Details`。
- 把 `show_control_snapshot`、`show_transcript` 和未来的 details 打开/关闭统一进同一套动作语义。
- footer 与 header 的状态文案应按焦点变化，而不是只按 `timeline.has_active_sessions()` 或 `scroll_status()` 做二值判断。

### 2. `crates/yunxi-agent-tui/src/bottom_pane.rs`

当前 `BottomPane` 已经持有：

- `mode`
- `composer_prompt`
- `composer: EditBuffer`
- `suspended_composer: Option<EditBufferSnapshot>`

这说明草稿恢复已经存在基础设施，下一步不该重写输入核心，而应把焦点切换和详情层切换建立在既有挂起恢复之上。

本阶段建议：

- Composer、Approval、UserInput 三种模式继续共用 `EditBuffer` 作为输入核心。
- 进入 Approval/Details 时保留 Composer 草稿快照；返回 Composer 时准确恢复文本和光标。
- 把 Approval 与 UserInput 的快捷键语义收束成明确的 action 返回值，避免 host 侧重复解释按键。
- 若新增 `input_map.rs`，`BottomPane` 只保留输入模型，不再承载全部键位策略。

### 3. `crates/yunxi-agent-tui/src/host.rs`

`host.rs` 当前负责 `read_prompt`、`request_approval`、`request_user_input`、`tick` 和真实终端事件循环。它是焦点路由与快捷键一致性的第一落点。

本阶段建议：

- 把焦点判断从散落的 `Event::Key` 分支中提取出来，形成统一的状态机入口。
- `Ctrl+C`、`Esc`、`Enter`、`Tab`、`PgUp/PgDown`、滚轮和 paste 事件都应先进入焦点路由，再决定具体动作。
- Windows IME 已提交文本应按普通文本插入处理，不得因 active view 切换丢字。
- 在流式 turn 进行时，草稿允许继续存在，但提交与取消必须按当前焦点语义执行。

### 4. `crates/yunxi-agent-tui/src/render.rs`

`render.rs` 当前已经根据 `BottomPaneMode` 渲染 Composer、Approval 和 UserInput，并在 `control_snapshot` 存在时切换到控制快照视图。它是详情层和焦点提示的视觉落点。

本阶段建议：

- 详情层应与 `control_snapshot` 逻辑兼容，不要把 debug 和 control snapshot 变成两套互不相干的“详情”。
- footer / hint 文案应按 `FocusTarget` 生成，不要继续仅靠“有无活动 turn”推断可用操作。
- 窄屏时优先保留提交、取消、返回、滚动等核心动作，避免把所有快捷键都塞进底栏。

### 5. `crates/yunxi-agent-tui/src/debug.rs` 与 `crates/yunxi-agent-tui/src/chat.rs`

`debug.rs` 只负责脱敏详情和可选调试摘要；`chat.rs` 仍负责 transcript cell 与 assistant cell 的稳定更新。这两个模块不应被硬塞进焦点状态机，但可以成为 Details 层的内容源。

本阶段建议：

- Details 打开时可复用 debug 详情、control snapshot 或 transcript 额外信息，但应通过统一的详情视图入口呈现。
- 对 assistant cell 的稳定更新逻辑保持不变，不要因为详情层切换而让 transcript 重复插入回答。
- 详情层的内容源与焦点状态要解耦，避免 debug 详情显示成为新的操作层。

### 6. `crates/yunxi-agent-tui/src/edit_buffer.rs`

`EditBuffer` 已经提供 grapheme 级插入、删除、移动、快照、恢复和 CRLF 归一化。这是本阶段交互一致性的底座。

本阶段建议：

- 继续把它作为 Composer 和 UserInput 的唯一编辑核心。
- 详情层或焦点切换不得直接操作 byte offset。
- 如果要扩展快捷键解析，应围绕 EditBuffer 的语义编写测试，而不是重新发明输入模型。

### 7. `crates/yunxi-agent-cli/src/interactive.rs`

CLI 交互层需要保持 no-TUI、JSON、JSONL 和 release binary 的 wire shape 不变。本阶段的焦点和详情层调整不得破坏 CLI 既有协议面。

## 六、源码参考建议

参考源码必须服务于整体能力迁移，不得把 YunXi 默认运行路径重新绑回上游源码。

1. Codex：参考 `D:\源码\codex` 中的 bottom pane active-view 优先级、焦点切换、details 打开/关闭和快捷键提示逻辑。只迁移设计与行为，不依赖 `vendor/codex-rs`、`codex-*` crate 或 `yunxi-agent-codex`。
2. Aider：参考 `D:\源码\aider` 中的简洁输入节奏、命令行编辑体验和回答生命周期；它是本次新增且此前本地缺失的参考源码，现已补拉到本地。Aider 为非 Rust 实现时，只参考逻辑并用 Rust 复刻。
3. Lazygit：参考 `D:\源码\lazygit` 中的焦点提示、快捷键一致性和窄屏提示方式，逻辑仍应以 Rust 状态机复刻，不引入 Go UI runtime。
4. 所有参考代码都必须被抽象为 YunXi 自有模块和测试，不允许通过默认功能路径引用外部源码目录。

## 七、推荐执行顺序

本阶段建议按批次推进，完成一批后统一验证，不要在单个按键、单个焦点或单个提示文案上反复纠结。

1. 定义 `FocusTarget`、`TuiAction` 和纯 Rust keymap resolver，先把焦点语义和事件路由统一起来。
2. 把 `host.rs` 的输入事件、`BottomPane` 的 action 返回和 `app.rs` 的状态切换串成同一条链。
3. 让 details / control snapshot / debug 详情共用统一入口，建立可恢复的 Composer 草稿生命周期。
4. 更新 `render.rs` 的 header/footer/hint 文案，让它们按当前焦点显示最小必要动作。
5. 补齐 `edit_buffer`、focus routing、details layer、shortcut mapping 的单元和快照测试。
6. 统一执行 workspace 验证，再做真实 Windows ConPTY 复核。
7. 更新文档和发布状态：同步开发报告、设计文档、索引、状态文档、日志和 release 记录。
8. 清理中间产物：在获得用户确认后清理精确列出的构建/采集目录，避免占用磁盘。
9. 发布：验证通过后提交、创建新的 annotated `v2.0.7` tag，并 non-force 推送。

## 八、测试与验收要求

必须新增或更新以下测试：

- `bottom_pane` 焦点测试：Composer、Approval、UserInput、Details 切换时草稿、光标和选择状态不丢失。
- keymap 测试：Esc、Enter、PgUp/PgDown、Tab、Ctrl+C、滚轮和 paste 在不同 active view 下不冲突。
- `render.rs` snapshot 测试：80、100、120、200 列下 header/footer/hint/details 不重叠、不越界。
- `debug.rs` / details 测试：详情文本脱敏、可复用、不会把原始敏感内容暴露到主视图。
- CLI 回归测试：no-TUI、JSON、JSONL wire shape 保持兼容。
- 真实 Windows ConPTY 证据：至少覆盖焦点切换、详情打开/关闭、快捷键一致性、草稿恢复和窄屏提示。

统一验证建议在一批构建完成后执行：

```powershell
cargo fmt --all -- --check
cargo check --workspace
cargo test --workspace
cargo build -p yunxi-agent-cli --release --bins
target\release\yunxi.exe --version
target\release\yunxi.exe eval companion --json
npm.cmd run verify --prefix scripts/conpty/v206
git diff --check
git status --short
```

如果本阶段修改了 ConPTY 采集脚本或证据目录，还必须重新运行 capture 与 verify，并写明真实 Provider、终端尺寸、交互步骤、脱敏结果和 manifest 哈希。普通离线 snapshot 不能替代真实 Windows ConPTY 证据。

## 九、文档、日志与发布要求

开发者完成 `v2.0.7` 后必须同步更新：

- `README.md`
- `docs/extraction-status.md`
- `docs/tui-presentation.md`
- `docs/development-log.md`
- 与本阶段相关的 `docs/reports/*.md`
- 如新增输入模型、快捷键 resolver 或焦点状态机，应补充对应设计说明或索引文档。

日志必须写明：

- 时间戳；
- 工作目标；
- 执行流程；
- 修改文件；
- 文件路径；
- 验证结果；
- 清理结果；
- 提交和推送状态；
- Git tag 状态；
- 结尾署名：开发报告撰写者。

验证通过前，开发者只能描述“已实现候选”“待验证”“待复核”，不得宣称 `v2.0.7` 完成。发布前不得删除旧 tag；发布后必须核对本地和远程 tag object、发布提交和远程分支指向。

## 十、当前本地源码参考状态

本次审核报告中明确提到、且本地原先缺失的新增参考源码是 `Aider`。它已被拉取到：

- `D:\源码\aider`

其余本地参考源码已存在于：

- `D:\源码\codex`
- `D:\源码\lazygit`

本节记录的是报告撰写检查点；后续实际实现状态由以下章节更新。

## 十一、实际实现

- Workspace 产品版本与 CLI/TUI 用户可见版本已提升到 `2.0.7`；persona
  context 与 Evaluation Harness 的独立 schema version 继续保持 `2.0.6`。
- 新增 `crates/yunxi-agent-tui/src/input_map.rs`，定义 `FocusTarget`、
  `TuiAction`、`resolve_key` 与 `resolve_event`。
- Composer、History、Approval、Details 统一路由 Enter、Esc、Tab、BackTab、
  Ctrl+C、PgUp/PgDown、paste 与滚轮事件；非输入焦点会消费不适用按键，
  不再透传给底层 Composer。
- Approval 的 approve/decline/cancel/selection 均通过 resolver 分类，保留
  默认拒绝的安全选择。
- Details 使用独立滚动偏移在主内容区显示脱敏 DebugBuffer 内容；关闭时
  恢复先前焦点，且不修改 Composer 文本/光标、Approval 选择或 transcript
  viewport anchor。
- footer 按当前焦点生成最小必要动作；80/100/120/200 列快照已更新。
- 新增 `scripts/conpty/v207`、八份 v207 脱敏帧、SHA-256 manifest 与证据说明。

## 十二、统一验证与发布前状态

- `cargo fmt --all -- --check`：通过。
- `cargo check --workspace`：通过。
- `cargo test --workspace`：通过；TUI 130/130。
- release 双 binary：均输出 `yunxi 2.0.7`。
- Companion Evaluation Harness：31/31，`golden_passed=true`。
- 真实 DeepSeek/Windows ConPTY v207：8/8；v207 与历史 v206 verifier 均返回
  `ok=true, scenarios=8`。
- `git diff --check`：通过。
- 用户于 2026-07-21 07:29:07 +08:00 明确确认清理后，已精确删除并核验
  `target`、v207 `node_modules`、v207 `.work`、`.tmp/v207-gates`、根目录
  `.yunxi` 和 CLI `.yunxi` 均不存在；collector、lockfile、八份证据和
  manifest 全部保留。
- 当前仍是已验证候选；尚未创建发布提交、annotated `v2.0.7` tag 或执行
  GitHub 推送。全部历史 tag 保持不移动、不删除、不覆盖。

署名：开发报告撰写者
