# YunXi Agent Web 记忆自由漂浮气泡整改日志

## 时间

- 开始时间：2026-08-03 13:49:00 +08:00
- 完成时间：2026-08-03 14:05:23 +08:00

## 工作目标

用户反馈上一版记忆气泡仍然固定在屏幕网格中，期望每条记忆像小气泡一样随机漂浮。

本次目标：

- 去除记忆气泡场的固定网格布局；
- 将每条记忆变成容器内随机分布、缓慢漂浮的自由气泡；
- 保留点击破裂、展开详情、收回和降级逻辑；
- 不修改记忆存储、写入、召回、合并、隐私策略。

## 参考与设计依据

- 已调用 21st MCP 查询 floating / orbital / memory bubble 类交互参考；结果主要是 floating dock、AI interface、circular carousel 等方向性参考，没有直接采用第三方代码。
- 已读取 `awesome-design-md` 中 Apple 与 Framer 设计参考：
  - Apple：界面 chrome 后退，内容主体突出，层级克制；
  - Framer：深色画布、艺术板式空间感、漂浮组件；
- 已按 GSAP Core 与 GSAP Performance 约束实现动画：
  - 只动画 `x`、`y`、`rotation`、`scale`、`autoAlpha`；
  - 不动画 `width`、`height`、`top`、`left`；
  - 保留 `prefers-reduced-motion` 降级。

## 修改内容

### 1. 自由漂浮布局

文件：

- `D:\YunXi Agent\crates\yunxi-agent-cli\src\web\app.css`

变更：

- `.memory-bubble-field` 从 `grid` 改为自由定位画布；
- 移除 `repeat(auto-fit, minmax(...))` 固定格子；
- `.memory-bubble-node` 改为 `position: absolute`；
- 设置独立气泡节点尺寸与 `will-change: transform`；
- 移动端仍保持有界画布高度，避免气泡漂出区域。

### 2. 随机但有界的位置生成

文件：

- `D:\YunXi Agent\crates\yunxi-agent-cli\src\web\app.js`

变更：

- 新增 `memoryHashSeed()`、`memoryRandom()`，基于记忆 ID 生成稳定随机种子；
- 新增 `positionMemoryBubbles()`，根据容器尺寸为每条记忆生成随机位置；
- 位置生成带简单碰撞避让评分，降低气泡互相压住的概率；
- 位置通过 `transform: translate3d(...)` 写入，不使用动画布局属性。

### 3. 慢速漂浮轨道

文件：

- `D:\YunXi Agent\crates\yunxi-agent-cli\src\web\app.js`

变更：

- `startMemoryIdleMotion()` 从原来的轻微上下浮动改为二维漂浮轨道；
- 每条气泡围绕自己的 home position 独立漂移；
- 漂移速度、方向、幅度由稳定种子决定；
- 点击气泡时停止该气泡漂移，气泡在当前漂浮位置破裂并展开详情；
- 收回后恢复漂移。

### 4. 响应式重排

文件：

- `D:\YunXi Agent\crates\yunxi-agent-cli\src\web\app.js`

变更：

- 新增 `scheduleMemoryReflow()` 与 `reflowMemoryBubbles()`；
- 浏览器 resize 时重新计算随机有界位置；
- 如果 resize 时详情卡片已打开，会先安全收回，再重排。

## 验证结果

- `node --check crates/yunxi-agent-cli/src/web/app.js`：通过；
- `cargo fmt --all -- --check`：通过；
- `cargo test -p yunxi-agent-cli web::tests`：5/5 通过；
- `cargo build --release -p yunxi-agent-cli`：通过；
- `cargo test`：全量通过；
- 安装版 `GET /api/health`：HTTP 200，`status=ok`；
- 安装版 `GET /api/memory`：HTTP 200，当前 6 条记忆、0 个警告；
- 安装版 `/assets/app.js`：
  - 包含 `positionMemoryBubbles`；
  - 包含 `memoryRandom`；
  - 包含 `driftX`；
  - 不再包含旧 `grid-template-columns: repeat(auto-fit...)`；
- 安装版 `/assets/app.css`：
  - 包含 absolute 气泡节点；
  - 包含自由画布高度；
  - 不再包含旧 auto-fit 网格；
  - 包含 `will-change: transform`。

## 安装替换

- 安装目录：`D:\Apps\YunXi Agent\bin`
- 已替换：
  - `D:\Apps\YunXi Agent\bin\yunxi.exe`
  - `D:\Apps\YunXi Agent\bin\yunxi-agent-cli.exe`
- 安装前备份：
  - `D:\Apps\YunXi Agent\bin\backup-20260803-140355-web-memory-floating-bubbles`
- 安装后版本：
  - `yunxi 2.3.3-hotfix.3`
- 当前运行进程：
  - Web：`D:\Apps\YunXi Agent\bin\yunxi.exe --provider-live web`
  - 微信网关：由 Web 启动同步拉起，路径同为 `D:\Apps\YunXi Agent\bin\yunxi.exe`

## 安全边界

- 未删除、递归清理、移动或覆盖用户目录；
- 未修改任何记忆数据文件；
- 未修改记忆写入、召回、合并、隐私和状态迁移逻辑；
- 仅停止并重启明确安装路径下的 YunXi 进程；
- 未执行 `git reset`、`git clean`、force 操作；
- 未创建或推送 GitHub tag。

开发者
