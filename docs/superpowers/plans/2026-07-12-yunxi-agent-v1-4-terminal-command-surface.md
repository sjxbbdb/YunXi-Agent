# YunXi Agent v1.4 Terminal Command Surface Implementation Plan

生成时间：2026-07-12 +08:00

## 硬性约束

- 构建阶段不做零散测试，不在单点问题上反复卡住。
- 所有源码改造完成后统一验证。
- 每个版本发布为新 tag，本轮 tag 为 `v1.4.0`，旧 tag 不删除、不移动。
- GitHub 远端读写全部走 REST API。
- API key、PAT 和 credential 内容不输出、不写日志、不提交。
- 发布后清理构建产物并确认 `target_exists=False`。
- 每次任务结束同步 `C:\Users\admin\Desktop\YunXi Agent开发日志.md`。

## 任务

1. 版本与文档准备
   - 将 workspace 版本升级到 `1.4.0`。
   - 更新 CLI about/banner 文案。
   - 更新 README、extraction status 和本报告。

2. 交互命令解析
   - 在 `commands.rs` 增加 `/tools`、`/mcp`、`/cost`、`/status`。
   - 更新 help 文案。

3. 交互 session 状态
   - 在 `interactive.rs` 增加 `TurnSummary`、`UsageTotals`、`InteractiveStats`。
   - 每轮完成后从 `AgentRunResult.events` 汇总状态。
   - `/cost` 和 `/status` 输出最近一轮与累计状态。

4. 工具与 MCP 可见性
   - CLI 依赖 YunXi-owned `yunxi-agent-tools` 和 `yunxi-agent-mcp`。
   - `/tools` 打印固定工具与 workspace 动态工具。
   - `/mcp` 打印 workspace MCP 配置和 fixture seed 状态。

5. 测试资产更新
   - 更新 v1.4.0 版本断言。
   - 增加 interactive `/tools`、`/mcp`、`/cost`、`/status` smoke 测试。

6. 统一验证、发布与清理
   - 运行完整验证门。
   - 安装 PATH 版。
   - 通过 GitHub REST API 发布 master 和 tag。
   - 同步 CodeGraph。
   - 写桌面日志。
   - 执行 `cargo clean`。
