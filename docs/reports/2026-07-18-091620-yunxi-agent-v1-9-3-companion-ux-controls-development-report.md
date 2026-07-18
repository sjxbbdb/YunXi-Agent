# YunXi Agent v1.9.3 Companion UX & Controls 开发报告

撰写时间：2026-07-18 09:16:20 +08:00  
审核报告：`D:\YunXi Agent\docs\reports\2026-07-18-090423-yunxi-agent-v1-9-2-reaudit-source-audit-report.md`  
开发目录：`D:\YunXi Agent`  
项目内报告位置：`D:\YunXi Agent\docs\reports\2026-07-18-091620-yunxi-agent-v1-9-3-companion-ux-controls-development-report.md`  
桌面报告位置：`C:\Users\24763\Desktop\YunXi Agent开发报告\2026-07-18-091620-yunxi-agent-v1-9-3-companion-ux-controls-development-report.md`

## 一、硬性约束

1. 开发目录固定在 `D:\YunXi Agent` 及其工作树内。
2. 核心目标是开发一个通用型陪伴 agent。
3. 涉及到源码参考需要先抽取、迁移和构建整体能力，不要在单个点上反复纠结；如果参考的源码不为 Rust 语言，则需参考其逻辑，进行 Rust 复刻。
4. 中间无需频繁验证，完成一批构建后再统一测试和验证。
5. 验证通过前不能宣称完成。
6. 每次阶段结束后要清理编译中间产物，避免占用大量硬盘空间。
7. 每次任务结束后，都要在日志中追加本次工作的详细记录。
8. 记录必须包含时间戳、工作目标、执行流程、修改文件、文件路径、验证结果、提交和推送状态，以及日志结尾处要进行署名，署名为开发者。
9. 后续开发报告、设计文档、索引和状态文档要与实际代码状态保持一致。
10. 回复用户使用中文。
11. 之后每个版本更迭都必须提交为新的 Git tag；以前的版本 tag 不得删除，方便出错后回滚。
12. 涉及到 shell 命令时要万分小心，例如递归删除、强制移动、清空目录、系统级安装/卸载、修改 PATH/注册表/系统配置等操作必须得到用户确认。
13. 不可因为过长的上下文忽略硬性要求；如果察觉到硬性要求的效力已经减弱，则必须汇报当前情况。
14. 工作尽量固定在项目文件夹，涉及到其他文件夹的操作，要明确告知用户，尤其是 C 盘的用户目录。

## 二、审核结论转开发目标

本次重新审核确认 YunXi Agent v1.9.2 审核通过，可以进入下一版本开发。当前发布基线如下：

- `Cargo.toml` workspace version：`1.9.2`。
- 当前 HEAD：`1467d3e`。
- 当前分支状态：`master...origin/master`，工作树干净。
- 本地存在 `v1.9.2` tag，解析到 `61ef2d3008faa9bf75b1247a238369037b25c24d`。
- `D:\YunXi Agent\target` 已清理，不再存在。
- 旧 `v1.9.1` 与 `v1.9.1-hotfix.1` tag 未被删除或移动。

v1.9.3 的开发目标是 `Companion UX & Controls`：围绕记忆审查、人格档案、关系状态、主动陪伴设置，提供开发者和用户都能看见、能关闭、能清除、能追溯的控制面，同时保持 v1.9.2 的主动陪伴规划器默认安全、可解释、不可绕过审批的特性。

## 三、v1.9.3 范围边界

### 必须完成

- 建立主动陪伴、记忆、人格档案、关系状态的统一查看入口。
- 提供可见的开关和状态入口，让用户能查看、开启、关闭主动陪伴。
- 提供清除入口，但清除必须显式确认，且要区分陪伴历史、记忆和关系状态等作用域。
- 在 CLI 和 TUI 中都暴露一致的控制语义，避免只有内部配置、没有外显操作路径。
- 若后续引入云同步或云端控制链路，必须把云开关与本地控制分开，默认关闭，且不得作为当前默认运行依赖。
- 所有控制动作都要有审计记录，尤其是关闭、清除、刷新和状态变更。
- 保持 v1.9.2 的 companion planner 不变成新的后台服务，不把控制层和计划层混在一起。

### 禁止提前扩展

- 不做新的云后台或常驻服务。
- 不做独立桌面应用、web 管理台、SDK 包装或 marketplace 功能。
- 不把记忆审查做成复杂的新数据库产品。
- 不改写 v1.9.2 的主动陪伴规划逻辑为另一套策略引擎。
- 不默认接入上游 Codex CLI 源码、`vendor/codex-rs`、`codex-*` crate 或 `yunxi-agent-codex`。

## 四、源码接入点

### `D:\YunXi Agent\crates\yunxi-agent-core\src\config.rs`

继续承载主动陪伴与控制面配置，建议把可见状态与持久化开关统一放入配置层，避免 CLI/TUI 和 runtime 各自维护一套状态。

建议方向：

- 复用现有 `CompanionSettings`，补充能表示查看/关闭/清除策略的控制相关字段。
- 如果需要区分本地与云控制，至少保留可序列化的开关位，但默认不启用任何云通道。
- 所有默认值继续保持保守，尤其是主动陪伴和远程控制都必须默认关闭。

### `D:\YunXi Agent\crates\yunxi-agent-cli\src\main.rs`

CLI 应提供显式、可发现的控制入口：

- 查看 companion 当前状态。
- 开启/关闭主动陪伴。
- 查看记忆、人格档案、关系状态。
- 清除 companion 历史或记忆前要求确认。

CLI 侧重点不是扩大命令数量，而是让状态和动作是同一套模型的不同呈现。

### `D:\YunXi Agent\crates\yunxi-agent-cli\src\render.rs`

命令输出需要适合开发者排查和用户确认：

- 当前状态必须一眼可见。
- 清除或关闭类操作要有明确的风险提示。
- 输出要能看出操作作用域，不要把 companion、persona、memory、relationship 混成一条模糊消息。

### `D:\YunXi Agent\crates\yunxi-agent-tui\src\app.rs`

TUI 需要补齐可见控制面：

- 提供 companion、memory、persona、relationship 的查看页或面板。
- 提供开关、刷新、清除、确认等动作入口。
- 清除类操作必须有显式确认框，不允许单键误触直接执行。

### `D:\YunXi Agent\crates\yunxi-agent-tui\src\render.rs`

TUI 渲染应避免装饰性大于信息性，重点是状态可读、动作可控、结果可追溯：

- 状态、开关、最近变化、清除范围、确认状态都应清晰显示。
- 对于主动陪伴和记忆清除，必须把影响范围写出来。
- 如果存在云控制抽象，也必须在界面上清晰区分为独立状态。

### `D:\YunXi Agent\crates\yunxi-agent-runtime\src\lib.rs`

runtime 侧只提供只读快照和安全边界，不把控制逻辑扩散到规划器内部：

- companion planner 继续只负责主动策略。
- 控制面只读取状态，不把清除、关闭、恢复等动作塞回 planner。
- 对外暴露的 snapshot 应能被 CLI/TUI 共享，避免两套口径。

### `D:\YunXi Agent\crates\yunxi-agent-persona\src\profile.rs`
### `D:\YunXi Agent\crates\yunxi-agent-persona\src\memory.rs`
### `D:\YunXi Agent\crates\yunxi-agent-persona\src\relationship_graph.rs`
### `D:\YunXi Agent\crates\yunxi-agent-persona\src\recall_router.rs`

这些模块应为审查页和状态页提供只读数据源：

- persona profile 用于展示身份、偏好和持久化人格信息。
- memory 用于展示记忆摘要和可清除范围。
- relationship graph 与 recall router 用于展示关系状态、活跃关系和时间线摘要。
- 这些数据只读展示，不应让 UI 直接改写底层记忆逻辑。

## 五、推荐技术设计

建议补一层统一控制模型，避免 CLI 和 TUI 各写各的状态树：

```rust
pub enum ControlScope {
    Companion,
    Memory,
    Persona,
    Relationship,
}

pub enum ControlVerb {
    Show,
    Enable,
    Disable,
    Clear,
    Refresh,
}

pub struct ControlRequest {
    pub scope: ControlScope,
    pub verb: ControlVerb,
    pub confirm_required: bool,
    pub confirm_token: Option<String>,
}

pub struct ControlSnapshot {
    pub companion_enabled: bool,
    pub quiet_hours: Option<String>,
    pub persona_summary: String,
    pub memory_summary: String,
    pub relationship_summary: String,
}
```

实现要求：

- 所有清除类请求默认都要进入 `confirm_required = true` 路径。
- `ControlSnapshot` 必须能被 CLI 和 TUI 共用。
- 如果未来出现云同步控制位，要作为独立状态保存，不能和本地关闭开关混用。
- 任何展示给用户的状态都要明确说明其来源是当前配置、runtime 快照还是只读历史摘要。

## 六、参考源码抽取建议

本版本指定的参考源码：

- `D:/源码/memU/readme`
- `D:/源码/nocturne_memory/frontend`
- `D:/源码/QwenPaw/console`

抽取原则：

- 先抽取控制语义，再映射到 Rust 的 CLI/TUI 和配置层。
- 如果参考内容不是 Rust，只保留其交互逻辑和状态组织方式，不迁入其运行时。
- 优先抽取“状态可见、动作可控、清除确认、结果可追溯”的结构，而不是页面风格。

建议映射：

- `memU` 的记忆审查与清除流程 -> YunXi 的 memory review / clear 语义。
- `nocturne_memory` 的前端浏览和关系状态呈现 -> YunXi 的 persona / relationship 状态页。
- `QwenPaw console` 的设置面与控制分发 -> YunXi 的 CLI/TUI 控制入口与状态显示。

## 七、实现顺序

1. 冻结控制面模型，统一 companion / memory / persona / relationship 的状态定义。
2. 在 `config.rs` 中补齐持久化控制字段，确保默认关闭和清除确认语义明确。
3. 从 persona / memory / relationship 模块生成只读快照。
4. 在 CLI 中补齐状态查看、关闭、开启、清除和确认命令。
5. 在 TUI 中补齐对应页面、开关和确认对话框。
6. 增加控制动作审计记录，确保关闭与清除可回放、可定位。
7. 同步 `docs/extraction-status.md`、`docs/persona-memory.md`、`docs/development-log.md` 和 README。
8. 完成一批构建后统一验证，不要在单个点上反复打转。
9. 阶段结束后按安全流程清理编译中间产物；若涉及递归删除或等价高风险命令，先征得用户确认。
10. 验证通过后再创建新的 `v1.9.3` Git tag，旧版本 tag 不得删除或移动。

## 八、验收标准

v1.9.3 完成时至少满足以下验收项：

- 用户能在 CLI/TUI 中查看 companion 状态、记忆摘要、人格档案和关系状态。
- 用户能开启或关闭主动陪伴，且状态会持久化。
- 记忆或 companion 清除类操作必须确认后才执行。
- 控制动作必须留下审计记录。
- 本地控制和未来可能的云控制必须分离，默认不启用任何云依赖。
- 现有 v1.9.2 主动陪伴逻辑不被破坏，默认不打扰、可解释、不可绕过审批的要求继续成立。
- 关系图、记忆和人格数据仍保持只读审查与受控修改边界。

## 九、统一验证要求

开发者应在完成一批构建后统一执行验证，建议顺序如下：

```powershell
cargo fmt --all
cargo check --workspace
cargo test --workspace
cargo build --workspace
git status --short
git diff --stat
```

补充检查：

- 检查 `Cargo.toml` workspace version 是否进入 `1.9.3`。
- 检查 CLI/TUI 状态与控制入口是否一致。
- 检查默认值是否继续保持主动陪伴关闭。
- 检查清除类命令是否都要求确认。
- 检查日志、状态文档、索引和 README 是否与实际状态一致。
- 检查是否已创建新 tag，且旧 tag 未被删除或移动。

注意：`cargo clean`、递归删除 `target`、强制移动目录、清空目录等清理动作必须遵守用户 shell 安全要求；需要确认时先征得用户同意。

## 十、文档同步要求

v1.9.3 开发过程中，以下文档需要与代码状态保持一致：

- `D:\YunXi Agent\docs\reports\2026-07-18-091620-yunxi-agent-v1-9-3-companion-ux-controls-development-report.md`
- `D:\YunXi Agent\docs\development-log.md`
- `D:\YunXi Agent\docs\extraction-status.md`
- `D:\YunXi Agent\docs\persona-memory.md`
- `D:\YunXi Agent\README.md`
- 若 CLI/TUI 暴露新控制入口，应同步命令帮助和版本说明。

## 十一、本报告生成状态

- 本轮工作仅撰写开发报告，没有修改 Rust 源码。
- 本轮未运行 `cargo fmt`、`cargo check`、`cargo test` 或 `cargo build`。
- 本轮未执行编译产物清理。
- 本轮未提交、未推送、未创建 Git tag。
- 后续开发者必须以实际代码实现、统一验证结果和日志记录更新对应文档。

署名：开发报告撰写者

## 十二、实际实现与统一验证结果（2026-07-18）

本轮已完成 v1.9.3 Companion UX & Controls 实现，实际代码状态优先于
本报告的初始撰写状态：

- `crates/yunxi-agent-core/src/control.rs` 新增统一 `ControlScope`、
  `ControlVerb`、`ControlRequest`、`ControlSnapshot`、审计记录和陪伴历史
  记录；`CompanionSettings` 增加独立的云控制位和清除确认策略，默认均为
  保守值。
- `crates/yunxi-agent-storage/src/lib.rs` 新增工作区控制存储，维护
  `.yunxi/controls/audit.jsonl` 与 `companion-history.jsonl`，清除陪伴历史
  仅清空本地陪伴历史账本，记忆清除仍以追加归档记录实现。
- `crates/yunxi-agent-runtime/src/lib.rs` 新增共享只读 `control_snapshot`，
  聚合当前配置、Persona、Memory Schema v3 和 Relationship Graph Lite；
  companion planner 仍在原 turn boundary 运行，并将计划写入受限历史记录，
  不新增后台服务或 ToolRouter 绕过路径。
- `crates/yunxi-agent-cli/src/main.rs` 新增 `controls status/show/enable/
  disable/clear/refresh/audit`，扩展 `companion status/on/off/history/clear`；
  `--confirm` 是清除脚本命令的硬性确认，Persona 与 Relationship 清除保持
  只读拒绝；既有 persona/memory 管理动作同步写入审计。
- `crates/yunxi-agent-cli/src/commands.rs`、`interactive.rs`、`render.rs`
  与 `crates/yunxi-agent-cli/src/tui/mod.rs` 同步交互语义；TUI 支持
  `/controls` 控制面、开关、刷新和带 `CLEAR SCOPE` 输入确认的清除路径。
- `crates/yunxi-agent-tui/src/app.rs`、`host.rs`、`render.rs` 新增共享快照
  控制面板，显示状态来源、清除影响、云开关和最近审计变化。
- `crates/yunxi-agent-persona/src/settings.rs` 持久化本地 companion/cloud
  开关；profile/compiler 版本进入 `1.9.3`；README、extraction status、
  persona-memory 和项目日志已同步。
- 参考抽取仅访问并吸收 `D:\源码\memU\readme`、
  `D:\源码\nocturne_memory\frontend`、`D:\源码\QwenPaw\console` 的
  审查、确认、刷新和状态组织语义，没有迁入其前端、Python、云服务或进程模型。

统一验证结果：

- `cargo fmt --all` 与 `cargo fmt --all -- --check`：通过。
- `cargo check --workspace`：通过。
- `cargo test --workspace`：通过，新增 core/storage/runtime/CLI/TUI 控制回归
  与既有 persona、memory、relationship、companion 回归全部通过。
- `cargo build --workspace`：通过。
- `cargo build -p yunxi-agent-cli --release --bins`：通过，Release 二进制显示
  `yunxi 1.9.3`。
- Release 隔离冒烟：默认 companion/cloud 均关闭；enable 状态可持久化；未带
  `--confirm` 的 clear 被拒绝并写入 rejected 审计；带确认的 companion clear
  成功；relationship clear 被只读边界拒绝；tool request 仅输出需确认建议，
  未执行工具。
- 冒烟临时目录已从 `D:\YunXi Agent\.tmp\v193-control-smoke` 清理；阶段结束已
  执行 `cargo clean`，`D:\YunXi Agent\target` 不存在。

本节完成后仍需执行：清理 `target`、核验工作树、提交 v1.9.3、创建新的
`v1.9.3` tag、使用 GitHub API key non-force 推送，并在日志末尾补写最终
提交/tag/推送状态。旧版本 tag 不得删除或移动。
