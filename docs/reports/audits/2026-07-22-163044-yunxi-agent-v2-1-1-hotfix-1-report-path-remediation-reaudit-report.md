# YunXi Agent v2.1.1-hotfix.1 历史报告路径整改独立复审报告

- 复审时间：2026-07-22 16:30:44 +08:00
- 复审对象：annotated tag `v2.1.1-hotfix.1`
- 开发报告：`D:\YunXi Agent\docs\reports\development\2026-07-22-155306-yunxi-agent-v2-1-1-hotfix-report-path-remediation-development-report.md`
- 原审核报告：`D:\YunXi Agent\docs\reports\audits\2026-07-22-154559-yunxi-agent-v2-1-1-audit-report.md`
- 原审核报告 SHA-256：`C31EF3846A50294FFC5C10BD6CEF3668CA1BC29A3035586232ABF9DDAA0291C6`
- 整改开发报告 SHA-256：`6E544DF1BE365A7E9A52CBA8C19252F0A82D71B821DD68677344100318E9493B`
- 项目目录：`D:\YunXi Agent`

## 一、复审结论

**v2.1.1-hotfix.1 独立复审通过。原 v2.1.1 审核报告指出的唯一阻塞点已经关闭，可以进入项目路线图中的 v2.1.2 开发阶段。**

本结论仅表示 v2.1.1 目录治理与本次历史报告路径整改满足现有总纲；它不代表 v2.1.2 已经实现，也不允许跳过 v2.1.2 自身的开发报告、测试、发布和审核门禁。

## 二、阻塞点关闭证据

| 检查项 | 复审结果 | 结论 |
| --- | --- | --- |
| 历史旧路径 | `docs/reports/2026-07-22-074211-yunxi-agent-v2-1-0-integrated-release-regression-development-report.md` 存在 | 通过 |
| 误迁移新路径 | `docs/reports/development/2026-07-22-074211-yunxi-agent-v2-1-0-integrated-release-regression-development-report.md` 不存在 | 通过 |
| 正文完整性 | 当前 SHA-256 为 `B70BF0D3F3BBDEEB7DE1DB515D8FA60B09BB161F63C47160AF28997EB61D0413`，与整改前误迁移文件一致 | 通过 |
| 单一正本 | 旧路径保留正文，新路径不保留副本 | 通过 |
| rename 门禁 | `git diff --find-renames=90% v2.1.0..v2.1.1-hotfix.1 -- docs/reports` 对该文件只显示旧路径上的 `M`，不再显示 `R092` | 通过 |
| 活动引用 | 误迁移新路径的活动 Markdown 引用为 0 | 通过 |
| 治理说明 | 文档索引、报告索引、目录治理基线和迁移映射均明确旧路径为唯一正本 | 通过 |

`v2.1.1..v2.1.1-hotfix.1` 会把该文件识别为从 `development/` 返回历史旧路径的 `R100`，这是本次精确整改动作本身；关键验收基准 `v2.1.0..v2.1.1-hotfix.1` 已不再显示 v2.1.1 对历史报告的迁移，因此满足“不迁移历史报告”的总纲要求。

## 三、版本、Git 与范围复核

- 复审开始时 `HEAD=origin/master=b01357dc12c04011639ad1501940d49580e0f1d4`，工作树干净。
- `v2.1.1-hotfix.1` 对象类型为 annotated `tag`；tag object 为 `12262fa6a19cd444403414606810077d6dfc81f3`，target 为 `d6312aebc8600697524e13a2ef96499ff60620f7`。
- GitHub master、tag object 和 tag target 与本地一致；远程 tag 总数为 52。
- 历史 `v2.1.1` tag object 仍为 `75c4169d09344a359239f820ca89f052408d1e76`，target 仍为 `58fb10f2f9e192056dea2660f34fc5c1bd8232b5`。
- `v2.1.1..v2.1.1-hotfix.1` 的 Rust、Cargo 变更数为 0；`Cargo.toml`、`Cargo.lock`、`crates/`、`evals/`、`vendor/`、`extracted/`、ConPTY 版本脚本和历史 evidence 均未发生功能变更。
- 未创建 `crates/yunxi-agent-weixin`，未提前实现 v2.1.2 微信模块、CLI 骨架或 iLink 协议客户端。

## 四、独立验证结果

### 1. 文档与 Git

- 6 个治理入口文件共检查 50 个本地 Markdown 链接，失效链接 0。
- 误迁移新路径的活动引用为 0。
- `git diff --check v2.1.0..v2.1.1-hotfix.1`：通过。
- `git ls-files -ci --exclude-standard -- scripts/conpty`：0 个已跟踪且被忽略文件。
- `scripts/conpty/v210/node_modules` 继续命中 `.gitignore:4` 的 `/scripts/conpty/**/node_modules/` 统一规则。

### 2. Rust 与陪伴评测

- `cargo fmt --all -- --check`：通过。
- `cargo check --workspace --offline`：通过。
- `cargo test --workspace --offline`：通过。
- CLI integration：45/45 通过。
- JSONL：10/10 通过。
- TUI：161/161 通过。
- 陪伴评测：31/31 通过，`golden_passed=true`，`tool_approval_bypass_count=0`，`proactive_boundary_violation_count=0`。

### 3. ConPTY、Provider 与 TUI 视觉

- v210 ConPTY verifier 返回 `ok=true`、`read_only=true`，正式 evidence SHA-256 保持 `a291a66cf91ee788bba9944edbafbf4c8dfce43bcd2d6c7b2cc78bc6c2990fc6`。
- 真实 Provider 继承 v2.1.0 已审核 DeepSeek 单轮结果 `YUNXI_V210_REAL_PROVIDER_OK`。本次热修复没有 Provider、Runtime、CLI 或 TUI 功能代码变化，因此该在线证据仍适用；复审没有重复发起会产生外部费用的 live smoke，也没有把 offline 输出冒充在线结果。
- 重新目视复核以下三张保留帧：
  - `D:\YunXi Agent\.tmp\audit-v210-live-provider-20260722\visual\conversation-100x30.png`
  - `D:\YunXi Agent\.tmp\audit-v210-live-provider-20260722\visual\details-100x30.png`
  - `D:\YunXi Agent\.tmp\audit-v210-live-provider-20260722\visual\conversation-58x18.png`
- 视觉结果：主界面、Details 和窄屏均无重叠、越界、Details 泄漏或不可读；这些帧明确标记为 offline，仅作为布局与交互继承证据。`offline offline` 重复文案仍是非阻塞观感观察项，不影响本次路径整改验收。

## 五、安全、清理与后续边界

- 复审期间未执行删除、递归清理、目录移动、`git clean`、gc、prune、系统安装、PATH/注册表/系统配置修改或用户目录清理。
- `target`、`.tmp`、`.yunxi`、`.codegraph` 和 ConPTY 本地依赖继续保留；本次复审不授权任何清理。
- 复审报告和日志属于 tag 后 docs-only 收口，只推进 master，不移动、删除或覆盖 `v2.1.1-hotfix.1`、`v2.1.1` 或任何历史 tag。
- 复审通过后可以接收并执行 v2.1.2 开发报告，但必须继续遵守新版本独立 tag、非强制推送、完整验证和日志纪律。

## 六、提交与分发状态

本报告生成时，功能发布提交、annotated tag 和发布后开发日志均已在 GitHub；本复审报告、报告索引和复审日志待作为 docs-only 审核收口提交到 master。桌面审核报告只作为项目正本的单向分发副本。

署名：审核者

## 七、提交与分发结果

记录时间：2026-07-22 16:34:27 +08:00

- 独立复审提交：`eb3ddd27099a9c1c3f7311b108ddcb1560437f3b`
- parent：`b01357dc12c04011639ad1501940d49580e0f1d4`
- author/committer：`开发者 <developer@yunxi-agent.local>`
- GitHub master：`eb3ddd27099a9c1c3f7311b108ddcb1560437f3b`
- GitHub tag 总数：52
- 推送前后 tag SHA 变化数：0
- `v2.1.1-hotfix.1` tag object：`12262fa6a19cd444403414606810077d6dfc81f3`，保持不变
- `v2.1.1` tag object：`75c4169d09344a359239f820ca89f052408d1e76`，保持不变
- force：未使用

本节与最终日志作为第二个 docs-only 收口提交仅推进 master，不移动任何 tag。项目正本已单向同步到桌面指定审核报告和开发日志文件，并执行 SHA-256 一致性校验。

署名：审核者
