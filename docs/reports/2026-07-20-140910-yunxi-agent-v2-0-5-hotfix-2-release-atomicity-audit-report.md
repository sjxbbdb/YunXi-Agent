# YunXi Agent v2.0.5-hotfix.2 发布原子性审核报告

- 审核时间：2026-07-20 14:09:10 +08:00
- 审核版本：`v2.0.5-hotfix.2`
- 发布提交：`63155ce1fc8785f17dabd3f11babd159222445d5`
- annotated tag object：`578e0db14fb232c004b89d8eb4ac244a53518c32`
- 审核目录：`D:\YunXi Agent`
- 审核依据：`C:\Users\24763\Desktop\YunXi Agent v2.0.0-v2.1.0 TUI与流式输出重构总纲图（复核版）.md` 的 `v2.0.5`、发布纪律和审核口径。
- 审核结论：**不通过。不得进入 v2.0.6 或其他下一版本开发，必须先完成当前版本发布原子性整改。**

## 一、功能与证据审核结果

`v2.0.5-hotfix.2` 已解决上次 ConPTY 可复现性问题：

- `scripts/conpty/v205/package.json` 已纳入 `allowScripts.node-pty@1.1.0=true`。
- `scripts/conpty/v205/README.md` 已补充原生依赖许可和重现边界。
- 审核者在项目 `.tmp` 下建立唯一临时安装副本，执行干净 `npm ci` 后无 pending-script 警告，且成功加载 `node-pty` 与 `@xterm/headless`。
- `npm run verify --prefix scripts/conpty/v205` 通过，8 个真实 ConPTY 场景的 SHA-256、交互检查点和凭据脱敏规则均成立。
- 上一轮独立真实 DeepSeek ConPTY 视觉验收的运行时代码未在 `hotfix.2` 中改变；80/100/120/200 列、审批批准/拒绝/Ctrl+C、无效 UTF-8、二进制、长输出和非零退出的视觉及交互结论继续有效。

审核者本次复跑验证：

| 验证项 | 结果 |
| --- | --- |
| `cargo fmt --all -- --check` | 通过 |
| `cargo check --workspace` | 通过 |
| `cargo test --workspace` | 通过，TUI 111/111；执行器 13/13；沙箱 8/8；doc tests 通过 |
| `cargo build --workspace` | 通过 |
| `cargo build -p yunxi-agent-cli --release --bins` | 通过 |
| `target\\release\\yunxi.exe --version` | 返回 `yunxi 2.0.5` |
| `target\\release\\yunxi.exe eval companion --json` | 31/31，`golden_passed=true`，`tool_approval_bypass_count=0` |
| `npm ci`（唯一临时目录） | 通过，原生依赖可加载 |
| `npm run verify --prefix scripts\\conpty\\v205` | 8 个 ConPTY 场景通过 |
| `git diff --check` | 通过 |

远程发布也已核验：`origin/master` 包含 hotfix 后续文档提交；远程存在且保留 `v2.0.5`、`v2.0.5-hotfix.1`、`v2.0.5-hotfix.2` 三个 annotated tag，`v2.0.5-hotfix.2` tag object 与本地一致。

## 二、阻断项

### P1：发布 tag 后追加 docs-only 提交，违反当前总纲图的一版本一发布提交纪律

总纲图明确要求：一个小版本只能有一个发布提交；全部源码、测试、版本号、README 和仓库内状态文档必须先纳入该提交，随后立即创建 annotated tag，且**之后不得以同一版本追加 docs-only 或 hotfix 提交**。

当前 `v2.0.5-hotfix.2` tag 指向 `63155ce`，但远程和本地 `master` 均在该 tag 之后追加了 `a2e7fc2`：

```text
a2e7fc2 docs: record v2.0.5 hotfix.2 release status
63155ce (tag: v2.0.5-hotfix.2) fix(conpty): make v2.0.5 evidence install reproducible
```

`v2.0.5-hotfix.2..HEAD` 的差异仅包括：

- `docs/development-log.md`
- `docs/reports/2026-07-20-131709-yunxi-agent-v2-0-5-conpty-native-install-reproducibility-remediation-development-report.md`

这正是总纲图禁止的 tag 后 docs-only 提交。即使运行时代码、测试和 ConPTY 证据均通过，发布提交与仓库内状态文档仍不原子，故不能判定当前版本通过。

## 三、当前版本整改要求

1. 不得移动、删除或覆盖 `v2.0.5`、`v2.0.5-hotfix.1`、`v2.0.5-hotfix.2`。
2. 开发者须将本应属于发布状态的仓库内日志、报告索引和状态文档与源码一起纳入一个新的当前版本修正发布提交。
3. 新修正版本/tag 名称必须先经用户确认；建议使用新的 hotfix 语义，而不是把 docs-only 提交伪装成 `v2.0.6` 的功能开发。
4. 新 tag 必须立即指向该唯一发布提交，再 non-force 推送；发布后不得再以该 tag 对应版本追加仓库内 docs-only 提交。
5. 桌面审核报告与桌面开发日志属于审核流程输出，不得作为开发者在 tag 后继续修改仓库内发布状态文档的理由。
6. 完成上述事项并复核远程 refs、工作树和 docs 同步后，才可重新提交当前版本审核；在审核通过前不得进入 `v2.0.6`。

## 四、下一阶段开发报告撰写指向

本节不是当前版本验收项。仅在当前修正版本通过审核后，后续开发报告应按总纲图进入 `v2.0.6 - Composer、输入恢复与对话一致性`：

- 将 `bottom_pane.rs` 的输入状态升级为结构化 edit buffer，cursor 使用 Grapheme index。
- 处理 Windows IME 已提交文本、CRLF/多行粘贴、长 paste 合并 redraw、流式中的取消/等待状态。
- 确保 final response 回写同一 assistant cell，避免 UI 层重复回答。
- 主要接入：`crates/yunxi-agent-tui/src/bottom_pane.rs`、`app.rs`、`render.rs`、`streaming.rs`、`host.rs`、`crates/yunxi-agent-cli/src/interactive.rs`。
- 参考 Codex composer 的 active-view 优先级和 Aider 的简洁输入节奏；非 Rust 参考只迁移逻辑，以 YunXi 的 `crossterm + ratatui` Rust 模块实现。

## 五、最终结论

`v2.0.5-hotfix.2` 的功能性、可复现性、自动化验证、真实 Provider 和真实 ConPTY 视觉审核均通过。本次不通过完全由发布纪律导致：tag 后仓库内 docs-only 提交违反总纲图的原子发布要求。

**当前版本审核不通过。必须完成发布原子性整改并重新审核；不得进入下一版本开发。**

署名：审核者
