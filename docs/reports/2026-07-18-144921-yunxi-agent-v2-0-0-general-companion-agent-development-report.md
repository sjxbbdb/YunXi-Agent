# YunXi Agent v2.0.0 General Companion Agent 开发报告

撰写时间：2026-07-18 14:49:21 +08:00
审核报告：`C:\Users\24763\Desktop\YunXi Agent审核报告\2026-07-18-143223-YunXi-Agent-v1.9.4-源码审核报告.md`
开发目录：`D:\YunXi Agent`
项目内报告位置：`D:\YunXi Agent\docs\reports\2026-07-18-144921-yunxi-agent-v2-0-0-general-companion-agent-development-report.md`
桌面报告位置：`C:\Users\24763\Desktop\YunXi Agent开发报告\2026-07-18-144921-yunxi-agent-v2-0-0-general-companion-agent-development-report.md`

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

本次审核报告判定 YunXi Agent v1.9.4 审核通过，可以进入下一版本开发。当前发布基线如下：

- `Cargo.toml` workspace version：`1.9.4`。
- 当前 HEAD：`fdb1793`，提交说明为 `docs: record v1.9.4 remote publication`。
- 当前分支状态：`master...origin/master`，工作树干净。
- 本地存在 annotated `v1.9.4` tag，解析到实现提交 `051002125158535023fa8bf7dbe41b430b398648`。
- `D:\YunXi Agent\target` 已清理，不存在。
- 旧 `v1.9.3`、`v1.9.2`、`v1.9.1` 等 tag 未被删除、移动或重写。
- 当前 HEAD 位于 `v1.9.4` tag 之后的 docs-only 发布状态提交；该提交不改变 v1.9.4 核心功能验收口径。

v2.0.0 的开发目标是 `General Companion Agent`：完成通用型陪伴 Agent 的总集成和发布闭环，把 v1.8.7 到 v1.9.4 已构建的人格、长期记忆、关系连续性、主动陪伴、审计控制和离线评测能力整合为可本地运行、可跨会话保持关系、可审查记忆、可关闭主动性、可验证质量的稳定版本。

## 三、v2.0.0 范围边界

### 必须完成

- 全 workspace 版本提升到 `2.0.0`，并确保 CLI release 输出 `yunxi 2.0.0`。
- 闭合人格、长期记忆、关系连续性、主动陪伴、控制审计和评测能力之间的实际运行链路。
- 默认 YunXi 运行路径必须摆脱上游 Codex CLI 源码依赖。
- 默认运行不得依赖 `vendor/codex-rs`、`codex-*` crate 或 `yunxi-agent-codex`。
- 本地运行必须可用：离线默认路径、受控 live provider 路径、CLI/TUI 基础路径都要有清晰边界。
- 跨会话关系保持必须可验证，关系状态要继续以只读派生视图展示，旧事实不得覆盖新事实。
- 记忆必须可审查、可拒绝、可归档、可确认清理；清除动作必须有确认和审计记录。
- 主动陪伴必须默认关闭、可关闭、可解释，不得绕过工具审批。
- v1.9.4 的 companion eval 必须继续可自动运行，并作为 v2.0.0 发布前质量闸门。
- 文档和测试报告必须完整，能够支撑审核者复核“通用型陪伴 Agent 已完成”的结论。

### 禁止提前扩展

- 不引入新的云服务、后台常驻服务、远程 judge、外部 scheduler 或桌面应用。
- 不把 v2.0.0 做成上游 Codex CLI 的包装版本；YunXi 默认路径必须是自有 Rust workspace。
- 不新增重型外部数据库、外部 memory runtime、外部 graph runtime 或非 Rust 默认运行依赖。
- 不把 v2.0.0 的目标扩散到 SDK packaging、marketplace、completion、doctor、app-server 等产品面。
- 不为了单个体验细节无限打磨；先完成全链路能力闭合、质量闸门、文档和发布闭环。

## 四、源码接入范围

v2.0.0 是全 workspace 总集成版本，重点接入范围如下：

### `D:\YunXi Agent\crates\yunxi-agent-core`

核心要求：

- `AgentConfig` 继续作为统一配置入口，不能让 CLI、runtime、TUI 各自维护冲突配置。
- `CompanionSettings`、control facade、approval/sandbox 语义必须保持保守默认值。
- 对外 facade 类型要清晰，避免泄漏上游 Codex 内部结构。

### `D:\YunXi Agent\crates\yunxi-agent-runtime`

核心要求：

- runtime 要负责把 persona、memory、relationship、companion 和 control snapshot 接到 turn boundary，而不是把策略分散到 CLI。
- 工具执行必须继续走既有审批与 tool runtime，不允许 proactive plan 直接触发工具。
- 跨会话状态读取、主动陪伴历史、记忆召回和关系摘要要在 runtime 层形成稳定边界。

### `D:\YunXi Agent\crates\yunxi-agent-persona`

核心要求：

- Persona Context Blocks、Memory Schema v3、L0-L3 Memory Pipeline、Relationship Graph Lite 必须保持回归稳定。
- 继续保证旧事实不会覆盖新事实，历史关系链可解释。
- 记忆写入、召回、迁移、失效链和审查控制要服务于“长期陪伴”主目标。

### `D:\YunXi Agent\crates\yunxi-agent-cli`

核心要求：

- CLI 必须提供本地可运行的主入口、记忆审查入口、人格状态入口、关系状态入口、主动陪伴控制入口和 eval 入口。
- `yunxi eval companion` 应作为 v2.0.0 发布前必跑命令之一。
- JSON/JSONL 输出继续保持可审计、可解析、无敏感信息泄漏。

### `D:\YunXi Agent\crates\yunxi-agent-tui`

核心要求：

- TUI 继续展示共享 `ControlSnapshot`，保持 memory/persona/relationship/companion 状态一致。
- TUI 不能成为必须路径；CLI 和 runtime 仍应可独立完成核心能力。
- 清除类操作必须有确认边界，不能因快捷键或面板操作绕过确认。

### `D:\YunXi Agent\crates\yunxi-agent-eval`
### `D:\YunXi Agent\evals\companion`

核心要求：

- v1.9.4 的 31 条场景继续作为 v2.0.0 基础质量闸门。
- 指标继续覆盖 persona consistency、memory precision、memory recall accuracy、relationship continuity、proactive boundary、tool approval bypass、control regression。
- v2.0.0 可以追加总集成 smoke 场景，但不得降低已有 golden 阈值。

## 五、推荐总集成模型

建议用“能力闭环矩阵”统筹 v2.0.0，而不是继续按单点功能堆叠：

```text
输入与运行：CLI/TUI -> runtime -> provider/offline backend
人格与记忆：persona profile -> memory pipeline -> recall router
关系连续：memory schema -> relationship graph -> control snapshot
主动陪伴：companion planner -> reasoned message -> approval boundary
控制审计：control request -> audit record -> review/list/show
质量闸门：eval scenario -> metrics -> golden threshold -> release gate
```

每个闭环都需要同时回答四个问题：

- 默认是否安全。
- 用户是否能看见和关闭。
- 行为是否可审计。
- 是否有自动化测试或 eval 指标证明。

## 六、参考源码抽取建议

v2.0.0 不建议新增新的重型参考源码。开发者应综合 v1.8.7 至 v1.9.4 已经抽取过的参考项目，只复用已经被证明有价值的逻辑：

- Persona / memory 方向：继续保留已复刻的人格块、记忆 schema、记忆审查和状态组织方式。
- Relationship 方向：继续保留 temporal graph、关系替代链、有效期、历史事件召回逻辑。
- Proactive companion 方向：继续保留主动触发、原因说明、默认关闭、静默边界和工具确认边界。
- Control UX 方向：继续保留状态可见、清除确认、审计记录和 CLI/TUI 共享快照。
- Evaluation 方向：继续保留 scenario/golden/metrics 的离线确定性评测方式。

如果仍需阅读参考源码，只允许抽取逻辑，不迁入其运行时；非 Rust 逻辑必须在 YunXi 内 Rust 复刻。

## 七、实现顺序

1. 将 workspace version 升级为 `2.0.0`，同步 CLI/TUI/README/docs 中的版本显示。
2. 核对默认运行路径，移除任何会让 YunXi 默认依赖上游 Codex CLI 源码或 `codex-*` crate 的路径。
3. 跑通本地离线 CLI 主流程，确认人格、记忆、关系、主动陪伴和控制入口不互相冲突。
4. 增加 v2.0.0 总集成 smoke 测试，覆盖跨会话记忆、关系连续、主动关闭、记忆审查和 eval 入口。
5. 复跑 v1.9.4 companion eval，必要时追加 v2.0.0 集成场景，但不得降低 golden 阈值。
6. 同步 README、`docs/extraction-status.md`、`docs/persona-memory.md`、`evals/companion/README.md`、开发报告和日志。
7. 完成一批构建后统一执行验证，验证通过前不得宣称完成。
8. 阶段结束后清理编译中间产物；若清理命令涉及递归删除或等价高风险操作，必须先得到用户确认。
9. 提交 v2.0.0 变更并推送，不得移动或删除旧 tag。
10. 创建新的 annotated `v2.0.0` Git tag，旧版本 tag 不得删除、移动或重写。

## 八、验收标准

v2.0.0 完成时至少满足以下验收项：

- 可本地运行，默认路径不依赖上游 Codex CLI 源码、`vendor/codex-rs`、`codex-*` crate 或 `yunxi-agent-codex`。
- 可跨会话保持关系，并能证明旧事实不会覆盖新事实。
- 用户可审查、拒绝、归档和确认清理记忆。
- 用户可查看人格档案、记忆摘要、关系状态和主动陪伴设置。
- 主动陪伴默认关闭、可关闭、可解释，不绕过工具审批。
- 控制动作和主动陪伴关键行为有审计记录。
- companion eval 通过，场景数不少于 31 条，golden 阈值通过。
- 全 workspace fmt/check/test/build 通过。
- release 二进制显示 `yunxi 2.0.0`。
- `target` 等编译中间产物在阶段结束后按安全流程清理。
- 新建 `v2.0.0` tag，旧 tag 未删除、未移动、未重写。
- 文档和日志记录与实际代码、验证结果、提交、推送、tag 状态一致。

## 九、统一验证要求

开发者应在完成一批构建后统一执行验证，建议顺序如下：

```powershell
cargo fmt --all
cargo fmt --all -- --check
cargo check --workspace
cargo test --workspace
cargo build --workspace
cargo build -p yunxi-agent-cli --release --bins
.\target\release\yunxi.exe --version
.\target\release\yunxi.exe eval companion --json
.\target\release\yunxi.exe eval companion --jsonl
git diff --check
git status --short
git diff --stat
```

补充检查：

- 检查 `Cargo.toml` workspace version 是否进入 `2.0.0`。
- 检查 release 输出是否为 `yunxi 2.0.0`。
- 检查默认运行路径是否摆脱上游 Codex CLI 源码依赖。
- 检查 `eval companion` 是否离线确定性通过，且 `golden_passed=true`。
- 检查 memory/persona/relationship/companion/control 文档是否与实际状态一致。
- 检查是否已经创建新的 `v2.0.0` tag，且旧 tag 未删除、未移动、未重写。

注意：`cargo clean`、递归删除 `target`、强制移动目录、清空目录等清理动作必须遵守用户 shell 安全要求；需要确认时先征得用户同意。

## 十、文档同步要求

v2.0.0 开发过程中，以下文档需要与代码状态保持一致：

- `D:\YunXi Agent\docs\reports\2026-07-18-144921-yunxi-agent-v2-0-0-general-companion-agent-development-report.md`
- `D:\YunXi Agent\docs\development-log.md`
- `D:\YunXi Agent\docs\extraction-status.md`
- `D:\YunXi Agent\docs\persona-memory.md`
- `D:\YunXi Agent\README.md`
- `D:\YunXi Agent\evals\companion\README.md`
- 如新增 v2.0.0 测试报告，应放在项目约定 docs/reports 或 evals 文档范围内，不提交大量临时运行产物。

## 十一、实际开发与验证状态

开发时间：2026-07-18 15:50:31 +08:00

### 实际实现

- workspace、CLI、TUI、persona profile/context 与 evaluation harness 已统一升级为 `2.0.0`。
- `crates/yunxi-agent-runtime/src/general_companion.rs` 新增
  `GeneralCompanionSnapshot` 与 `general_companion_snapshot`，以公开、可序列化
  facade 汇总 YunXi 运行时所有权、persona、Memory Schema v3、只读关系视图、
  proactive 默认值、cloud control 和共享 `ControlSnapshot`。
- `control_snapshot` 的 active memory 统计改为使用 `is_recallable_at`，不再把
  已过期、失效或被替代的旧事实计入 active 数量。
- `crates/yunxi-agent-runtime/tests/general_companion_tests.rs` 新增两个独立 session
  的总集成回归：第一会话持久化语言偏好，第二会话验证 recall；旧关系事实通过
  supersession chain 保留在 append-only 历史中，但不会覆盖新事实或进入活动上下文。
- CLI 新增 v2 发布门禁回归，覆盖离线 one-shot、共享 control snapshot、
  companion/cloud 默认关闭、relationship 只读、31 场景 eval 与工具审批零绕过。
- 保留全部 31 条 v1.9.4 companion eval 场景与原 golden 阈值，只升级 harness
  版本和 persona/context 版本断言，没有引入 live provider、云 judge、Python、
  外部 scheduler、数据库或上游 Codex 默认运行依赖。

### 修改路径

- `D:\YunXi Agent\Cargo.toml`
- `D:\YunXi Agent\Cargo.lock`
- `D:\YunXi Agent\crates\yunxi-agent-runtime\src\lib.rs`
- `D:\YunXi Agent\crates\yunxi-agent-runtime\src\general_companion.rs`
- `D:\YunXi Agent\crates\yunxi-agent-runtime\tests\general_companion_tests.rs`
- `D:\YunXi Agent\crates\yunxi-agent-cli\src\main.rs`
- `D:\YunXi Agent\crates\yunxi-agent-cli\src\render.rs`
- `D:\YunXi Agent\crates\yunxi-agent-cli\tests\cli_tests.rs`
- `D:\YunXi Agent\crates\yunxi-agent-eval\src\lib.rs`
- `D:\YunXi Agent\crates\yunxi-agent-persona\src\compiler.rs`
- `D:\YunXi Agent\crates\yunxi-agent-persona\src\profile.rs`
- `D:\YunXi Agent\crates\yunxi-agent-persona\tests\evaluation_regression_tests.rs`
- `D:\YunXi Agent\crates\yunxi-agent-persona\tests\persona_tests.rs`
- `D:\YunXi Agent\crates\yunxi-agent-tui\src\app.rs`
- `D:\YunXi Agent\crates\yunxi-agent-tui\src\render.rs`
- `D:\YunXi Agent\README.md`
- `D:\YunXi Agent\docs\extraction-status.md`
- `D:\YunXi Agent\docs\persona-memory.md`
- `D:\YunXi Agent\evals\companion\README.md`
- 本报告与 `D:\YunXi Agent\docs\development-log.md`

### 统一验证结果

- `cargo fmt --all`：通过。
- `cargo fmt --all -- --check`：通过。
- `cargo check --workspace`：通过，workspace crate 全部解析为 2.0.0。
- `cargo test --workspace`：通过；新增跨会话总集成测试、44 条 CLI 集成测试、
  44 条 runtime 集成测试及全 workspace 单元/集成/doc tests 均为零失败。
- `cargo build --workspace`：通过。
- `cargo build -p yunxi-agent-cli --release --bins`：通过。
- release 冒烟：`yunxi 2.0.0`；离线 one-shot 输出 `[offline]`；controls 显示
  companion/cloud control 均为 false。
- `yunxi eval companion`：31/31 通过，`golden_passed=true`；persona consistency、
  memory precision、memory recall accuracy、relationship continuity、control regression
  均为 1.0；proactive boundary violation 与 tool approval bypass 均为 0。
- JSON 可解析；JSONL 恰好一行且 golden 通过。
- 默认 CLI 正常依赖树共 413 行，`codex-*`/`yunxi-agent-codex` 匹配数为 0。
- `git diff --check`：通过，仅有 Git 的 CRLF 转换提示，无空白错误。

### 清理与发布状态

- 已严格核验目标路径并执行 `cargo clean`，删除 9,540 个文件、约 2.9 GiB；
  `D:\YunXi Agent\target` 不存在。
- 已删除仅由本轮 CLI 测试生成的
  `D:\YunXi Agent\crates\yunxi-agent-cli\.yunxi`；未触碰项目根 `.yunxi`、
  用户配置、已安装二进制或 PATH。
- 实现提交、annotated `v2.0.0` tag 与 GitHub non-force 推送将在发布收口后
  把实际 commit/tag object/远程核验结果回写到本节。

署名：开发者
