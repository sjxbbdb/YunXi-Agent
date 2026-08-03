# YunXi Agent Web 记忆气泡呼吸光开发报告

时间：2026-08-04 07:27:19 +08:00

## 开发目标

在不改变记忆检索、记忆写入、气泡布局、点击破裂与详情展开逻辑的前提下，为记忆气泡增加克制、错峰、低开销的律动呼吸光效。

## 设计与实现依据

- 已先参考 21st MCP 的 Circular Carousel 与 Orbiting Circles 交互方向，仅用于错峰节奏和空间动态判断，未复制组件代码
- 使用 `gsap-core` 与 `gsap-performance` 规范约束动画属性、弱动画降级和持续动画开销
- 呼吸光只动画独立伪元素的 `transform` 与 `opacity`，没有动画布局属性、滤镜或阴影参数
- 每条记忆使用稳定种子生成呼吸周期、相位、缩放幅度和最大亮度，重新加载后节奏稳定，气泡之间不会同步闪烁

## 实际修改

### JavaScript

- 为每个记忆气泡生成 `5.6–8.8 秒`的确定性呼吸周期
- 使用负延迟将不同气泡分散到不同呼吸相位
- 为每个气泡生成受限的缩放幅度与最大光晕透明度
- 参数通过 CSS 自定义属性传递，不增加持续 JavaScript 事件循环

### CSS

- 新增 `memoryBreath` 关键帧
- 在气泡独立伪元素上增加双层静态光晕，并通过缩放与透明度形成呼吸感
- 保留现有漂浮、指针折射、悬浮和点击破裂动画
- `prefers-reduced-motion` 下关闭呼吸循环，保留静态低强度微光

## 修改路径

- `D:\YunXi Agent\crates\yunxi-agent-cli\src\web\app.js`
- `D:\YunXi Agent\crates\yunxi-agent-cli\src\web\app.css`

## 验证结果

- `node --check crates/yunxi-agent-cli/src/web/app.js`：通过
- `cargo fmt --check`：通过
- `cargo test`：通过，零失败，1 个仓库既有 ignored 测试
- `cargo build -p yunxi-agent-cli --release --bins`：通过
- 桌面端气泡数量：6
- 不同时间采样发生变化的气泡：6/6
- 呼吸周期样本：`6.45–8.17 秒`
- 气泡重叠对数：0
- 活跃动画帧间隔 p95：`7.1 ms`
- 移动端页面：`390 x 844`，无横向或纵向意外滚动
- reduced motion：`animation-name=none`、静态透明度 `0.22`、无 `will-change`
- 控制台错误：0
- 请求失败：0

## 视觉证据

- `D:\YunXi Agent\docs\reports\development\evidence\2026-08-04-memory-breathing-glow-desktop.png`
- `D:\YunXi Agent\docs\reports\development\evidence\2026-08-04-memory-breathing-glow-mobile.png`

## 运行与安装状态

- 新构建验证地址：`http://127.0.0.1:17861/`
- Web 健康检查：`status=ok`，`version=2.3.3-hotfix.5`
- 已确认实际服务的 CSS 与 JavaScript 包含呼吸光实现
- 安装前备份已创建并校验：`D:\Apps\YunXi Agent\bin\backups\20260804-072527`
- 首次替换因安装入口被用户运行中的 YunXi CLI 占用而立即停止，没有覆盖、删除或移动安装文件

正式安装完成时间：2026-08-04 07:39:26 +08:00

- 用户确认继续后，仅停止已核验路径和父子关系的 YunXi CLI 与微信子进程
- 已替换 `D:\Apps\YunXi Agent\bin\yunxi.exe`
- 已替换 `D:\Apps\YunXi Agent\bin\yunxi-agent-cli.exe`
- 两个安装文件均与 release 构建完成 SHA-256 一致性验证
- 正式 Web 进程来源：`D:\Apps\YunXi Agent\bin\yunxi.exe`
- 正式 Web 健康检查：`status=ok`，`version=2.3.3-hotfix.5`
- 微信网关由正式 Web 自动拉起，状态 `ready`
- 微信凭据状态 `present`，待入站与待发送数量均为 0
- 临时预览端口 `17862` 未监听

署名：开发者
