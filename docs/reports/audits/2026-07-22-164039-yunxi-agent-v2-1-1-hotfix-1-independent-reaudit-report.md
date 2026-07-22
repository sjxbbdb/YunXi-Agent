# YunXi Agent v2.1.1-hotfix.1 独立复审报告

- 复审时间：2026-07-22 16:40:39 +08:00
- 复审对象：annotated tag `v2.1.1-hotfix.1`
- 对照范围：仅复核 v2.1.1 总纲“项目目录治理、Git 忽略边界与文档索引基线”及其历史报告路径整改，不比较其他版本完成度。
- 总纲正本：`D:\YunXi Agent\docs\superpowers\plans\2026-07-22-yunxi-agent-v2-1-1-to-v2-2-0-personal-wechat-roadmap.md`
- 开发目录：`D:\YunXi Agent`

## 一、复审结论

**v2.1.1-hotfix.1 独立复审通过，可以进入总纲图中的 v2.1.2 开发。**

上次 v2.1.1 审核的唯一阻塞点是历史 v2.1.0 开发报告被迁移到 `docs/reports/development/`，违反总纲“不迁移历史报告”的要求。本次确认整改已经落在 Git 树中：旧历史路径恢复保留，新分类路径不再存在正文副本，`v2.1.0..v2.1.1-hotfix.1` 不再显示该历史报告的迁移。

本结论只表示 v2.1.1 目录治理基线及整改通过，不表示 v2.1.2 已完成；v2.1.2 仍必须独立开发、验证、创建新 tag 并重新审核。

## 二、上次阻塞点整改核验

| 核验项 | 结果 | 证据 |
| --- | --- | --- |
| 历史旧路径保留 | 通过 | `D:\YunXi Agent\docs\reports\2026-07-22-074211-yunxi-agent-v2-1-0-integrated-release-regression-development-report.md` 存在。 |
| 误迁移的新分类路径 | 通过 | `D:\YunXi Agent\docs\reports\development\2026-07-22-074211-yunxi-agent-v2-1-0-integrated-release-regression-development-report.md` 不存在。 |
| 历史报告正文完整性 | 通过 | 旧路径 SHA-256 为 `B70BF0D3F3BBDEEB7DE1DB515D8FA60B09BB161F63C47160AF28997EB61D0413`。 |
| 单一正本 | 通过 | 旧路径保留正文，新路径没有正文副本。 |
| v2.1.1 总纲迁移门禁 | 通过 | `git diff --find-renames=90% v2.1.0..v2.1.1-hotfix.1 -- docs/reports` 不再显示该历史报告的 rename；旧路径只显示正常内容变更。 |
| 活动导航引用 | 通过 | 新路径没有活动 Markdown 导航引用；现存文字均为“不存在”说明、历史迁移映射或复审事实记录。 |

`v2.1.1..v2.1.1-hotfix.1` 将该文件识别为从分类目录返回旧路径的 `R100`，这是整改动作本身；总纲验收基准是最终树相对 `v2.1.0` 不再迁移历史报告，该条件已满足。

## 三、总纲逐项核验

| v2.1.1 要求 | 结果 | 核验说明 |
| --- | --- | --- |
| 根目录资产清单 | 通过 | `docs/directory-governance.md` 记录稳定源码、参考输入、文档、脚本、本地状态和可再生产物的用途、跟踪边界、清理条件与引用方。 |
| 既有目录与证据边界不变 | 通过 | v2.1.1-hotfix.1 没有 Rust、Cargo、vendor、extracted、evals、ConPTY 版本脚本或历史 evidence 功能变更。 |
| 统一 `.gitignore` 规则 | 通过 | 现存 `scripts/conpty/v210/node_modules` 命中 `/scripts/conpty/**/node_modules/`；当前 `.work` 不存在。 |
| 不隐藏已跟踪文件 | 通过 | `git ls-files -ci --exclude-standard -- scripts/conpty` 返回 0 个文件。 |
| 文档索引和两步导航 | 通过 | 6 个治理入口文件共检查 52 个本地 Markdown 链接，失效 0 个；路线图、报告、ConPTY、提取索引均可达。 |
| 报告落位规则 | 通过 | 新增审核报告位于 `docs/reports/audits/`，整改开发报告位于 `docs/reports/development/`；历史 v2.1.0 报告保留原路径。 |
| ConPTY 总览不改脚本路径 | 通过 | `scripts/conpty/README.md` 保持版本矩阵和 capture/verify 说明，v210 verifier 只读通过。 |
| 只记录清理候选 | 通过 | 未执行删除、移动、递归清理、`git clean`、gc 或 prune。 |
| 路线图正本与桌面副本一致 | 通过 | 项目内与桌面路线图 SHA-256 均为 `2DF30D503F46CFE7496567F5011BF5CBFB8BA73C91C2D2FA3E9F29AF0932EA0F`。 |
| 独立 annotated tag | 通过 | `v2.1.1-hotfix.1` 为 annotated tag，tag object 为 `12262fa6a19cd444403414606810077d6dfc81f3`，target 为 `d6312aebc8600697524e13a2ef96499ff60620f7`；原 `v2.1.1` tag object `75c4169d09344a359239f820ca89f052408d1e76` 保持不变。 |

## 四、源码与版本边界

- 已使用 CodeGraph 复核 CLI、TUI、Runtime、Storage、Provider 和 Codex compatibility 边界；整改没有触及运行时代码。
- `v2.1.1..v2.1.1-hotfix.1` 的功能性路径过滤为 0：没有 `Cargo.toml`、`Cargo.lock`、`crates/`、`evals/`、`vendor/`、`extracted/`、ConPTY 版本脚本或历史 evidence 改动。
- 没有新增 `crates/yunxi-agent-weixin`，没有提前实现 v2.1.2 的微信模块、CLI 骨架或 iLink 协议客户端。
- 当前 `HEAD=74da1c4` 比 `v2.1.1-hotfix.1` tag 多出 3 个 docs-only 收口提交。按既定审核规则，该发布后文档收口不是阻塞点；tag 本身未移动、未覆盖。
- `Cargo.toml` 与调试二进制仍为 `2.1.0`，符合 v2.1.1 总纲“不修改 Cargo/Rust 运行边界”的要求，记录为非阻塞观察项。

## 五、统一验证结果

### 1. Rust、CLI 与陪伴评测

- `cargo fmt --all -- --check`：通过。
- `cargo check --workspace --offline`：通过。
- `cargo test --workspace --offline`：通过。
- CLI integration：45/45 通过。
- JSONL：10/10 通过。
- Provider：46/46 通过。
- TUI：161/161 通过。
- `target\debug\yunxi.exe --version`：`yunxi 2.1.0`。
- `yunxi eval companion --json`：31/31，`golden_passed=true`，`failed_scenarios=0`，`tool_approval_bypass_count=0`，`proactive_boundary_violation_count=0`。

### 2. Git、忽略规则与文档

- `git check-ignore -v`：现存 ConPTY `node_modules` 统一命中 `.gitignore:4`。
- 现存 ConPTY 生成目录探针：1 个，均已被忽略；已跟踪且被忽略文件：0 个。
- 本地 Markdown 链接：52 个，失效 0 个。
- 路线图项目正本/桌面副本：SHA-256 一致。
- `git diff --check v2.1.0..v2.1.1-hotfix.1`：通过。
- `git fsck --full`：退出码 0；存在历史 dangling 对象，但没有对象损坏或 tag 引用错误。本次不执行 gc、prune 或清理。

### 3. 真实 Provider、ConPTY 与 TUI

- v210 ConPTY verifier：`ok=true`、`read_only=true`，正式 evidence SHA-256 为 `a291a66cf91ee788bba9944edbafbf4c8dfce43bcd2d6c7b2cc78bc6c2990fc6`。
- 真实 Provider：v2.1.0 发布审核已记录真实 DeepSeek 单轮结果 `YUNXI_V210_REAL_PROVIDER_OK`。本次 hotfix 没有 Provider、Runtime、CLI 或 TUI 功能代码变化，因此该在线证据作为当前修复版本的继承基线；本次没有重复发起会产生外部费用的 live smoke，也没有将 offline 输出冒充在线结果。
- 重新目视复核：
  - `D:\YunXi Agent\.tmp\audit-v210-live-provider-20260722\visual\conversation-100x30.png`
  - `D:\YunXi Agent\.tmp\audit-v210-live-provider-20260722\visual\details-100x30.png`
  - `D:\YunXi Agent\.tmp\audit-v210-live-provider-20260722\visual\conversation-58x18.png`
- 视觉结论：主会话、Details 和 58x18 窄屏没有重叠、越界、Details 泄漏或不可读；Details 关闭后恢复 transcript。帧明确标记为 offline，仅作为布局与交互继承证据。`offline offline` 重复文案仍是非阻塞观感观察。

## 六、总纲要求的参考源码

本版本总纲要求参考本机已有 Reasonix 的目录治理方式，审核确认以下路径存在并已被开发报告明确：

- `D:\源码\reasonix\.gitignore`：统一忽略构建产物、Node 依赖、局部状态与索引。
- `D:\源码\reasonix\docs\`：文档分类和稳定导航。
- `D:\源码\reasonix\scripts\`：脚本入口与说明边界。
- `D:\源码\reasonix\REASONIX.md`：稳定项目约束入口。

本次整改没有拉取新的源码，也没有把 Reasonix 的 Go、站点、桌面端或发布系统结构移入 YunXi。v2.1.1 已满足当前总纲对参考源码的要求。

## 七、清理、分发与署名

- 清理结果：未执行删除、移动、重命名、递归清理、`git clean`、gc、prune、系统安装、PATH/注册表/系统配置修改或用户目录清理；`target`、`.tmp`、`.yunxi`、`.codegraph`、ConPTY 依赖和审计采集资产均保留。
- 提交、推送与 tag：本次审核不创建 commit、不推送、不创建新 tag；`v2.1.1-hotfix.1`、`v2.1.1` 及全部历史 tag 保持不变。
- 审核报告项目内路径：`D:\YunXi Agent\docs\reports\audits\2026-07-22-164039-yunxi-agent-v2-1-1-hotfix-1-independent-reaudit-report.md`。
- 桌面审核报告路径：`C:\Users\24763\Desktop\YunXi Agent审核报告\2026-07-22-164039-YunXi-Agent-v2.1.1-hotfix.1-独立复审审核报告.md`。

署名：审核者
