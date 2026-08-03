# YunXi Agent Web UI 优雅度审计与重构开发报告

时间：2026-08-03 21:51:31 +08:00

## 目标

在不改变聊天、记忆、人格、运行时、审批、陪伴与微信边界的前提下，对现有 Web 界面进行视觉审计与成品级重构。设计目标是安静、克制、长期耐看，避免营销首页、通用 AI 紫色光效、过量玻璃卡片和无目的动画。

设计判断：本地优先的中文陪伴型 Agent 高频工作界面，采用冷静的银灰与深炭底色，使用单一低饱和鼠尾草绿强调色。

设计参数：

- DESIGN_VARIANCE：6
- MOTION_INTENSITY：5
- VISUAL_DENSITY：3

## 参考与规范

已使用并落实以下设计与动画规范：

- design-taste-frontend：审计现有信息层级、材质、颜色、排版与 AI 模板化特征
- gsap-core：统一 transform、opacity、quickTo 与 reduced motion 行为
- gsap-timeline：为记忆展开和人格详情建立有语义的时间线
- gsap-utils：导航 Dock 的距离映射与数值约束
- gsap-performance：限制动画属性、避免布局抖动、清理活动时间线

已按项目要求先参考 21st MCP 中的 Dock、AI Assistant Interface、Circular Carousel 与 Radial Orbital Timeline。参考仅用于设计判断，未覆盖 YunXi Agent 的运行时和数据边界。

## 审计结论

改造前的主要问题：

- 聊天页使用巨大产品名与玻璃面板，表现更接近营销首屏而不是工作界面
- 记忆气泡使用多色外发光，视觉偏游戏化，原始记忆前缀使摘要难以阅读
- 人格页同时展示五张大卡片，侧卡文字截断并制造视觉噪声
- 人格详情层过暗，阅读层级与卡片页差异不足
- Dock 提示在点击导航后残留，并可能压住页面标题
- 图标由多套 CSS 形状组成，线宽和视觉语义不统一

改造后的评价：

- 视觉层级已经从展示型首页收敛为单一工作空间
- 色彩、边框、圆角、字体重量和交互反馈形成统一系统
- 动效只用于状态变化，不使用模糊、布局属性动画或无目的循环装饰
- 桌面与移动端均保持清晰、稳定、无页面级意外滚动
- 当前成品达到克制、安静、精致的目标，不依赖流行 AI 视觉符号制造质感

## 实际修改

### 导航

- 使用同一套 Lucide 线性图标替换 CSS 手绘图标
- 保留图标导航和悬浮文字提示
- 使用 GSAP quickTo 保留邻近放大效果
- 点击导航后立即隐藏提示，直到指针离开 Dock
- 触控和粗指针设备完全禁用悬浮提示

### 聊天

- 移除营销式巨大标题和“本地陪伴智能体”眉题
- 将产品名、陪伴短句和真实运行状态整理成紧凑工作区标题栏
- 去掉聊天面板外层玻璃卡片，只保留消息区与主输入面板
- 助手回复使用无框阅读布局，用户消息保留明确但克制的对比
- 修复输入框聚焦时的双层高亮边框
- 发送按钮使用统一线性箭头图标

### 记忆

- 移除操作说明文案和装饰轨道
- 活跃记忆统一为鼠尾草绿光学玻璃质感
- 仅对 pending、rejected、archived 保留真实语义色
- 新增摘要清理逻辑，去除“用户纠正/限制”“目标候选”等存储前缀
- 小尺寸气泡自动隐藏难以阅读的摘要
- 保留确定性分区布局、随机漂浮和零碰撞点击区域
- 记忆展开使用 GSAP 时间线依次呈现扩散层、正文与元数据

### 人格

- 可见卡片由五张收敛为三张，仅展示当前卡和左右相邻预览
- 侧卡只显示层级标题，避免截断长正文
- 移除重复的“点击查看”“切换”和页码文字
- 缩短卡片高度，改善页面中心视觉重心
- 人格详情改为两栏阅读结构，左侧展示个性化解释，右侧展示人格原文
- 详情动效使用 transform 与 opacity 时间线，不使用模糊或 clip-path
- 修复移动端箭头隐藏导致卡组进入错误网格列的问题

## 修改路径

- `D:\YunXi Agent\crates\yunxi-agent-cli\src\web\index.html`
- `D:\YunXi Agent\crates\yunxi-agent-cli\src\web\app.css`
- `D:\YunXi Agent\crates\yunxi-agent-cli\src\web\app.js`

## 验证结果

Playwright 实机验证：

- 桌面视口：1440 x 960
- 移动视口：390 x 844
- Dock 悬浮提示：悬浮 opacity 1，点击后 opacity 0
- 触控设备提示：display none
- 记忆气泡数量：6
- 气泡重叠对数：0
- 记忆页页面级滚动：无
- 人格可见卡片：3
- 移动端活动卡片范围：left 54.60px，right 335.40px，完整位于视口内
- 人格键盘切换、详情打开、快速关闭重开：通过
- prefers-reduced-motion：通过
- 控制台错误：0
- 请求失败：0

代码与集成验证：

- `node --check crates/yunxi-agent-cli/src/web/app.js`：通过
- `cargo fmt --check`：通过
- `cargo test`：通过，零失败，1 个仓库既有 ignored 测试
- `cargo build -p yunxi-agent-cli --release --bins`：通过
- Git whitespace 检查：通过
- 禁用破折号、负字距和视口缩放字体扫描：无命中

第一次执行 `cargo test` 时，预览进程锁定 `target\debug\yunxi.exe`，Cargo 无法移除该中间构建产物并立即停止。确认占用者为本轮 `17862` 预览后，仅停止该进程，未手动删除文件，未触碰用户目录；重新执行后全量测试通过。

## 证据

基线与最终截图位于：

`D:\YunXi Agent\docs\reports\development\evidence\`

最终截图文件前缀：

`2026-08-03-ui-review-final-`

## 安装记录

安装时间：2026-08-03 22:02:56 +08:00

- 正式安装目录：`D:\Apps\YunXi Agent\bin`
- 已替换入口：`yunxi.exe`、`yunxi-agent-cli.exe`
- 安装版本：`2.3.3-hotfix.4`
- 旧版备份：`D:\Apps\YunXi Agent\bin\backups\20260803-220021`
- 正式 Web 地址：`http://127.0.0.1:17861/`
- 健康检查：status ok，version 2.3.3-hotfix.4
- Web 静态资源：新紧凑标题与 SVG Dock 已确认加载
- 微信网关：随正式 Web 服务自动启动并报告 ready
- 临时预览：`17862` 已停止，仅保留正式 `17861` 服务

## 雾冰蓝主题变更

变更与安装时间：2026-08-03 22:16:02 +08:00

- 主题方向：深空银灰 + 雾冰蓝
- 页面背景：`#0b0d0e`
- 主表面：`#14181a`
- 抬升表面：`#1a2023`
- 主强调色：`#9fc5d6`
- 强调高亮：`#c7dfe8`
- 等待语义色：`#c5a16f`
- 拒绝语义色：`#bd858c`
- 已清除 Web CSS 与记忆气泡逻辑中的旧鼠尾草绿色
- 最低文字对比度：`5.57:1`，满足 WCAG AA
- Playwright 验证：桌面与移动端无控制台错误、无请求失败、气泡零重叠
- 全仓 `cargo test`：通过，零失败
- release 构建：通过
- 本次安装前备份：`D:\Apps\YunXi Agent\bin\backups\20260803-221343`
- 正式服务：`http://127.0.0.1:17861/`
- 微信网关：随 Web 服务重新启动并报告 ready

## 交互动画与组件质感打磨

变更与验证时间：2026-08-03 22:34:24 +08:00

本轮在雾冰蓝主题和既有页面结构上增加克制的状态动效，不改路由 ID、API 契约、运行时、审批、记忆、人格、陪伴或微信逻辑。

实现内容：

- 聊天、记忆和人格页面使用可中断的 GSAP 入场时间线，快速切换时先清理旧时间线，避免页面残影
- Dock 图标点击增加短促回弹，并在动画结束后清除内联 transform，不干扰既有邻近放大交互
- 新消息到达时使用位移、缩放和透明度组合进入；pending 转正式回复时按消息签名识别状态变化
- 输入面板增加随指针移动的低强度表面高光和聚焦抬升，发送图标提供即时按压反馈
- 记忆气泡增加随指针变化的折射高光和轻微悬浮反馈，保留原有漂浮、碰撞约束与点击展开逻辑
- 人格活动卡片增加受限的 `quickTo` 3D 倾斜与表面高光，侧卡仅提供轻微悬浮反馈
- 所有主要动画仅使用 transform 与 opacity，未引入布局属性动画、滚动劫持、模糊动画或持续装饰动画
- `prefers-reduced-motion`、粗指针和触控设备自动停用指针材质与 3D 响应

实测结果：

- GSAP 正常加载
- 消息进入与 Dock 回弹完成后 transform 均恢复为 `none`
- 快速连续导航后仅目标人格视图可见
- 记忆气泡悬浮状态重叠对数：0
- 合成指针负载帧间隔 p95：7.1 ms，最大值：7.1 ms
- reduced motion 动画时长：近零，且不添加指针交互类
- 移动端不添加指针交互类，页面无意外滚动
- 控制台错误：0
- 请求失败：0
- `cargo fmt --check`：通过
- `cargo test`：通过，零失败，1 个仓库既有 ignored 测试
- `cargo build -p yunxi-agent-cli --release --bins`：通过

新增证据：

- `D:\YunXi Agent\docs\reports\development\evidence\2026-08-03-interaction-polish-chat.png`
- `D:\YunXi Agent\docs\reports\development\evidence\2026-08-03-interaction-polish-memory.png`
- `D:\YunXi Agent\docs\reports\development\evidence\2026-08-03-interaction-polish-persona.png`
- `D:\YunXi Agent\docs\reports\development\evidence\2026-08-03-interaction-polish-mobile-persona.png`

### 交互打磨版安装记录

安装与运行验证时间：2026-08-03 22:40:19 +08:00

- 安装前备份：`D:\Apps\YunXi Agent\bin\backups\20260803-223604`
- 正式入口：`D:\Apps\YunXi Agent\bin\yunxi.exe`
- 兼容入口：`D:\Apps\YunXi Agent\bin\yunxi-agent-cli.exe`
- 两个备份文件均完成 SHA-256 一致性验证
- 两个安装文件均与本轮 release 构建完成 SHA-256 一致性验证
- 正式 Web 健康检查：`status: ok`，版本 `2.3.3-hotfix.4`
- 实际静态资源已确认包含页面入场、指针材质、发送反馈与 reduced motion 逻辑
- 微信网关由正式 Web 服务自动拉起，状态 `ready`
- 微信凭据状态 `present`，待发送、待入站与待审批数量均为 0
- 临时 `17862` 预览已停止，正式服务继续监听 `http://127.0.0.1:17861/`

署名：开发者
