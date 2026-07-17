# YunXi Agent v1.9.2 Proactive Companion Loop 开发报告

撰写时间：2026-07-17 22:02:39 +08:00  
审核报告：`C:\Users\24763\Desktop\YunXi Agent审核报告\2026-07-17-215555-YunXi-Agent-v1.9.1-源码审核报告.md`  
开发目录：`D:\YunXi Agent`  
项目内报告位置：`D:\YunXi Agent\docs\reports\2026-07-17-220239-yunxi-agent-v1-9-2-proactive-companion-loop-development-report.md`  
桌面报告位置：`C:\Users\24763\Desktop\YunXi Agent开发报告\2026-07-17-220239-yunxi-agent-v1-9-2-proactive-companion-loop-development-report.md`

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

本次审核报告判定 YunXi Agent v1.9.1 审核通过，可以进入下一版本开发。当前 `master` 以 `v1.9.1-hotfix.1` 为安全基线：

- `Cargo.toml` workspace version：`1.9.1`。
- 当前 HEAD：`c99e268`。
- 当前 tag：`v1.9.1-hotfix.1`。
- 原始 `v1.9.1` tag 指向 `e9c14152e8e4b96b12fecd56f93063a3dcd90a8b`，不得移动或删除。
- `v1.9.1-hotfix.1` tag 指向 `c99e268007c1d4544ddecf71b5d0852527af7554`。
- hotfix 仅修复 CLI 在受保护默认工作目录下回退到用户目录的问题，不改变 v1.9.1 Relationship Graph Lite 的验收口径。

v1.9.2 的开发目标是实现 `Proactive Companion Loop`：在不打扰、不越权、不绕过工具审批的前提下，为通用型陪伴 agent 增加主动提醒、关心、话题延续和阶段性总结能力。

## 三、v1.9.2 范围边界

### 必须完成

- 新增主动陪伴策略层，建议建立 `crates/yunxi-agent-companion` crate。
- 在 `AgentConfig` 中增加主动陪伴配置，默认关闭或保守关闭。
- 在 runtime turn boundary 或显式检查入口接入 companion planner。
- 支持以下触发类型的规划能力：
  - 到期提醒。
  - 未完成任务轻提示。
  - 长时间空闲后的关心。
  - 话题延续。
  - 阶段性总结。
  - 基于 Relationship Graph Lite 的关系里程碑或上下文变化提醒。
- 主动消息必须携带明确触发原因，不能生成无来源、无解释的主动打扰。
- 所有主动行为必须可关闭，并支持限制频率和静默时段。
- 涉及工具执行时只能生成“请求用户确认”的计划，不能直接调用工具。

### 禁止提前扩展

- 不做 TUI memory inspector。
- 不做 evaluation harness。
- 不做云服务、外部 scheduler 或系统级后台常驻服务。
- 不做 SDK packaging、marketplace、completion、doctor、desktop app、app-server 等产品面。
- 不引入外部 Python runtime 作为 YunXi 默认运行依赖。
- 不默认依赖上游 Codex CLI 源码、`vendor/codex-rs`、`codex-*` crate 或 `yunxi-agent-codex`。

## 四、源码接入点

### `D:\YunXi Agent\crates\yunxi-agent-core\src\config.rs`

在 `AgentConfig` 中增加主动陪伴配置。建议新增结构：

```rust
pub struct CompanionSettings {
    pub enabled: bool,
    pub quiet_hours: Option<QuietHours>,
    pub max_proactive_per_session: u32,
    pub max_proactive_per_day: u32,
    pub require_reason: bool,
    pub allow_tool_requests: bool,
}
```

要求：

- `enabled` 默认必须为 `false`，或等价的默认不主动打扰状态。
- `require_reason` 默认必须为 `true`。
- `allow_tool_requests` 不能代表可直接执行工具，只能允许生成需要用户审批的工具建议。
- 配置变更要同步文档和默认配置示例，避免用户误以为主动陪伴默认开启。

### `D:\YunXi Agent\crates\yunxi-agent-runtime\src\lib.rs`

runtime 只负责在安全边界内调用 companion planner：

- 推荐在每轮用户 turn 结束、显式 companion check 命令、或 session resume 时调用规划。
- planner 返回 `CompanionPlan` 后，runtime 只把 message-only 或 suggestion 作为候选输出。
- planner 如果提出工具相关动作，runtime 必须转为“请求用户确认”的消息，实际工具执行仍走既有 approval/tool runtime。
- 不允许 companion planner 获得绕过审批的工具运行入口。

### 新增 `D:\YunXi Agent\crates\yunxi-agent-companion`

建议该 crate 保持纯 Rust 策略层，不绑定 CLI、TUI 或外部服务。推荐模块：

- `settings.rs`：主动陪伴配置和默认值。
- `trigger.rs`：触发类型与来源。
- `planner.rs`：触发聚合、限频、静默时段、计划生成。
- `action.rs`：主动动作表达。
- `reason.rs`：触发原因渲染和敏感内容裁剪。
- `tests`：默认关闭、限频、静默时段、审批边界、原因可见性回归测试。

## 五、推荐核心类型

```rust
pub enum CompanionTrigger {
    ReminderDue,
    UnfinishedTask,
    LongIdleCheckIn,
    TopicContinuation,
    PeriodicSummary,
    RelationshipMilestone,
}

pub enum CompanionAction {
    MessageOnly,
    SuggestNextStep,
    SummarizeStage,
    AskPermissionForTool,
}

pub struct CompanionPlan {
    pub trigger: CompanionTrigger,
    pub action: CompanionAction,
    pub reason: String,
    pub message: String,
    pub requires_user_confirmation: bool,
}

pub trait CompanionPlanner {
    fn plan(&self, input: CompanionInput) -> Vec<CompanionPlan>;
}
```

实现要求：

- `CompanionPlan.reason` 不得为空。
- `AskPermissionForTool` 必须设置 `requires_user_confirmation = true`。
- 默认配置下 `plan()` 应返回空计划。
- 限频和静默时段必须在 planner 层完成，runtime 不应重复猜测策略。
- 关系图触发原因要裁剪敏感内容，只表达“因为最近的偏好变化/未完成事项/阶段进展”等必要上下文。

## 六、参考源码抽取建议

审核报告指定参考源码：

- `D:\源码\QwenPaw\src\qwenpaw\app`
- `D:\源码\QwenPaw\src\qwenpaw\app\routers\skills.py`
- `D:\源码\QwenPaw\console\src\constants\channel.ts`

抽取原则：

- 只抽取主动触发、渠道分发、技能路由、提醒/关心策略的逻辑。
- 不迁入 Python runtime、前端 channel runtime 或 QwenPaw 的进程模型。
- 如果 QwenPaw 中存在非 Rust 逻辑，只进行 Rust 复刻。
- 参考源码不能成为 YunXi 默认运行路径依赖。
- 先搭建 YunXi 自有 companion crate 和 runtime 接入，再补充局部策略细节。

建议映射：

- QwenPaw app 的主动触发逻辑 -> `CompanionTrigger`。
- QwenPaw skills router 的能力分发逻辑 -> `CompanionAction` 与 planner 的 action selection。
- QwenPaw channel 常量 -> YunXi 内部消息 channel 枚举或 metadata，不直接复用前端常量。

## 七、实现顺序

1. 在 workspace 中新增 `yunxi-agent-companion` crate，并接入 `Cargo.toml` workspace members。
2. 定义 companion settings、trigger、action、plan、planner facade。
3. 在 `yunxi-agent-core` 配置层增加 `CompanionSettings`，默认关闭。
4. 在 `yunxi-agent-runtime` 增加最小接入点，仅在安全边界返回候选主动消息。
5. 接入 memory、recall router、relationship graph 的只读信号，生成原因明确的触发计划。
6. 增加限频、静默时段、用户关闭开关和工具审批边界测试。
7. 同步 README、设计文档、索引、状态文档和本开发报告中涉及的实际路径与状态。
8. 完成一批构建后统一验证，验证通过前不得宣称完成。
9. 阶段结束后清理编译中间产物；若清理命令涉及递归删除或等价高风险操作，必须先得到用户确认。
10. 为 v1.9.2 创建新的 Git tag，旧版本 tag 不得删除或移动。

## 八、验收标准

v1.9.2 完成时至少满足以下验收项：

- 默认配置下不主动发送消息。
- 用户可以关闭所有主动行为。
- 主动消息必须说明触发原因。
- 静默时段内不主动打扰。
- 单 session 和单日主动消息数量受限。
- 任何工具相关动作都不能绕过审批。
- 未经用户确认不得执行工具。
- reminder、unfinished task、topic continuation、periodic summary 至少具备最小可测试路径。
- Relationship Graph Lite 只作为只读信号源，不破坏 v1.9.1 的实体、关系、有效期、失效链和时间召回能力。
- persona、memory schema v3、L0-L3 memory pipeline、boot context、dynamic recall、relationship graph 的既有回归测试必须继续通过。

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

- 检查 `Cargo.toml` workspace version 是否进入 `1.9.2`。
- 检查 docs、索引、状态文档是否与实际代码状态一致。
- 检查默认配置示例是否明确“主动陪伴默认关闭”。
- 检查日志是否记录时间戳、目标、流程、修改文件、路径、验证结果、提交和推送状态。
- 检查是否已经创建新 tag，且未删除或移动旧 tag。

注意：`cargo clean`、递归删除 `target`、强制移动目录、清空目录等清理动作必须遵守用户 shell 安全要求；需要确认时先征得用户同意。

## 十、文档同步要求

v1.9.2 开发过程中，以下文档需要与代码状态保持一致：

- `D:\YunXi Agent\docs\reports\2026-07-17-220239-yunxi-agent-v1-9-2-proactive-companion-loop-development-report.md`
- `D:\YunXi Agent\docs\extraction-status.md`
- `D:\YunXi Agent\docs\persona-memory.md`
- `D:\YunXi Agent\README.md`
- 若新增 crate，应补充对应 crate 说明和 workspace 索引。
- 若 CLI 暴露开关，应同步 CLI 使用说明。

## 十一、本报告生成状态

- 本轮工作仅撰写开发报告，没有修改 Rust 源码。
- 本轮未运行 `cargo fmt`、`cargo check`、`cargo test` 或 `cargo build`。
- 本轮未执行编译产物清理。
- 本轮未提交、未推送、未创建 Git tag。
- 后续开发者必须以实际代码实现和验证结果更新设计文档、状态文档、索引和日志。

署名：开发报告撰写者

---

## 十二、本轮实际实现状态（2026-07-18）

本轮已开始并完成 v1.9.2 Proactive Companion Loop 的源码实现，实际状态以代码和统一验证结果为准：

- workspace 版本已从 `1.9.1` 升至 `1.9.2`。
- 新增 `D:\YunXi Agent\crates\yunxi-agent-companion` 纯 Rust crate，包含 settings 使用、trigger/action/plan、限频、静默时段、原因渲染、工具确认边界和 6 组策略单元测试。
- `D:\YunXi Agent\crates\yunxi-agent-core\src\config.rs` 新增 `CompanionSettings` 与 `QuietHours`；默认 `enabled=false`、`require_reason=true`、`allow_tool_requests=false`。
- `D:\YunXi Agent\crates\yunxi-agent-runtime\src\lib.rs` 在 turn 完成边界调用 planner；普通 turn 无明确 companion signal 时不产生主动消息；工具相关计划只输出需要用户确认的消息，不调用 `ToolRouter`。
- `D:\YunXi Agent\crates\yunxi-agent-cli\src\main.rs` 新增全局 `--companion` 和 `companion check` 显式入口。
- Relationship Graph Lite、memory recall 与 persona 只作为只读信号源；未新增 scheduler、云服务、外部 Python runtime、数据库或后台常驻服务。
- 已同步 `README.md`、`docs\extraction-status.md`、`docs\persona-memory.md`、CLI/TUI/Persona 当前版本显示。

已完成的阶段验证：

- `cargo fmt --all` 通过。
- `cargo check -p yunxi-agent-companion -p yunxi-agent-core` 通过。
- `cargo check -p yunxi-agent-runtime -p yunxi-agent-cli` 通过。
- companion crate 6/6 单元测试通过。
- runtime 当前测试 43/43 通过，包含默认关闭、显式 companion/tool confirmation 回归。

尚未宣称最终完成：全 workspace `cargo check/test/build`、清理构建产物、最终日志追加、提交、创建 `v1.9.2` tag 和 GitHub 推送需在统一验证全部通过后执行。

## 十三、最终实现与验证结果（2026-07-18）

最终实现已完成并通过统一验证：

- `cargo fmt --all -- --check`：通过。
- `cargo check --workspace`：通过。
- `cargo test --workspace`：通过；companion crate 6/6，runtime 43/43，既有 CLI、Persona、Memory Schema v3、L0-L3、Boot/Dynamic Recall、Relationship Graph Lite、Storage、Provider、Tools、TUI 回归全部通过。
- `cargo build --workspace`：通过。
- `cargo build -p yunxi-agent-cli --release --bins`：通过；两个二进制输出 `yunxi 1.9.2`。
- Release CLI 冒烟：默认配置不产生 companion 计划；`--companion` 的工具建议包含原因并明确“不会自动执行”；`companion check unfinished task` 输出下一步建议和原因。
- 发现并修复了 CLI companion 入口的两个问题：显式入口开启工具建议确认边界；`companion check` 前缀在 signal parser 中正确剥离。
- companion session 限频使用显式 session id，缺省时使用 workspace 作为稳定 key，不会因底层 turn id 每轮变化而重置。
- 旧 `v1.9.1` tag object `efd1302eff252b2aae0f0e4637c37e9401a4e8f0` 与 `v1.9.1-hotfix.1` tag object `b211aca4bd2af17c60b6dd95fa344ba45e260407` 均未移动或删除。
- 已按用户确认执行 `cargo clean`，清理 `D:\YunXi Agent\target`，未删除源码、Git 数据或安装目录。

发布状态：

- 本地 commit：`61ef2d3008faa9bf75b1247a238369037b25c24d`。
- 本地 annotated `v1.9.2` tag object：`77e08a2f3b6982c37f6e86de2a96e3b016ea879e`，解析到上述 commit。
- 两次使用 GitHub API key 的 non-force 远程连接均在 `github.com:443` 连接阶段失败，无法读取远程 master，因此本轮尚未推送；没有执行 force push，也没有修改旧 tag。
- 本地工作树在提交后干净；当前相对远程为 `master...origin/master [ahead 1]`。
