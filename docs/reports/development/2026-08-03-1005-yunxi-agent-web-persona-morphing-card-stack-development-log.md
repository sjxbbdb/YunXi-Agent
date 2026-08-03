# YunXi Agent Web 人格页 Morphing Card Stack 开发日志

时间戳：2026-08-03 10:05:25 +08:00

## 本次目标

依据用户提供的 React / Framer Motion `MorphingCardStack` 源码，将 YunXi Agent 网页端人格模块中的人格层展示改造为等价的 stack / grid / list 三态卡片组件。

## 更改路径

- `D:\YunXi Agent\crates\yunxi-agent-cli\src\web\index.html`
  - 将旧人格层轮播区替换为 MorphingCardStack 等价结构。
  - 保留三种布局切换入口：stack / grid / list。
  - 移除旧的左右箭头、标题控制区和旧轮播控制面。

- `D:\YunXi Agent\crates\yunxi-agent-cli\src\web\app.css`
  - 新增 `.persona-morph-stack` 视觉样式。
  - 新增 icon-only 布局切换按钮。
  - 新增 stack / grid / list 三种卡片布局样式。
  - 新增 top card 拖拽、点击展开、圆点状态等视觉反馈。

- `D:\YunXi Agent\crates\yunxi-agent-cli\src\web\app.js`
  - 将旧 carousel 定位逻辑替换为源码同款 stack order 逻辑。
  - 新增 `personaLayout = stack | grid | list` 状态。
  - 新增 `personaExpandedCard` 点击展开状态。
  - 新增 top card pointer drag / swipe 切换逻辑。
  - 新增 stack 模式圆点索引切换。
  - 保持 `/api/persona` 只读加载，不改 CLI、微信、记忆、工具调用运行管线。

## 验证记录

- `node --check D:\YunXi Agent\crates\yunxi-agent-cli\src\web\app.js`：通过
- `cargo test -p yunxi-agent-cli web::tests`：通过
- `cargo build --release -p yunxi-agent-cli`：通过
- `cargo test`：通过
- 本地安装后验证：
  - `http://127.0.0.1:17861/api/health` 返回 `ok`
  - 版本：`2.3.3-hotfix.3`
  - `/` 返回的新 HTML 包含 `persona-morph-stack`
  - `/assets/app.css` 返回的新 CSS 包含 `.persona-morph-stack`
  - `/assets/app.js` 返回的新 JS 包含 `setPersonaLayout` 与 `personaSwipeThreshold`
  - `/api/persona` 返回 enabled=true，profile.display_name=`YunXi Agent`

## 安装记录

- 安装目录：`D:\Apps\YunXi Agent\bin`
- 已替换：
  - `D:\Apps\YunXi Agent\bin\yunxi.exe`
  - `D:\Apps\YunXi Agent\bin\yunxi-agent-cli.exe`
- 安装前备份目录：
  - `D:\Apps\YunXi Agent\bin\backup-20260803-100329-web-persona-morph-stack`

## 说明

用户当前如果打开的是 `http://127.0.0.1:17861/api/health`，看到 JSON 是正常接口返回，不是网页端损坏。人格页面入口应为：

- `http://127.0.0.1:17861/#persona`

署名：开发者
