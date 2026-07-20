# YunXi Agent v2.0.5-hotfix.1 ConPTY 独立复审报告

- 审核时间：2026-07-20 12:48:13 +08:00
- 审核版本：`v2.0.5-hotfix.1`
- 发布提交：`7d6c18b73a1f4a0d4ec9b7cc64ed501e76f72bf4`
- annotated tag object：`74053c8ad44bcea463a3fd08422510abbff20768`
- 审核目录：`D:\YunXi Agent`
- 审核依据：`v2.0.5` 总纲图、`2026-07-20-113410` 整改复审报告与当前 hotfix 发布内容。
- 审核结论：**不通过。不得进入下一版本开发；当前仅剩 ConPTY 采集器可复现性发布门禁。**

## 一、已通过项

### 1. 发布、源码和自动化验证

- `v2.0.5-hotfix.1` 是新的 annotated tag，旧 `v2.0.5` tag 未移动。
- GitHub 远程已核验：`master` 指向 `7d6c18b`；远程存在 `v2.0.5` 与 `v2.0.5-hotfix.1`，后者 tag object 与本地一致。
- `cargo fmt --all -- --check`、`cargo check --workspace`、`cargo test --workspace`、`cargo build --workspace`、release CLI 构建全部通过。
- TUI 为 111/111；执行器为 13/13；沙箱验收为 8/8；workspace 及 doc tests 无失败。
- release 二进制返回 `yunxi 2.0.5`；`eval companion --json` 为 31/31、`golden_passed=true`、`tool_approval_bypass_count=0`。
- `ErrorPresentation` 的 provider/tool/approval/cancel/terminal/unknown 六类稳定 code、`retryable`、`next:` 与 details/debug 隔离均由事件层集成测试覆盖。

### 2. 审核者独立 ConPTY 在线视觉与交互复验

审核者按项目内 `scripts/conpty/v205` 的锁定依赖、采集器和真实 DeepSeek 配置独立运行 8 个 Windows ConPTY 场景。采集器退出成功，并重新生成逐场景 JSON 帧和 SHA-256 manifest；随后 `npm run verify --prefix scripts/conpty/v205` 通过，验证了哈希、必需检查点、必需文本和凭据脱敏规则。

实测视觉与交互结论：

- 80x24：仅显示 YunXi、live Provider 和必要状态；transcript 与 composer 完整可见。
- 100x30：显示产品、Provider 和 model；无悬挂分隔符或区域重叠。
- 120x40、200x50：显示 model 与紧凑工作区路径；layout、滚动和 composer 无重叠。
- 审批 pane：默认焦点为 Decline；明确显示 cwd、风险、命令、Approve/Decline 与快捷键；其高度和 transcript 区域不冲突。
- 拒绝：工具不执行，同一 cell 显示 `YX-APPROVAL-001`、`retryable=no`、安全下一步；后续输入成功。
- 批准：同一 cell 更新为 completed，普通视图仅显示安全输出摘要；没有重复 approved/declined notice。
- Ctrl+C：同一 activity 由 `declined` 精确更新为 `cancelled`，显示 `YX-CANCEL-001`；工具未执行，下一轮输入成功。
- 非零退出：显示 `YX-TOOL-001`，会话可继续输入。
- 无效 UTF-8、二进制：普通视图没有原始字节或 UTF-8 解码异常；details 显示 `replacement_count=1` 与 `integrity=Lossy`。
- 2000 行输出：普通视图保持紧凑；details 显示 `original_bytes=26000`、`truncated=true`、`integrity=Partial`，未破坏继续输入。

本次独立采集的 8 个场景为 `responsive`、`decline`、`approve`、`cancel`、`nonzero`、`invalid`、`binary`、`long`。它们全部由真实 Windows ConPTY 子进程和 DeepSeek live 会话产生，非单元测试、非手工注入事件。

## 二、阻断项

### P1：已发布 hotfix 不可由 README 的安装步骤直接复现 ConPTY 采集

`v2.0.5-hotfix.1` 已提交的 `scripts/conpty/v205/package.json` 未声明 npm 的 `allowScripts`。在本机执行 README 所列的 `npm ci --prefix scripts\\conpty\\v205` 时，npm 报告 `node-pty@1.1.0` 的 `node-gyp rebuild` 仍为 pending，原生 ConPTY binding 不会可用。

本次审核者必须在用户明确授权后执行 `npm approve-scripts node-pty --prefix scripts\\conpty\\v205` 才能运行真实采集。该命令将下列配置写入本地工作树，但它不在 `v2.0.5-hotfix.1` tag 中：

```json
"allowScripts": {
  "node-pty@1.1.0": true
}
```

因此，当前已发布 tag 的 README 重现步骤不完整。新的审核者仍会在安装原生依赖时被阻断，不能满足“审核者可独立重演 ConPTY 视觉帧”的要求。

**当前版本整改要求：**

1. 将 `scripts/conpty/v205/package.json` 中的 `allowScripts.node-pty@1.1.0=true` 纳入提交。
2. 更新 `scripts/conpty/v205/README.md`，明确说明该本地原生依赖允许步骤及其安全边界；不得把它伪装为系统级安装。
3. 以新的当前版本 hotfix 提交、annotated tag 和 non-force 推送发布该可复现性修正。`v2.0.5-hotfix.1` 不得移动、删除或覆盖；新 tag 名称须经用户确认。
4. 新 tag 发布后，在干净的项目级 Node 安装环境中执行 README 中的完整重现步骤，再申请最终复审。

## 三、参考源码与整改建议

本次无需再修改 ToolActivity、审批或 ExecOutputDecoder 的功能性实现。当前整改只涉及采集器的发布可复现性：

- `scripts/conpty/v205/package.json` 与 `package-lock.json`：作为 `node-pty` / `@xterm/headless` 的锁定本地依赖边界。
- `scripts/conpty/v205/capture-scenario.js`：继续以 Windows ConPTY 和真实 Provider 驱动，不得改为伪造 TUI 事件。
- `scripts/conpty/v205/verify.js`：保持 SHA-256、完整检查点和 secret-like token 过滤，作为重跑后的离线验收入口。
- Codex `bottom_pane/approval_overlay.rs` 与 k9s 的事件压缩/详情展开逻辑：作为已完成的审批焦点和单 cell 呈现设计参考，不需要在本次仅文档/依赖修正中重复迁移。

## 四、工作树状态

本次独立采集按设计更新了已提交的脱敏 JSON 帧和 manifest；npm 本地批准也修改了 `scripts/conpty/v205/package.json`。这些均是审核过程生成的未提交项目内变更，未删除、未回退。`master` 另外包含一个尚未推送的发布状态文档提交，但 `origin/master` 和 `v2.0.5-hotfix.1` 已正确指向发布源码提交。

未执行递归删除、强制移动、目录清空、系统配置修改或历史 tag 改写。

## 五、最终结论

`v2.0.5-hotfix.1` 的 TUI 功能、错误呈现、真实 Provider、真实 ConPTY 视觉与审批交互均通过审核者独立复验。当前不通过只因发布 tag 中缺少本地 native dependency 的可复现性配置和完整 README 步骤。

**开发者必须先完成上述 P1 可复现性发布整改并重新审核；不得进入下一版本开发。**

署名：审核者
