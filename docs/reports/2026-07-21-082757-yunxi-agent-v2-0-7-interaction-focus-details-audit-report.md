# YunXi Agent v2.0.7 交互焦点、详情层与快捷键一致性审核报告

- 审核时间：2026-07-21 08:27:57 +08:00
- 审核版本：`v2.0.7`
- 发布提交：`a3b2c043a37af5e287ba16356a2300f5db62737e`
- annotated tag 对象：`8ff0350189a8d91019ba95e84c0e677ef17ef257`
- 当前 `HEAD`：`b193041b3b17d333f97d239d85a5b8f5729acdde`
- 审核目录：`D:\YunXi Agent`
- 审核依据：`C:\Users\24763\Desktop\YunXi Agent v2.0.0-v2.1.0 TUI与流式输出重构总纲图（复核版）.md` 中的 `v2.0.7 - 交互焦点、详情浏览与快捷键一致性` 要求。

## 审核结论

**不通过。不得进入 `v2.0.8` 或其他后续版本开发，必须先完成当前 `v2.0.7` 的焦点路由整改并重新审核。**

本版本的结构、构建、测试、版本、陪伴评估、既有真实 Provider/ConPTY 证据和发布 tag 均有通过项；但下述 P1 缺陷直接违反本版本的焦点与鼠标交互硬性验收，不能以测试绿灯或历史场景证据替代。

## 一、阻塞项

### P1：Approval 与 Details 焦点下，鼠标滚轮/滚动条仍可操作底层 transcript

**总纲图要求：** 鼠标滚轮在 transcript 与 details 中行为一致；Approval 中只能移动审批选择，不能意外滚动历史。四种焦点下的 Esc、Enter、PgUp/PgDown、滚轮和 Ctrl+C 必须有明确且无冲突的结果。

**源码事实：**

1. `crates/yunxi-agent-tui/src/input_map.rs:29-35` 对 `Event::Mouse(ScrollUp)` 和 `Event::Mouse(ScrollDown)` 无条件返回 `TuiAction::ScrollUp` / `TuiAction::ScrollDown`，没有按 `FocusTarget` 区分 Approval、Details、History 或 Composer。
2. `crates/yunxi-agent-tui/src/host.rs:305-340` 收到滚轮事件后，只要鼠标坐标位于 `metrics.layout.transcript`，就调用 `scroll_up` / `scroll_down`。该分支没有拦截 `FocusTarget::Approval`，因此审批覆盖层打开时仍可滚动底层历史。
3. 同一文件 `331-335` 将左键按下和拖动直接交给 transcript scrollbar；`407-440` 的 scrollbar 处理也没有 Approval/Details 焦点保护，审批中同样可能经点击/拖动改变历史视口。
4. Details 焦点下，键盘 PgUp/PgDown 在 `host.rs:357-370` 正确调用 `detail_scroll_up` / `detail_scroll_down`，但滚轮没有等价的 details 路由，而会命中上述 transcript 滚动分支。因此 details 的滚轮不是滚动详情，就是在可命中 transcript 区域时错误滚动底层历史。

**影响：** 用户在审批决策或只读详情期间会看到无关历史移动，破坏焦点可预测性；details 的鼠标滚动无法履行其只读详情浏览职责。该问题属于本版本的核心交互语义，不是视觉微调。

**测试缺口：** `input_map.rs` 的 `paste_and_wheel_are_classified_before_view_handlers` 只断言 History 的滚轮映射；没有断言 Approval 必须忽略滚轮，也没有断言 Details 必须路由到详情滚动。`host.rs` 也没有覆盖 Approval/Details 下滚轮、scrollbar 点击和拖动后 transcript anchor 不变的集成测试。

## 二、真实 TUI / ConPTY 证据评审

`v207` 的真实 DeepSeek / Windows ConPTY 证据与离线 verifier 均通过，八个场景为 `ordinary`、`multiline`、`crlf`、`long`、`ime`、`stream-cancel`、`approval-restore`、`final-single`。这些证据有效地证明了 Composer、粘贴、IME、取消、审批草稿恢复和单一 assistant cell 没有回归。

但 v207 collector 沿用了 v206 的八个场景，未覆盖：

- 打开/关闭 details 后的鼠标滚轮与 PgUp/PgDown；
- Approval 焦点中的滚轮、scrollbar 点击和拖动；
- History/Details/Approval/Composer 四焦点的完整快捷键矩阵；
- 详情打开/关闭后 transcript anchor 不变的真实终端视觉验证。

因此，现有真实 ConPTY 帧不能作为本版本新增焦点功能的验收证据。结合上述源码路径，本项不具备通过条件。

## 三、已通过项

| 验证项 | 结果 |
| --- | --- |
| `cargo fmt --all -- --check` | 通过 |
| `cargo check --workspace` | 通过 |
| `cargo test --workspace` | 通过，无失败项 |
| `cargo test -p yunxi-agent-tui` | 131/131 通过 |
| `cargo build -p yunxi-agent-cli --release --bins` | 通过 |
| `target\\release\\yunxi.exe --version` | 返回 `yunxi 2.0.7` |
| `target\\release\\yunxi.exe eval companion --json` | 31/31 通过，`golden_passed=true`，`tool_approval_bypass_count=0` |
| `npm.cmd run verify --prefix scripts\\conpty\\v207` | 通过，8 个场景 |
| `npm.cmd run verify --prefix scripts\\conpty\\v206` | 通过，8 个历史场景 |
| `git diff --check` | 通过 |

`v2.0.7` annotated tag 正确指向发布提交 `a3b2c04`。tag 后仅有 `docs/development-log.md` 与 v2.0.7 开发报告的 `docs-only` 状态提交；依照用户已确认的规则，该事项仅作观察项，不构成阻塞。

## 四、参考源码与整改要求

本版本的参考方向仍正确：

- `D:\源码\codex`：bottom pane active-view 优先级、details 打开/关闭与可恢复焦点；
- `D:\源码\lazygit`：焦点状态、窄屏提示、快捷键一致性；
- `D:\源码\aider`：简洁输入节奏与终端交互的可预测性。

参考逻辑必须继续以 YunXi 自有 Rust 状态机实现，不能引入 Codex、Go 或 Python 的 UI runtime。

当前版本整改必须完成以下内容：

1. 在 `crates/yunxi-agent-tui/src/input_map.rs` 按 `FocusTarget` 显式解析鼠标行为：Approval 不得产生 transcript 滚动动作；Details 必须产生独立的详情滚动动作，或由清晰的等价语义承接。
2. 在 `crates/yunxi-agent-tui/src/host.rs` 对滚轮、scrollbar 点击和拖动增加焦点守卫：Approval 不得改变 transcript viewport；Details 不得把鼠标事件传给底层 transcript，而应只更新 `details_scroll`；History/Composer 的行为需与 footer 提示一致。
3. 新增 `input_map` 单元测试，覆盖 Composer、History、Approval、Details 下的滚轮、Esc、Enter、PgUp/PgDown、Tab、Ctrl+C 与 Paste；每一项要有唯一、可断言的结果。
4. 新增 `host` / `app` 集成测试：在 Approval 中发送滚轮及 scrollbar 点击/拖动前后 transcript anchor 不变；在 Details 中滚轮只改变 `details_scroll`，关闭详情后恢复原先 anchor、草稿与审批选择。
5. 扩展 `scripts/conpty` 的真实 Windows ConPTY 场景或新建本版本修复场景，至少覆盖 details 打开/关闭及滚轮、Approval 下滚轮/拖动无副作用、四种焦点 footer/help 提示和窄终端行为；不得以普通 Composer 场景替代。
6. 完成统一验证和真实 Provider 复测后，创建新的修复发布提交与新的 annotated tag。`v2.0.7` 及所有历史 tag 不得移动、删除或覆盖；新 tag 名称须先获得用户确认。

完成上述整改并复审通过前，开发报告不得安排 `v2.0.8` 功能。

署名：审核者
