# YunXi Agent v2.3.3-hotfix.12 “她”页面参考元素转译开发日志

时间：2026-08-05 00:29:59 +08:00

## 工作目标

分析用户提供的两份前端源码，提取其中有价值的视觉组织与动效思路，并转译到 YunXi Agent 现有 Web 设计语言中。改动聚焦“她”页面，不复制参考项目的框架与营销式页面结构，不改变 `/api/her`、人格、权威灵魂、长期记忆、关系推导、聊天、信箱、TUI 或微信运行边界。

## 参考源码分析

### 源码 1：动画 ASCII 媒体

- 优点：用单一动态媒体形成持续但低干扰的视觉纹理，能够让静态页面具有生命感；组件接口简单，适合作为内容背后的中景。
- 新意：ASCII 并非正文或装饰图标，而是被当作可循环的材质层使用。
- 未直接采用：远程 MP4、远程 poster、对 21st 资产地址的运行时依赖，以及未处理减少动效偏好的自动播放行为。
- YunXi 转译：使用本地 JavaScript 确定性生成字符场，不下载、不复制视频；字符层只使用 YunXi、记忆、灵魂和本地存在等抽象词，并保持低对比度。

### 源码 2：极简人物 Hero

- 优点：人物位于视觉中心，左右信息形成不对称编辑式构图；主体、背景和文字分层入场；移动端通过顺序重排保留人物优先级。
- 新意：将人物图、几何中景和边缘信息组织为一个舞台，而不是普通资料卡。
- 未直接采用：React、Framer Motion、Tailwind、黄色圆形背景、超大营销标题、社交链接、占满首屏的落地页叙事和图片失败外链占位图。
- YunXi 转译：保留“中心人物 + 两侧关系信息”的空间关系，改为石墨黑、冷白和冰蓝体系；左侧显示关系阶段，右侧显示灵魂与性格信号，中间继续使用用户提供的 YunXi 人物设定图。

## 设计与实现

1. 使用 21st MCP 按项目约束检索人物档案、ASCII 纹理、AI 助手、Dock 和环形交互参考；结果再次指向 `dd`、`Minimalist Hero` 等方向，只采用设计原理，不拉取或复制组件代码。
2. 使用 `design-taste-frontend` 审核视觉层级、信息密度、材质、移动端和非模板化程度。
3. 使用 `gsap-core`、`gsap-timeline`、`gsap-performance` 约束动画：多元素入场使用 timeline，指针高频更新使用 `quickTo`，主要动画只操作 transform 与 opacity。
4. 将“她”页面从双栏档案改为三段式舞台：左侧关系档案、中间人物形象、右侧灵魂与性格信号；信息区保持无卡片、细分隔线和紧凑排版。
5. 新增本地字符纹理生成器 `buildHerAsciiTexture`，固定种子生成 112 x 54 字符场，避免网络依赖和每次刷新随机跳变。
6. 字符纹理优先由 GSAP 缓慢往返；GSAP CDN 不可用时自动切换到 Web Animations API；`prefers-reduced-motion: reduce` 下不创建动画。
7. 人物窗口新增独立 `her-art-plane`，指针交互通过 GSAP `quickTo` 做极轻量的 x/y 与 3D 倾斜；没有对图像本身施加强烈透视或弹跳。
8. 人物全身、表情、完整设定图切换去掉 blur 滤镜，改为短透明度过渡，减少重绘和切换时的模糊观感。
9. 人物窗口加入克制的坐标角、顶部状态标记和居中圆形视图控制条；保留原有图标、`aria-label`、`title` 与 `aria-pressed` 状态。
10. 性格倾向从大边框表格调整为五条微型信号线，真实使用后端 score 设置长度，不新增虚假数据。
11. 1160px 以下将关系与人格信号合并为独立内部滚动列，人物保持另一列；780px 以下人物置顶，档案在下半屏内部滚动，文档本身始终不产生页面级滚动。
12. `/api/her`、关系阶段算法、权威灵魂脱敏分支和人物原图未修改；Web 仍只显示“权威灵魂已载入”等安全摘要，不暴露灵魂正文或记忆正文。
13. 工作区版本从 `2.3.3-hotfix.11` 更新为 `2.3.3-hotfix.12`。

## 修改路径

- `D:\YunXi Agent\Cargo.toml`
- `D:\YunXi Agent\Cargo.lock`
- `D:\YunXi Agent\crates\yunxi-agent-cli\src\web.rs`
- `D:\YunXi Agent\crates\yunxi-agent-cli\src\web\index.html`
- `D:\YunXi Agent\crates\yunxi-agent-cli\src\web\app.css`
- `D:\YunXi Agent\crates\yunxi-agent-cli\src\web\app.js`
- `D:\YunXi Agent\docs\reports\development\2026-08-05-002959-yunxi-agent-v2-3-3-hotfix-12-her-reference-elements-development-log.md`

## 测试与审核

- `node --check crates/yunxi-agent-cli/src/web/app.js`：通过。
- `cargo fmt --all`：通过。
- `git diff --check`：通过，仅有仓库既有的 LF/CRLF 转换提示。
- `cargo test -p yunxi-agent-cli web::tests`：9 项 Web 测试全部通过。
- `cargo test`：全仓通过；保留原有 1 项 Windows 条件性 ignored 测试。
- 第一次全仓测试曾因独立 `17862` 预览进程占用 debug 二进制而无法替换文件；核验 PID、路径和命令行后只停止该测试进程，重新执行全仓测试通过。这不是代码失败，也未影响正式服务。
- `cargo build -p yunxi-agent-cli --bins --release`：通过。
- Playwright + 系统 Edge 覆盖 `1920 x 1080`、`1440 x 960`、`1294 x 912`、`1024 x 768`、`390 x 844`。
- 所有视口的文档宽高均等于 viewport，没有页面级横向或纵向滚动；桌面三栏无重叠，移动端档案内部可滚动至灵魂与性格信号。
- 人物全身、表情、完整设定图按钮均可切换，`data-art-mode` 与 `aria-pressed` 正确更新。
- 正常网络、GSAP CDN 阻断和减少动效三种场景均已验证：断网时本地字符层仍移动；减少动效时不加载 GSAP 且字符层无动画。
- 正式安装版 `1294 x 912` 截图无控制台错误，`/api/health` 返回 `2.3.3-hotfix.12`。
- 视觉证据：`D:\YunXi Agent\.tmp\her-reference-preview\final-desktop.png`、`final-mobile.png`、`final-mobile-signals.png`、`installed-1294x912.png`。

## 安装

- 安装目录：`D:\Apps\YunXi Agent\bin`
- 回滚备份：`D:\Apps\YunXi Agent\bin\backup-20260805-002436-her-reference-elements`
- 备份前两个旧二进制均已逐文件比对 SHA-256，一致后才执行替换。
- 安装版 `yunxi.exe` SHA-256：`AD00ED2647AE39B4A73B352C857C73FA5DF442668DFAC6C25004E9D0FBE10758`
- 安装版 `yunxi-agent-cli.exe` SHA-256：`75E9024D06D5A3E2E5A2C42AFB55ED4E26CDA8C880B7CBD44EDD5FECC437E637`
- 两个安装二进制的 `--version` 均返回 `2.3.3-hotfix.12`。
- 正式 Web 已由安装版 `yunxi.exe` 重新监听 `127.0.0.1:17861`，启动参数继续包含 `--no-weixin-autostart`，未额外启动微信网关。

## 安全与发布约束

未执行目录移动、递归清理、用户目录删除、`git clean`、工作区重置、force push 或历史 tag 删除。两份桌面参考源码只读分析；人物原图、权威灵魂和长期记忆未修改。安装替换前已创建可回滚的逐文件备份。本版本发布为新 annotated tag `v2.3.3-hotfix.12`，保留全部历史 tag。

署名：开发者
