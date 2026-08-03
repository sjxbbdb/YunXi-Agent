# YunXi Agent Web 记忆气泡场开发日志

## 时间

- 开始时间：2026-08-03 13:20:00 +08:00
- 完成时间：2026-08-03 13:42:57 +08:00

## 工作目标

将 Web 端“长期记忆”界面改造成可交互的记忆气泡场：

- 每条记忆以带微光的气泡呈现；
- 点击气泡后播放破裂粒子动画并展示完整记忆；
- 支持收回、Escape、Enter/Space 和 reduced-motion 降级；
- 不修改记忆写入、召回、合并、隐私和状态迁移逻辑。

设计参考了 21st 的轨道/环形交互方向，以及 Apple 的克制层级、Linear 的深色技术界面语言；没有复制第三方组件代码或品牌素材。

## 修改内容

### 1. 只读记忆 API

文件：

- `D:\YunXi Agent\crates\yunxi-agent-cli\src\web.rs`

变更：

- 新增 `GET /api/memory`；
- 通过 `FilePersonaMemoryStore::for_workspace(...).list(PersonaMemoryScope::All)` 读取最新记录；
- 返回 workspace fingerprint、记忆记录和解析警告；
- 接口只读，不改变任何记忆状态；
- 增加 Web 路由静态测试断言。

### 2. 记忆页面结构

文件：

- `D:\YunXi Agent\crates\yunxi-agent-cli\src\web\index.html`

变更：

- 替换原来的测试说明卡；
- 新增 `#memory-bubble-field` 气泡容器；
- 新增记忆状态信号、空状态和“去聊天测试召回”入口；
- 保留语义化 `role=list`、`aria-live` 和键盘操作提示。

### 3. 视觉与响应式样式

文件：

- `D:\YunXi Agent\crates\yunxi-agent-cli\src\web\app.css`

变更：

- 新增气泡微光、核心、粒子、展开卡片和空状态样式；
- 气泡尺寸由 importance 决定，颜色由 status 决定；
- 使用暗色苹果风格的低对比层级和紫青色微光；
- 动画元素使用 `will-change: transform, opacity`；
- 移动端调整为两列/单列布局；
- `prefers-reduced-motion` 下关闭过渡。

### 4. 交互与动画

文件：

- `D:\YunXi Agent\crates\yunxi-agent-cli\src\web\app.js`

变更：

- 新增 `/api/memory` 加载、缓存和渲染逻辑；
- 新增气泡点击展开、收回、Escape 关闭和焦点可访问性；
- 使用现有 `ensureGsap()` 动态加载 GSAP；
- 破裂动画由 11 个粒子沿 `x/y` 扩散并淡出；
- 气泡使用 CSS 变量 `--bubble-scale` 配合 `autoAlpha`，避免覆盖定位变换；
- 空状态、API 失败和 GSAP 不可用时均立即降级；
- 仅对 `transform`、`scale`、`x/y`、`autoAlpha` 做动画，不改变布局尺寸；
- 增加异常 score 的边界处理，避免无效数据生成 `NaN` 尺寸。

## 构建与安装

- Release 构建：
  - `D:\YunXi Agent\target\release\yunxi.exe`
  - `D:\YunXi Agent\target\release\yunxi-agent-cli.exe`
- 安装目录：
  - `D:\Apps\YunXi Agent\bin\yunxi.exe`
  - `D:\Apps\YunXi Agent\bin\yunxi-agent-cli.exe`
- 最终安装前备份：
  - `D:\Apps\YunXi Agent\bin\backup-20260803-134047-web-memory-bubbles-final`
- 安装文件与 Release 文件 SHA-256 已核对一致；
- 安装后版本：`yunxi 2.3.3-hotfix.3`；
- 当前 Web 服务：`http://127.0.0.1:17861`；
- 当前 Web 启动同时拉起本地微信网关，进程路径均为 `D:\Apps\YunXi Agent\bin\yunxi.exe`。

## 验证结果

- `node --check crates/yunxi-agent-cli/src/web/app.js`：通过；
- `cargo fmt --all -- --check`：通过；
- `cargo test -p yunxi-agent-cli web::tests`：5/5 通过；
- `cargo build --release -p yunxi-agent-cli`：通过；
- `cargo test`：全量通过；
- `GET /api/health`：HTTP 200，返回 `status=ok`；
- `GET /api/memory`：HTTP 200，当前返回 6 条记录，0 个警告；
- `/`、`/assets/app.js`、`/assets/app.css`：HTTP 200；
- 页面、脚本和样式均确认包含气泡场、破裂动画和 reduced-motion 降级逻辑。

## 安全边界

- 未删除、递归清理、移动或覆盖用户目录；
- 未修改记忆存储文件；
- 仅停止并重启已确认路径下的 YunXi Web/微信进程；
- 未执行 `git reset`、`git clean`、force 操作；
- 未创建或推送 GitHub tag，本轮仅完成本地开发、安装与验证。

开发者
