# YunXi Agent v2.3.3-hotfix.16 “她”背景流光与名称完整显示开发日志

时间：2026-08-05 13:26:20 +08:00

## 工作目标

修复 Web“她”页面角色卡中 `YunXi Agent` 名称显示不完整的问题，并在不替换、不重绘、不修改现有人物背景图的前提下，为整页背景加入克制的动态流光。保持角色卡、表情图集、详情展开、`/api/her`、人格、权威灵魂脱敏、长期记忆、聊天、信箱、TUI 和微信逻辑不变。

## 设计与实现

1. 使用 CodeGraph 复核 `startHerAmbientMotion`、角色卡标题区、背景层级和 Web 静态资源测试，确认改动边界只在 Web“她”页面和工作区版本。
2. 使用 21st MCP 检索电影感人物背景与环境光参考，仅吸收“影像缓慢呼吸、光线分层、动效克制”的原则，不复制第三方组件代码。
3. 使用 `design-taste-frontend`、`gsap-core`、`gsap-timeline` 和 `gsap-performance` 审核视觉与动画，参数为 `DESIGN_VARIANCE 6`、`MOTION_INTENSITY 5`、`VISUAL_DENSITY 3`。
4. 角色卡名称移除省略号裁切，改为可用剩余宽度内完整显示；窄屏可自然换行，并为关系阶段标签保留独立空间。
5. 背景新增两条宽幅柔光带和一条细流光，不增加装饰球体、散景或高饱和霓虹。
6. 原背景图路径仍为 `/assets/yunxi-her-background.jpg`，仓库中的 JPEG 文件未修改。
7. GSAP 主时间线同时控制原背景缓慢漂移与三层流光，离开“她”页面时暂停，重新进入时恢复。
8. 动画只更新 `transform` 和 `opacity`，流光元素才使用 `will-change`，避免布局抖动和持续重排。
9. GSAP CDN 不可用时自动回退到 Web Animations API，不影响页面显示和流光运动。
10. `prefers-reduced-motion` 下停止动态流光并保留低强度静态层；`prefers-reduced-transparency` 下停止并隐藏流光层。
11. Web 静态测试新增流光结构、JavaScript 节点和名称不再省略裁切的断言。
12. 工作区版本由 `2.3.3-hotfix.15` 提升到 `2.3.3-hotfix.16`。

## 修改路径

- `D:\YunXi Agent\Cargo.toml`
- `D:\YunXi Agent\Cargo.lock`
- `D:\YunXi Agent\crates\yunxi-agent-cli\src\web.rs`
- `D:\YunXi Agent\crates\yunxi-agent-cli\src\web\index.html`
- `D:\YunXi Agent\crates\yunxi-agent-cli\src\web\app.css`
- `D:\YunXi Agent\crates\yunxi-agent-cli\src\web\app.js`
- `D:\YunXi Agent\docs\reports\development\2026-08-05-132620-yunxi-agent-v2-3-3-hotfix-16-her-flow-motion-development-log.md`

## 测试与审核

- `node --check crates/yunxi-agent-cli/src/web/app.js`：通过。
- `cargo fmt --all -- --check`：通过。
- `git diff --check`：通过，仅保留仓库既有 LF/CRLF 转换提示。
- `cargo test -p yunxi-agent-cli web::tests::static_assets_are_wired_to_api_routes`：通过。
- `cargo test`：全仓通过；0 项失败，保留 1 项既有 Windows 条件性 ignored 测试。
- `cargo build --release -p yunxi-agent-cli`：通过。
- Playwright + 系统 Edge 验证桌面 `1294 x 912` 和移动端 `390 x 844`。
- 两档视口均完整显示 `YunXi Agent`，名称与“初识”无重叠，document 无横向溢出，控制台与 page error 为空。
- 两张背景图片均成功加载，路径和自然宽度一致；人物背景资源未替换。
- GSAP 路径与无 GSAP 回退路径中的流光前后帧 transform 均发生变化。
- 减少动态效果时不创建流光动画；减少透明度时 JavaScript 不启动流光，CSS 提供隐藏规则。
- 验证证据：`D:\YunXi Agent\.tmp\web-hotfix16`。

## 安装

- 安装目录：`D:\Apps\YunXi Agent\bin`。
- 回滚备份：`D:\Apps\YunXi Agent\bin\backup-20260805-132302-her-flow-motion`。
- 备份中的两个 `.15` 二进制逐文件核对 SHA-256，与替换前安装文件一致。
- 安装版 `yunxi.exe` SHA-256：`760CB319547ACF5307A8814EB5D93CD895D261E5F4364D89C27E9C3955B24895`。
- 安装版 `yunxi-agent-cli.exe` SHA-256：`3D5D2789D494DFADA08331D9B356FA2992019876E2146B0B8556F93D68356A8B`。
- 安装版 `yunxi --version` 返回 `2.3.3-hotfix.16`。
- 正式 Web 由安装版监听 `127.0.0.1:17861`，`/api/health` 返回 `status=ok`、`version=2.3.3-hotfix.16`。
- 正式进程参数包含 `--no-weixin-autostart`，本轮未创建或重启微信网关。
- 第一次替换包装脚本因误读 PowerShell 遗留 `$LASTEXITCODE` 主动触发回滚；只读复核确认 `.15` 文件、哈希和 Web 服务全部恢复后，修正包装判断并完成第二次安装。该过程未造成不可恢复状态。

## 安全边界

未执行目录移动、递归清理、用户目录删除、`git clean`、工作区重置、force push 或历史 tag 删除。人物背景图、角色卡图、表情图、权威灵魂、长期记忆、人格和微信状态均未改写。安装替换前已创建可回滚备份并完成逐文件哈希校验。

署名：开发者
