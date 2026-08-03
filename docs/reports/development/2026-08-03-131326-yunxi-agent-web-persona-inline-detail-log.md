# YunXi Agent Web 人格网格/列表详情交互整改日志

时间戳：2026-08-03 13:13:26 +08:00

## 问题

网格和列表布局中的卡片虽然绑定了点击事件，但详情面板位于整组卡片之后，点击后不在当前视口内，用户会感觉卡片没有展开。

## 更改

- `D:\YunXi Agent\crates\yunxi-agent-cli\src\web\app.js`
  - 网格、列表布局点击卡片后，在当前卡片内部插入完整人格层详情。
  - 堆叠布局继续使用独立详情面板。
  - 网格/列表模式不再显示重复的下方详情面板。

- `D:\YunXi Agent\crates\yunxi-agent-cli\src\web\app.css`
  - 新增 `.persona-layer-inline-detail` 样式。
  - 网格选中卡片横跨整行并展开高度。
  - 列表选中卡片增加详情高度。
  - 保持 transform/opacity 动画原则，不增加 blur 或布局动画。

## 验证

- `node --check`：通过
- `cargo test -p yunxi-agent-cli web::tests`：5/5 通过
- `cargo build --release -p yunxi-agent-cli`：通过
- `cargo test`：全部通过
- HTTP：
  - `/api/health` 返回 `ok`
  - 静态 JS 包含 `persona-layer-inline-detail`
  - 静态 CSS 包含网格和列表展开规则

## 安装

- 安装目录：`D:\Apps\YunXi Agent\bin`
- 已替换 `yunxi.exe` 与 `yunxi-agent-cli.exe`
- SHA-256 与 release 构建一致
- 安装前备份：
  - `D:\Apps\YunXi Agent\bin\backup-20260803-131052-web-persona-inline-detail`

署名：开发者
