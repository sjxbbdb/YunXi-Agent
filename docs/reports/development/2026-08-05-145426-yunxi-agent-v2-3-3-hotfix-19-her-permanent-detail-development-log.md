# YunXi Agent v2.3.3-hotfix.19 “她”常驻关系档案开发日志

时间：2026-08-05 14:54:26 +08:00

## 工作目标

既然 Web“她”页面默认展示关系档案，就删除已经失去意义的退出按钮、人物卡展开箭头和整套开关状态，让详情成为页面的常驻内容，同时保留人物卡质感、指针倾斜、表情切换和背景显影。

## 设计与实现

1. 使用 CodeGraph 定位关系档案的 DOM、状态、事件、键盘和样式链路。
2. 使用 21st MCP 检索常驻人物档案参考，仅采纳“展示内容不应伪装成弹层”的信息层级原则。
3. 使用 `design-taste-frontend`、`gsap-core` 与 `gsap-performance` 审核交互语义和动画性能。
4. 人物卡由 `<button>` 改为纯展示 `<div>`，移除 `aria-controls`、`aria-expanded` 和点击展开事件。
5. 删除人物卡右上角展开箭头及全部相关 CSS。
6. 删除关系档案关闭按钮、Esc 退出、焦点恢复、`openHerDetail`、`closeHerDetail`、`herDetailTimeline`、`lastHerSource` 和 `is-detail-open` 状态。
7. 关系档案改为默认可见的普通页面区域，直接参与“她”页面的统一入场动画。
8. 人物卡指针倾斜、景深层、形象与表情切换工具栏、光标背景显影保持不变。
9. 移动端继续直接显示关系档案，并隐藏被详情覆盖的左侧人物卡区域，避免无意义的重叠与页面滚动。
10. 工作区版本由 `2.3.3-hotfix.18` 提升到 `2.3.3-hotfix.19`。

## 修改路径

- `D:\YunXi Agent\Cargo.toml`
- `D:\YunXi Agent\Cargo.lock`
- `D:\YunXi Agent\crates\yunxi-agent-cli\src\web.rs`
- `D:\YunXi Agent\crates\yunxi-agent-cli\src\web\index.html`
- `D:\YunXi Agent\crates\yunxi-agent-cli\src\web\app.js`
- `D:\YunXi Agent\crates\yunxi-agent-cli\src\web\app.css`
- `D:\YunXi Agent\docs\reports\development\2026-08-05-145426-yunxi-agent-v2-3-3-hotfix-19-her-permanent-detail-development-log.md`

## 测试与审核

- `node --check crates/yunxi-agent-cli/src/web/app.js`：通过。
- `cargo fmt --all -- --check`：通过。
- `git diff --check`：通过，仅保留仓库既有 LF/CRLF 转换提示。
- `cargo test -p yunxi-agent-cli static_assets_are_wired_to_api_routes`：通过。
- `cargo test`：全仓通过，0 项失败，保留 1 项既有 Windows 条件性 ignored 测试。
- `cargo build --release -p yunxi-agent-cli`：通过。
- Playwright + 系统 Edge 验证桌面 `1294 x 912` 和移动端 `390 x 844`。
- 两个视口均确认关闭按钮与展开箭头不存在，人物卡标签为 `DIV`，详情 `opacity=1`、`visibility=visible`、`pointer-events=auto`，且不存在 `is-detail-open` 状态。
- 桌面验证人物卡指针倾斜、背景显影、形象与表情切换正常。
- 两个视口的文档横向与纵向溢出均为 0，控制台与 page error 为空。
- 测试证据：`D:\YunXi Agent\target\visual-evidence\hotfix19-desktop.png` 与 `hotfix19-mobile.png`。

## 安装

- 安装目录：`D:\Apps\YunXi Agent\bin`。
- 回滚备份：`D:\Apps\YunXi Agent\bin\backup-20260805-145202-her-permanent-detail`。
- 安装版 `yunxi.exe` SHA-256：`85597C69AF26159A99CB8B9B961096C991334BF05443C4D3AE47FFCC565C3F2D`。
- 安装版 `yunxi-agent-cli.exe` SHA-256：`A0FC1FAD89F5544A506BB7DB8968C18AFE15B20A82E8F0810B98669CE2135953`。
- 两个安装二进制与 Release 包逐文件哈希一致，版本命令均返回 `2.3.3-hotfix.19`。
- 正式 Web 监听 `127.0.0.1:17861`，`/api/health` 返回 `status=ok`、`version=2.3.3-hotfix.19`。
- 正式进程参数包含 `--no-weixin-autostart`，本轮未创建或重启微信网关。

## 安全边界

未执行目录移动、递归清理、用户目录删除、`git clean`、工作区重置、force push 或历史 tag 删除。人物背景图、角色卡图、表情图、权威灵魂、长期记忆、人格和微信状态均未改写。安装替换前已创建可回滚备份并逐文件校验。

署名：开发者
