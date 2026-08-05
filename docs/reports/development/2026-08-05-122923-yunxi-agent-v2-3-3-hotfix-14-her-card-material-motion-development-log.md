# YunXi Agent v2.3.3-hotfix.14 “她”角色卡材质与交互开发日志

时间：2026-08-05 12:29:23 +08:00

## 工作目标

依据用户提供的两段视频改造 Web“她”页面左侧角色卡：第一段视频用于提取半透明磨砂、边缘折射、柔和高光与冷色阴影质感；第二段视频用于提取稳定命中框、指针跟随倾斜、多层背板视差和缓动回正交互。改动只作用于角色卡，不改变 `/api/her`、关系阶段、人格、权威灵魂脱敏、长期记忆、聊天、信箱、TUI 或微信运行逻辑。

## 视频与设计参考

- 材质视频：`C:\Users\24763\Desktop\第一个视频\屏幕录制 2026-08-05 111844.mp4`。
- 交互视频：`C:\Users\24763\Desktop\第二个视频\屏幕录制 2026-08-05 111756.mp4`。
- 材质联系表：`D:\YunXi Agent\.tmp\video-card-references\material-contact-sheet.png`。
- 交互联系表：`D:\YunXi Agent\.tmp\video-card-references\interaction-contact-sheet.png`。
- 使用 CodeGraph 复核 `#her-profile-card`、`setupPointerMaterials`、`updateHerParallax`、详情打开/关闭和人物画面切换链路。
- 使用 21st MCP 检索透明卡片、卡片堆叠和指针视差参考；仅吸收层次与交互规律，不复制第三方组件代码。
- 使用 `design-taste-frontend` 审核视觉方向，设计参数为 `DESIGN_VARIANCE 8`、`MOTION_INTENSITY 8`、`VISUAL_DENSITY 3`。
- 使用 `gsap-core`、`gsap-timeline`、`gsap-performance` 约束动画；高频指针数据使用可复用的 `quickTo()` 补间，只更新 transform 与 CSS 变量。

## 实现内容

1. 在固定的角色卡按钮命中框内新增独立 `.her-card-surface`。外层按钮不随 hover 位移，内层卡面负责倾斜、轻微抬升和缩放，避免指针追逐移动中的命中框。
2. 新增三张 `.her-card-depth` 背板。每层保留不同基础偏移、缩放和指针运动系数，形成第二段视频中的实体叠层感；退出卡片后各层平滑回到精确初始位置。
3. 人物平面与卡面采用相反方向的轻微二维视差，使人物与卡壳产生纵深，同时避免内容被过度缩放或模糊。
4. 新增 `.her-card-glass` 材质层，以指针坐标驱动径向高光，并使用内描边、顶部折射线和低透明度冷灰蓝渐变模拟第一段视频的半透明材质。未在运动平面上使用实时背景模糊，避免遮挡人物和增加 GPU 压力。
5. GSAP 可用时使用 `quickTo()` 复用卡面、人物和三张背板的补间控制器；GSAP CDN 不可用时使用原生 transform 降级，交互仍可工作。
6. 系统启用“减少动态效果”时，不绑定指针视差并关闭 hover 位移；系统启用“减少透明度”时使用实色材质降级。
7. 保留角色卡点击详情、键盘焦点、`aria-expanded`、人物形象/表情/设定图切换和移动端详情布局。
8. 浏览器截图审核发现三张背板与主卡共处 `preserve-3d` 空间时，旋转平面会发生几何穿插，Chromium 将背板三角网格错误绘制到人物前方。最终将角色卡区域改为隔离的平面堆叠上下文，由 `z-index` 管理层级；背板仍独立倾斜和位移，但不再穿透人物。
9. Web 静态资源测试新增角色卡内层、背板、玻璃材质和 `updateHerParallax` 结构断言，防止后续重构遗漏关键节点。
10. 工作区版本由 `2.3.3-hotfix.13` 提升到 `2.3.3-hotfix.14`。

## 修改路径

- `D:\YunXi Agent\Cargo.toml`
- `D:\YunXi Agent\Cargo.lock`
- `D:\YunXi Agent\crates\yunxi-agent-cli\src\web.rs`
- `D:\YunXi Agent\crates\yunxi-agent-cli\src\web\index.html`
- `D:\YunXi Agent\crates\yunxi-agent-cli\src\web\app.css`
- `D:\YunXi Agent\crates\yunxi-agent-cli\src\web\app.js`
- `D:\YunXi Agent\docs\reports\development\2026-08-05-122923-yunxi-agent-v2-3-3-hotfix-14-her-card-material-motion-development-log.md`

## 测试与审核

- `node --check crates/yunxi-agent-cli/src/web/app.js`：通过。
- `cargo fmt --all`：通过。
- `git diff --check`：通过，仅保留仓库既有 LF/CRLF 转换提示。
- `cargo check -p yunxi-agent-cli`：通过，全部工作区包解析为 `2.3.3-hotfix.14`。
- `cargo test -p yunxi-agent-cli web::tests`：9 项 Web 测试全部通过。
- `cargo test`：全仓通过；0 项失败，保留 1 项既有 Windows 条件性 ignored 测试。
- `cargo build -p yunxi-agent-cli --release --bins`：通过。
- Playwright + 系统 Edge 验证桌面 `1440 x 1000`、降级 `1280 x 900` 和移动端 `390 x 844`；document/body 均无横向或纵向溢出，控制台与 page error 均为空。
- 已验证卡片四向倾斜、三层背板差异位移、人物反向视差、退出回正、固定外层命中框、详情打开/关闭、三种人物画面切换、GSAP 阻断降级和减少动态效果。
- 诊断并消除 Chromium 背板三角形穿透伪影；最终安装版截图中人物完整清晰。
- 开发验证证据：`D:\YunXi Agent\.tmp\web-card-video-preview`。
- 正式安装验证证据：`D:\YunXi Agent\.tmp\web-card-video-installed`。

## 安装

- 安装目录：`D:\Apps\YunXi Agent\bin`。
- 回滚备份：`D:\Apps\YunXi Agent\bin\backup-20260805-122428-her-card-material-motion`。
- 备份中的两个旧二进制均逐文件核对 SHA-256，与替换前安装文件一致。
- 安装版 `yunxi.exe` SHA-256：`DA228761B462406A24AD7D2FD937D8F0967121D270764AA1D00C396532A2BABB`。
- 安装版 `yunxi-agent-cli.exe` SHA-256：`9AF47F27A63B8A820278DA2A261D6FF35634BF014A36BA86A1C08DD26D6BC1C7`。
- 两个安装二进制的 `--version` 均返回 `2.3.3-hotfix.14`。
- 正式 Web 由安装版 `D:\Apps\YunXi Agent\bin\yunxi.exe` 监听 `127.0.0.1:17861`，进程 PID 为 `27080`。
- 正式 `/api/health` 返回 `status=ok`、`version=2.3.3-hotfix.14`；启动参数包含 `--no-weixin-autostart`，本轮未创建或重启微信网关。

## 安全与发布约束

未执行目录移动、递归清理、用户目录删除、`git clean`、工作区重置、force push 或历史 tag 删除。两段用户视频只读分析，未修改或移动；权威灵魂、长期记忆、人格和人物图片均未改写。安装替换前已创建可回滚备份并完成逐文件哈希校验。本版本将发布为新的 annotated tag `v2.3.3-hotfix.14`，保留全部历史 tag。

署名：开发者
