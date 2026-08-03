# YunXi Agent Web 人格页紧凑化整改日志

时间戳：2026-08-03 11:06:26 +08:00

## 本次目标

依据用户在浏览器中的标注，对网页端人格页进行紧凑化整改：

- 删除占据过大空间的身份展示卡。
- 删除占据过大空间的 Persona / Soul 画布。
- 删除人格构成大面板。
- 删除 Runtime Signal 面板。
- 保留 MorphingCardStack 核心卡片交互。
- 新增小巧的预设人格展示组件。
- 修复布局切换时出现的视觉模糊感。

## 更改路径

- `D:\YunXi Agent\crates\yunxi-agent-cli\src\web\index.html`
  - 移除 `.persona-identity-stage`。
  - 移除 `.persona-profile-card`。
  - 移除 `.persona-soul-canvas`。
  - 移除 `.persona-composition`。
  - 移除 `.persona-metrics-panel`。
  - 保留 `.persona-morph-stack`。
  - 新增 `.persona-preset-dock` 与 `#persona-preset-grid`。

- `D:\YunXi Agent\crates\yunxi-agent-cli\src\web\app.css`
  - 将人格页最大宽度收缩到 720px。
  - 将 MorphingCardStack 容器收缩为小型居中组件。
  - 新增预设人格小卡片样式。
  - 移除布局切换中的宽高动画。
  - 显式覆盖人格卡装饰层 `filter: none`，避免 blur 影响观感。
  - 移除卡片交互 transition 中的 filter 动画。
  - 修正移动端人格页宽度和预设卡响应式布局。

- `D:\YunXi Agent\crates\yunxi-agent-cli\src\web\app.js`
  - 新增 `renderPersonaPresets()`。
  - 新增 `buildPersonaPresets()`。
  - 删除卡片布局切换时的 `autoAlpha` 淡入重绘效果。
  - 页面入场动画改为只做轻量 y 轴 transform，不再对已删除旧面板执行动画。
  - 保持 `/api/persona` 只读使用，不新增假切换按钮。

## 验证记录

- `node --check D:\YunXi Agent\crates\yunxi-agent-cli\src\web\app.js`：通过
- `cargo test -p yunxi-agent-cli web::tests`：通过
- `cargo build --release -p yunxi-agent-cli`：通过
- `cargo test`：通过
- 本地安装后 HTTP 验证：
  - `http://127.0.0.1:17861/api/health` 返回 `ok`
  - 版本：`2.3.3-hotfix.3`
  - HTML 包含 `persona-morph-stack`
  - HTML 包含 `persona-preset-dock`
  - HTML 不再包含 `persona-identity-stage`
  - HTML 不再包含 `persona-composition`
  - HTML 不再包含 `persona-metrics-panel`
  - CSS 包含 `filter: none;`
  - JS 包含 `renderPersonaPresets`
  - JS 不再包含卡片切换用的 `{ autoAlpha: 0 }`

## 安装记录

- 安装目录：`D:\Apps\YunXi Agent\bin`
- 已替换：
  - `D:\Apps\YunXi Agent\bin\yunxi.exe`
  - `D:\Apps\YunXi Agent\bin\yunxi-agent-cli.exe`
- 安装前备份目录：
  - `D:\Apps\YunXi Agent\bin\backup-20260803-110137-web-persona-compact-presets`

## 说明

预设人格组件本次做成展示型小卡片。原因是当前 Web 端尚未提供“激活预设人格”的后端 API；如果直接做可点击切换，会变成假交互。后续若需要真正切换，需要新增安全的 persona profile 激活 API，并接入当前 persona store。

署名：开发者
