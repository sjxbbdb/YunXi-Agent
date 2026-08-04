# YunXi Agent v2.3.3-hotfix.11 “她”角色关系档案开发日志

时间：2026-08-04 23:32:34 +08:00

## 工作目标

在不改变现有人格、灵魂、长期记忆、关系推导、聊天、工具审批、TUI、Web、信箱和微信运行边界的前提下，为本地 Web 增加名为“她”的角色关系档案页面。该页面展示 YunXi 的人物形象、当前关系阶段和适合公开呈现的人格摘要，并随现有关系记忆和活动人格自动更新。

## 实现内容

1. Web Dock 新增“她”入口，继续使用图标导航、悬停提示、键盘焦点和当前页面状态。
2. 新增 `GET /api/her`，返回显示名、结构化人格摘要、五项性格倾向、关系阶段、可用记忆数量、关系记忆数量、最近一次有意义互动时间和人格/记忆/陪伴开关状态。
3. 将 runtime 的 `derive_relationship_state` 开放为公共只读门面，Web、CLI、TUI 和微信继续使用同一套关系阶段算法，不在前端复制规则。
4. 关系阶段使用 `初识 / 熟悉 / 相知` 三段展示，真实对应 `New / Familiar / Established`；页面只显示计数和阶段，不返回长期记忆正文。
5. 使用用户提供的原始人物设定图作为第一视觉信号，原样复制为 Web 静态资产；源文件未修改。
6. 人物窗口提供全身、表情、完整设定图三种可切换视图，三个图标按钮均具备 `aria-label`、`title` 和 `aria-pressed` 状态。
7. 页面采用桌面不对称双栏布局；移动端将人物窗口置于上半屏，关系档案置于独立滚动的下半屏，页面本身不产生横向或纵向滚动。
8. 新增加载、API 错误、重试、图片加载失败和运行开关状态；对话成功后静默刷新“她”的关系数据。
9. 使用现有 GSAP 按需加载路径实现页面入场与图像模式切换，动画只使用透明度、滤镜和 transform；网络不可用时保留 CSS 过渡，`prefers-reduced-motion` 下自动降级。
10. 针对权威 `soul.txt` 增加专门隐私分支：不把完整灵魂正文、身体设定、用户记忆或私密关系文字放入 `/api/her`。Web 只显示“权威灵魂已载入”等安全状态。
11. 非权威普通人格继续使用原有结构化 identity、soul signature、voice、companion style 和 addressing 字段，并统一限制显示摘要长度。
12. 工作区版本从 `2.3.3-hotfix.10` 更新为 `2.3.3-hotfix.11`。

## 设计依据

- 设计读法：面向长期陪伴关系的沉浸式角色档案，而不是社交资料卡或游戏属性面板。
- 设计参数：`DESIGN_VARIANCE 7`、`MOTION_INTENSITY 6`、`VISUAL_DENSITY 4`。
- 21st MCP 已检索 profile card、AI assistant、dock 和 character dossier 方向；现成结果偏社交卡片，因此只采用“强人物媒体、紧凑信息层、模式切换”的组织方式，没有直接复制不匹配的组件。
- 使用 `design-taste-frontend` 审核视觉层级、色彩、密度、移动布局和状态完整性。
- 使用 `gsap-core`、`gsap-timeline`、`gsap-performance`约束交互时序、减少动画和 transform/opacity 优先策略。
- 延续现有石墨黑背景、冰蓝强调色和克制玻璃控制条，没有引入第二套主题。

## 人物资产

- 原始来源：`C:\Users\24763\Desktop\a5ced580bbc1e40a98dbd2e8533b1a8c.jpg`
- 项目资产：`D:\YunXi Agent\crates\yunxi-agent-cli\src\web\yunxi-character-design.jpg`
- 文件大小：236,509 字节
- SHA-256：`2779CAC324928A6FFE3E03550F941E8E6FF8D5B4D08E64C57053C024B61BAE0A`

## 修改路径

- `D:\YunXi Agent\Cargo.toml`
- `D:\YunXi Agent\Cargo.lock`
- `D:\YunXi Agent\crates\yunxi-agent-runtime\src\lib.rs`
- `D:\YunXi Agent\crates\yunxi-agent-cli\src\web.rs`
- `D:\YunXi Agent\crates\yunxi-agent-cli\src\web\index.html`
- `D:\YunXi Agent\crates\yunxi-agent-cli\src\web\app.js`
- `D:\YunXi Agent\crates\yunxi-agent-cli\src\web\app.css`
- `D:\YunXi Agent\crates\yunxi-agent-cli\src\web\yunxi-character-design.jpg`
- `D:\YunXi Agent\docs\reports\development\2026-08-04-233234-yunxi-agent-v2-3-3-hotfix-11-her-character-relationship-profile-development-log.md`

## 测试与审核

- `node --check crates/yunxi-agent-cli/src/web/app.js`：通过。
- `cargo fmt --all` 与 `git diff --check`：通过。
- `cargo test -p yunxi-agent-cli`：通过，包含新增关系标签、Unicode 摘要和权威灵魂隐私测试。
- `cargo test`：全仓通过；保留原有 1 项 Windows 条件性 ignored 测试。
- `cargo build -p yunxi-agent-cli --bins --release`：通过。
- Playwright + 系统 Edge：3 个场景、5 张截图，覆盖桌面全身/表情/设定图、移动端、API 错误重试、图片加载、无页面溢出、控制台错误和 reduced-motion。
- 调试包和最终安装包分别通过同一套 Playwright 用例。
- 视觉证据：`D:\YunXi Agent\.tmp\her-web-test\her-desktop-portrait.png`、`her-desktop-expressions.png`、`her-desktop-sheet.png`、`her-mobile-portrait.png`、`her-mobile-details.png`。
- 安装版真实 API：`/api/health` 返回 `2.3.3-hotfix.11`；`/api/her` 返回安全摘要；人物 JPG 返回 `200 image/jpeg` 和 236,509 字节。

## 安装

- 安装目录：`D:\Apps\YunXi Agent\bin`
- 回滚备份：`D:\Apps\YunXi Agent\bin\backup-20260804-232928-her-character-profile`
- `yunxi.exe` SHA-256：`FB1C1FD868306625EFF0393BE4B6DA709FF718AB9D69A4A2A388A14C5FACEDDD`
- `yunxi-agent-cli.exe` SHA-256：`551DFE5D1434BDCC74378EAE2691041F6E8F3CED7CD4A086C5CBA8CDED9E1D57`
- 安装后 Web：由 `D:\Apps\YunXi Agent\bin\yunxi.exe` 绑定 `127.0.0.1:17861`，健康检查通过。

## 安全与发布约束

未执行目录移动、递归清理、用户目录删除、`git clean`、工作区重置、force push 或历史 tag 删除。安装前对旧二进制逐文件备份并校验，人物源图只读复制，权威灵魂文件未修改。本版本发布为新 annotated tag `v2.3.3-hotfix.11`，保留全部历史 tag。

署名：开发者
