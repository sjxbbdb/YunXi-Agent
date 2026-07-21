# YunXi Agent v2.0.7-hotfix ConPTY Evidence

- 采集时间：2026-07-21 09:50:26-09:50:56 +08:00（manifest 使用 UTC）
- 版本：`2.0.7-hotfix`
- Provider：真实 `DeepSeek` / `deepseek-chat`
- 终端：真实 Windows ConPTY，经 `node-pty@1.1.0` 与 `@xterm/headless@5.5.0`
- 采集器：`D:\YunXi Agent\scripts\conpty\v207-hotfix\capture.js`
- 离线验证：`npm.cmd run verify --prefix scripts\conpty\v207-hotfix` 返回
  `ok=true, scenarios=2`

## 场景

### `approval-freeze`

独立真实会话先生成可滚动 transcript，再打开 Approval。采集器发送
SGR wheel-up、wheel-down、scrollbar left-click、left-drag 和 mouse-up，
然后在 120x40 与 58x18 终端记录帧。`approval-after-mouse` 的
`transcript_unchanged=true`；Approval footer 只显示 `Tab select`、
`Enter confirm`、`Esc decline`、`Ctrl+C cancel`，不宣称可滚动历史。

- evidence SHA-256：`8c73c76fc340b5884cd190b7953c6353bead406e550bd1c05e989168837c288a`
- output bytes：`162369`
- checkpoints：`12`

### `details-scroll`

独立真实会话打开 debug 事件，执行安全的项目内 PowerShell fixture，生成
160 行 `YUNXI_HOTFIX_DETAIL_LINE_*` 工具输出，使用真实 transcript 的
`[debug] #27 shell result` 引用打开 Details。鼠标 wheel 与 PgDown 均只
改变 Details 内容；随后调整到 58x18，Esc 关闭并恢复 Composer。证据字段
为 `details_changed=true`、`transcript_anchor_untouched=true`、
`transcript_anchor_restored=true`。

- evidence SHA-256：`42204e981a26ebe44fa73573ece18a3b4f0f1d9862716b400a0c843cceecea91`
- output bytes：`65501`
- checkpoints：`19`

## Manifest

- 路径：`D:\YunXi Agent\docs\reports\evidence\frames\v207-hotfix-conpty\manifest.json`
- SHA-256：`E5D6C9C4E35DE3BB4E30E29D7136D600DC1C865508CCA3894511A54320F40BB7`
- schema version：`2`
- 场景数：`2`
- 凭据处理：Provider 凭据仅由 ConPTY 子进程继承；帧、evidence 和 manifest
  已执行 secret-like pattern 扫描，未写入 API key。

历史证据未回退：`scripts\conpty\v207` verifier 返回
`ok=true, scenarios=8`，`scripts\conpty\v206` verifier 返回
`ok=true, scenarios=8`。正式证据目录保留原 v206/v207 文件，hotfix 使用
独立 `v207-hotfix-conpty` 目录。

署名：开发报告撰写者
