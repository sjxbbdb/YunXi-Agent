# YunXi Agent v1.7.1 Codex TUI Parity Implementation Plan

生成时间：2026-07-12 20:18:00 +08:00

## 目标

基于 `docs/reports/2026-07-12-yunxi-agent-v1-7-1-codex-tui-parity-development-report.md`，把 YunXi Agent v1.7.0 的原型 TUI 升级为 Codex CLI 风格的自主 TUI 应用层。

本阶段重点是修复当前真实终端截图暴露的问题：

- reasoning delta 不能逐 token 刷屏。
- approval 不能穿透为 `approve? y/N:` 行式 prompt。
- 输入应由 TUI composer 接管，而不是 alternate screen 内继续调用 reedline。
- TUI 源码要进入 `D:\YunXi Agent` 并自主运行，不依赖 `D:\源码\codex`。
- plain、one-shot、`--json`、`--jsonl`、sessions/parity 子命令保持兼容。

## 硬性约束

- 构建阶段只做源码迁移和实现，不在单个点上反复测试或验证。
- 源码完成后再执行统一验证。
- 每个版本必须创建新 tag，本阶段为 `v1.7.1`，不得删除或移动旧 tag。
- GitHub 读写/推送必须走 REST API，不使用 `git push/fetch/ls-remote`。
- API key 可从 `C:\Users\admin\Desktop\api.txt` 读取，但不得输出、提交、写入日志。
- 每次工作结束必须追加 `C:\Users\admin\Desktop\YunXi Agent开发日志.md`。
- 发布后执行 `cargo clean`，并确认 `target` 已清理。

## 实施步骤

1. 新增 `crates/yunxi-agent-tui` crate。
2. 按 Codex TUI 的分层迁入 YunXi 自主 TUI 架构：
   - `host`：terminal guard、alternate screen、raw mode、bracketed paste、focus/resize 处理。
   - `app`：TUI 总状态、banner、transcript、bottom pane mode。
   - `chat`：history cell、reasoning 合并、assistant streaming tail、事件压缩。
   - `bottom_pane`：composer、approval overlay、request_user_input overlay、footer hint。
   - `streaming`：newline-gated markdown/source collector 与 token 合并策略。
   - `render`：ratatui frame render、header/transcript/bottom pane。
3. 改造 `yunxi-agent-cli`：
   - workspace 版本更新到 `1.7.1`。
   - TUI 模式不再使用 `ReedlineInput`。
   - `InteractiveRenderer` 增加 approval/user_input 接管接口。
   - TUI renderer 通过新 crate 渲染事件、approval、user_input。
   - plain renderer 保持原有输出语义。
4. 增加 TUI 单元测试覆盖：
   - reasoning token 合并。
   - assistant duplicate 抑制。
   - approval overlay 渲染。
   - composer 输入状态渲染。
5. 统一验证：
   - `cargo fmt`
   - `cargo fmt -- --check`
   - `cargo test`
   - `cargo check --workspace`
   - `cargo build -p yunxi-agent-cli --release --bins`
   - `yunxi --version`
   - plain interactive smoke
   - DeepSeek live smoke（隐私化）
   - dependency scan 确认无 `codex-*` 运行时依赖
   - secret scan
   - `git diff --check`
   - `codegraph sync`
   - PATH 安装/冒烟
6. GitHub API 发布：
   - commit 到 `master`。
   - 创建 annotated tag `v1.7.1`。
   - 通过 GitHub API 更新 `master` 和 tag refs。
   - 验证旧 tag 未删除/未移动。
7. 清理与日志：
   - `cargo clean`
   - 确认 `target_exists=False`
   - 追加桌面开发日志，记录改动文件、验证结果、commit/tag/API 推送、清理结果。
