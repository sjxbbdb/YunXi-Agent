# YunXi Agent v2.1.0 进入 v2.1.1 开发准入审核报告

- 审核时间：2026-07-22 12:08:29 +08:00
- 审核对象：当前发布版本 `v2.1.0`。
- 发布标识：annotated tag object `c42ca8b4e2837dcff1e8ae0cd3860936c947875d`；发布提交 `a8293905af55d659d647515786699ab313a51a07`。
- 当前源码版本：`Cargo.toml` 与当前调试二进制均为 `2.1.0`。
- 审核目的：仅判断 `v2.1.0` 是否稳定且具备进入总纲图 `v2.1.1`“项目目录治理、Git 忽略边界与文档索引基线”开发的条件；本报告**不审核 v2.1.1 是否已完成**，也不比较任何更后版本。
- 总纲正本：`D:\YunXi Agent\docs\superpowers\plans\2026-07-22-yunxi-agent-v2-1-1-to-v2-2-0-personal-wechat-roadmap.md`，审核时 SHA-256 为 `2DF30D503F46CFE7496567F5011BF5CBFB8BA73C91C2D2FA3E9F29AF0932EA0F`。

## 一、审核结论

`v2.1.0` 审核通过，**可以进入总纲图 v2.1.1 的开发**。

当前 tag、版本元数据、核心源码、Rust 全工作区回归、已安装二进制、陪伴评测、终端证据与
历史真实 Provider 证据均未发现会阻止目录治理工作的缺陷。`v2.1.0` tag 之后未引入任何
Rust、CLI、TUI、Runtime、Provider、存储或协议功能代码变更，后续提交只涉及 `.gitignore`、
文档、报告索引和 ConPTY 资产索引；因此它们不改变本次对 `v2.1.0` 功能发布的结论。

本准入结论不代表 v2.1.1 已完成。进入 v2.1.1 后，必须严格按总纲实施目录治理，并在
v2.1.1 完成、版本更新和 annotated tag 创建后重新接受单独审核。

## 二、准入条件对照

| v2.1.1 开发前提 | 审核结果 | 证据 |
| --- | --- | --- |
| 当前版本必须是可追溯、可回滚的稳定发布 | 通过 | `Cargo.toml=2.1.0`；`v2.1.0` 为 annotated tag；tag 指向发布提交 `a8293905...`；本地共 50 个 tag，`v2.1.0` 是当前 `HEAD` 的祖先。 |
| 目录治理不得建立在未确认的功能源码改动之上 | 通过 | `git diff --name-status v2.1.0..HEAD` 仅包含 `.gitignore`、README、报告、索引和 ConPTY 文档；没有 `crates/`、Cargo 依赖、TUI、Runtime、Provider、存储或协议代码改动。 |
| 既有 CLI/TUI/Runtime 边界须清楚，目录整理不得成为功能重构 | 通过 | CodeGraph 核验：CLI 入口位于 `crates/yunxi-agent-cli`，TUI 位于 `crates/yunxi-agent-tui`，`YunXiRuntimeBackend` 位于 `crates/yunxi-agent-runtime`，`SessionStore` 位于 `crates/yunxi-agent-storage`；目录治理无需改动这些边界。 |
| 当前功能基线必须通过统一回归 | 通过 | 本次独立执行 `cargo fmt --all -- --check`、`cargo check --workspace`、`cargo test --workspace`，均成功。 |
| 通用型陪伴能力与工具审批边界必须保持 | 通过 | 已安装 `yunxi 2.1.0` 运行 `eval companion --json`：31/31，`golden_passed=true`，`tool_approval_bypass_count=0`，主动行为越界计数为 0。 |
| 真实终端/TUI 与 Provider 基线应可追溯 | 通过 | v2.1.0 发布审核中保存的真实 DeepSeek 单轮和 Windows ConPTY 证据仍有效；tag 后无功能代码改动。本次独立重放 v210/v209 正式 evidence 的只读 verifier，均通过。 |
| v2.1.1 所需的参考源码应可用 | 通过 | 本机 `D:\源码\reasonix` 存在；其根目录、`.gitignore`、`REASONIX.md`、`docs/` 和 `scripts/` 已被只读核查，可作为目录治理参考。 |

## 三、源码与仓库审核

1. `v2.1.0` 发布后的提交为文档/整理提交：`a8d5398`、`6116369`、`13ea19b`、`fe9137b`、
   `ddd4cd4`、`b1e6124`、`d6b6132` 等均未修改产品功能代码。它们不会改变 `v2.1.0`
   的 Runtime、TUI、Provider 或会话行为。
2. CodeGraph 显示 CLI 后端选择仍集中在
   `crates/yunxi-agent-cli/src/main.rs`；TUI 由
   `crates/yunxi-agent-cli/src/tui/mod.rs` 与 `crates/yunxi-agent-tui` 组合；
   `YunXiRuntimeBackend` 和 `SessionStore` 继续保留独立 crate 边界。v2.1.1 应只治理
   根目录、忽略规则、文档索引与脚本索引，不得借机移动或改写这些核心模块。
3. `git fsck --full` 未报告损坏对象或 tag 引用错误；仓库存在历史 dangling 对象，与此前
   v2.1.0 审核结论一致。未执行 `gc`、`prune`、对象清理、tag 移动或 tag 删除。
4. 审核时工作树含有本次审核相关的路线图、整理报告和日志文档修改；均不属于功能代码。
   它们应在 v2.1.1 文档治理提交中妥善归档，不能作为 v2.1.0 发布功能变更混入 tag。

未发现阻止进入 v2.1.1 的源码问题。

## 四、独立验证与真实证据

本次在 `D:\YunXi Agent` 执行并通过：

- `cargo fmt --all -- --check`
- `cargo check --workspace`
- `cargo test --workspace`
- `D:\YunXi Agent\target\debug\yunxi.exe --version`：`yunxi 2.1.0`
- `D:\Apps\YunXi Agent\bin\yunxi.exe --version`：`yunxi 2.1.0`
- `D:\Apps\YunXi Agent\bin\yunxi.exe eval companion --json`：31/31 通过；工具审批绕过计数为 0。
- `npm.cmd run verify --prefix scripts/conpty/v210`：`ok=true`、`read_only=true`，正式 evidence SHA-256 为 `A291A66CF91EE788BBA9944EDBAFBF4C8DFCE43BCD2D6C7B2CC78BC6C2990FC6`。
- `npm.cmd run verify --prefix scripts/conpty/v209`：`ok=true`、`read_only=true`，正式 evidence SHA-256 为 `714B9C2D9C01B1616FFC8789E940EA778559335E20998679A904B5C335FCFDC8`。
- `git diff --check`：通过。

两处已安装 `yunxi.exe` 的 SHA-256 相同：
`350766A63FE978E404B112CB0BB4623646514B4A9630F0D3849DEF552C514355`。

### 真实 Provider 与 TUI 视觉/交互复核

- 已归档的 `v2.1.0` 审核报告记录：release binary 使用真实 DeepSeek 完成无工具单轮，
  返回预期标记 `YUNXI_V210_REAL_PROVIDER_OK`；并完成真实 Windows ConPTY capture。
  本次确认该发布 tag 后没有任何功能源码变更，因此该真实 Provider 证据可以作为
  v2.1.0 的有效基线继承。
- 本次未重复运行 `scripts/provider/deepseek-live-smoke.ps1`。该脚本在 finally 中包含
  对 `%TEMP%` 会话目录的 `Remove-Item -Recurse -Force`；根据用户的 shell 安全强约束，
  未获得对该精确清理行为的单独确认前，审核者不得执行。此项没有被伪造成新的 live
  验证结果。
- 已人工查看 `D:\YunXi Agent\.tmp\audit-v210-live-provider-20260722\visual\`
  下 `conversation-100x30.png`、`details-100x30.png`、`conversation-58x18.png`：主会话、
  Details 与窄屏均无控件重叠、文字越界或不可读截断；Details 内容只在显式视图出现，
  关闭后可恢复 transcript。帧为 forced-offline 视觉场景，适用于布局与交互观感复核，
  不被表述为新的 live Provider 截图。
- 非阻塞观感观察：forced-offline header 存在 `offline offline` 的重复文案。该问题不影响
  v2.1.1 的目录治理准入，也不影响现有布局或安全边界；不得将其伪装为已修复。

## 五、总纲参考源码

总纲 v2.1.1 指定的目录治理参考来源已明确，可在开发中仅提取分类与治理逻辑：

| 参考路径 | v2.1.1 可借鉴内容 | 禁止事项 |
| --- | --- | --- |
| `D:\源码\reasonix\.gitignore` | 构建产物、Node 依赖、局部状态、CodeGraph 索引统一忽略。 | 不复制 Go 项目专用模式或隐藏 YunXi 已跟踪文件。 |
| `D:\源码\reasonix\docs\`、`scripts\` | 文档、运行手册、维护脚本与版本资产分层。 | 不照搬其站点、桌面端或发布系统。 |
| `D:\源码\reasonix\REASONIX.md` | 简短、稳定的项目约束和入口治理。 | 不替换 YunXi 的 `AGENTS.md`、Cargo workspace 或既有硬性约束。 |
| `D:\YunXi Agent\docs\extraction-index\`、`vendor\codex-rs\`、`extracted\codex-core-agent-sources\` | 本项目已有参考映射和迁移输入。 | 在 v2.1.1 中移动、重命名、删除或直接修改这些目录。 |

## 六、版本与文档状态

- `v2.1.0` annotated tag 未移动、未删除、未覆盖；当前 `HEAD` 仅在其后增加整理/文档提交。
- 用户指定的桌面路线图副本与项目内总纲正本 SHA-256 当前不一致。总纲正本已规定项目内
  `docs/superpowers/plans/` 为唯一可编辑来源；桌面副本为待同步分发件。该同步状态是
  文档流程观察项，不阻止 `v2.1.0` 进入 v2.1.1 开发，但在 v2.1.1 发布/审核前必须如实
  记录其状态，不得让两份路线图分别演进。
- 本次未创建 commit、未推送、未创建 tag，也未修改 v2.1.0 发布提交。
- 用户于 2026-07-22 12:12:58 +08:00 明确授权写入桌面审核目录后，首次受控复制尝试返回
  `AccessDenied`，未将该次失败表述为同步成功。用户随后完成提权；审核者于
  2026-07-22 12:19:37 +08:00 对同一份报告执行精确单文件复制，已成功写入
  `C:\Users\24763\Desktop\YunXi Agent审核报告\2026-07-22-120829-YunXi-Agent-v2.1.0-进入v2.1.1开发准入审核报告.md`。
  复制后已核验项目内与桌面报告 SHA-256 一致；桌面副本现为已同步分发件。

## 七、最终结论

`v2.1.0` 的功能发布基线稳定，**审核通过，可以进入总纲图 v2.1.1 的开发**。

v2.1.1 开发必须仅执行总纲规定的目录治理、Git 忽略边界和文档索引工作；不得将微信
crate、iLink 网络、凭证存储或通道 Runtime 提前混入本版本。v2.1.1 完成前不得宣称
v2.1.1 已通过，也不得跳过其独立 tag 与后续审核。

署名：审核者
