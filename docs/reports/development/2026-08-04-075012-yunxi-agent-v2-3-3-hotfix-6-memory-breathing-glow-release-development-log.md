# YunXi Agent v2.3.3-hotfix.6 记忆呼吸光发布日志

时间：2026-08-04 07:50:12 +08:00

## 发布信息

- 发布版本：`2.3.3-hotfix.6`
- 发布 tag：`v2.3.3-hotfix.6`
- GitHub 仓库：`https://github.com/sjxbbdb/YunXi-Agent`
- 目标分支：`master`
- 历史 tag：全部保留，不移动、不覆盖

## 发布目标

在保持 CLI、运行时、审批、记忆数据、人格、陪伴和微信边界不变的前提下，将 Web 记忆气泡的错峰呼吸光整理为独立新包体。

## 实际修改

- 工作区版本由 `2.3.3-hotfix.5` 更新为 `2.3.3-hotfix.6`
- README 当前版本与安装说明同步更新
- Cargo.lock 中工作区包版本同步更新
- 每个记忆气泡使用稳定种子生成呼吸周期、负延迟、缩放幅度和最大亮度
- 新增独立 `memoryBreath` 光晕动画，不改变气泡定位、漂浮、点击破裂或详情展开逻辑
- 呼吸光只动画伪元素的 `transform` 与 `opacity`
- `prefers-reduced-motion` 下关闭循环并保留静态微光
- 增加桌面与移动端视觉证据及完整开发报告

## 修改路径

- `D:\YunXi Agent\Cargo.toml`
- `D:\YunXi Agent\Cargo.lock`
- `D:\YunXi Agent\README.md`
- `D:\YunXi Agent\crates\yunxi-agent-cli\src\web\app.js`
- `D:\YunXi Agent\crates\yunxi-agent-cli\src\web\app.css`
- `D:\YunXi Agent\docs\reports\development\2026-08-04-072719-yunxi-agent-web-memory-breathing-glow-development-report.md`
- `D:\YunXi Agent\docs\reports\development\evidence\2026-08-04-memory-breathing-glow-desktop.png`
- `D:\YunXi Agent\docs\reports\development\evidence\2026-08-04-memory-breathing-glow-mobile.png`

## 验证结果

- `node --check crates/yunxi-agent-cli/src/web/app.js`：通过
- `cargo fmt --check`：通过
- `cargo test`：通过，零失败，1 个仓库既有 ignored 测试
- `cargo build -p yunxi-agent-cli --release --bins`：通过
- 6/6 气泡在不同时间采样中发生呼吸变化
- 气泡重叠对数：0
- 活跃动画帧间隔 p95：`7.1 ms`
- 移动端 `390 x 844`：无横向或纵向意外滚动
- reduced motion：`animation-name=none`、静态透明度 `0.22`、无 `will-change`
- 控制台错误：0
- 请求失败：0

## 本机运行与安装状态

- hotfix.6 release 构建当前运行于 `http://127.0.0.1:17861/`
- Web 健康检查：`status=ok`，`version=2.3.3-hotfix.6`
- 为避免重复微信服务，当前 Web 使用 `--no-weixin-autostart`
- 用户在 2026-08-04 07:44:01 启动的交互式 YunXi CLI 与微信子进程仍在运行
- 未强行终止该用户会话
- 安装前备份已创建并校验：`D:\Apps\YunXi Agent\bin\backups\20260804-074832`
- 正式安装目录仍为 `2.3.3-hotfix.5`；关闭活动 CLI 后再执行 hotfix.6 二进制替换

## 发布约束

- 不使用 force
- 不删除、不移动、不覆盖历史 tag
- 使用 GitHub CLI 已配置的 HTTPS 凭据执行仓库核验与推送
- 不记录或输出 GitHub token、微信凭据和用户消息内容

署名：开发者
