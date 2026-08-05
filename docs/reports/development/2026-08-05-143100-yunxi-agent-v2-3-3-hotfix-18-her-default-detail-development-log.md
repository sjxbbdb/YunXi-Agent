# YunXi Agent v2.3.3-hotfix.18 “她”默认关系档案与通透背景开发日志

时间：2026-08-05 14:31:00 +08:00

## 工作目标

让 Web“她”页面默认展示关系档案详情，同时提高详情区域的通透度，使光标揭示的真实人物背景清晰可见，并继续保证文字阅读、角色卡操作和移动端布局稳定。

## 设计与实现

1. 使用 CodeGraph 复核 `showView`、`openHerDetail`、`closeHerDetail`、关系档案状态与透明面板样式。
2. 使用 21st MCP 检索透明编辑式档案和深色玻璃参考，仅采用分层透明与可读性原则，不复制第三方代码。
3. 使用 `gsap-core` 与 `gsap-performance` 审核默认展开和光标跟随动画，继续只使用既有 transform、opacity 和 `quickTo` 路径。
4. 进入“她”页面时自动打开关系档案，不自动把键盘焦点移到关闭按钮。
5. 用户可以通过右上角按钮关闭详情；离开“她”页面再返回时重新恢复默认展开。
6. 详情面板由高不透明深色背景改为从 `82%`、`62%` 到右侧 `18%` 的横向暗衬，人物所在右侧最通透。
7. 降低详情面板边界和阴影强度，不增加大面积模糊；正文增加轻量文字阴影保证背景显现时仍可阅读。
8. 移动端继续使用更稳定的实色详情背景，默认详情完整铺开并保留可见的关闭按钮。
9. 工作区版本由 `2.3.3-hotfix.17` 提升到 `2.3.3-hotfix.18`。

## 修改路径

- `D:\YunXi Agent\Cargo.toml`
- `D:\YunXi Agent\Cargo.lock`
- `D:\YunXi Agent\crates\yunxi-agent-cli\src\web.rs`
- `D:\YunXi Agent\crates\yunxi-agent-cli\src\web\app.css`
- `D:\YunXi Agent\crates\yunxi-agent-cli\src\web\app.js`
- `D:\YunXi Agent\docs\reports\development\2026-08-05-143100-yunxi-agent-v2-3-3-hotfix-18-her-default-detail-development-log.md`

## 测试与审核

- `node --check crates/yunxi-agent-cli/src/web/app.js`：通过。
- `cargo fmt --all -- --check`：通过。
- `git diff --check`：通过，仅保留仓库既有 LF/CRLF 转换提示。
- `cargo test -p yunxi-agent-cli web::tests::static_assets_are_wired_to_api_routes`：通过。
- `cargo test`：全仓通过；0 项失败，保留 1 项既有 Windows 条件性 ignored 测试。
- `cargo build --release -p yunxi-agent-cli`：通过。
- Playwright + 系统 Edge 验证桌面 `1294 x 912` 和移动端 `390 x 844`。
- 桌面验证默认详情开启、`aria-hidden=false`、`inert=false`、默认不抢焦点；手动关闭后状态正确，离开再返回自动恢复默认展开。
- 光标揭示时背景可见度为 1，详情背景右侧透明度为 18%，正文和人物背景均清晰。
- 移动端详情默认开启、关闭按钮位于视口内、无横向溢出，控制台与 page error 为空。
- 测试证据：`D:\YunXi Agent\.tmp\web-hotfix18`。

## 安装

- 安装目录：`D:\Apps\YunXi Agent\bin`。
- 回滚备份：`D:\Apps\YunXi Agent\bin\backup-20260805-142808-her-default-detail`。
- 安装版 `yunxi.exe` SHA-256：`E9C0949345BFA66550B1906DAA86699D534F93A02B913BE3B48431C527DB5551`。
- 安装版 `yunxi-agent-cli.exe` SHA-256：`B171E95DDF0AE5CE3CA13859AAEB081BB91AECB4639228EB93F4C3105AA6114A`。
- 安装二进制与 release 逐文件哈希一致，`yunxi --version` 返回 `2.3.3-hotfix.18`。
- 正式 Web 监听 `127.0.0.1:17861`，`/api/health` 返回 `status=ok`、`version=2.3.3-hotfix.18`。
- 正式进程参数包含 `--no-weixin-autostart`，本轮未创建或重启微信网关。

## 安全边界

未执行目录移动、递归清理、用户目录删除、`git clean`、工作区重置、force push 或历史 tag 删除。人物背景图、角色卡图、表情图、权威灵魂、长期记忆、人格和微信状态均未改写。安装替换前已创建可回滚备份并逐文件校验。

署名：开发者
