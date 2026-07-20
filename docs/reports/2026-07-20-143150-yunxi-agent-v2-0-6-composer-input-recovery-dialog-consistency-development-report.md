# YunXi Agent v2.0.6 Composer、输入恢复与对话一致性开发报告

- 撰写时间：2026-07-20 14:31:50 +08:00
- 审核依据：`C:\Users\24763\Desktop\YunXi Agent审核报告\2026-07-20-142235-YunXi-Agent-v2.0.5-hotfix.2-复核审核通过报告.md`
- 项目内审核报告归档：`D:\YunXi Agent\docs\reports\2026-07-20-142235-yunxi-agent-v2-0-5-hotfix-2-reaudit-report.md`
- 审核结论转化：`v2.0.5-hotfix.2` 复核审核通过，可进入 `v2.0.6` 开发。
- 发布基线：`v2.0.5-hotfix.2`，发布提交 `63155ce1fc8785f17dabd3f11babd159222445d5`，annotated tag object `578e0db14fb232c004b89d8eb4ac244a53518c32`。
- 当前分支提示：审核报告记录当前分支提交为 `a2e7fc2688d6da17ea13813694cd78f08232`，该提交位于 tag 之后且为 `docs-only` 状态记录，按用户最新规则不构成阻塞。
- 开发目录：`D:\YunXi Agent`
- 报告类型：下一阶段开发指令报告，主要面向开发者。

## 一、硬性约束

以下约束是本项目后续开发、验证、发布和文档同步的前置要求，不得因为上下文过长、任务拆分、实现困难或局部修复而忽略、弱化或绕过。

1. 开发目录固定在 `D:\YunXi Agent` 及其工作树内。
2. 核心目标是开发一个通用型陪伴 agent。
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

`v2.0.6` 的主题为 Composer、输入恢复与对话一致性。开发者必须把本阶段视为 TUI 输入模型与流式对话生命周期的一次整体整理，而不是单点按键修复。

本阶段必须交付：

1. 用结构化 edit buffer 替代现有零散的 `String + byte cursor` 输入状态。
2. 用 grapheme index 管理光标、删除、移动、宽字符、组合字符和 emoji，不得用裸 byte offset 作为用户可见光标语义。
3. 覆盖 Windows IME 已提交文本、CRLF、多行粘贴、长粘贴、长行换行和 Unicode 混排输入。
4. 在流式输出、审批弹层、用户输入请求、取消和等待状态之间保持 Composer 草稿可恢复。
5. 保证 final response 回写同一条 assistant cell，不在 UI 层生成重复回答。
6. 保持 `v2.0.5-hotfix.2` 已通过的 ToolActivity、审批安全默认值、错误呈现、ExecOutputDecoder、真实 Provider 和 ConPTY 证据能力不回退。
7. 在真实 Windows ConPTY 中补齐 `v2.0.6` 输入与流式一致性证据，验证通过前不得宣称完成。

## 三、版本与发布边界

当前有效结论是：`v2.0.5-hotfix.2` 已通过复核，可进入 `v2.0.6`。开发者不得再把本阶段描述为 `v2.0.5` 整改，也不得移动、删除或覆盖既有 tag。

必须遵守：

- `v2.0.5`、`v2.0.5-hotfix.1`、`v2.0.5-hotfix.2` 及更早历史 tag 全部保留。
- `v2.0.6` 代码、文档、验证和发布记录必须最终收敛到新的提交和新的 annotated Git tag。
- 创建 `v2.0.6` tag 前必须完成统一验证、证据归档、清理中间产物和日志记录。
- tag 后若仅追加仓库内 docs-only 状态、日志或报告提交，按用户最新规则不再构成版本审核阻塞；但不得在 tag 后追加功能代码、测试代码或运行时行为变更。
- 如果开发中发现总纲图、审核报告和实际代码状态冲突，以当前代码和用户最新确认规则为准，并在日志中写明差异。

## 四、必须保持的既有能力

以下能力已在 `v2.0.5-hotfix.2` 复核中被视为通过，`v2.0.6` 开发不得破坏：

1. `ToolActivity` 单 cell 原位更新、终态冻结和拒绝/取消/批准分支。
2. 审批 bottom pane/overlay 默认安全决策，Enter 默认拒绝，批准、拒绝和 Ctrl+C 互不混淆。
3. provider/tool/approval/cancel/terminal/unknown 六类错误统一呈现，普通视图显示稳定 error code、摘要和用户下一步，原始诊断保留在 details/debug。
4. `ExecOutputDecoder` 对 stdout/stderr、非法 UTF-8、二进制、截断、非零退出和完整性元数据的统一处理。
5. no-TUI、JSON、JSONL、release binary、companion evaluation 和 release tag 纪律。
6. 真实 Windows ConPTY 证据链，包括 responsive、decline、approve、cancel、nonzero、invalid、binary、long 八个场景，以及 80、100、120、200 列视觉检查。
7. 主视图不得泄露 thinking、memory/context、工具参数、原始二进制、Provider wire 或未脱敏诊断。

## 五、源码接入点

### 1. `crates/yunxi-agent-tui/src/bottom_pane.rs`

当前 `BottomPaneMode::Composer` 与 `BottomPaneMode::UserInput` 使用 `buffer: String` 和 `cursor: usize`。这能处理部分 char boundary，但不能作为完整的用户可见编辑模型。开发者应抽出结构化输入模型，例如：

- 新增 `EditBuffer` 或 `ComposerBuffer`，封装 `text`、`cursor_grapheme`、`selection` 或未来可扩展状态。
- 提供 `insert_text`、`insert_newline`、`delete_previous_grapheme`、`delete_next_grapheme`、`move_left`、`move_right`、`move_home`、`move_end`、`clear`、`submit_text`、`snapshot`、`restore` 等方法。
- 内部使用 grapheme cluster 计算用户可见索引；如果新增依赖，优先使用成熟 Rust crate，例如 `unicode-segmentation`，并在 workspace dependency 中显式记录。
- `paste(&str)` 必须走同一套 buffer API，不允许粘贴路径绕过光标、换行、CRLF 或宽字符处理。
- Composer 与 user input request 应复用同一底层编辑模型，避免两套输入行为分叉。

### 2. `crates/yunxi-agent-tui/src/app.rs`

`YunxiTuiApp` 当前统一持有 `presentation`、`timeline`、`transcript`、`viewport`、`bottom_pane` 和 `control_snapshot`。本阶段应在这里建立输入恢复的状态边界：

- `start_approval`、`start_user_input` 和 `show_control_snapshot` 不得不可逆清空 Composer 草稿。
- 进入审批、等待或流式状态时，应保存当前 Composer 草稿；回到 Composer 时恢复原输入、光标和多行布局。
- `start_prompt` 只应在新会话初始化或明确重置时清空输入，不应因普通 redraw 或状态切换丢失草稿。
- footer/subheader 中关于 `Enter submit`、`Ctrl+C cancel`、`streaming current turn` 的状态提示必须与实际 active view 保持一致。

### 3. `crates/yunxi-agent-tui/src/host.rs`

Host 层负责把真实终端事件映射到 app 状态，是本阶段风险最高的接入点之一。开发者应完成：

- 明确 active-view 优先级：approval/user input 高于 composer；control snapshot 只影响显示，不应吞掉必要的恢复状态。
- `Event::Paste`、`KeyEvent`、`Resize`、`Ctrl+C`、`Esc`、`Enter`、`Alt+Enter`、`Ctrl+J` 必须走统一状态机。
- Windows IME 已提交文本应被视为普通文本插入，不得按 byte 或单 char 粗暴截断。
- 流式输出期间如果允许用户预输入，输入必须进入草稿区；如果暂不允许提交，状态提示必须清楚，且不能丢失草稿。
- 取消当前 turn 后，Composer 应恢复到可输入状态，后续输入可继续发起新 turn。

### 4. `crates/yunxi-agent-tui/src/render.rs`

渲染层必须使用 edit buffer 暴露的用户可见 cursor 位置，而不是自行重新解释 byte offset。

必须处理：

- 多行 Composer 在 80、100、120、200 列下不遮挡 transcript、footer 或审批区域。
- CRLF 粘贴统一呈现为 TUI 可控的多行文本，不产生额外空行或错位。
- 长粘贴和长行换行不导致 bottom pane 无限增高；必须有稳定高度上限、滚动或裁剪策略。
- 光标在宽字符、emoji、组合字符、中文、英文和换行混排时保持在正确视觉列。
- Composer、approval、user input 三类 bottom pane 不能互相复用错误标题、边框或状态文本。

### 5. `crates/yunxi-agent-tui/src/streaming.rs`、`chat.rs`、`timeline.rs`

`Transcript::apply_assistant_update` 和 `push_assistant` 已具备按 `TuiCellId` 更新 assistant cell 的基础。`v2.0.6` 必须把流式 delta、completed/final response 和错误/取消终态统一到这一机制上：

- 每个 assistant turn 只生成一个稳定 assistant cell ID。
- provider delta 更新 active assistant cell。
- final response 到达时更新并冻结同一 cell，不能另插一条 assistant message。
- cancelled、provider error、tool error 不得把 partial assistant 误标为最终回答。
- 流式结束后，Composer 的 draft 和 cursor 恢复必须与 transcript cell 更新解耦。
- 对重复事件、迟到事件和 completed 后 delta 继续沿用已有去重/忽略策略，不得让 UI 出现重复回答。

### 6. `crates/yunxi-agent-cli/src/interactive.rs`

CLI interactive 层需要保持 no-TUI 与 TUI 的行为边界清晰：

- no-TUI、JSON、JSONL 输出格式不因 TUI Composer 改造发生 wire shape 变化。
- TUI 运行入口应传递必要的初始 prompt、session、backend 和 provider 状态，但不直接操纵底层 edit buffer。
- 若新增恢复状态或输入配置，优先定义在 TUI crate 内，再通过清晰 facade 暴露给 CLI。

## 六、源码参考建议

参考源码必须服务于整体能力迁移，不得把 YunXi 默认运行路径重新绑回上游源码。

1. Codex Composer：优先参考 `D:\源码\codex` 中的 TUI composer、active view、input history、textarea/edit buffer 和 streaming transcript 更新逻辑。只迁移设计与行为，不依赖 `vendor/codex-rs`、`codex-*` crate 或 `yunxi-agent-codex`。
2. Aider 输入节奏：如本地存在 Aider 源码，可参考其命令行输入恢复、简洁交互节奏和回答生命周期；如本地不存在，需经用户确认后再从 GitHub 拉取到 `D:\源码`。Aider 为非 Rust 实现时，只参考逻辑并用 Rust 复刻。
3. `D:\源码\lazygit` 与 `D:\源码\k9s`：可作为终端焦点、modal 优先级、resize 和键盘路由的辅助参考；它们不是业务逻辑来源，不得引入 Go 运行时依赖。
4. 所有参考代码都必须被抽象为 YunXi 自有模块和测试，不允许通过默认功能路径引用外部源码目录。

## 七、推荐执行顺序

本阶段建议按批次推进，完成一批后统一验证，不要在单个按键、单个宽字符或单个 snapshot 上反复纠结。

1. 建立 edit buffer：新增或重构 `EditBuffer`，完成 grapheme 索引、插入、删除、移动、提交和恢复 API。
2. 合并 Composer 与 UserInput 行为：让 `BottomPaneMode::Composer` 与 `BottomPaneMode::UserInput` 共用输入核心，保留不同提交/取消语义。
3. 接入 host 状态机：整理 active-view 优先级、paste/key/resize/cancel/submit 路由，保证审批和用户输入请求不破坏 Composer 草稿。
4. 接入 render：按 edit buffer 的 wrap 与 cursor 信息渲染 bottom pane，稳定高度和视觉列。
5. 接入 streaming 与 transcript：统一 delta、final response、cancel/error 的 assistant cell 生命周期。
6. 补齐测试与证据：先做单元和集成测试，再跑统一 workspace 验证，最后做真实 ConPTY 复核。
7. 更新文档和发布状态：同步开发报告、设计文档、索引、状态文档、日志和 release 记录。
8. 清理中间产物：在获得用户确认后清理精确列出的构建/采集中间目录。
9. 发布：验证通过后提交、创建新的 annotated `v2.0.6` tag，并 non-force 推送。

## 八、测试与验收要求

必须新增或更新以下测试：

- `bottom_pane` 或新增 edit buffer 单元测试：中文、emoji、组合字符、CRLF、多行粘贴、长粘贴、Home/End、Backspace/Delete、Left/Right。
- host 事件路由测试：approval/user input/composer 的 active-view 优先级，粘贴、取消、恢复、resize 后状态不丢失。
- render snapshot 测试：80、100、120、200 列下 Composer 多行、长行、宽字符和审批弹层不重叠、不越界。
- streaming/transcript 测试：delta 与 final response 使用同一 assistant cell，completed 后不重复插入回答，cancel/error 不伪装成正常完成。
- CLI 回归测试：no-TUI、JSON、JSONL wire shape 保持兼容。
- 真实 Windows ConPTY 证据：至少覆盖普通输入、多行粘贴、CRLF 粘贴、长粘贴、IME 已提交文本、流式输出期间取消、等待状态恢复、final response 不重复。

统一验证建议在一批构建完成后执行：

```powershell
cargo fmt --all -- --check
cargo check --workspace
cargo test --workspace
cargo build -p yunxi-agent-cli --release --bins
target\release\yunxi.exe --version
target\release\yunxi.exe eval companion --json
npm.cmd run verify --prefix scripts/conpty/v205
git diff --check
git status --short
```

如果本阶段修改 ConPTY 采集脚本或证据目录，还必须重新运行 capture 与 verify，并写明真实 Provider、终端尺寸、交互步骤、脱敏结果和 manifest 哈希。普通离线 snapshot 不能替代真实 Windows ConPTY 证据。

## 九、文档、日志与发布要求

开发者完成 `v2.0.6` 后必须同步更新：

- `README.md`
- `docs/extraction-status.md`
- `docs/tui-presentation.md`
- `docs/development-log.md`
- 与本阶段相关的 `docs/reports/*.md`
- 如新增输入模型或状态机，应补充对应设计说明或索引文档。

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

验证通过前，开发者只能描述“已实现候选”“待验证”“待复核”，不得宣称 `v2.0.6` 完成。发布前不得删除旧 tag；发布后必须核对本地和远程 tag object、发布提交和远程分支指向。

## 十、实施候选与真实证据记录

- 实施记录时间：2026-07-20 17:35:20 +08:00
- 当前状态：功能实现、定向测试和真实 Provider/ConPTY 证据已通过；统一 workspace 门禁、清理、提交、tag 和推送仍待执行，因此此处只声明“已实现候选”，不提前声明发布完成。

### 1. 已实施能力

1. 新增 `crates/yunxi-agent-tui/src/edit_buffer.rs`，以 grapheme index 作为用户可见光标语义，提供插入、换行、前后删除、左右/Home/End、clear、submit、snapshot 和 restore；CRLF 与孤立 CR 统一为 LF。
2. `bottom_pane.rs` 让 Composer 永久持有 `EditBuffer`，Approval/UserInput 成为覆盖视图；进入覆盖层保存草稿，返回后恢复原文本与光标。UserInput 复用相同编辑核心但保留独立提交/取消语义。
3. `host.rs` 在 active turn 中接收字符、IME 已提交文本、Paste、删除与移动操作；Enter/Esc 不提交或清空草稿，Ctrl+C 只取消当前 turn。
4. Windows ConPTY 真实验证发现 crossterm 0.28 将粘贴内 LF 表示为 Ctrl+Enter key record。新增 Windows 输入突发分类器，只把快速连续文本后的嵌入 Enter 映射为换行；一次 active-turn drain 开始后持续到 5ms 静默期。
5. `crates/yunxi-agent-cli/src/interactive.rs` 在进入 Approval/UserInput 阻塞视图前强制执行一次 renderer tick，先把排队终端输入归入 Composer，避免草稿字符被审批快捷键消费。
6. `render.rs` 直接消费 `EditBuffer`，Composer 正文最多 6 行并围绕光标选择窗口；UserInput 使用独立 `Input` 标题。80/100/120/200 列长输入与 Unicode 布局均有回归。
7. delta、final 和 completed 沿用稳定 turn-cell 机制；新增测试确认同一 assistant cell 原位更新，不生成重复回答，也不修改预输入草稿。

### 2. 主要修改与证据路径

- Rust：`D:\YunXi Agent\crates\yunxi-agent-tui\src\edit_buffer.rs`、`bottom_pane.rs`、`host.rs`、`render.rs`、`app.rs`、`lib.rs`，以及 `D:\YunXi Agent\crates\yunxi-agent-cli\src\interactive.rs`。
- 版本与回归：workspace `Cargo.toml`/`Cargo.lock`、CLI、evaluation、persona、runtime 测试和 4 份 TUI full-frame snapshot 已收敛到 `2.0.6`。
- Collector：`D:\YunXi Agent\scripts\conpty\v206`，依赖锁定 `node-pty@1.1.0` 与 `@xterm/headless@5.5.0`。
- 原始脱敏证据：`D:\YunXi Agent\docs\reports\evidence\frames\v206-conpty`。
- 证据说明：`D:\YunXi Agent\docs\reports\evidence\2026-07-20-v2-0-6-composer-input-conpty-evidence.md`。

### 3. 已完成验证

- `cargo test -p yunxi-agent-tui`：126/126 通过。
- `cargo test -p yunxi-agent-cli`：crate、CLI 集成和 JSONL 回归全部通过。
- `cargo build -p yunxi-agent-cli --release --bins`：通过；两个 release binary 均已生成。
- 真实 DeepSeek/Windows ConPTY：ordinary、multiline、crlf、long、ime、stream-cancel、approval-restore、final-single 共 8/8 场通过，采集时间为 2026-07-20 17:29:22 至 17:29:56 +08:00。
- `npm.cmd run verify --prefix scripts\conpty\v206`：`ok=true, scenarios=8`。
- `npm.cmd run verify --prefix scripts\conpty\v205`：`ok=true, scenarios=8`，旧证据能力未回退。

### 4. 统一门禁与清理结果

- `cargo fmt --all -- --check`、`cargo check --workspace`、`cargo test --workspace`、release 双 binary 构建与版本核对全部通过；两个 binary 均报告 `yunxi 2.0.6`。
- Companion Evaluation Harness 的 text、JSON、JSONL 均为 31/31 且 `golden_passed=true`；离线 one-shot JSON、22 行 JSONL 和 no-TUI REPL 均通过。
- v206/v205 两个 ConPTY verifier 均返回 `ok=true, scenarios=8`；`git diff --check` 无空白错误。
- 用户于本轮明确确认清理后，精确删除并核验以下路径均不存在：`D:\YunXi Agent\target`、`D:\YunXi Agent\scripts\conpty\v206\node_modules`、`D:\YunXi Agent\scripts\conpty\v206\.work`、`D:\YunXi Agent\.yunxi`、`D:\YunXi Agent\crates\yunxi-agent-cli\.yunxi`。正式 collector、lockfile、证据和 manifest 全部保留。
- 在发布前检查点时尚未创建发布提交、annotated `v2.0.6` tag 或执行 GitHub 推送；该状态随后已由“十一、发布结果”更新，所有历史 tag 始终保持不移动、不删除、不覆盖。

## 十一、发布结果

- 发布时间：2026-07-20 17:58:30 +08:00
- 发布提交：`30842cf3bae3053d36a3f9229c90eb75597d8eac`，提交说明为 `feat(tui): complete v2.0.6 composer recovery`。
- annotated tag：`v2.0.6`，tag object 为 `12e64e0cd31e612046c24f4a073b95c6bc887dff`，peeled commit 为 `30842cf3bae3053d36a3f9229c90eb75597d8eac`。
- 推送方式：使用 GitHub API key 转换出的单进程临时 HTTP header，显式 non-force 推送 `refs/heads/master` 与 `refs/tags/v2.0.6`；API key 未打印、未持久化到仓库、Git 配置、remote URL、报告或日志。
- 远程核验：远程 `master`、新 tag object 和 peeled commit 与本地完全一致；推送前存在的 88 条历史 tag ref 行逐一保持不变，未移动、删除或覆盖任何旧 tag；远程版本 tag 共 45 个。
- 本节与开发日志将作为 tag 后的 docs-only 收尾提交继续 non-force 推送到 `master`，不移动 `v2.0.6`，也不追加功能代码或测试代码。

署名：开发报告撰写者
