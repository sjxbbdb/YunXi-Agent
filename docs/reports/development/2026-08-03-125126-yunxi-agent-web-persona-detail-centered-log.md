# YunXi Agent Web 人格卡片居中与详情交互开发日志

时间戳：2026-08-03 12:51:26 +08:00

## 本次目标

依据用户对人格页的反馈完成两项调整：

- 修复宽屏下人格卡片组没有真正居中的问题。
- 将人格卡片从摘要预览扩展为“点击查看详情”，为后续人格定制保留明确入口。

## 更改路径

- `D:\YunXi Agent\crates\yunxi-agent-cli\src\web\index.html`
  - 在人格卡片组和预设人格组件之间新增 `#persona-detail-panel`。
  - 增加详情图标、人格层标题、完整描述、关闭按钮和定制入口预留提示。

- `D:\YunXi Agent\crates\yunxi-agent-cli\src\web\app.js`
  - `buildPersonaLayers()` 为每个人格层保留未截断的 `detail` 内容。
  - 新增 `renderPersonaDetail()`、`closePersonaDetail()` 和详情面板 GSAP 动画。
  - 卡片点击打开/关闭详情，支持 Escape 关闭。
  - 增加 `aria-expanded`、`aria-controls`，保持键盘和读屏状态同步。
  - 切换卡片、布局或拖拽导航时自动收起详情，避免详情内容与当前卡片错位。
  - 未新增模型调用、未修改 `/api/persona`，未改变 CLI、微信、记忆和工具调用逻辑。

- `D:\YunXi Agent\crates\yunxi-agent-cli\src\web\app.css`
  - 为 `.persona-view`、`.persona-archive`、人格卡片组和预设区增加明确的自动外边距与满宽约束，修复宽屏居中。
  - 新增紧凑的 `.persona-detail-panel` 及其响应式样式。
  - 详情动画仅使用 `transform` 与 `autoAlpha`，不使用 blur、宽高或位置布局动画。

## 验证记录

- `node --check D:\YunXi Agent\crates\yunxi-agent-cli\src\web\app.js`：通过
- `cargo test -p yunxi-agent-cli web::tests`：5/5 通过
- `cargo build --release -p yunxi-agent-cli`：通过
- `cargo test`：全部通过
- 本地 HTTP 验证：
  - `http://127.0.0.1:17861/api/health` 返回 `ok`
  - `http://127.0.0.1:17861/api/persona` 返回启用中的 `yunxi_companion_strong`，版本 `2.3.3`
  - HTML 包含 `persona-detail-panel`
  - JS 包含 `renderPersonaDetail`
  - CSS 包含居中规则和 `persona-detail-panel[hidden]`

## 安装记录

- 安装目录：`D:\Apps\YunXi Agent\bin`
- 已替换：
- `D:\Apps\YunXi Agent\bin\yunxi.exe`
- `D:\Apps\YunXi Agent\bin\yunxi-agent-cli.exe`
- 安装前备份目录：
  - `D:\Apps\YunXi Agent\bin\backup-20260803-124845-web-persona-detail-centered`
- GSAP 加载失败回退补丁完成后再次备份：
  - `D:\Apps\YunXi Agent\bin\backup-20260803-125430-web-persona-detail-fallback`
- 完成窄屏详情页脚注换行适配后最终安装备份：
  - `D:\Apps\YunXi Agent\bin\backup-20260803-125809-web-persona-responsive-detail`
- 两个安装文件均已与 `target\release` 对比 SHA-256，结果一致。
- 本地 Web 服务已重新拉起并通过健康检查。

署名：开发者
