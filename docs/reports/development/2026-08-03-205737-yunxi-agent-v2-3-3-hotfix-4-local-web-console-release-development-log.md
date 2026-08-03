# YunXi Agent v2.3.3-hotfix.4 本地 Web 控制台发布开发日志

## 基本信息

- 完成时间：2026-08-03 20:57:37 +08:00
- 发布版本：`2.3.3-hotfix.4`
- 发布分支：`master`
- 发布 tag：`v2.3.3-hotfix.4`
- 远端仓库：`https://github.com/sjxbbdb/YunXi-Agent`
- 项目路径：`D:\YunXi Agent`
- 署名：开发者

## 发布范围

### 本地 Web 服务

- 在 `crates/yunxi-agent-cli/src/web.rs` 增加基于 Axum 的本地 Web 服务。
- 增加 `yunxi web` 命令，默认仅绑定 `127.0.0.1:17861`。
- 提供健康、运行状态、人格、长期记忆和聊天接口。
- Web 聊天继续使用现有 Runtime、Provider、审批、工具、记忆和陪伴边界，没有建立第二套 Agent 逻辑。
- `yunxi web` 支持与交互式 CLI 相同的微信自动拉起策略，并支持 `--no-weixin-autostart` 显式关闭。

### Web 界面

- 增加聊天、长期记忆和人格三个主界面以及仅图标悬浮导航。
- 长期记忆以不重叠气泡呈现，点击后显示详情；漂浮视觉层与稳定点击命中区分离。
- 人格层以可切换卡片呈现，点击后进入全屏详情。
- 统一桌面和移动端布局，保留键盘焦点、关闭、快速重开和减少动态效果路径。
- 动画优先使用 transform 和 opacity；GSAP 未加载时自动回退 Web Animations API。
- 移除大面积动态模糊、`150vmax` 扩散层、人格 3D 滤镜和全屏 `clip-path` 动画。

### 微信自动拉起

- 增加微信工作区解析，Web、TUI 和交互式 CLI 可定位实际保存账户状态的工作区。
- 已存在活跃账户锁时不会重复启动微信网关。
- 启动日志、状态存储和运行配置统一使用解析后的工作区。

### 文档与治理

- README 更新到 `v2.3.3-hotfix.4`，增加本地 Web 启动、访问地址、接口边界和微信自动拉起说明。
- AGENTS.md 增加 Web 设计时优先参考 21st MCP 的约束，并明确不得破坏运行、审批、记忆、人格、陪伴和微信边界。
- 保存每轮 Web 人格、记忆、重构、动画修复日志及桌面/移动端截图证据。

## 验证

- `cargo fmt --check`：通过。
- `node --check crates/yunxi-agent-cli/src/web/app.js`：通过。
- `cargo test`：全仓通过，0 失败；1 项既有 Windows 独立进程测试按配置忽略。
- `cargo build -p yunxi-agent-cli --release --bins`：通过。
- `http://127.0.0.1:17862/api/health`：返回 `status=ok`、`version=2.3.3-hotfix.4`。
- Playwright + 本机 Edge：聊天、记忆、人格、详情展开和关闭通过，无失败请求或页面脚本错误。
- 桌面 `1440x1000` 与移动端 `390x844`：记忆气泡重叠为 0，横向和纵向溢出为 0。
- 动画性能：人格详情和 Dock 扫动最大帧耗时约 7.1 毫秒；记忆展开无超过 50 毫秒的帧。

## 发布产物

- `D:\YunXi Agent\target\release\yunxi.exe`
  - 大小：13,626,368 bytes
  - SHA-256：`442C871B955BB9D0BF07D0DB7ED14164963E05D2F20D5CC57029747B7CBD00EC`
- `D:\YunXi Agent\target\release\yunxi-agent-cli.exe`
  - 大小：13,631,488 bytes
  - SHA-256：`783B6D8A8D0D3753B20F9CF3F7ECE3D1B78D4C6D8E92230A3A2609D3BCEA524B`

## Git 安全记录

- 不删除、不覆盖、不移动任何历史 tag。
- 不使用 force push。
- 本地旧 LF/CRLF 双轨提交历史保留在 `legacy/local-crlf-history-20260803` 分支。
- 新 `master` 直接基于远端 `origin/master` 整理，发布可正常快进。
- 发布整理 stash 保留为恢复点，未删除。

## 署名

开发者
