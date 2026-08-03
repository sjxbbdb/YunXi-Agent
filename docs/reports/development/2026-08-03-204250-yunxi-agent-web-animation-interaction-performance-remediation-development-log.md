# YunXi Agent Web 动画与交互性能修复开发日志

## 基本信息

- 完成时间：2026-08-03 20:42:50 +08:00
- 当前版本：2.3.3-hotfix.3
- 工作目录：`D:\YunXi Agent`
- 安装目录：`D:\Apps\YunXi Agent\bin`
- 开发者：开发者

## 修复目标

在不改变聊天、长期记忆、人格陪伴、工具调用和微信运行边界的前提下，消除 Web 端动画卡顿、视觉拖影、连续交互迟滞和移动端点击不稳定问题。

## 代码改动

### `D:\YunXi Agent\crates\yunxi-agent-cli\src\web\app.js`

- 增加统一、可取消的合成层动画入口；GSAP 已加载时使用 `fromTo`，未加载或网络不可用时立即回退到 Web Animations API。
- 页面切换只动画轻量文字层，不再移动带毛玻璃的大面积聊天面板和记忆画布。
- Dock 指针位置先批量读取，再通过单个 `requestAnimationFrame` 写入；GSAP 可用时使用 `quickTo` 更新 `scale` 和 `y`。
- 文本框自动增高、窗口缩放和记忆气泡重新布局统一到下一帧执行，避免交错读写布局。
- 记忆粒子数量和持续时间下调，保留破裂反馈并减少同时绘制的元素。
- 记忆气泡拆为稳定按钮命中区和独立漂浮视觉层，解决触控和自动化点击期间目标持续位移的问题。
- 记忆与人格详情增加关闭定时器取消逻辑，快速关闭再打开不会被旧定时器误隐藏。
- 人格详情子内容动画缩短，并避免人格卡组重复执行入场动画。

### `D:\YunXi Agent\crates\yunxi-agent-cli\src\web\app.css`

- GSAP 接管 Dock 后禁用 CSS 的 `transform` 过渡，消除双重插值。
- 记忆展开层由 `150vmax` 模糊圆形改为固定视口覆盖层，只动画 `transform` 和 `opacity`。
- 记忆详情取消动态毛玻璃，展开和关闭时长收敛到 180 至 220 毫秒。
- 人格卡片移除 `translateZ`、`rotateY`、动态亮度滤镜和卡片毛玻璃，改为二维位移、旋转和缩放。
- 人格全屏详情移除 `clip-path` 和动态背景模糊，改为 180 至 240 毫秒的透明度与缩放过渡。
- 保留 `prefers-reduced-motion` 与 `prefers-reduced-transparency` 降级路径。

### `D:\YunXi Agent\crates\yunxi-agent-cli\src\web\index.html`

- 增加内联空 favicon 声明，消除浏览器自动请求 `/favicon.ico` 产生的 404 控制台噪声。

## 验证结果

- `node --check crates/yunxi-agent-cli/src/web/app.js`：通过。
- `cargo test -p yunxi-agent-cli web::tests`：通过，两组共 10 次 Web 测试，无失败。
- `cargo test`：全仓通过，无失败；1 项 Windows 隔离测试按原配置忽略。
- `cargo build -p yunxi-agent-cli --release`：通过。
- Playwright 使用本机 Edge 在 `1440x1000` 和 `390x844` 视口完成路由、Dock、文本框、记忆、人格和快速重开测试。
- 桌面和移动端均展示 6 个记忆气泡，交叉重叠为 0，横向和纵向页面溢出均为 0。
- 减少动态效果模式下粒子数为 0，气泡漂浮关闭，过渡时长降为近零。
- 记忆快速关闭再打开保持 `hidden=false`、`open=true`，人格详情快速交互正常。
- 最终安装版浏览器回归无失败请求、无页面脚本错误，漂浮期间记忆按钮命中区域坐标保持稳定。

## 帧耗时对比

- 修复前路由切换最大帧耗时：55.6 毫秒，超过 32 毫秒 1 帧。
- 修复后记忆路由最大帧耗时：34.9 毫秒，P95 为 7.1 毫秒，超过 50 毫秒 0 帧。
- 修复后记忆展开最大帧耗时：34.7 毫秒，P95 为 7.1 毫秒，超过 50 毫秒 0 帧。
- 修复后人格路由、人格详情和 Dock 扫动最大帧耗时均为 7.1 毫秒，超过 32 毫秒 0 帧。

## 截图证据

- `D:\YunXi Agent\docs\reports\development\evidence\2026-08-03-web-animation-remediation-desktop.png`
- `D:\YunXi Agent\docs\reports\development\evidence\2026-08-03-web-animation-remediation-mobile.png`

## 安装与恢复

- 安装来源：`D:\YunXi Agent\target\release\yunxi.exe`
- 替换目标：`D:\Apps\YunXi Agent\bin\yunxi.exe`
- 兼容入口：`D:\Apps\YunXi Agent\bin\yunxi-agent-cli.exe`
- 备份目录：`D:\Apps\YunXi Agent\bin\backup-20260803-203805-web-animation-interaction`
- 源文件与两个安装入口 SHA-256：`9E4B292B09032964FC750089C13C90DDF7E3935AF42BDF64C15C59F46CC18C6D`
- 首次覆盖遇到 Windows 进程句柄短暂未释放，操作立即停止；确认无残留进程、旧安装和备份哈希一致后才重试，未删除任何文件。
- Web 已恢复至 `http://127.0.0.1:17861/`，健康检查返回 `ok`。
- 微信账户状态为 `ready`，凭据存在，账户锁活跃；启动器已交接给常驻子进程。

## 署名

开发者
