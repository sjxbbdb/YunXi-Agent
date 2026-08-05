# YunXi Agent v2.3.3-hotfix.15 “她”表情图集开发日志

时间：2026-08-05 13:00:29 +08:00

## 工作目标

将用户提供的 6 张 YunXi 表情图接入 Web“她”页面角色卡，删除用户标注为不需要的“查看完整设定图”入口。角色卡只保留默认人物图与表情图两个入口；首次点击表情按钮进入表情集，继续点击按固定顺序循环切换。保持角色卡材质、指针倾斜、多层背板、详情展开、`/api/her`、人格、权威灵魂脱敏、长期记忆、聊天、信箱、TUI 和微信运行逻辑不变。

## 设计与实现

1. 使用 CodeGraph 复核 `setHerArtMode`、`herArtModes`、静态图片路由和 Web 静态资源测试，确认改动只涉及角色卡图片视图。
2. 使用 21st MCP 检索紧凑图片选择器和轮播参考，仅吸收“当前项清晰、控制器克制”的原则，不复制第三方组件代码。
3. 使用 `design-taste-frontend`、`gsap-core`、`gsap-timeline` 和 `gsap-performance` 审核视觉与动画，参数为 `DESIGN_VARIANCE 7`、`MOTION_INTENSITY 6`、`VISUAL_DENSITY 2`。
4. 角色卡工具栏由 3 个按钮缩减为 2 个按钮，彻底移除 `sheet` 模式、第三个按钮、旧设定图数据源和对应 CSS。
5. 新增结构化表情资源表，固定顺序为：克制、平静、思索、侧望、微笑、温柔。
6. 表情按钮首次点击展示第 1 张；激活状态下再次点击切换下一张；第 6 张之后回到第 1 张。默认人物按钮随时恢复原角色卡图片。
7. 表情名称和 `1/6` 至 `6/6` 序号只通过 tooltip 与无障碍标签提供，不在小卡片上增加常驻文字。
8. 表情资源首次使用时执行本地预加载；换图动画等待图片加载完成后再运行，并使用约 360 ms 的透明度过渡，不使用模糊，不改变人物比例。
9. 快速连续切换时使用递增序号丢弃旧图片的迟到动画，避免过期 load 事件覆盖当前画面。
10. 6 张图片统一使用 `object-fit: cover`。其中 5 张为 `971 x 1619`，侧望图为 `1122 x 1402`；实机截图确认侧望图裁切后面部、手势和发丝均完整。
11. Rust Web 新增 6 条本地 JPEG 路由与内嵌资源，图片不依赖外部网络。
12. Web 静态测试新增“恰好两个人物视图按钮”“不存在 sheet 模式”“表情资源非空”和表情路由引用断言。
13. 工作区版本由 `2.3.3-hotfix.14` 提升到 `2.3.3-hotfix.15`。

## 新增图片资产

- `crates/yunxi-agent-cli/src/web/yunxi-her-expression-reserved.jpg`
- `crates/yunxi-agent-cli/src/web/yunxi-her-expression-calm.jpg`
- `crates/yunxi-agent-cli/src/web/yunxi-her-expression-thoughtful.jpg`
- `crates/yunxi-agent-cli/src/web/yunxi-her-expression-wistful.jpg`
- `crates/yunxi-agent-cli/src/web/yunxi-her-expression-smile.jpg`
- `crates/yunxi-agent-cli/src/web/yunxi-her-expression-gentle.jpg`

6 张仓库资产均由用户源图只读复制，复制后逐文件 SHA-256 与源图一致；未修改、移动或删除微信临时目录中的原文件。

## 其他修改路径

- `D:\YunXi Agent\Cargo.toml`
- `D:\YunXi Agent\Cargo.lock`
- `D:\YunXi Agent\crates\yunxi-agent-cli\src\web.rs`
- `D:\YunXi Agent\crates\yunxi-agent-cli\src\web\index.html`
- `D:\YunXi Agent\crates\yunxi-agent-cli\src\web\app.css`
- `D:\YunXi Agent\crates\yunxi-agent-cli\src\web\app.js`
- `D:\YunXi Agent\docs\reports\development\2026-08-05-130029-yunxi-agent-v2-3-3-hotfix-15-her-expression-gallery-development-log.md`

## 测试与审核

- `node --check crates/yunxi-agent-cli/src/web/app.js`：通过。
- `cargo fmt --all`：通过。
- `git diff --check`：通过，仅保留仓库既有 LF/CRLF 转换提示。
- `cargo check -p yunxi-agent-cli`：通过，全部工作区包解析为 `2.3.3-hotfix.15`。
- `cargo test -p yunxi-agent-cli web::tests`：9 项 Web 测试全部通过。
- `cargo test`：全仓通过；0 项失败，保留 1 项既有 Windows 条件性 ignored 测试。
- `cargo build -p yunxi-agent-cli --release --bins`：通过。
- Playwright + 系统 Edge 验证桌面 `1440 x 1000`、降级 `1280 x 900` 和移动端 `390 x 844`。
- 6 张图片均返回正确尺寸、alt、tooltip、序号和循环索引；第 7 次点击回到第 1 张。
- 两个视图按钮不会误触角色详情；角色卡详情仍可正常打开。
- 桌面和移动端 document/body 均无溢出，控制台与 page error 均为空；GSAP 被阻断时图片切换仍正常。
- 开发验证证据：`D:\YunXi Agent\.tmp\web-expression-gallery`。
- 正式安装验证证据：`D:\YunXi Agent\.tmp\web-expression-gallery-installed`。

## 安装

- 安装目录：`D:\Apps\YunXi Agent\bin`。
- 回滚备份：`D:\Apps\YunXi Agent\bin\backup-20260805-125640-her-expression-gallery`。
- 备份中的两个旧二进制均逐文件核对 SHA-256，与替换前安装文件一致。
- 安装版 `yunxi.exe` SHA-256：`6A9684DFCE13A8978488B9436265C59E65CB04AA3B99D956ADAE63C86CE8B374`。
- 安装版 `yunxi-agent-cli.exe` SHA-256：`D38FF11C7BB1FEAFD938CDD253538DF411B0EFC37F9393603F2572BEE60F37E6`。
- 两个安装二进制的 `--version` 均返回 `2.3.3-hotfix.15`。
- 正式 Web 由安装版 `D:\Apps\YunXi Agent\bin\yunxi.exe` 监听 `127.0.0.1:17861`，进程 PID 为 `21500`。
- 正式 `/api/health` 返回 `status=ok`、`version=2.3.3-hotfix.15`；启动参数包含 `--no-weixin-autostart`，本轮未创建或重启微信网关。

## 安全与发布约束

未执行目录移动、递归清理、用户目录删除、`git clean`、工作区重置、force push 或历史 tag 删除。用户提供的 6 张源图只读处理；权威灵魂、长期记忆、人格、背景图和默认角色卡图均未改写。安装替换前已创建可回滚备份并完成逐文件哈希校验。本版本将发布为新的 annotated tag `v2.3.3-hotfix.15`，保留全部历史 tag。

署名：开发者
