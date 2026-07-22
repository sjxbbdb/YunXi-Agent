# Windows ConPTY 验证资产总览

本目录保存 YunXi Agent 的版本化 Windows ConPTY 采集器和离线 verifier。Node.js、`node-pty@1.1.0` 与 `@xterm/headless@5.5.0` 只用于证据采集/复核，不进入 YunXi 默认运行路径。

## 版本矩阵

| 目录 | 主要场景 | 采集入口 | 只读复核入口 | 本地生成目录 | 正式 evidence |
| --- | --- | --- | --- | --- | --- |
| [`v205`](v205/README.md) | 响应式布局、Approval 同意/拒绝/取消、非零退出、无效 UTF-8、二进制与长输出 | `npm.cmd run capture --prefix scripts\conpty\v205` | `node scripts\conpty\v205\verify.js` | `v205/node_modules/`、`v205/.work/` | [`v205-conpty`](../../docs/reports/evidence/frames/v205-conpty/) |
| [`v206`](v206/README.md) | Composer、长粘贴、CRLF、IME、stream cancel、Approval draft 恢复、单一 final cell | `npm.cmd run capture --prefix scripts\conpty\v206` | `node scripts\conpty\v206\verify.js` | `v206/node_modules/`、`v206/.work/` | [`v206-conpty`](../../docs/reports/evidence/frames/v206-conpty/) |
| [`v207`](v207/README.md) | v2.0.7 Composer/streaming 一致性与交互焦点发布门禁 | `npm.cmd run capture --prefix scripts\conpty\v207` | `node scripts\conpty\v207\verify.js` | `v207/node_modules/`、`v207/.work/` | [`v207-conpty`](../../docs/reports/evidence/frames/v207-conpty/) |
| [`v207-hotfix`](v207-hotfix/README.md) | Approval viewport freeze、Details 独立滚动和 58x18 焦点恢复 | `npm.cmd run capture --prefix scripts\conpty\v207-hotfix` | `node scripts\conpty\v207-hotfix\verify.js` | `v207-hotfix/node_modules/`、`v207-hotfix/.work/` | [`v207-hotfix-conpty`](../../docs/reports/evidence/frames/v207-hotfix-conpty/) |
| [`v208`](v208/README.md) | responsive density 与 `NO_COLOR` semantic low-color | `npm.cmd run capture --prefix scripts\conpty\v208` | `node scripts\conpty\v208\verify.js` | `v208/node_modules/`、`v208/.work/` | [`v208-conpty`](../../docs/reports/evidence/frames/v208-conpty/) |
| [`v209`](v209/README.md) | 非 TUI mode matrix、终端恢复、Provider recovery、长流取消后下一轮 | `npm.cmd run capture --prefix scripts\conpty\v209 -- --output-dir .tmp\conpty\v209-capture` | `node scripts\conpty\v209\verify.js` | `v209/node_modules/`、显式 `.tmp/` output/`.work/` | [`v209-conpty`](../../docs/reports/evidence/frames/v209-conpty/) |
| [`v210`](v210/README.md) | v2.1.0 集成发布、v209 baseline、宽字符/resize/mouse/copy、Rust golden 绑定 | `npm.cmd run capture --prefix scripts\conpty\v210 -- --output-dir .tmp\conpty\v210-release` | `node scripts\conpty\v210\verify.js` | `v210/node_modules/`、显式 `.tmp/` output/work | [`v210-conpty`](../../docs/reports/evidence/frames/v210-conpty/) |

## 依赖与版本化边界

- 每个版本目录自己的 `package.json`、`package-lock.json`、采集/复核脚本和 README 是版本化资产。
- 所有版本固定使用 `node-pty@1.1.0` 与 `@xterm/headless@5.5.0`；原生安装脚本许可仅存在于对应项目级 `package.json`。
- `node_modules/`、`.work/` 与 `.tmp/` 是可再生产物，由根 `.gitignore` 统一忽略，永远不提交。
- v205–v208 的 capture 维护对应正式 evidence；v209/v210 的 capture 只写显式 `.tmp` 输出，默认 verifier 对正式 evidence 只读。
- Provider 凭据只能由采集进程临时继承，不得进入脚本、证据、manifest、日志或 Git。

## 标准复核

从仓库根目录直接运行 verifier，可以避免 npm 对用户级缓存的潜在写入：

```powershell
node scripts\conpty\v205\verify.js
node scripts\conpty\v206\verify.js
node scripts\conpty\v207\verify.js
node scripts\conpty\v207-hotfix\verify.js
node scripts\conpty\v208\verify.js
node scripts\conpty\v209\verify.js
node scripts\conpty\v210\verify.js
```

上述命令只读取已归档 evidence。需要重新采集时，先阅读对应版本 README，并将 v209/v210 输出显式指向仓库内 `.tmp/`；正式 evidence 只在受控发布收口中更新。

## 共享代码纪律

当前不创建 `shared/`，也不改写旧采集器。只有当至少两套脚本以测试证明能够共享同一行为、且所有既有 verifier 继续通过后，才在独立提交中抽取共用辅助函数。

署名：开发者
