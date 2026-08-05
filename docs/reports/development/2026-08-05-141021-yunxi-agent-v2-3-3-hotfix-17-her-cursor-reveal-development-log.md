# YunXi Agent v2.3.3-hotfix.17 “她”背景光标揭示开发日志

时间：2026-08-05 14:10:21 +08:00

## 工作目标

将 Web“她”页面改为深色主界面：默认不显示真实背景人物图，只有光标经过的局部区域才显现原背景；停止移动后短暂停留并平滑恢复深色。人物背景、角色卡和表情图片均不替换。

## 设计与实现

1. 使用 CodeGraph 复核 `setupHerReveal`、背景双图层、流光层和静态资源测试。
2. 使用 21st MCP 的 Cursor Spotlight、Torch Reveal 和 Spotlight Background 作为交互参考，不复制第三方代码。
3. 使用 `gsap-core` 与 `gsap-performance` 审核光标跟随和平滑性能，继续复用已有 `gsap.quickTo`，不新增每帧对象或布局属性动画。
4. 默认背景改为接近纯黑的冷灰黑渐变；原背景基础图层固定为 `opacity: 0`。
5. 真实背景揭示层使用半径 `15.5rem` 的柔边径向 mask，光标位置通过 `--spot-x` 和 `--spot-y` 同步。
6. 流光层默认隐藏，并使用半径 `18rem` 的同心 mask；只有光标揭示时才显示，避免默认黑场漏光。
7. 光标停止后保留约 900 ms，再用 1.45 s 退回黑场。
8. 关系档案关闭态与展开态共用同一揭示逻辑，正文始终位于背景之上。
9. 移动端、减少动态效果和减少透明度路径保持稳定深色，不强制播放光标动画。
10. 原图片路径继续使用 `/assets/yunxi-her-background.jpg`，图片文件未修改。
11. Web 静态测试新增基础图层与流光默认透明、流光 mask 和揭示状态选择器断言。
12. 工作区版本由 `2.3.3-hotfix.16` 提升到 `2.3.3-hotfix.17`。

## 修改路径

- `D:\YunXi Agent\Cargo.toml`
- `D:\YunXi Agent\Cargo.lock`
- `D:\YunXi Agent\crates\yunxi-agent-cli\src\web.rs`
- `D:\YunXi Agent\crates\yunxi-agent-cli\src\web\app.css`
- `D:\YunXi Agent\docs\reports\development\2026-08-05-141021-yunxi-agent-v2-3-3-hotfix-17-her-cursor-reveal-development-log.md`

## 测试与审核

- `node --check crates/yunxi-agent-cli/src/web/app.js`：通过。
- `cargo fmt --all -- --check`：通过。
- `git diff --check`：通过，仅保留仓库既有 LF/CRLF 转换提示。
- `cargo test -p yunxi-agent-cli web::tests::static_assets_are_wired_to_api_routes`：通过。
- `cargo test`：全仓通过；0 项失败，保留 1 项既有 Windows 条件性 ignored 测试。
- `cargo build --release -p yunxi-agent-cli`：通过。
- Playwright + 系统 Edge 在 `1294 x 912` 下验证默认黑场、光标揭示、超时退黑、档案展开黑场和档案展开揭示五种状态。
- 默认状态基础图、揭示图和流光可见度分别为 `0 / 0 / 0`；揭示状态分别为 `0 / 1 / 0.72`；退黑后恢复 `0 / 0 / 0`。
- 光标测试点 `1000 x 420` 和档案展开测试点 `1000 x 350` 均与两个径向 mask 中心一致。
- 原背景图成功加载，名称仍完整显示，document 无横向溢出，控制台与 page error 为空。
- 测试证据：`D:\YunXi Agent\.tmp\web-hotfix17`。

## 安装

- 安装目录：`D:\Apps\YunXi Agent\bin`。
- 回滚备份：`D:\Apps\YunXi Agent\bin\backup-20260805-140806-her-cursor-reveal`。
- 安装版 `yunxi.exe` SHA-256：`28A423F229CB216BB5EA6185E2CB0A568E66A9006664FC0811A1633355E6A71D`。
- 安装版 `yunxi-agent-cli.exe` SHA-256：`7D339DB04D4BCD077B5F7CD0C445D556EAB3F748D7995BAA07B2569E25B1454E`。
- 安装二进制与 release 逐文件哈希一致，`yunxi --version` 返回 `2.3.3-hotfix.17`。
- 正式 Web 监听 `127.0.0.1:17861`，`/api/health` 返回 `status=ok`、`version=2.3.3-hotfix.17`。
- 正式进程参数包含 `--no-weixin-autostart`，本轮未创建或重启微信网关。

## 安全边界

未执行目录移动、递归清理、用户目录删除、`git clean`、工作区重置、force push 或历史 tag 删除。人物背景图、角色卡图、表情图、权威灵魂、长期记忆、人格和微信状态均未改写。安装替换前已创建可回滚备份并逐文件校验。

署名：开发者
