# YunXi Agent 人格层卡片切换开发日志

## 时间戳

2026-08-02 04:10（Asia/Shanghai）

## 本次目标

将 Web 人格页改造成可切换的人格层卡片展示：中央展示当前层，左右展示相邻层，并保留原有 active persona 档案、灵魂星图、人格构成和运行指标。

## 变更内容

1. 在 `crates/yunxi-agent-cli/src/web/index.html` 增加 Persona layers 卡片台：
   - 中央当前卡片；
   - 左右相邻卡片；
   - 前后切换按钮；
   - 圆点索引；
   - 键盘左右方向键支持；
   - 可访问的 `aria-label`、`role` 和当前状态标记。
2. 在 `crates/yunxi-agent-cli/src/web/app.js` 增加人格层数据和切换逻辑：
   - 从 `/api/persona` 的真实 active profile 生成 Identity、Soul、Values、Voice、Companion、Work style、Boundaries、Addressing、Constraints 九层卡片；
   - 支持卡片点击、箭头按钮、圆点和键盘切换；
   - API 数据缺失时使用安全的展示降级文本；
   - 尊重 `prefers-reduced-motion`；
   - 使用 GSAP 的 `xPercent`、`y`、`scale`、`rotation`、`autoAlpha` 和 `overwrite: "auto"` 实现堆叠过渡。
3. 在 `crates/yunxi-agent-cli/src/web/app.css` 增加卡片台视觉系统：
   - 深色 Apple-inspired 画布；
   - 中央卡片高亮、邻近卡片降饱和；
   - 层级专属强调色；
   - 响应式布局；
   - 键盘焦点、hover 和 reduced-motion 样式。

## 验证

- `node --check crates/yunxi-agent-cli/src/web/app.js`：通过。
- `cargo fmt --all -- --check`：通过。
- `cargo test -p yunxi-agent-cli web::tests`：5/5 通过。
- `cargo build --release -p yunxi-agent-cli`：通过。
- `cargo test`：全仓测试通过，0 失败。
- `GET http://127.0.0.1:17861/api/health`：HTTP 200，版本 `2.3.3-hotfix.3`。
- `GET http://127.0.0.1:17861/api/persona`：HTTP 200，返回真实 `yunxi_companion_strong` profile。
- Web 首页检查：已包含 `persona-card-deck` 和 `persona-layer-counter`。

## 安装替换

- 已停止旧的 `D:\Apps\YunXi Agent\bin\yunxi.exe` 进程。
- 已安装 release 构建：
  - `D:\Apps\YunXi Agent\bin\yunxi.exe`
  - `D:\Apps\YunXi Agent\bin\yunxi-agent-cli.exe`
- 已保留旧版本备份：
  - `D:\Apps\YunXi Agent\bin\backup-20260803-041017-web-persona-carousel`
- 已重新拉起本地 Web 服务，监听 `127.0.0.1:17861`。

## GitHub

本次仅完成本地开发、测试和安装替换，没有创建或推送新 tag，也没有修改历史 tag。

## 署名

开发者
