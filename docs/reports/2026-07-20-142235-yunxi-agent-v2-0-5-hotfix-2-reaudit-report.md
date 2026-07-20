# YunXi Agent v2.0.5-hotfix.2 复核审核报告

- 审核时间：2026-07-20 14:22:35 +08:00
- 审核版本：`v2.0.5-hotfix.2`
- 发布提交：`63155ce1fc8785f17dabd3f11babd159222445d5`
- annotated tag 对象：`578e0db14fb232c004b89d8eb4ac244a53518c32`
- 当前分支提交：`a2e7fc2688d6da17ea13813694cd78f08232a933`
- 审核目录：`D:\YunXi Agent`
- 审核依据：`C:\Users\24763\Desktop\YunXi Agent v2.0.0-v2.1.0 TUI与流式输出重构总纲图（复核版）.md` 的 `v2.0.5` 要求，以及用户于本次复核前确认的更新规则。

## 一、复核范围与规则修订

本报告复核并取代 `2026-07-20-140910-yunxi-agent-v2-0-5-hotfix-2-release-atomicity-audit-report.md` 中仅由发布顺序导致的“不通过”结论。

用户已明确：同一版本 tag 创建后，若仅追加仓库内 `docs-only` 状态、日志或报告提交，不再构成版本审核阻塞点。因此，该事项仅保留为发布过程观察项；不会否定已经通过的源码、测试、真实 Provider、ConPTY 或 TUI 验收。

本规则不豁免以下问题：tag 与版本号不一致、tag 后追加功能或测试代码、未验证的运行时行为变更、移动/删除历史 tag，或任何源码与交互层面的缺陷。

## 二、发布与源码完整性

- `v2.0.5-hotfix.2` 为 annotated tag，解析到发布提交 `63155ce`；本地保留 `v2.0.5`、`v2.0.5-hotfix.1` 与 `v2.0.5-hotfix.2` 的 tag 对象。
- `63155ce..HEAD` 的差异仅涉及 `docs/development-log.md` 和 `docs/reports/2026-07-20-131709-yunxi-agent-v2-0-5-conpty-native-install-reproducibility-remediation-development-report.md`。复核命令确认不存在 tag 后的非文档源码变更。
- 本版本 ConPTY 可复现性修复仍位于发布提交中：`scripts/conpty/v205/package.json` 固化 `node-pty` 原生安装许可；`README.md` 说明项目级依赖及复现边界；采集与校验脚本、脱敏帧和 manifest 均在项目内受版本控制。
- CodeGraph 复核的审批链路对批准、拒绝和取消具有独立分支。`McpToolApprovalDecision` 的 `Decline` 与 `Cancel` 不会写入批准状态；该结论与实际 ConPTY 交互记录一致。

## 三、验证结果

| 验证项 | 本次结果 |
| --- | --- |
| `cargo fmt --all -- --check` | 通过 |
| `cargo check --workspace` | 通过 |
| `cargo test --workspace` | 通过，所有工作区测试与 doc tests 无失败 |
| `cargo build -p yunxi-agent-cli --release --bins` | 通过 |
| `target\\release\\yunxi.exe --version` | 返回 `yunxi 2.0.5` |
| `target\\release\\yunxi.exe eval companion --json` | `31/31` 通过，`golden_passed=true`，`tool_approval_bypass_count=0` |
| `npm.cmd run verify --prefix scripts/conpty/v205` | 通过，8 个 ConPTY 场景的 manifest、哈希与脱敏校验一致 |
| `git diff --check 63155ce..HEAD` | 通过 |

此前已独立完成、且因本次确认无功能代码变动而继续有效的真实 Windows ConPTY / DeepSeek live 视觉与交互证据覆盖：`responsive`、`decline`、`approve`、`cancel`、`nonzero`、`invalid`、`binary`、`long` 八个场景；80、100、120、200 列下无控件重叠或越界；审批默认拒绝，批准、拒绝和 Ctrl+C 均只更新同一条 ToolActivity；非法 UTF-8、二进制、长输出和非零退出可正确呈现。

## 四、观察项

`a2e7fc2` 位于 `v2.0.5-hotfix.2` tag 之后，内容为发布状态记录，属于 `docs-only` 提交。依照用户最新规则，本项不构成阻塞，不要求为此创建新的 hotfix tag，也不影响当前版本验收。

## 五、审核结论

**v2.0.5-hotfix.2 审核通过，可进入 v2.0.6 开发。**

当前版本在源码、构建、测试、陪伴能力评估、工具审批安全、真实 Provider、ConPTY 证据以及 TUI 视觉与交互层面均无阻塞项。此前报告中的发布原子性“不通过”结论不再有效；本报告是该版本当前有效的最终审核结论。

## 六、下一阶段开发报告撰写指向

本节不是 `v2.0.5-hotfix.2` 的验收项。后续开发报告应依据总纲图进入 `v2.0.6`，主题为 Composer、输入恢复与对话一致性：

- 主要接入文件：`crates/yunxi-agent-tui/src/bottom_pane.rs`、`app.rs`、`render.rs`、`streaming.rs`、`host.rs`，以及 `crates/yunxi-agent-cli/src/interactive.rs`。
- 以结构化 edit buffer 和 Grapheme index 处理光标；覆盖 Windows IME 已提交文本、CRLF、多行粘贴、长粘贴、流式输出中取消与等待状态。
- 使 final response 回写同一条 assistant cell，避免 UI 层产生重复回答；渲染和输入状态必须保持一致。
- 参考 Codex composer 的 active-view 优先级与 Aider 的简洁输入节奏。参考逻辑应以 YunXi 的 `crossterm + ratatui` Rust 模块复刻，不直接迁移非 Rust UI 实现。

署名：审核者
