# YunXi Agent v2.3.3-hotfix.13 “她”沉浸式角色关系场景开发日志

时间：2026-08-05 03:06:05 +08:00

## 工作目标

使用用户提供的新背景图和人物卡图重构 Web“她”页面。桌面端采用“左侧角色卡 + 右侧人物背景”的构图；默认背景柔和压暗，指针经过时局部显现并在短暂停留后淡出；点击角色卡后，在保留左侧卡片的同时展开关系阶段、人格摘要、长期记忆状态和陪伴信号。保持 `/api/her`、权威灵魂脱敏、关系阶段推导、长期记忆、聊天、人格、信箱、TUI 和微信运行边界不变。

## 设计参考与约束

1. 先通过 CodeGraph 定位 `#view-her`、`renderHer`、`setHerArtMode`、`/api/her` 和 Rust 静态资源路由，确认改动边界。
2. 使用 21st MCP 检索沉浸式人物档案、人物主视觉、聚光揭示和卡片展开参考；重点吸收 `Minimalist Hero` 的人物留白关系与 `Morphing Card Stack` 的展开层级，不复制 React、Tailwind 或第三方运行时结构。
3. 使用 `design-taste-frontend` 审核视觉方向，采用冷静、克制、电影感的冷灰蓝场景；设计参数为 `DESIGN_VARIANCE 8`、`MOTION_INTENSITY 7`、`VISUAL_DENSITY 3`。
4. 使用 `gsap-core`、`gsap-timeline`、`gsap-performance` 约束动画。指针高频坐标通过 GSAP `quickTo` 数值代理平滑更新；GSAP CDN 不可用时保留原生 CSS/Web API 降级；减少动态效果时关闭局部揭示和持续动画。

## 实现内容

1. 新增本地背景资产 `yunxi-her-background.jpg`，原始尺寸 `1672 x 941`；人物侧脸保持在右侧，发丝向中部延展，左侧保留角色卡负空间。
2. 新增本地角色卡资产 `yunxi-her-profile-card.jpg`，原始尺寸 `971 x 1619`；角色卡固定在左侧，保留人物形象、表情设定和完整设定图三种视图入口。
3. Rust Web 路由新增 `/assets/yunxi-her-background.jpg` 与 `/assets/yunxi-her-profile-card.jpg`，统一使用只读 JPEG 响应与缓存头；原人物设定图路由继续作为表情和设定图回退素材。
4. 将“她”页面从旧三栏档案舞台重构为全视口背景层、左侧角色卡层和关系详情层。默认态只突出角色卡与背景人物；详情态从卡片方向展开，桌面端保留左卡，手机端使用完整单列档案。
5. 背景包含压暗基础层和局部揭示层。指针位置驱动径向 mask；停止移动后保持约 900 ms，再按约 1.45 s 淡出。定时探针确认 700 ms 时揭示 opacity 仍为 `1`，约 1 s 时进入淡出，最终回到 `0`。
6. 角色卡点击、键盘焦点、详情关闭、Escape 关闭和焦点恢复均已实现；详情使用 `role="dialog"`、`aria-hidden`、`aria-expanded` 与 `inert` 管理可访问状态。
7. 人物形象、表情设定、完整设定图继续支持切换；当前人物形象使用新角色卡，表情与完整设定图使用原设定图，未来独立素材可直接替换数据源。
8. 新增背景和人物图片失败降级；即使图片在事件监听器注册前已经失败，也会通过 `complete`/`naturalWidth` 同步故障状态，不显示破损图片图标。
9. 修复既有 CSS 中人格详情关闭图标 hover 规则缺少右花括号的问题。该错误曾导致后续样式被浏览器解析为嵌套规则；新增 CSS 花括号数量回归断言，避免同类结构错误再次漏过。
10. `/api/her` 和 `her_display_fields` 未修改。权威灵魂仍只向 Web 返回“权威灵魂已载入”等安全摘要，不暴露灵魂正文；记忆正文也未加入“她”页面响应。
11. 工作区版本由 `2.3.3-hotfix.12` 提升到 `2.3.3-hotfix.13`。

## 修改路径

- `D:\YunXi Agent\Cargo.toml`
- `D:\YunXi Agent\Cargo.lock`
- `D:\YunXi Agent\crates\yunxi-agent-cli\src\web.rs`
- `D:\YunXi Agent\crates\yunxi-agent-cli\src\web\index.html`
- `D:\YunXi Agent\crates\yunxi-agent-cli\src\web\app.css`
- `D:\YunXi Agent\crates\yunxi-agent-cli\src\web\app.js`
- `D:\YunXi Agent\crates\yunxi-agent-cli\src\web\yunxi-her-background.jpg`
- `D:\YunXi Agent\crates\yunxi-agent-cli\src\web\yunxi-her-profile-card.jpg`
- `D:\YunXi Agent\docs\superpowers\specs\2026-08-05-yunxi-agent-her-page-background-image-prompt.md`
- `D:\YunXi Agent\docs\superpowers\specs\2026-08-05-yunxi-agent-her-page-image-generation-prompts.md`
- `D:\YunXi Agent\docs\superpowers\specs\2026-08-05-yunxi-agent-her-page-visual-assets-interaction-requirements.md`
- `D:\YunXi Agent\docs\reports\development\2026-08-05-030605-yunxi-agent-v2-3-3-hotfix-13-her-immersive-character-scene-development-log.md`

## 测试与审核

- `node --check crates/yunxi-agent-cli/src/web/app.js`：通过。
- `cargo fmt --all`：通过。
- `git diff --check`：通过，仅保留仓库既有 LF/CRLF 转换提示。
- `cargo test -p yunxi-agent-cli web::tests`：9 项 Web 测试全部通过。
- `cargo test`：全仓通过；0 项失败，保留 1 项既有 Windows 条件性 ignored 测试。
- `cargo build --release -p yunxi-agent-cli`：通过。
- Playwright + 系统 Edge 验证 `1920 x 1080`、`1440 x 960`、`1294 x 912`、`1024 x 768`、`390 x 844`；所有“她”页面视口的 document/body 溢出均为 `0 x 0`。
- 桌面和手机分别回归聊天、记忆、人格、信箱、“她”五个页面：全部正确激活、无页面级溢出、无控制台错误。
- 已验证背景揭示、停留、淡出、人物素材切换、详情打开/关闭、GSAP CDN 阻断、减少动态效果、背景图片失败、人物图片失败和键盘 Escape 路径。
- 自动化证据目录：`D:\YunXi Agent\.tmp\web-hotfix13`。
- 正式安装截图：`D:\YunXi Agent\.tmp\web-hotfix13\installed-1294x912.png`、`D:\YunXi Agent\.tmp\web-hotfix13\installed-1294x912-detail.png`。

## 安装

- 安装目录：`D:\Apps\YunXi Agent\bin`。
- 回滚备份：`D:\Apps\YunXi Agent\bin\backup-20260805-025931-her-immersive-redesign`。
- 备份前后两个旧二进制均逐文件比对 SHA-256，一致后才执行替换。
- 安装版 `yunxi.exe` SHA-256：`EF01636C5C2F7D3AA0C1D23588F019AB8B0536CE544CDF3411283B1446BE07DE`。
- 安装版 `yunxi-agent-cli.exe` SHA-256：`F2F2DC4F77B72377673FFEF9AB5683602CE02D0C5CA231C3BD0C08B594D4A17F`。
- 两个安装二进制的 `--version` 均返回 `2.3.3-hotfix.13`。
- 正式 Web 已由安装版 `yunxi.exe` 监听 `127.0.0.1:17861`，`/api/health` 返回 `ok` 和 `2.3.3-hotfix.13`；启动参数保留 `--no-weixin-autostart`，未创建重复微信网关。

## 安全与发布约束

未执行目录移动、递归清理、用户目录删除、`git clean`、工作区重置、force push 或历史 tag 删除。两张用户源图只读复制到仓库静态资源目录；权威灵魂、长期记忆和用户原图均未改写。安装替换前已创建可回滚备份并校验。本版本发布为新的 annotated tag `v2.3.3-hotfix.13`，保留全部历史 tag。

署名：开发者
