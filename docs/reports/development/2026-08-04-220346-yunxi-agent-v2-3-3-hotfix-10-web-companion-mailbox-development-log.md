# YunXi Agent v2.3.3-hotfix.10 Web 陪伴信箱开发日志

时间：2026-08-04 22:03:46 +08:00

## 工作目标

在不改变情书生成、加密邮箱存储、聊天、人格、长期记忆、工具审批、TUI 和微信运行边界的前提下，把现有后端陪伴信箱接入本地 Web。Web 只负责读取同一工作区的正式邮箱门面、展示信件并更新阅读状态，不新建第二套生成或存储逻辑。

## 实现内容

1. Web 顶部 Dock 新增信箱入口，与聊天、记忆、人格保持相同的图标导航和焦点语义。
2. 新增 `GET /api/mailbox` 列表接口，读取当前工作区的 `FileCompanionMailboxStore`，按可用时间倒序返回最多 50 封情书及未读计数。
3. 新增 `GET /api/mailbox/{item_id}` 详情接口，用户打开信件时才返回完整正文。
4. 新增 `POST /api/mailbox/{item_id}/state` 状态接口，支持正式邮箱状态机中的 `unread/read/archived`，Web 当前使用自动已读与手动归档两条路径。
5. Web DTO 不返回 `owner_scope`、`source_id`、`source_revision`、`content_ref` 或密钥信息；列表只显示主题、有限预览、状态和时间。
6. 新增信箱加载骨架、空信箱、读取失败、未读、已读、归档和重试状态，避免静态占位按钮。
7. 新增全屏阅读层、正文独立滚动、关闭和归档操作；桌面为双列信件列表，窄屏切换为单列并保持视口无横向或页面级纵向溢出。
8. 打开未读信件后自动标记为已读并同步列表计数；归档完成后同步阅读层与列表状态。
9. 支持点击遮罩关闭、`Escape` 关闭和关闭后的键盘焦点恢复。实机测试发现列表重绘后旧按钮引用失效，已改为通过信件 ID 找回新按钮，修复焦点丢失。
10. 使用现有 GSAP 按需加载路径增加阅读层入场动画，只变换 `transform/opacity`；网络不可用时保留 CSS 基础过渡，`prefers-reduced-motion` 下自动降级。
11. 聊天成功后触发信箱静默刷新，使后台新生成的情书能在当前 Web 会话中出现。
12. 工作区版本从 `2.3.3-hotfix.9` 更新为 `2.3.3-hotfix.10`。

## 设计与交互约束

- 设计延续现有 Web 的安静石墨色、冰蓝强调色和克制玻璃材质，不引入新的视觉体系。
- 设计参数：`DESIGN_VARIANCE 6`、`MOTION_INTENSITY 5`、`VISUAL_DENSITY 4`。
- 参考 21st MCP 的 Dock 与独立阅读层组织思路；具体实现保持 YunXi 现有原生 HTML/CSS/JavaScript 架构。
- 按 `design-taste-frontend`、`gsap-core` 和 `gsap-performance` 规则审核交互状态、减少动画模式和动画性能。

## 修改路径

- `D:\YunXi Agent\Cargo.toml`
- `D:\YunXi Agent\Cargo.lock`
- `D:\YunXi Agent\crates\yunxi-agent-cli\Cargo.toml`
- `D:\YunXi Agent\crates\yunxi-agent-cli\src\web.rs`
- `D:\YunXi Agent\crates\yunxi-agent-cli\src\web\index.html`
- `D:\YunXi Agent\crates\yunxi-agent-cli\src\web\app.js`
- `D:\YunXi Agent\crates\yunxi-agent-cli\src\web\app.css`
- `D:\YunXi Agent\docs\reports\development\2026-08-04-220346-yunxi-agent-v2-3-3-hotfix-10-web-companion-mailbox-development-log.md`

## 测试与审核

- `node --check crates/yunxi-agent-cli/src/web/app.js`：通过。
- `cargo fmt --all -- --check`：通过。
- `cargo test -p yunxi-agent-cli`：通过，33 + 33 + 58 + 10 项测试全部成功。
- `cargo test`：全仓通过；保留原有 1 项 Windows 条件性 ignored 测试。
- `cargo check -p yunxi-agent-cli`：通过，工作区包版本均为 `2.3.3-hotfix.10`。
- 真实本地 API 冒烟：列表 `200 application/json`；当前工作区条目数 0、未读数 0；不存在的详情 ID 返回 `404`。
- Playwright + 系统 Edge 脱敏实机验证：桌面与移动共 2 个场景、4 张截图，覆盖列表、详情、自动已读、归档、Escape、焦点恢复与 reduced-motion。
- 浏览器断言：无页面级横向/纵向溢出，无 JavaScript page error，无未处理 console error。
- 脱敏截图保存在 `D:\YunXi Agent\.tmp\web-mailbox-test`；未使用或输出真实信件正文。

## 安全与发布约束

未执行目录移动、递归清理、用户目录删除、`git clean`、工作区重置、force push 或历史 tag 删除。测试服务只绑定 `127.0.0.1:17861`。本版本发布为新 annotated tag `v2.3.3-hotfix.10`，保留全部历史 tag 以供回滚。

署名：开发者
