# YunXi Agent v2.0.5 ConPTY 原生安装可复现性整改开发报告

- 开发时间：2026-07-20 13:17:09 +08:00
- 当前版本：`v2.0.5-hotfix.1` 后续整改候选
- 审核依据：`C:\Users\24763\Desktop\YunXi Agent审核报告\2026-07-20-124813-YunXi-Agent-v2.0.5-hotfix.1-ConPTY独立复审报告.md`
- 审核结论：不通过；不得进入下一版本开发，必须先关闭 ConPTY 采集器可复现性发布门禁。
- 开发目录：`D:\YunXi Agent`

## 一、不可压缩的硬性要求

1. `scripts/conpty/v205/package.json` 必须提交 `allowScripts.node-pty@1.1.0=true`。
2. `scripts/conpty/v205/README.md` 必须明确本地原生依赖许可步骤和安全边界，不得伪装为系统级安装。
3. 必须在干净的项目级 Node 安装环境中执行 README 的完整重现步骤。
4. 必须使用新的当前版本 hotfix 提交、annotated tag 和 non-force 推送发布整改。
5. `v2.0.5-hotfix.1` 与全部历史 tag 不得移动、删除或覆盖。
6. 新 tag 名称必须由用户明确确认后才能创建。
7. 不修改已经通过审核的 ToolActivity、审批、错误呈现和 ExecOutputDecoder 功能。
8. 审核者生成的脱敏帧、manifest、`package.json` 和审核报告不得擅自回退。

## 二、整改实现

审核者已通过 `npm approve-scripts node-pty` 将以下项目级许可写入未提交的 `package.json`，本次开发保留并纳入整改候选：

```json
"allowScripts": {
  "node-pty@1.1.0": true
}
```

`package-lock.json` 继续精确锁定 `node-pty 1.1.0`、`@xterm/headless 5.5.0`、`node-addon-api 7.1.1` 和完整性哈希。README 新增 native binding 加载检查，并明确：

- 许可只覆盖项目目录中的 `node-pty@1.1.0` install script；
- 不安装全局包或 Windows 服务；
- 不修改 PATH、注册表或系统配置；
- 如果没有可用预构建，npm 只会在项目目录使用本地编译工具链；
- pending/blocked 必须视为安装失败，不得依赖未提交的额外本地批准绕过。

## 三、干净安装与真实重演

从项目根目录执行：

```powershell
npm ci --prefix scripts\conpty\v205
node -e "require('./scripts/conpty/v205/node_modules/node-pty'); console.log('node-pty binding ready')"
npm run capture --prefix scripts\conpty\v205
npm run verify --prefix scripts\conpty\v205
```

`npm ci` 在干净项目级安装环境中直接安装三个锁定依赖，没有执行单独的 `npm approve-scripts`。native binding 随后成功加载并输出 `node-pty binding ready`。

普通命令沙箱禁止外网，首次完整 capture 与单独 responsive 重跑均在 Provider 请求处显示安全的 `YX-PROVIDER-001`；同一 no-TUI 请求在沙箱内返回 network failure，在获准真实网络环境立即返回 `YUNXI_PROVIDER_DIAGNOSTIC_OK`。因此正式在线证据只采用获准真实网络环境的完整重跑。

八场景从 `2026-07-20 13:13:48 +08:00` 至 `13:14:37 +08:00` 全部通过。新 manifest 生成于 `2026-07-20T05:14:37.825Z`，场景为 responsive、decline、approve、cancel、nonzero、invalid、binary、long，checkpoint 数分别为 7、6、6、6、6、8、8、9。离线 verifier 返回 `ok=true, scenarios=8`，重新核验全部 SHA-256、交互检查点、错误 code、decoder 元数据、后续输入和凭据脱敏。

## 四、统一门禁结果

- `node --check`：三个 collector JavaScript 文件全部通过。
- native `node-pty` binding：加载通过。
- `npm run verify --prefix scripts\conpty\v205`：8/8 通过。
- `cargo fmt --all -- --check`：通过。
- `cargo check --workspace`：通过。
- `cargo test --workspace`：通过；TUI 111/111、执行器 13/13、沙箱 8/8，其他 crate、集成测试和 doc tests 无失败。
- `cargo build --workspace`：通过。
- `cargo build -p yunxi-agent-cli --release --bins`：通过。
- `yunxi.exe --version` 与 `yunxi-agent-cli.exe --version`：均为 `yunxi 2.0.5`。
- Evaluation Harness：31/31、`golden_passed=true`、memory precision 1.0、tool approval bypass 0。
- `git diff --check`：通过，无空白错误。

## 五、发布边界

本轮仍属于 v2.0.5 当前版本整改，不进入下一版本。用户已于 `2026-07-20 13:29:45 +08:00` 明确确认新 annotated tag 使用 `v2.0.5-hotfix.2`，并授权清理五个精确运行目录。

清理前将每个目标解析为绝对路径并确认位于 `D:\YunXi Agent` 内；随后删除 `target`、`scripts\conpty\v205\node_modules`、`scripts\conpty\v205\.work`、根目录 `.yunxi` 与 `crates\yunxi-agent-cli\.yunxi`，五项目标均经 `Test-Path` 核验为不存在。collector 源码、`package-lock.json`、审核报告、正式 evidence、八份新脱敏帧和 manifest 均保留。

当前本地 `master` 还包含上一轮未推送的 docs-only `16024ce`，本轮整改文件待创建唯一整改提交和 `v2.0.5-hotfix.2`。发布必须采用 non-force 推送，核验远程 `master`、新 tag、`v2.0.5-hotfix.1` 及全部历史 refs。GitHub API key 不得打印或写入仓库、Git 配置、remote URL、报告或日志。

署名：开发者
