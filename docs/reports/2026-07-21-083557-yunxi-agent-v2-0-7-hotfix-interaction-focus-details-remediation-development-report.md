# YunXi Agent v2.0.7-hotfix 交互焦点、详情层与滚轮一致性整改开发报告

- 撰写时间：2026-07-21 08:35:57 +08:00
- 审核依据：`C:\Users\24763\Desktop\YunXi Agent审核报告\2026-07-21-082757-YunXi-Agent-v2.0.7-交互焦点详情层审核报告.md`
- 项目内审核报告归档：`D:\YunXi Agent\docs\reports\2026-07-21-082757-yunxi-agent-v2-0-7-interaction-focus-details-audit-report.md`
- 审核结论转化：`v2.0.7` 未通过，不得进入 `v2.0.8` 或其他后续版本开发。
- 下个整改版本：`v2.0.7-hotfix`
- 发布基线：`v2.0.7`，发布提交 `a3b2c043a37af5e287ba16356a2300f5db62737e`，annotated tag object `8ff0350189a8d91019ba95e84c0e677ef17ef257`。
- 审核时 `HEAD`：`b193041b3b17d333f97d239d85a5b8f5729acdde`
- 开发目录：`D:\YunXi Agent`
- 报告类型：整改开发指令报告，主要面向开发者；本报告不是完成声明。

## 一、硬性约束

以下约束是 `v2.0.7-hotfix` 及后续版本开发、验证、发布和文档同步的前置要求，不得因为上下文过长、任务拆分、实现困难或局部修复而忽略、弱化或绕过。

1. 开发目录固定在 `D:\YunXi Agent` 及其工作树内。
2. 核心目标是开发一个通用型陪伴 agent；默认运行路径不得依赖 `vendor/codex-rs`、`codex-*` crate 或 `yunxi-agent-codex`。
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

## 二、版本边界

`v2.0.7` 的发布 tag 已存在，但本次审核结论是不通过。开发者必须把下一阶段收敛为 `v2.0.7-hotfix`，目标是修复交互焦点、详情层滚动与鼠标/scrollbar 路由问题；不得把工作推进到 `v2.0.8` 功能开发。

本阶段只允许围绕下列边界展开：

1. 修复 Approval 与 Details 焦点下鼠标滚轮、scrollbar 点击和拖动错误影响 transcript viewport 的问题。
2. 补齐 Composer、History、Approval、Details 四类焦点的快捷键与鼠标动作矩阵测试。
3. 扩展真实 Windows ConPTY 场景，证明 details 打开/关闭、滚轮、拖动、footer/help 提示和窄终端行为符合焦点语义。
4. 保持 `v2.0.7` 已通过的 Composer、IME、粘贴、取消、审批草稿恢复、流式 assistant cell、真实 Provider 和 companion evaluation 能力。
5. 验证、清理、日志、提交和 tag 全部完成前，不得对外宣称 `v2.0.7-hotfix` 完成。

## 三、阻塞问题摘要

审核报告指出的核心阻塞是：Approval 与 Details 焦点下，鼠标滚轮或 scrollbar 操作仍可能改变底层 transcript。这不是视觉微调问题，而是焦点路由语义错误，会让审批决策或详情浏览期间出现历史视口移动。

当前代码事实需要开发者正面处理：

1. `crates/yunxi-agent-tui/src/input_map.rs:29-35` 的 `resolve_event` 对鼠标滚轮返回 `TuiAction::ScrollUp` / `TuiAction::ScrollDown` 时没有区分 `FocusTarget`。
2. `crates/yunxi-agent-tui/src/host.rs:305-340` 在鼠标命中 transcript 区域后会直接执行 `scroll_up` / `scroll_down` 或进入 scrollbar 分支，缺少 Approval/Details 焦点保护。
3. `crates/yunxi-agent-tui/src/host.rs:407-440` 附近的 scrollbar 点击和拖动处理应增加焦点守卫，避免 Approval 和 Details 操作底层 transcript anchor。
4. `crates/yunxi-agent-tui/src/host.rs:357-370` 已经让 Details 的键盘 PgUp/PgDown 走 `detail_scroll_up` / `detail_scroll_down`，鼠标滚轮必须与该语义对齐。
5. `crates/yunxi-agent-tui/src/app.rs:102-136` 已经具备 `details_scroll`、`previous_focus`、`show_control_snapshot` 和 `close_details`，本次整改应复用现有状态，不要再另起一套 details view 栈。

## 四、源码接入点与整改要求

### 1. `crates/yunxi-agent-tui/src/input_map.rs`

`input_map.rs` 必须成为焦点输入语义的第一层入口。开发者应按 `FocusTarget` 显式解析键盘、粘贴和鼠标滚轮行为，避免滚轮事件在 Approval、Details、History、Composer 之间共享一个模糊动作。

整改要求：

- Approval 焦点下，滚轮不得产生 transcript 滚动动作；建议解析为 `TuiAction::None` 或等价的 no-op 语义。
- Details 焦点下，滚轮必须有独立且可测试的详情滚动语义；可以新增 `TuiAction::DetailScrollUp` / `TuiAction::DetailScrollDown`，也可以在 host 层用焦点守卫把 `ScrollUp` / `ScrollDown` 明确路由到 `details_scroll`，但结果必须唯一、可断言。
- History 焦点下，滚轮继续操作 transcript。
- Composer 焦点下，滚轮行为必须与 footer/help 提示一致；如果允许历史滚动，需要在测试中确认不会破坏输入草稿。
- Paste 仅应进入 Composer/UserInput 可编辑路径，Approval 和 Details 不应吞入可见文本。

### 2. `crates/yunxi-agent-tui/src/host.rs`

`host.rs` 是真实终端事件和状态变更的落点，必须承担最终焦点守卫。即使 `input_map.rs` 已经区分动作，host 也不能把鼠标事件无条件交给 transcript viewport。

整改要求：

- 在滚轮分支进入 `scroll_up` / `scroll_down` 前读取 `self.app.focus_target()`，Approval 必须直接拦截，Details 必须只更新 `details_scroll`。
- 在 scrollbar left-click、drag 和 mouse-up 流程中加入焦点守卫；Approval 与 Details 不得启动 `TranscriptScrollDrag`，也不得触发 `page_up` / `page_down`。
- Details 焦点下的鼠标滚轮应复用 `detail_scroll_up(3)` / `detail_scroll_down(3)` 或一致的步长策略；键盘和鼠标不能使用两套相互矛盾的滚动单位。
- History/Composer 的 transcript 滚动必须保留既有行为，并在测试中确认 transcript anchor 变化只发生在允许的焦点下。
- 若新增辅助函数，建议集中为 `handle_mouse_scroll_by_focus`、`allows_transcript_scrollbar` 或等价小函数，避免把同一条件散落在多个分支。

### 3. `crates/yunxi-agent-tui/src/app.rs`

`YunxiTuiApp` 已经持有 `details_scroll`、`focus` 和 `previous_focus`。本次修复不应大规模重写 app 状态，而应在现有结构上补齐焦点闭环。

整改要求：

- Details 打开后，滚轮和 PgUp/PgDown 只改变 `details_scroll`。
- Details 关闭后，恢复原先焦点，并保持 transcript anchor、Composer 草稿、光标和 Approval 选择不变。
- Approval 打开期间，底层 transcript viewport 必须冻结；用户只能对审批选择或审批确认/拒绝产生影响。

### 4. `crates/yunxi-agent-tui/src/render.rs`

渲染层必须反映真实可用动作。footer/help 的提示不能宣称某焦点支持会被拦截的操作，也不能暗示 Approval 可通过滚轮浏览历史。

整改要求：

- Details footer 保留关闭详情和详情滚动提示。
- Approval footer 只显示审批相关的确认、拒绝、取消或当前可执行动作。
- History/Composer 的滚轮、拖动、PgUp/PgDown 提示必须与实际路由一致。
- 窄终端下优先保留核心动作，避免提示文本溢出或遮挡主内容。

### 5. `crates/yunxi-agent-tui/src/bottom_pane.rs` 与 `crates/yunxi-agent-tui/src/edit_buffer.rs`

本次 hotfix 不应重写输入核心。`BottomPane` 与 `EditBuffer` 已经承担 Composer 草稿、审批覆盖、UserInput 和 grapheme 编辑能力，开发者只在焦点整改暴露出真实耦合问题时做最小改动。

必须保持：

- Composer 草稿在 Approval、UserInput、Details 往返期间不丢失。
- Ctrl+C、Esc、Enter 的含义继续由当前焦点决定。
- IME committed text、CRLF 粘贴、Emoji/CJK/组合字符编辑不得回退。

## 五、参考源码建议

本次审核报告没有提出新的第三方源码库。当前参考源仍为本地已有路径：

1. `D:\源码\codex`：参考 bottom pane active view 优先级、details 打开/关闭、焦点恢复和快捷键路由的职责划分。
2. `D:\源码\lazygit`：参考焦点状态、footer/help 提示分级、窄屏提示和快捷键一致性。
3. `D:\源码\aider`：参考简洁输入节奏、终端交互可预测性和低干扰的交互反馈。

这些源码只作为逻辑参考。YunXi 默认运行路径必须继续使用自有 Rust 实现，不得引入 Codex、Go、Python 或其他上游 UI runtime 作为默认依赖。若后续审核报告点名新的源码项目，先确认 `D:\源码` 是否已有；本地没有时再按用户确认拉取到 `D:\源码`。

## 六、推荐执行顺序

开发者应按整体能力批量构建，不要在单个点上反复验证：

1. 先整理 `input_map.rs` 的焦点动作矩阵，让四种焦点的键盘、粘贴和鼠标滚轮行为都有唯一结果。
2. 再在 `host.rs` 增加鼠标滚轮与 scrollbar 的焦点守卫，确保 Approval/Details 不再影响 transcript viewport。
3. 同步调整 `app.rs` 和 `render.rs` 中与 details_scroll、焦点恢复、footer/help 文案有关的细节。
4. 补齐 `input_map` 单元测试，覆盖 Composer、History、Approval、Details 下的滚轮、Esc、Enter、PgUp/PgDown、Tab、Ctrl+C 与 Paste。
5. 补齐 `host` / `app` 集成测试，重点断言 Approval 下滚轮、scrollbar 点击和拖动前后 transcript anchor 不变；Details 下滚轮只改变 `details_scroll`。
6. 新建或扩展本版本 ConPTY 场景，覆盖 details 打开/关闭、Approval 下滚轮/拖动无副作用、四焦点 footer/help 提示、窄终端行为。
7. 完成上述一批改动后统一执行验证；通过前不得宣称完成。
8. 验证通过后按绝对路径确认并清理编译与采集中间产物，危险清理命令必须先获得用户确认。
9. 更新开发报告、设计文档、脚本索引、状态文档和日志，确保与实际代码一致。
10. 创建新的修复提交和新的 annotated Git tag；历史 tag 不得移动、删除或覆盖。

## 七、测试与验收清单

完成代码和测试批量整改后，至少执行以下统一验证：

| 验证项 | 目的 |
| --- | --- |
| `cargo fmt --all -- --check` | 确认 Rust 格式未漂移 |
| `cargo check --workspace` | 确认 workspace 类型与依赖闭环 |
| `cargo test --workspace` | 确认全仓库回归 |
| `cargo test -p yunxi-agent-tui` | 确认 TUI 焦点、输入、渲染和交互测试 |
| `cargo build -p yunxi-agent-cli --release --bins` | 确认发布二进制可构建 |
| `target\release\yunxi.exe --version` | 确认版本输出为 hotfix 目标版本 |
| `target\release\yunxi.exe eval companion --json` | 确认陪伴评估与工具审批边界无回退 |
| `npm.cmd run verify --prefix scripts\conpty\<hotfix目录>` | 确认真实 Windows ConPTY 新场景 |
| `npm.cmd run verify --prefix scripts\conpty\v207` | 确认历史 v207 场景未回退 |
| `git diff --check` | 确认无空白和行尾问题 |

新增测试必须能证明：

- Approval 焦点下，滚轮、scrollbar 点击、拖动和 mouse-up 都不会改变 transcript anchor。
- Details 焦点下，滚轮和 PgUp/PgDown 只改变 `details_scroll`。
- Details 关闭后，原先 transcript anchor、Composer 草稿、光标和 Approval 选择保持不变。
- Composer、History、Approval、Details 的 Esc、Enter、PgUp/PgDown、Tab、Ctrl+C、Paste 和滚轮结果唯一且可断言。
- 真实 ConPTY 帧能看到 details 与 approval 场景的焦点结果，而不是只复用普通 Composer 场景。

## 八、清理、日志与发布纪律

阶段结束后必须清理中间产物，但清理前要先列出精确绝对路径并确认目标位于 `D:\YunXi Agent` 工作树内。涉及递归删除、强制移动、清空目录、系统级安装/卸载、修改 PATH/注册表/系统配置的命令，必须先得到用户确认。

日志必须追加到：

- `D:\YunXi Agent\docs\development-log.md`
- `C:\Users\24763\Desktop\YunXi Agent开发日志.md`

日志必须包含：

- 时间戳；
- 工作目标；
- 执行流程；
- 修改文件；
- 文件路径；
- 验证结果；
- 提交和推送状态；
- 结尾署名：开发报告撰写者。

发布纪律：

1. `v2.0.7` 及所有历史 tag 均不得移动、删除或覆盖。
2. `v2.0.7-hotfix` 完成验证、清理、日志和文档同步后，必须创建新的 annotated Git tag。
3. tag 名称建议使用 `v2.0.7-hotfix`，正式创建前按用户最新指令确认。
4. tag 后如仅追加 docs-only 状态记录，应在日志中明确；不得在 tag 后追加未验证功能代码。

## 九、当前任务状态

本报告只完成 `v2.0.7-hotfix` 的开发整改指令撰写，没有修改 YunXi Rust 源码，也没有运行构建、测试、ConPTY 采集或发布流程。本次审核报告没有点名新的外部源码项目，因此无需新增拉取参考源码。

署名：开发报告撰写者

## 十、实际整改实现

截至 2026-07-21 09:52:22 +08:00，已在 v2.0.7 发布基线之上完成本报告范围内的
`v2.0.7-hotfix` 候选实现，未进入 v2.0.8：

- `crates/yunxi-agent-tui/src/input_map.rs` 为 Composer、History、Approval、
  Details 显式解析滚轮动作；Approval 返回 no-op，Details 返回独立
  `DetailScrollUp`/`DetailScrollDown`，Composer 的 PgUp/PgDown 与 footer 提示一致。
- `crates/yunxi-agent-tui/src/host.rs` 将键盘、滚轮、scrollbar click/drag/up 和
  resize 汇入可测试状态处理函数。Approval/Details 在 host 层再次阻止
  `TranscriptScrollDrag` 和 transcript viewport 变更；Details 滚轮使用
  `details_scroll`，`read_prompt` 在 Details 焦点下保留详情层直到 Esc 关闭。
- `crates/yunxi-agent-tui/src/app.rs` 新增 prompt preparation 生命周期守卫，保留
  Details 的 previous focus、Composer 草稿/光标、Approval 选择和 transcript anchor。
- `crates/yunxi-agent-tui/src/render.rs`、`approval_layout.rs` 更新 Details/Approval
  标题与 footer/help 提示；窄终端快照更新到 hotfix 版本。
- Workspace/CLI 用户可见版本提升为 `2.0.7-hotfix`；persona context 与
  Evaluation Harness schema 继续保持 `2.0.6`。
- 新增 `scripts/conpty/v207-hotfix` 和
  `docs/reports/evidence/frames/v207-hotfix-conpty`，旧 v207/v206 证据未覆盖。
- 新增证据说明：
  `docs/reports/evidence/2026-07-21-v2-0-7-hotfix-focus-routing-conpty-evidence.md`。

## 十一、统一验证结果

- `cargo fmt --all -- --check`：通过。
- `cargo check --workspace`：通过。
- `cargo test --workspace`：通过；最新 TUI 回归为 136/136。
- `cargo test -p yunxi-agent-tui`：136/136，通过四焦点动作矩阵、Approval
  鼠标冻结、Details 滚轮/PgDown/关闭恢复和响应式 footer 测试。
- `cargo build -p yunxi-agent-cli --release --bins`：通过。
- release 双 binary：均输出 `yunxi 2.0.7-hotfix`。
- `target\release\yunxi.exe eval companion --json`：31/31，
  `golden_passed=true`，`tool_approval_bypass_count=0`；Harness schema 仍为 `2.0.6`。
- `npm.cmd run verify --prefix scripts\conpty\v207-hotfix`：通过，真实 DeepSeek/
  Windows ConPTY 场景 2/2；Approval freeze、Details wheel/PgDown、58x18 和
  close restoration 均有脱敏 checkpoint。
- `npm.cmd run verify --prefix scripts\conpty\v207`：通过，8/8。
- `npm.cmd run verify --prefix scripts\conpty\v206`：通过，8/8。
- `node --check`：hotfix 三个采集脚本通过。
- `git diff --check`：通过，仅有 Windows LF-to-CRLF 警告。

证据 manifest：

- `D:\YunXi Agent\docs\reports\evidence\frames\v207-hotfix-conpty\manifest.json`
- SHA-256：`E5D6C9C4E35DE3BB4E30E29D7136D600DC1C865508CCA3894511A54320F40BB7`
- `approval-freeze.json`：`8c73c76fc340b5884cd190b7953c6353bead406e550bd1c05e989168837c288a`
- `details-scroll.json`：`42204e981a26ebe44fa73573ece18a3b4f0f1d9862716b400a0c843cceecea91`

## 十二、清理与发布前状态

截至 2026-07-21（北京时间），用户确认后已按绝对路径逐项清理并核验以下中间目录
不存在：

- `D:\YunXi Agent\target`
- `D:\YunXi Agent\scripts\conpty\v207-hotfix\node_modules`
- `D:\YunXi Agent\scripts\conpty\v207-hotfix\.work`
- `D:\YunXi Agent\.yunxi`
- `D:\YunXi Agent\crates\yunxi-agent-cli\.yunxi`

正式证据、lockfile、collector、manifest 和历史 v2.0.7/v2.0.6 tag 均保留。
本报告随整改代码和验证证据进入发布提交；`v2.0.7-hotfix` annotated tag 将在提交
核验后创建，不移动、删除或覆盖 `v2.0.7` 及任何历史 tag。

署名：开发报告撰写者
