# YunXi Agent v1.9.4 Evaluation Harness 开发报告

撰写时间：2026-07-18 11:49:05 +08:00
审核报告：`C:\Users\24763\Desktop\YunXi Agent审核报告\2026-07-18-104346-YunXi-Agent-v1.9.3-源码审核报告.md`
开发目录：`D:\YunXi Agent`
项目内报告位置：`D:\YunXi Agent\docs\reports\2026-07-18-114905-yunxi-agent-v1-9-4-evaluation-harness-development-report.md`
桌面报告位置：`C:\Users\24763\Desktop\YunXi Agent开发报告\2026-07-18-114905-yunxi-agent-v1-9-4-evaluation-harness-development-report.md`

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

本次审核报告判定 YunXi Agent v1.9.3 审核通过，可以进入下一版本开发。当前发布基线如下：

- `Cargo.toml` workspace version：`1.9.3`。
- 当前 HEAD：`87ea400`，提交说明为 `docs: finalize v1.9.3 publication status`。
- 当前分支状态：`master...origin/master`，工作树干净。
- 本地存在 annotated `v1.9.3` tag，解析到实现提交 `3bcd02146f03c430574a8d55110894d5247546b7`。
- `D:\YunXi Agent\target` 已清理，不存在。
- 旧 `v1.9.2`、`v1.9.1`、`v1.9.1-hotfix.1` 等 tag 未被删除、移动或重写。

v1.9.4 的开发目标是 `Evaluation Harness`：为通用型陪伴 agent 建立可自动运行、可复现、可量化的评测框架，覆盖人格一致性、记忆准确率、关系连续性、误记率和主动性边界。该版本不是扩展新陪伴功能，而是为已经具备的 persona、memory、relationship、companion UX/control 能力建立质量闸门。

## 三、v1.9.4 范围边界

### 必须完成

- 新增 `D:\YunXi Agent\evals\companion` 评测目录，用于存放陪伴场景、golden 期望、指标配置和结果说明。
- 至少提供 30 条陪伴场景测试，覆盖 persona、memory、relationship、companion proactive boundary 和 CLI/control 操作。
- 记忆写入必须有 precision 指标，能统计正确写入、误写入、漏写入和不应写入却写入的场景。
- persona regression 必须可自动运行，不能只靠人工阅读输出。
- 关系连续性需要覆盖旧事实被替代、关系有效期、只读关系状态展示和历史链保留。
- 主动性边界评测必须覆盖默认不打扰、显式关闭、quiet hours、频率限制、工具确认不执行。
- 评测输出应生成小型结构化摘要，便于审核者复核，不应把大量临时产物提交进仓库。

### 禁止提前扩展

- 不引入外部评测服务、云端 judge 或常驻后台。
- 不把 live LLM provider 作为默认评测路径；默认评测应使用可复现的固定输入、规则 judge、mock backend 或离线夹具。
- 不为了评测引入新的上游 Codex CLI 依赖、`vendor/codex-rs`、`codex-*` crate 或 `yunxi-agent-codex`。
- 不把评测结果写成庞大二进制或大量快照文件。
- 不为了单个指标反复纠结；先建立整体评测骨架、场景集、指标输出和自动化入口。

## 四、源码接入点

### `D:\YunXi Agent\crates\yunxi-agent-persona\tests`

persona 测试是 v1.9.4 的核心入口之一。建议补充：

- persona consistency regression：同一用户事实、偏好和人格档案在多轮场景中保持一致。
- memory precision regression：只写入应该写入的记忆，拒绝临时噪声、幻觉事实和敏感误记。
- relationship continuity regression：旧事实不会覆盖新事实，关系时间线可解释。
- persona / memory / relationship 的组合场景测试，不只测试单个函数。

### `D:\YunXi Agent\crates\yunxi-agent-cli\tests`

CLI 测试应覆盖评测入口和控制面边界：

- `yunxi eval companion` 或等价命令的 smoke test。
- JSON/JSONL 输出的稳定性测试，便于审核工具读取指标。
- control / companion / memory / persona 命令在 eval fixture 下的行为一致性。
- 未确认清除、关系只读拒绝、工具请求不执行等边界继续回归。

### `D:\YunXi Agent\evals\companion`

建议目录结构：

```text
evals/companion/
  README.md
  scenarios/
    persona_consistency.jsonl
    memory_precision.jsonl
    relationship_continuity.jsonl
    proactive_boundaries.jsonl
  golden/
    companion_expected_metrics.json
  schemas/
    scenario.schema.json
    result.schema.json
```

该目录只保存小型、可审阅、可版本化的场景和期望。运行时临时输出可写入 `target/evals` 或临时目录；阶段结束后必须清理，且涉及递归删除时先得到用户确认。

## 五、推荐评测模型

建议以结构化场景作为统一输入：

```rust
pub struct CompanionEvalScenario {
    pub id: String,
    pub category: EvalCategory,
    pub turns: Vec<EvalTurn>,
    pub expected_persona: Vec<ExpectedFact>,
    pub expected_memory_writes: Vec<ExpectedMemoryWrite>,
    pub forbidden_memory_writes: Vec<String>,
    pub expected_relationships: Vec<ExpectedRelationship>,
    pub proactive_expectations: ProactiveExpectations,
}
```

建议输出指标：

- `persona_consistency_rate`：人格档案在场景中的一致性通过率。
- `memory_precision`：正确记忆写入数 / 总写入数。
- `memory_false_positive_rate`：不应写入却写入的比例。
- `memory_recall_accuracy`：应召回事实是否在正确场景中出现。
- `relationship_continuity_rate`：关系替代链、有效期和历史保留是否正确。
- `proactive_boundary_violation_count`：主动陪伴越界次数。
- `tool_approval_bypass_count`：工具审批绕过次数，必须为 0。

最低验收建议：

- 陪伴场景数量不少于 30 条。
- memory precision 指标必须输出，并在报告中记录阈值。
- persona regression 必须可通过单条 cargo test 或 CLI eval 命令自动运行。
- 工具审批绕过计数必须为 0。
- 主动陪伴默认关闭和显式关闭场景必须为 0 proactive output。

## 六、参考源码抽取建议

审核报告指定参考源码：

- `D:\源码\yantrikdb\src\yantrikdb\eval`
- `D:\源码\mem0\evaluation`
- `D:\源码\cognee\evals`

抽取原则：

- 只抽取评测组织、数据集结构、指标聚合、golden comparison 和结果摘要逻辑。
- 参考源码如果不是 Rust，只参考逻辑并在 YunXi 内 Rust 复刻。
- 不迁入外部 Python runtime、外部评测框架、云 judge 或远程服务作为默认路径。
- 先搭建 `evals/companion` 场景集、Rust 测试入口和指标输出，再补充细分指标。

建议映射：

- yantrikdb eval 的评测 runner 与结果汇总思路 -> YunXi 的 companion eval runner。
- mem0 evaluation 的记忆准确率、precision/recall 思路 -> YunXi 的 memory precision 指标。
- cognee evals 的数据集和 golden 断言结构 -> YunXi 的 scenario/golden 目录。

## 七、实现顺序

1. 建立 `evals/companion` 目录、README、scenario schema、result schema 和最小 golden 文件。
2. 设计 `CompanionEvalScenario`、`CompanionEvalResult`、`CompanionEvalMetrics` 等 Rust 类型，优先放在测试辅助模块或轻量 eval crate 中。
3. 编写首批不少于 30 条场景，先覆盖五类核心目标，再逐步补充边界。
4. 在 `crates\yunxi-agent-persona\tests` 中接入 persona/memory/relationship regression。
5. 在 `crates\yunxi-agent-cli\tests` 中接入 CLI eval 命令和 JSON/JSONL 输出验证。
6. 增加主动性边界评测，确认默认关闭、显式关闭、quiet hours、限频和工具审批边界。
7. 生成小型结构化评测摘要，并在文档中记录如何复跑。
8. 同步 `README.md`、`docs/extraction-status.md`、`docs/persona-memory.md`、开发报告和开发日志。
9. 完成一批构建后统一验证，验证通过前不得宣称完成。
10. 阶段结束后清理编译中间产物；若清理命令涉及递归删除或等价高风险操作，必须先得到用户确认。
11. 为 v1.9.4 创建新的 Git tag，旧版本 tag 不得删除或移动。

## 八、验收标准

v1.9.4 完成时至少满足以下验收项：

- `evals/companion` 中存在不少于 30 条陪伴场景。
- persona consistency regression 可自动运行。
- memory precision 指标可自动计算并输出。
- relationship continuity 指标覆盖关系替代链、有效期和历史保留。
- proactive boundary 指标覆盖默认关闭、显式关闭、quiet hours、频率限制和工具审批。
- CLI 或 cargo test 至少有一个稳定入口可一键跑完核心 eval。
- 评测结果具备结构化摘要，审核者可复核指标。
- 全 workspace 验证通过后才能宣称完成。

## 九、统一验证要求

开发者应在完成一批构建后统一执行验证，建议顺序如下：

```powershell
cargo fmt --all
cargo fmt --all -- --check
cargo check --workspace
cargo test --workspace
cargo build --workspace
git status --short
git diff --stat
```

补充检查：

- 检查 `Cargo.toml` workspace version 是否进入 `1.9.4`。
- 检查 eval 场景数是否不少于 30 条。
- 检查 memory precision、persona regression、relationship continuity、proactive boundary 指标是否都有自动化输出。
- 检查默认评测路径是否不依赖 live provider、云服务或外部 Python runtime。
- 检查文档、索引、状态文档和日志是否与实际代码状态一致。
- 检查是否已经创建新的 `v1.9.4` tag，且旧 tag 未删除、未移动、未重写。

注意：`cargo clean`、递归删除 `target`、强制移动目录、清空目录等清理动作必须遵守用户 shell 安全要求；需要确认时先征得用户同意。

## 十、文档同步要求

v1.9.4 开发过程中，以下文档需要与代码状态保持一致：

- `D:\YunXi Agent\docs\reports\2026-07-18-114905-yunxi-agent-v1-9-4-evaluation-harness-development-report.md`
- `D:\YunXi Agent\docs\development-log.md`
- `D:\YunXi Agent\docs\extraction-status.md`
- `D:\YunXi Agent\docs\persona-memory.md`
- `D:\YunXi Agent\README.md`
- `D:\YunXi Agent\evals\companion\README.md`

## 十一、实际实现结果

实现时间：2026-07-18 12:20:29 +08:00

- 新增独立 Rust workspace crate `crates/yunxi-agent-eval`，提供场景、检查结果、聚合指标、结构化报告、golden 阈值比较和文本摘要。
- 新增 `evals/companion` 版本化评测集、README、scenario/result schema 与 golden 指标文件。
- 共实现 31 条离线确定性场景：persona 6 条、memory 8 条、relationship 6 条、proactive 6 条、controls 5 条。
- 评测 runner 直接调用现有 persona 编译器、memory extractor、relationship graph、companion planner 和 control facade，不使用 live provider、云 judge、Python runtime、外部服务或上游 Codex runtime。
- memory 指标区分正确写入、误写、漏写和禁止写入，并输出 precision、false-positive rate 与 recall accuracy。
- relationship 指标覆盖替代链、历史保留、过期、未来有效期、时间线顺序与只读状态。
- proactive/control 指标覆盖默认关闭、显式关闭、quiet hours、会话/每日限频、原因可见、工具确认、clear 确认、cloud 默认关闭、只读来源和审计序列化。
- CLI 新增 `yunxi eval companion`，并支持 `--json` 和单行 `--jsonl` 结构化输出；任一场景失败或 golden 阈值未通过时以失败退出码结束。
- persona 与 CLI 测试分别新增自动化 regression 和命令/输出集成覆盖。
- workspace、CLI、persona context/profile 与 TUI 可见版本统一更新为 `1.9.4`。

主要修改路径：

- `D:\YunXi Agent\crates\yunxi-agent-eval`
- `D:\YunXi Agent\evals\companion`
- `D:\YunXi Agent\crates\yunxi-agent-cli\Cargo.toml`
- `D:\YunXi Agent\crates\yunxi-agent-cli\src\main.rs`
- `D:\YunXi Agent\crates\yunxi-agent-cli\src\render.rs`
- `D:\YunXi Agent\crates\yunxi-agent-cli\tests\cli_tests.rs`
- `D:\YunXi Agent\crates\yunxi-agent-persona\src\compiler.rs`
- `D:\YunXi Agent\crates\yunxi-agent-persona\src\profile.rs`
- `D:\YunXi Agent\crates\yunxi-agent-persona\tests\evaluation_regression_tests.rs`
- `D:\YunXi Agent\crates\yunxi-agent-tui\src\app.rs`
- `D:\YunXi Agent\crates\yunxi-agent-tui\src\render.rs`
- `D:\YunXi Agent\Cargo.toml`、`Cargo.lock`、`README.md`
- `D:\YunXi Agent\docs\extraction-status.md`
- `D:\YunXi Agent\docs\persona-memory.md`

## 十二、统一验证与清理结果

- `cargo fmt --all`：通过。
- `cargo fmt --all -- --check`：通过。
- `cargo check --workspace`：通过。
- `cargo test --workspace`：通过。
- `cargo build --workspace`：通过。
- `cargo build -p yunxi-agent-cli --release --bins`：通过。
- Release 冒烟：`yunxi 1.9.4`；31/31 场景通过；`golden_passed=true`。
- Release 指标：persona consistency `1.0`、memory precision `1.0`、memory recall accuracy `1.0`、relationship continuity `1.0`、control regression `1.0`；误写、漏写、禁止写入、proactive violation 和 tool approval bypass 均为 `0`。
- JSON 输出可解析；JSONL 输出恰好一行。
- `git diff --check`：通过，仅存在 Git 的 LF/CRLF 工作区提示，没有空白错误。
- 已在确认仓库根目录为 `D:\YunXi Agent` 后执行 `cargo clean`，清理 10,430 个文件、约 3.1 GiB；测试生成的 crate 局部 `.yunxi` 状态目录也已清理；`D:\YunXi Agent\target` 不存在。

## 十三、提交与发布状态

实现提交已创建：`051002125158535023fa8bf7dbe41b430b398648`。

annotated `v1.9.4` tag 已创建，tag object 为
`7b99ae5422cdf904aef9a5893ee7c1cfc601435c`，解析到上述实现提交。旧的
`v1.9.3`、`v1.9.2` 及更早 tag 未删除、未移动、未重写。当前只剩本报告和
项目日志的发布状态收尾提交，以及 GitHub non-force 推送待执行；最终远程
`master`、`v1.9.4` peeled tag 和旧 tag 状态以推送后的 Git 引用核验为准。

署名：开发者
