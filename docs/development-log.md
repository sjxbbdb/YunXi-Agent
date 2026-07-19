## 2026-07-18 09:04:23 +08:00

工作目标：基于用户纠正后的最新信息，重新审核 YunXi Agent v1.9.2 当前源码与发布状态，并在项目内写入重新审核报告与日志。

执行流程：
1. 重新核对 `D:\YunXi Agent` 当前 Git 状态、HEAD、workspace version、tag 与 `target` 清理状态。
2. 只读复核 v1.9.2 companion 相关实现：`yunxi-agent-core`、`yunxi-agent-companion`、`yunxi-agent-runtime`、`yunxi-agent-cli`。
3. 复核 `docs\extraction-status.md` 与 v1.9.2 开发报告中的最终验证与发布闭环记录。
4. 按总纲图只对 v1.9.2 的当前要求做重新审核，不沿用上一轮因信息滞后造成的旧结论。
5. 写入本次重新审核报告，并在项目内追加本次工作日志。

修改文件：
- 新增重新审核报告：`D:\YunXi Agent\docs\reports\2026-07-18-090423-yunxi-agent-v1-9-2-reaudit-source-audit-report.md`
- 新增日志：`D:\YunXi Agent\docs\development-log.md`

文件路径：
- 项目源码目录：`D:\YunXi Agent`
- 审核报告目录：`D:\YunXi Agent\docs\reports`
- 项目日志：`D:\YunXi Agent\docs\development-log.md`

验证结果：
- workspace version 为 `1.9.2`。
- 当前 HEAD 为 `1467d3e`，`master` 与 `origin/master` 对齐。
- 本地存在 `v1.9.2` tag，解析到 `61ef2d3008faa9bf75b1247a238369037b25c24d`。
- `D:\YunXi Agent\target` 已清理。
- 当前源码中的 companion policy、runtime boundary、CLI 入口、测试和状态文档已对齐 v1.9.2 要求。
- 当前版本满足默认不打扰、可关闭、可解释、工具不绕过审批、统一验证与发布闭环等硬指标。

提交和推送状态：本次仅写入项目内审核报告与日志；源码未改动，Git 工作树保持干净，前序版本 tag 未被修改。

署名：审核者

## 2026-07-18 09:16:20 +08:00

工作目标：根据用户提供的正确重新审核报告，撰写 YunXi Agent v1.9.3 Companion UX & Controls 开发报告，并同步项目内开发日志。

执行流程：
1. 读取 `D:\YunXi Agent\docs\reports\2026-07-18-090423-yunxi-agent-v1-9-2-reaudit-source-audit-report.md`，确认 v1.9.2 已重新审核通过。
2. 核对当前项目状态与现有报告目录，确认需要进入 v1.9.3 Companion UX & Controls 的开发报告撰写。
3. 根据审核报告要求，围绕记忆审查、人格档案、关系状态、主动陪伴设置、CLI/TUI 控制入口和源码参考建议编写开发报告。
4. 将开发报告写入项目内 `docs/reports`，并同步到桌面开发报告目录。
5. 追加本次工作日志，记录当前报告撰写与文件同步状态。

修改文件：
- 新增开发报告：`D:\YunXi Agent\docs\reports\2026-07-18-091620-yunxi-agent-v1-9-3-companion-ux-controls-development-report.md`
- 追加日志：`D:\YunXi Agent\docs\development-log.md`
- 新增/覆盖复制：`C:\Users\24763\Desktop\YunXi Agent开发报告\2026-07-18-091620-yunxi-agent-v1-9-3-companion-ux-controls-development-report.md`
- 追加日志：`C:\Users\24763\Desktop\YunXi Agent开发日志.md`

文件路径：
- 项目开发目录：`D:\YunXi Agent`
- 项目报告目录：`D:\YunXi Agent\docs\reports`
- 项目日志：`D:\YunXi Agent\docs\development-log.md`
- 桌面报告目录：`C:\Users\24763\Desktop\YunXi Agent开发报告`
- 桌面开发日志：`C:\Users\24763\Desktop\YunXi Agent开发日志.md`

验证结果：
- 已完成开发报告撰写，并已同步到桌面开发报告目录。
- 尚未运行 `cargo fmt`、`cargo check`、`cargo test` 或 `cargo build`。
- 尚未执行编译产物清理、提交、推送或创建 Git tag。
- 后续仍需按固定流程完成实际开发、统一验证、清理和发布闭环后，才能宣称完成。

提交和推送状态：本次仅撰写开发报告并更新项目日志，未提交、未推送、未创建 Git tag。

署名：开发报告撰写者

## 2026-07-18 10:00:25 +08:00

工作目标：依据 v1.9.3 Companion UX & Controls 开发报告，在 `D:\YunXi Agent`
完成统一控制面、状态审查、清除确认和审计追溯能力，并保持 v1.9.2 companion
planner 的安全边界。

执行流程：
1. 读取 v1.9.3 开发报告与 v1.9.2 重新审核报告；确认 workspace 基线为
   `1467d3e`、v1.9.2 tag 存在且 `target` 初始不存在。
2. 使用仓库 `.codegraph` 通过 `codegraph.cmd explore` 定位 core config、
   CLI 命令/渲染、TUI app/render、runtime turn boundary、persona/memory/
   relationship 只读数据源和调用关系。
3. 只读抽取报告指定的 `D:\源码\memU\readme`、
   `D:\源码\nocturne_memory\frontend`、`D:\源码\QwenPaw\console` 控制
   语义，仅复刻状态可见、确认、刷新、清除和审计逻辑。
4. 新增 core 控制 facade 与 storage 控制账本；runtime 提供共享快照和陪伴
   历史记录；CLI/TUI 接入统一入口和显式清除确认。
5. 同步 README、extraction-status、persona-memory、v1.9.3 报告和本日志。

修改文件与路径：
- `D:\YunXi Agent\Cargo.toml`、`Cargo.lock`
- `D:\YunXi Agent\crates\yunxi-agent-core\src\control.rs`、`config.rs`、`lib.rs`
- `D:\YunXi Agent\crates\yunxi-agent-storage\src\lib.rs`
- `D:\YunXi Agent\crates\yunxi-agent-runtime\src\lib.rs` 与 runtime tests
- `D:\YunXi Agent\crates\yunxi-agent-persona\src\settings.rs`、`profile.rs`、`compiler.rs`
- `D:\YunXi Agent\crates\yunxi-agent-cli\src\main.rs`、`commands.rs`、`interactive.rs`、`render.rs`、`tui/mod.rs`
- `D:\YunXi Agent\crates\yunxi-agent-cli\tests\cli_tests.rs`
- `D:\YunXi Agent\crates\yunxi-agent-tui\src\app.rs`、`host.rs`、`render.rs`
- `D:\YunXi Agent\README.md`
- `D:\YunXi Agent\docs\extraction-status.md`
- `D:\YunXi Agent\docs\persona-memory.md`
- `D:\YunXi Agent\docs\reports\2026-07-18-091620-yunxi-agent-v1-9-3-companion-ux-controls-development-report.md`

验证结果：
- `cargo fmt --all`、`cargo fmt --all -- --check`、`cargo check --workspace`、
  `cargo test --workspace`、`cargo build --workspace`、
  `cargo build -p yunxi-agent-cli --release --bins` 全部通过。
- Release 冒烟确认默认 companion/cloud 关闭、开关持久化、clear 确认、
  relationship 只读、控制审计和 tool request 不自动执行；临时目录
  `D:\YunXi Agent\.tmp\v193-control-smoke` 已清理。
- `cargo clean` 已执行并清理 `D:\YunXi Agent\target`；尚未提交、创建
  v1.9.3 tag 或推送。

提交和推送状态：实现和文档已完成并通过验证；实现提交已创建，annotated
`v1.9.3` tag 已固定到实现提交，旧 tag 未改动。最后的 docs-only 发布状态
收尾提交和 GitHub non-force 推送待执行。

署名：开发者

## 2026-07-18 10:04:02 +08:00

工作目标：完成 v1.9.3 实现 tag 后的发布状态收尾，确保项目报告和项目日志
不再停留在“待提交/待清理”的初始撰写状态。

执行流程：
1. 提交 v1.9.3 实现与文档，创建 annotated `v1.9.3` tag。
2. 核对实现 tag 指向、旧 tag ref、`target` 清理状态和工作树内容。
3. 将最终发布状态写入本报告和项目日志；随后执行 docs-only 收尾提交和远程
   non-force 推送。

修改文件：
- `D:\YunXi Agent\docs\reports\2026-07-18-091620-yunxi-agent-v1-9-3-companion-ux-controls-development-report.md`
- `D:\YunXi Agent\docs\development-log.md`

验证结果：实现提交为 `3bcd02146f03c430574a8d55110894d5247546b7`；
`v1.9.3` annotated tag object 为 `8d036d5ddc202a2b10c4c8edd6523cc9425e5d01`，
解析到实现提交；`D:\YunXi Agent\target` 不存在。旧 tag 未删除、移动或重写。

提交和推送状态：本轮发布闭环包含 docs-only 收尾提交与 GitHub non-force
推送；最终远程状态以 Git 历史核验结果为准。

署名：开发者

## 2026-07-18 11:49:05 +08:00

工作目标：根据 `C:\Users\24763\Desktop\YunXi Agent审核报告\2026-07-18-104346-YunXi-Agent-v1.9.3-源码审核报告.md` 撰写 YunXi Agent v1.9.4 Evaluation Harness 开发报告，并同步项目内开发日志、桌面开发报告与桌面开发日志。

执行流程：
1. 读取 v1.9.3 源码审核报告，确认 v1.9.3 审核通过，可以进入 v1.9.4 开发报告撰写。
2. 核对当前项目 Git 状态：`master...origin/master`，工作树在报告撰写前干净。
3. 使用 CodeGraph 参考 persona tests、CLI tests、companion planner、memory record 等现有结构，确认 v1.9.4 应以 `evals/companion` 和现有测试入口为主要落点。
4. 按固定流程在开发报告前部写入 14 条硬性约束。
5. 围绕 v1.9.4 Evaluation Harness 撰写开发目标、范围边界、源码接入点、评测模型、参考源码抽取建议、实现顺序、验收标准、统一验证要求和文档同步要求。
6. 将开发报告保存到项目内报告目录，并复制到桌面开发报告目录。
7. 追加项目内开发日志和桌面开发日志，记录本次报告撰写与文件同步状态。

修改文件：
- 新增开发报告：`D:\YunXi Agent\docs\reports\2026-07-18-114905-yunxi-agent-v1-9-4-evaluation-harness-development-report.md`
- 追加项目日志：`D:\YunXi Agent\docs\development-log.md`
- 新增/覆盖复制：`C:\Users\24763\Desktop\YunXi Agent开发报告\2026-07-18-114905-yunxi-agent-v1-9-4-evaluation-harness-development-report.md`
- 追加桌面日志：`C:\Users\24763\Desktop\YunXi Agent开发日志.md`

文件路径：
- 项目开发目录：`D:\YunXi Agent`
- 项目报告目录：`D:\YunXi Agent\docs\reports`
- 项目日志：`D:\YunXi Agent\docs\development-log.md`
- 桌面报告目录：`C:\Users\24763\Desktop\YunXi Agent开发报告`
- 桌面开发日志：`C:\Users\24763\Desktop\YunXi Agent开发日志.md`

验证结果：
- 已完成开发报告撰写，并已同步到桌面开发报告目录。
- 尚未运行 `cargo fmt`、`cargo check`、`cargo test` 或 `cargo build`。
- 尚未执行编译产物清理、提交、推送或创建 Git tag。
- 后续仍需按固定流程完成实际开发、统一验证、清理和发布闭环后，才能宣称完成。

提交和推送状态：本次仅撰写开发报告并追加日志，未提交、未推送、未创建 Git tag。

署名：开发报告撰写者

## 2026-07-18 21:13:29 +08:00

工作目标：在唯一发布提交前完成 v2.0.1 日志时序封口，明确上方报告生成记录
属于开发前状态，实际开发、验证和清理结果以 21:09:56 记录及本版本开发报告
为准。

执行流程与修改路径：复核 `D:\YunXi Agent\docs\development-log.md`、
`docs\reports\2026-07-18-203508-yunxi-agent-v2-0-1-presentation-quiet-transcript-development-report.md`
和最终 Git diff；确认完整实现、验证证据、清理结果、文件路径和发布规则均已
落盘，且 `target` 与冒烟临时状态不存在。

验证结果：`git diff --check` 通过；v2.0.1 完整验证结果保持为 workspace
fmt/check/test/build/release 全部通过、release `yunxi 2.0.1`、Evaluation
Harness 31/31 通过、JSONL 一行、TUI 60 项回归通过。

提交和推送状态：准备创建唯一 v2.0.1 发布提交并立即创建 annotated tag，
随后 non-force 推送 master 与 tag；最终远程状态以 Git refs 为准，旧 tag 保持
不变，不追加同版本 docs-only/hotfix 提交。

署名：开发者

## 2026-07-18 21:09:56 +08:00

工作目标：依据 v2.0.1 呈现边界与安静 transcript 开发报告，建立 TUI
`AgentEvent -> TuiEvent` 唯一映射、稳定 cell/detail ID、安静默认对话和
debug/details 分层，并完成验证、清理与单提交发布准备。

执行流程：
1. 完整读取 v2.0.1 开发报告和 v2.0.0 基线审核报告，以 CodeGraph 核对
   CLI/TUI/core 调用链，并只读参考本机 Codex TUI 的 render、chatwidget、
   approval overlay 与 width 边界。
2. 新增 `presentation.rs`，将 runtime event 分类、稳定 ID、安全摘要、详情和
   Markdown stream state 集中到唯一 presentation 入口。
3. 将 event filter 收窄为纯 visibility gate；重构 chat/debug/host/render，
   使 transcript 仅消费已分类 cell，CLI TUI bridge 不再旁路 assistant。
4. 默认隐藏 thinking、memory/context、hidden prompt、provider wire、
   `arguments_json`、完整 stdout/stderr、stack 和未脱敏参数，并通过稳定
   details ID 保留脱敏诊断能力。
5. 增加 presentation、streaming、renderer、quiet transcript 与 details
   回归，升级 workspace/CLI/TUI/persona/eval/current docs 至 2.0.1。
6. 统一执行 fmt/check/test/build/release、二进制版本和 evaluation JSON/JSONL
   冒烟；经用户授权核验路径后执行清理。

修改文件与路径：
- `D:\YunXi Agent\Cargo.toml`、`Cargo.lock`、`README.md`
- `D:\YunXi Agent\crates\yunxi-agent-cli\src\main.rs`、`src\render.rs`、
  `src\tui\mod.rs`、`tests\cli_tests.rs`
- `D:\YunXi Agent\crates\yunxi-agent-tui\src\presentation.rs`、`app.rs`、
  `chat.rs`、`debug.rs`、`event_filter.rs`、`host.rs`、`lib.rs`、
  `output_summary.rs`、`render.rs`、`streaming.rs`、`timeline.rs`、
  `transcript_layout.rs`
- `D:\YunXi Agent\crates\yunxi-agent-eval\src\lib.rs`
- `D:\YunXi Agent\crates\yunxi-agent-persona\src\compiler.rs`、`src\profile.rs`、
  `tests\evaluation_regression_tests.rs`、`tests\persona_tests.rs`
- `D:\YunXi Agent\crates\yunxi-agent-runtime\tests\general_companion_tests.rs`
- `D:\YunXi Agent\evals\companion\README.md`
- `D:\YunXi Agent\docs\extraction-status.md`、`persona-memory.md`、
  `tui-presentation.md`、本版本开发报告及本日志。

验证结果：`cargo fmt --all`、fmt check、workspace check/test/build、release 双
二进制构建全部成功。TUI 60 项、CLI integration 44 项、CLI JSONL 10 项及其余
workspace 测试全部通过。release 返回 `yunxi 2.0.1`；Evaluation Harness
31/31 通过，golden 为 true，关键成功率均为 1.0，false positive/missed/
forbidden/proactive violation/tool bypass 均为 0；JSON 可解析，JSONL 恰好一行。
`cargo clean` 清除 9,556 个文件、2.9 GiB，`target` 和本次冒烟生成的临时
`.yunxi` 状态目录最终均不存在。

提交和推送状态：全部源码、测试、版本、文档、报告、状态和日志将形成唯一
v2.0.1 发布提交；提交后立即创建 annotated `v2.0.1` tag 并 non-force 推送
master 与新 tag。不会追加 v2.0.1 docs-only/hotfix 提交，旧 tag 不删除、不
移动、不重写；最终远程 master 以 Git 历史为准。

署名：开发者

## 2026-07-18 15:50:31 +08:00

工作目标：依据 v2.0.0 General Companion Agent 开发报告，闭合 persona、
memory、relationship、proactive companion、controls 与 evaluation 的本地运行链，
完成统一验证和编译产物清理，并准备不可变 tag 发布。

执行流程：
1. 以 CodeGraph 核对 turn boundary、persona recall、Relationship Graph Lite、
   companion planner、ControlSnapshot 与 CLI/eval 入口，确认默认运行链已经由
   YunXi Rust workspace 持有。
2. 新增 `GeneralCompanionSnapshot` 公共 runtime facade，并修正 active memory
   统计，使过期、失效、被替代记录不再作为活动事实计数。
3. 新增两会话总集成测试，验证语言偏好跨会话召回、新关系事实替代旧事实、
   append-only 历史保留、主动与云控制默认关闭以及关系控制只读。
4. 将 workspace/CLI/TUI/persona/eval 版本同步为 2.0.0，增加 CLI 发布门禁，
   保留 31 条场景和原 golden 阈值。
5. 同步 README、extraction status、persona-memory、evaluation README 与开发报告。
6. 一次性执行 fmt/check/test/build/release 和真实 CLI text/JSON/JSONL 冒烟。
7. 审计默认 CLI 依赖树无 Codex 运行依赖，随后核验路径并清理 target 与测试状态。

修改文件与路径：
- workspace/版本：`D:\YunXi Agent\Cargo.toml`、`D:\YunXi Agent\Cargo.lock`
- runtime：`D:\YunXi Agent\crates\yunxi-agent-runtime\src\lib.rs`、
  `D:\YunXi Agent\crates\yunxi-agent-runtime\src\general_companion.rs`、
  `D:\YunXi Agent\crates\yunxi-agent-runtime\tests\general_companion_tests.rs`
- CLI/eval/persona/TUI：`D:\YunXi Agent\crates` 下对应 v2 版本源文件与测试文件
- 文档：`D:\YunXi Agent\README.md`、`D:\YunXi Agent\docs\extraction-status.md`、
  `D:\YunXi Agent\docs\persona-memory.md`、`D:\YunXi Agent\evals\companion\README.md`、
  `D:\YunXi Agent\docs\reports\2026-07-18-144921-yunxi-agent-v2-0-0-general-companion-agent-development-report.md`

验证结果：fmt/check/test/build/release 全部通过；release 输出 `yunxi 2.0.0`；
离线 one-shot 正常；eval 31/31、golden 通过，五项比例指标均为 1.0，主动边界
违规和工具审批绕过均为 0；JSON 可解析，JSONL 恰好一行；CLI 正常依赖树
`codex-*`/`yunxi-agent-codex` 匹配为 0；`git diff --check` 通过。

清理结果：`cargo clean` 删除 9,540 个文件、约 2.9 GiB；项目 `target` 和
测试生成的 `crates\yunxi-agent-cli\.yunxi` 均不存在。

提交和推送状态：验证与清理已完成；实现提交、annotated `v2.0.0` tag、
GitHub non-force 推送及远程核验待发布收口后回写。

署名：开发者

## 2026-07-18 12:20:29 +08:00

工作目标：依据 v1.9.4 Evaluation Harness 开发报告，在 `D:\YunXi Agent`
完成可复现、可量化、默认离线的陪伴型 Agent 评测框架，并按固定流程完成
统一验证、编译产物清理和发布准备。

执行流程：
1. 读取开发报告与 v1.9.3 审核结论，保留报告中的 14 条硬性约束；核对
   `master`/`origin/master` 基线、v1.9.3 annotated tag 与干净的初始构建状态。
2. 使用仓库 `.codegraph` 定位 persona、memory、relationship、companion、
   CLI 与测试边界；只读参考 yantrikdb、mem0、cognee 的评测组织方式，使用
   Rust 复刻场景加载、规则 judge、指标聚合与 golden comparison。
3. 新增 `yunxi-agent-eval` crate 和 `evals/companion` 数据集，实现 31 条
   persona、memory、relationship、proactive 与 controls 场景。
4. 新增 `yunxi eval companion` 及 JSON/JSONL 输出，接入 CLI 集成测试和
   persona 自动化 regression tests；统一更新 workspace 版本为 `1.9.4`。
5. 同步 README、extraction-status、persona-memory、评测 README、本开发
   报告和本日志。
6. 完成统一格式化、检查、测试、构建、Release 冒烟与场景计数检查；确认
   路径后执行 `cargo clean`，并清理 CLI 测试生成的 crate 局部状态目录。

修改文件与路径：
- `D:\YunXi Agent\crates\yunxi-agent-eval\Cargo.toml`、`src\lib.rs`
- `D:\YunXi Agent\evals\companion\README.md`
- `D:\YunXi Agent\evals\companion\scenarios\*.jsonl`
- `D:\YunXi Agent\evals\companion\schemas\*.json`
- `D:\YunXi Agent\evals\companion\golden\companion_expected_metrics.json`
- `D:\YunXi Agent\crates\yunxi-agent-cli\Cargo.toml`、`src\main.rs`、
  `src\render.rs`、`tests\cli_tests.rs`
- `D:\YunXi Agent\crates\yunxi-agent-persona\src\compiler.rs`、`src\profile.rs`、
  `tests\persona_tests.rs`、`tests\evaluation_regression_tests.rs`
- `D:\YunXi Agent\crates\yunxi-agent-tui\src\app.rs`、`src\render.rs`
- `D:\YunXi Agent\Cargo.toml`、`Cargo.lock`、`README.md`
- `D:\YunXi Agent\docs\extraction-status.md`、`docs\persona-memory.md`
- `D:\YunXi Agent\docs\reports\2026-07-18-114905-yunxi-agent-v1-9-4-evaluation-harness-development-report.md`
- `D:\YunXi Agent\docs\development-log.md`

验证结果：
- `cargo fmt --all`、`cargo fmt --all -- --check`、`cargo check --workspace`、
  `cargo test --workspace`、`cargo build --workspace`、
  `cargo build -p yunxi-agent-cli --release --bins` 全部通过。
- Release 冒烟确认 `yunxi 1.9.4`，31/31 场景通过，golden 通过；persona、
  memory precision/recall、relationship、control 指标均为 `1.0`，误写、
  漏写、禁止写入、主动越界和工具审批绕过均为 `0`。
- JSON 可解析，JSONL 恰好一行；31 条场景分类计数为 6/8/6/6/5。
- `git diff --check` 通过；`cargo clean` 清理 10,430 个文件、约 3.1 GiB；
  `D:\YunXi Agent\target` 与测试生成的 crate 局部 `.yunxi` 均不存在。

提交和推送状态：实现、文档、验证与清理已完成；实现提交、annotated
`v1.9.4` tag 与 GitHub non-force 推送待执行，旧 tag 不会改动。

署名：开发者

## 2026-07-18 12:24:54 +08:00

工作目标：完成 v1.9.4 实现提交和 annotated tag，并更新项目文档中的发布
状态，准备执行 GitHub non-force 推送。

执行流程：
1. 暂存并检查全部 v1.9.4 源码、评测场景、测试、文档和日志变更；修正
   新报告头部 4 处 Markdown 尾随空格后，`git diff --cached --check` 通过。
2. 创建实现提交 `051002125158535023fa8bf7dbe41b430b398648`。
3. 创建 annotated `v1.9.4` tag，tag object 为
   `7b99ae5422cdf904aef9a5893ee7c1cfc601435c`，解析到实现提交；核对旧
   `v1.9.3`、`v1.9.2` tag 对象未变化。
4. 更新 v1.9.4 开发报告和本日志，准备 docs-only 发布状态收尾提交。

修改文件与路径：
- `D:\YunXi Agent\docs\reports\2026-07-18-114905-yunxi-agent-v1-9-4-evaluation-harness-development-report.md`
- `D:\YunXi Agent\docs\development-log.md`

验证结果：实现提交和 annotated tag 已存在，`v1.9.4^{commit}` 为
`051002125158535023fa8bf7dbe41b430b398648`；旧 tag 未删除、未移动、未重写；
`D:\YunXi Agent\target` 仍不存在。

提交和推送状态：实现提交与 tag 已完成；docs-only 收尾提交和 GitHub
non-force 推送待执行。

署名：开发者

## 2026-07-18 12:28:04 +08:00

工作目标：完成 v1.9.4 GitHub 发布闭环，并记录 API key non-force 推送的
最终结果。

执行流程：
1. 从 `C:\Users\24763\Desktop\GitHub apikey.txt` 读取 API key 到内存，
   未输出 key 内容。
2. 使用 GitHub Basic Authorization 执行非强制推送 `master` 与 `v1.9.4`。
3. GitHub 接受推送后，将远程状态回写到 v1.9.4 开发报告和本日志，并准备
   最终 docs-only 收尾提交。

修改文件与路径：
- `D:\YunXi Agent\docs\reports\2026-07-18-114905-yunxi-agent-v1-9-4-evaluation-harness-development-report.md`
- `D:\YunXi Agent\docs\development-log.md`

验证结果：GitHub 返回 `87ea400..09d32e1 master -> master` 和
`[new tag] v1.9.4 -> v1.9.4`；推送未使用 force。`v1.9.4` tag 仍解析到
实现提交 `051002125158535023fa8bf7dbe41b430b398648`。

提交和推送状态：远程推送已成功；本次文档状态收尾提交随后以 non-force
方式推送，旧 tag 不会改动。

署名：开发者

## 2026-07-18 14:49:21 +08:00

工作目标：根据 `C:\Users\24763\Desktop\YunXi Agent审核报告\2026-07-18-143223-YunXi-Agent-v1.9.4-源码审核报告.md` 撰写 YunXi Agent v2.0.0 General Companion Agent 开发报告，并同步项目内开发日志、桌面开发报告与桌面开发日志。

执行流程：
1. 读取 v1.9.4 源码审核报告，确认 v1.9.4 审核通过，可以进入 v2.0.0 开发报告撰写。
2. 核对当前项目 Git 状态：`master...origin/master`，工作树在报告撰写前干净。
3. 使用 CodeGraph 参考 `yunxi-agent-eval`、`ControlSnapshot`、control facade 等当前源码结构，确认 v2.0.0 应做全 workspace 总集成和发布闭环。
4. 按固定流程在开发报告前部写入 14 条硬性约束。
5. 围绕 v2.0.0 General Companion Agent 撰写开发目标、范围边界、源码接入范围、总集成模型、参考源码抽取建议、实现顺序、验收标准、统一验证要求和文档同步要求。
6. 将开发报告保存到项目内报告目录，并复制到桌面开发报告目录。
7. 追加项目内开发日志和桌面开发日志，记录本次报告撰写与文件同步状态。

修改文件：
- 新增开发报告：`D:\YunXi Agent\docs\reports\2026-07-18-144921-yunxi-agent-v2-0-0-general-companion-agent-development-report.md`
- 追加项目日志：`D:\YunXi Agent\docs\development-log.md`
- 新增复制：`C:\Users\24763\Desktop\YunXi Agent开发报告\2026-07-18-144921-yunxi-agent-v2-0-0-general-companion-agent-development-report.md`
- 追加桌面日志：`C:\Users\24763\Desktop\YunXi Agent开发日志.md`

文件路径：
- 项目开发目录：`D:\YunXi Agent`
- 项目报告目录：`D:\YunXi Agent\docs\reports`
- 项目日志：`D:\YunXi Agent\docs\development-log.md`
- 桌面报告目录：`C:\Users\24763\Desktop\YunXi Agent开发报告`
- 桌面开发日志：`C:\Users\24763\Desktop\YunXi Agent开发日志.md`

验证结果：
- 已完成开发报告撰写，并已同步到桌面开发报告目录。
- 尚未运行 `cargo fmt`、`cargo check`、`cargo test` 或 `cargo build`。
- 尚未执行编译产物清理、提交、推送或创建 Git tag。
- 后续仍需按固定流程完成实际开发、统一验证、清理和发布闭环后，才能宣称完成。

提交和推送状态：本次仅撰写开发报告并追加日志，未提交、未推送、未创建 Git tag。

署名：开发报告撰写者

## 2026-07-18 15:54:00 +08:00

工作目标：冻结 v2.0.0 已验证实现，创建新的 annotated tag，并在 GitHub
推送前记录可核验的提交与 tag 对象。

执行流程：
1. 暂存本次源码、测试、版本和文档变更，修正开发报告头部 4 处行尾空白。
2. `git diff --cached --check` 通过后创建实现提交。
3. 确认本地不存在同名 tag，创建 annotated `v2.0.0`，并核对对象类型为 tag。
4. 核对 `v1.9.4` tag 对象和解析提交未变化，旧 tag 未删除、移动或重写。

修改文件与路径：
- `D:\YunXi Agent\docs\reports\2026-07-18-144921-yunxi-agent-v2-0-0-general-companion-agent-development-report.md`
- `D:\YunXi Agent\docs\development-log.md`

验证结果：实现提交为 `4cf890b86889e72c47f0e56881152053c47d76ae`；
annotated `v2.0.0` tag object 为
`e8537bc89433512ce03eae71eafd85f571e88ec3`，解析到实现提交；旧 `v1.9.4`
tag object 仍为 `7b99ae5422cdf904aef9a5893ee7c1cfc601435c`，解析提交仍为
`051002125158535023fa8bf7dbe41b430b398648`。

提交和推送状态：实现提交和新 tag 已完成；本次发布状态文档将作为 docs-only
提交，随后与 `v2.0.0` 一起以 non-force 方式推送。

署名：开发者

## 2026-07-18 15:56:32 +08:00

工作目标：完成 v2.0.0 GitHub 发布与远程引用核验，并记录旧 tag 保持不变。

执行流程：
1. 从 `C:\Users\24763\Desktop\GitHub apikey.txt` 将 API key 读入内存，
   未打印、未写入仓库或 Git 配置。
2. 使用临时 Basic Authorization header 非强制推送 `master` 和 `v2.0.0`。
3. 通过 `git ls-remote` 核验远程 master、新 annotated tag、peeled commit、
   `v1.9.4` 和 `v1.9.3` 旧 tag 对象。
4. 将远程证据写回开发报告和本日志，准备最后一个 docs-only 状态提交。

修改文件与路径：
- `D:\YunXi Agent\docs\reports\2026-07-18-144921-yunxi-agent-v2-0-0-general-companion-agent-development-report.md`
- `D:\YunXi Agent\docs\development-log.md`

验证结果：GitHub 返回 `fdb1793..30de692 master -> master` 与
`[new tag] v2.0.0 -> v2.0.0`。远程 `v2.0.0` tag object 为
`e8537bc89433512ce03eae71eafd85f571e88ec3`，peeled commit 为
`4cf890b86889e72c47f0e56881152053c47d76ae`。远程 `v1.9.4` tag object/
peeled commit 仍为 `7b99ae5422cdf904aef9a5893ee7c1cfc601435c` /
`051002125158535023fa8bf7dbe41b430b398648`，`v1.9.3` tag object 仍为
`8d036d5ddc202a2b10c4c8edd6523cc9425e5d01`。

提交和推送状态：实现、tag 和首轮远程推送均成功；本条远程证据将作为
docs-only 收尾提交继续 non-force 推送，旧 tag 不改动。

署名：开发者

## 2026-07-18 16:06:53 +08:00

工作目标：按用户明确授权，将 PATH 中现有 YunXi 安装从 1.9.1 升级到
v2.0.0，并消除 C/D 两个现有安装目录的版本差异。

执行流程：
1. 核对 Machine/User/当前进程 PATH、官方安装脚本和两个现有安装副本。
2. 从已发布且工作树干净的 v2.0.0 源码重新构建 release 双二进制。
3. 使用 `scripts/install/install-yunxi.ps1` 覆盖 C 盘 User PATH 安装目录与
   当前进程仍命中的 D 盘安装目录；未新增重复 PATH 项。
4. 刷新 Machine+User PATH 后验证解析路径、四个二进制版本和 SHA-256。
5. 严格核验 `D:\YunXi Agent\target` 后执行 `cargo clean`。

修改文件与路径：
- 安装：`C:\Users\24763\AppData\Local\YunXi Agent\bin\yunxi.exe`
- 安装：`C:\Users\24763\AppData\Local\YunXi Agent\bin\yunxi-agent-cli.exe`
- 安装：`D:\Apps\YunXi Agent\bin\yunxi.exe`
- 安装：`D:\Apps\YunXi Agent\bin\yunxi-agent-cli.exe`
- 日志：`D:\YunXi Agent\docs\development-log.md`
- 桌面日志：`C:\Users\24763\Desktop\YunXi Agent开发日志.md`

验证结果：刷新持久 PATH 后 `yunxi` 解析到
`C:\Users\24763\AppData\Local\YunXi Agent\bin\yunxi.exe`，返回
`yunxi 2.0.0`。C/D 两处 `yunxi.exe` 与 `yunxi-agent-cli.exe` 均返回
2.0.0；两组 SHA-256 分别与 release 源文件完全一致。User PATH 中安装目录
计数为 1。`cargo clean` 删除 1,527 个文件、529.4 MiB，最终 `target` 不存在。

提交和推送状态：本次仅更新安装二进制与日志，不移动或重写 `v2.0.0` 及旧
tag；本日志变更将以 docs-only 提交并 non-force 推送。

署名：开发者

## 2026-07-18 16:08:56 +08:00

工作目标：核验 v2.0.0 PATH 升级日志已发布，并确认安装操作未改变 release tag。

执行流程：创建 docs-only 安装审计提交，使用指定 API key 临时认证执行
non-force master 推送，再以 `git ls-remote` 核验远程 master、annotated tag
object 与 peeled 实现提交。

修改文件与路径：`D:\YunXi Agent\docs\development-log.md`。

验证结果：远程 master 已接受 `c1b80e3..9b59165`；安装审计提交为
`9b5916523d8f7271ba5308bedd718e07a9c23f60`。远程 `v2.0.0` tag object 仍为
`e8537bc89433512ce03eae71eafd85f571e88ec3`，peeled 实现提交仍为
`4cf890b86889e72c47f0e56881152053c47d76ae`。

提交和推送状态：安装审计已提交并 non-force 推送；本条远程核验作为最后的
docs-only 状态提交推送，最终远程 master 以 Git 历史为准，所有 tag 保持不变。

署名：开发者

## 2026-07-18 20:35:08 +08:00

工作目标：根据 `C:\Users\24763\Desktop\YunXi Agent审核报告\2026-07-18-203016-YunXi-Agent-v2.0.0-基线源码审核报告.md` 撰写 YunXi Agent v2.0.1 呈现边界与安静对话基线开发报告，并同步项目内开发日志、桌面开发报告与桌面开发日志。

执行流程：
1. 读取 v2.0.0 基线源码审核报告，确认 v2.0.0 基线通过，可以进入 v2.0.1 开发报告撰写。
2. 核对当前项目 Git 状态：`master...origin/master`，工作树在报告撰写前干净。
3. 使用 CodeGraph 参考 TUI `streaming.rs`、CLI `tui/mod.rs`、core `stream.rs` 等当前源码结构，确认 v2.0.1 应以 TUI、事件呈现、流式输出和 details/debug 分层重构为核心。
4. 按固定流程在开发报告前部写入 14 条硬性约束。
5. 围绕 v2.0.1 呈现边界与安静对话基线撰写开发目标、TUI 与流式输出重构方向、源码接入点、普通 transcript 与 details 边界、参考源码抽取建议、实现顺序、验收标准、统一验证要求和文档同步要求。
6. 将开发报告保存到项目内报告目录，并复制到桌面开发报告目录。
7. 追加项目内开发日志和桌面开发日志，记录本次报告撰写与文件同步状态。

修改文件：
- 新增开发报告：`D:\YunXi Agent\docs\reports\2026-07-18-203508-yunxi-agent-v2-0-1-presentation-quiet-transcript-development-report.md`
- 追加项目日志：`D:\YunXi Agent\docs\development-log.md`
- 新增/覆盖复制：`C:\Users\24763\Desktop\YunXi Agent开发报告\2026-07-18-203508-yunxi-agent-v2-0-1-presentation-quiet-transcript-development-report.md`
- 追加桌面日志：`C:\Users\24763\Desktop\YunXi Agent开发日志.md`

文件路径：
- 项目开发目录：`D:\YunXi Agent`
- 项目报告目录：`D:\YunXi Agent\docs\reports`
- 项目日志：`D:\YunXi Agent\docs\development-log.md`
- 桌面报告目录：`C:\Users\24763\Desktop\YunXi Agent开发报告`
- 桌面开发日志：`C:\Users\24763\Desktop\YunXi Agent开发日志.md`

验证结果：
- 已完成开发报告撰写，并已同步到桌面开发报告目录。
- 尚未运行 `cargo fmt`、`cargo check`、`cargo test` 或 `cargo build`。
- 尚未执行编译产物清理、提交、推送或创建 Git tag。
- 后续仍需按固定流程完成实际开发、统一验证、清理和发布闭环后，才能宣称完成。

提交和推送状态：本次仅撰写开发报告并追加日志，未提交、未推送、未创建 Git tag。

署名：开发报告撰写者

## 2026-07-18 21:15:00 +08:00

工作目标：封口 v2.0.1 实际开发记录，并明确本条之前的 20:35:08 记录仅是
开发报告生成时的开发前状态。

执行流程与修改路径：复核 `D:\YunXi Agent\docs\development-log.md`、
`docs\reports\2026-07-18-203508-yunxi-agent-v2-0-1-presentation-quiet-transcript-development-report.md`
和最终 Git diff；实际源码、测试、版本、设计、状态、报告、验证与清理详情见
21:09:56 记录及本版本开发报告。

验证结果：fmt/check/test/build/release 全部通过，release 为 `yunxi 2.0.1`，
Evaluation Harness 31/31 通过且 JSONL 恰好一行，TUI 60 项回归通过；
`git diff --check` 通过，`target` 和本次冒烟临时状态均不存在。

提交和推送状态：准备创建唯一 v2.0.1 发布提交及 annotated tag，随后
non-force 推送 master 与 tag；旧 tag 保持不变，不追加同版本 docs-only 或
hotfix 提交，最终远程状态以 Git refs 为准。

署名：开发者

## 2026-07-19 07:18:26 +08:00

工作目标：根据 `C:\Users\24763\Desktop\YunXi Agent审核报告\2026-07-18-213015-YunXi-Agent-v2.0.1-源码与TUI视觉审核报告.md` 撰写 YunXi Agent v2.0.2 流式状态机与消息幂等化开发报告，并同步项目内开发日志、桌面开发报告与桌面开发日志。

执行流程：
1. 读取 v2.0.1 源码与 TUI 视觉审核报告，确认 v2.0.1 审核通过，可以进入 v2.0.2 开发报告撰写。
2. 核对当前项目 Git 状态：`master...origin/master`，工作树在报告撰写前干净。
3. 使用 CodeGraph 参考 TUI `streaming.rs`、`chat.rs`、`presentation.rs`、core `stream.rs` 等当前源码结构，确认 v2.0.2 应以 TurnId/StreamSession/timeline store 和 canonical assistant cell 为核心。
4. 按固定流程在开发报告前部写入 14 条硬性约束。
5. 围绕 v2.0.2 流式状态机与消息幂等化撰写开发目标、live provider 重复回复问题、源码接入点、必须覆盖场景、参考源码抽取建议、实现顺序、验收标准、统一验证要求和文档同步要求。
6. 将开发报告保存到项目内报告目录，并复制到桌面开发报告目录。
7. 追加项目内开发日志和桌面开发日志，记录本次报告撰写与文件同步状态。

修改文件：
- 新增开发报告：`D:\YunXi Agent\docs\reports\2026-07-19-071826-yunxi-agent-v2-0-2-streaming-state-machine-idempotency-development-report.md`
- 追加项目日志：`D:\YunXi Agent\docs\development-log.md`
- 新增/覆盖复制：`C:\Users\24763\Desktop\YunXi Agent开发报告\2026-07-19-071826-yunxi-agent-v2-0-2-streaming-state-machine-idempotency-development-report.md`
- 追加桌面日志：`C:\Users\24763\Desktop\YunXi Agent开发日志.md`

文件路径：
- 项目开发目录：`D:\YunXi Agent`
- 项目报告目录：`D:\YunXi Agent\docs\reports`
- 项目日志：`D:\YunXi Agent\docs\development-log.md`
- 桌面报告目录：`C:\Users\24763\Desktop\YunXi Agent开发报告`
- 桌面开发日志：`C:\Users\24763\Desktop\YunXi Agent开发日志.md`

验证结果：
- 已完成开发报告撰写，并已同步到桌面开发报告目录。
- 尚未运行 `cargo fmt`、`cargo check`、`cargo test` 或 `cargo build`。
- 尚未执行编译产物清理、提交、推送或创建 Git tag。
- 后续仍需按固定流程完成实际开发、统一验证、清理和发布闭环后，才能宣称完成。

提交和推送状态：本次仅撰写开发报告并追加日志，未提交、未推送、未创建 Git tag。

署名：开发报告撰写者

## 2026-07-19 09:18:11 +08:00

工作目标：严格依据 v2.0.2 开发报告完成流式状态机、消息幂等化、canonical assistant cell、版本升级、自动化与 live provider 验证，并按单一发布提交和不可移动 annotated tag 的约束准备发布。

执行流程：
1. 核对 `master`/`origin/master` 基线 `2b07a13ae49380b63aa966801e9a52c628107708`、开发前工作树、`v2.0.1` annotated tag 与旧 tag 保护状态。
2. 使用 `.codegraph` 定位 core、runtime、TUI、CLI 与测试边界；只读参考本机 Codex TUI rendering/wrapping/width 等实现思路。
3. 在 core 增加不改变 JSON 形状的流式身份元数据，在 runtime 将 provider thread/turn/item/source sequence/phase 映射为稳定 stream identity，并避免 delta 与 completed snapshot 重复累加。
4. 新增 `crates/yunxi-agent-tui/src/timeline_store.rs`，以 TurnId、StreamSessionId、SourceSequence、canonical cell 和显式状态机处理 started/delta/retry/final/finish/cancel。
5. 调整 TUI presentation、chat、app、host 与 render，使 final 原位更新、重复 final 幂等、cancel 后迟到事件失效、合法重复 delta 保留、工具边界不拆流，历史回看不被 final 抢回尾部。
6. 增加 core JSON 契约、runtime 协议映射、TUI 状态机、provider-shaped 单 canonical cell、历史回看，以及 CJK/假名/Emoji ZWJ/Markdown fence/长 token 等回归测试。
7. 将 workspace、CLI/TUI/persona/evaluation harness 与 README/docs 版本和说明同步为 `2.0.2`。
8. 统一执行 fmt、check、workspace tests、workspace build、release build、版本冒烟、Evaluation Harness、JSON/JSONL 与 diff 检查；修正首次测试暴露的两个 fixture 字段后完整重跑并通过。
9. 经用户授权，在独立临时 `YUNXI_HOME` 与空工作目录执行真实 DeepSeek 流式单轮复核，确认 transcript 只有一个 canonical assistant cell且无重复回答。
10. 经用户授权核验绝对路径后执行 `cargo clean`，清理测试状态和 live 临时目录；准备创建唯一 v2.0.2 发布提交并立即创建 annotated tag，再以 API key 临时认证 non-force 推送。

主要修改文件与路径：
- workspace 与锁文件：`D:\YunXi Agent\Cargo.toml`、`D:\YunXi Agent\Cargo.lock`
- 流式事件契约：`D:\YunXi Agent\crates\yunxi-agent-core\src\event.rs`
- provider/runtime 映射：`D:\YunXi Agent\crates\yunxi-agent-runtime\src\lib.rs`
- TUI 状态机：`D:\YunXi Agent\crates\yunxi-agent-tui\src\timeline_store.rs`
- TUI 接入：`D:\YunXi Agent\crates\yunxi-agent-tui\src\app.rs`、`chat.rs`、`host.rs`、`presentation.rs`、`render.rs`
- 兼容性与回归测试：core、runtime、TUI、CLI、storage、persona、Codex adapter 对应测试文件
- 文档：`D:\YunXi Agent\README.md`、`docs/extraction-status.md`、`docs/persona-memory.md`、`docs/tui-presentation.md`
- 开发报告：`D:\YunXi Agent\docs\reports\2026-07-19-071826-yunxi-agent-v2-0-2-streaming-state-machine-idempotency-development-report.md`
- 项目日志：`D:\YunXi Agent\docs\development-log.md`
- 桌面同步目标：`C:\Users\24763\Desktop\YunXi Agent开发报告\2026-07-19-071826-yunxi-agent-v2-0-2-streaming-state-machine-idempotency-development-report.md`、`C:\Users\24763\Desktop\YunXi Agent开发日志.md`

验证结果：
- `cargo fmt --all`、`cargo fmt --all -- --check`、`cargo check --workspace`、`cargo test --workspace`、`cargo build --workspace`、release build 全部通过。
- release 输出 `yunxi 2.0.2`；Evaluation Harness 31/31 通过、golden 通过，JSON 正常，JSONL 恰好一行。
- 自动化覆盖 delta/final/retry/cancel、重复 final、合法重复文本、CJK/假名/Emoji ZWJ/Markdown fence/4096 字符长 token、单 canonical cell 与历史回看不抢尾部。
- 隔离 live provider 使用 `deepseek-v4-flash` 回答固定提示，等待 15 秒后仍为 1 个 user cell + 1 个 canonical assistant cell，无重复回答。
- `git diff --check` 通过；`cargo clean` 移除 15,211 个文件、4.2 GiB，最终 `target`、CLI 测试状态目录与 live 临时目录均不存在。

提交和推送状态：所有 v2.0.2 实现与文档已准备纳入唯一发布提交；该提交后立即创建 annotated `v2.0.2` tag，并以 non-force 方式推送 `master` 和 tag。发布后不追加同版本 docs-only/hotfix 提交，最终远程状态以 Git refs 为准。

署名：开发者

## 2026-07-19 10:06:11 +08:00

工作目标：根据 `C:\Users\24763\Desktop\YunXi Agent审核报告\2026-07-19-094830-YunXi-Agent-v2.0.2-源码与TUI视觉审核报告.md` 撰写 YunXi Agent v2.0.2 流式状态机整改开发报告，并同步项目内开发日志、桌面开发报告与桌面开发日志。

执行流程：1. 读取 v2.0.2 源码与 TUI 视觉审核报告，确认审核结论为不通过，不可进入 v2.0.3 开发。2. 核对当前 Git 状态为 `master...origin/master`，并确认项目内最近报告为 v2.0.2 流式状态机与消息幂等化开发报告。3. 将审核报告中的四项硬性缺口转化为开发者整改要求：稳定 `event_id` 与 reliable/fallback sequence、Markdown/grapheme 安全提交边界、completed session 归档释放、真实 live provider 下 active stream `Ctrl+C` 取消。4. 在报告前部写入固定 14 条硬性约束，并明确现有 `v2.0.2` tag 不得移动或删除，修复发布版本号/tag 必须由用户确认。5. 在项目内新增整改开发报告。6. 追加项目内开发日志，将报告复制到桌面开发报告目录，并追加桌面开发日志。

修改文件：
- 新增开发报告：`D:\YunXi Agent\docs\reports\2026-07-19-100611-yunxi-agent-v2-0-2-streaming-remediation-development-report.md`
- 追加项目日志：`D:\YunXi Agent\docs\development-log.md`
- 新增/覆盖复制目标：`C:\Users\24763\Desktop\YunXi Agent开发报告\2026-07-19-100611-yunxi-agent-v2-0-2-streaming-remediation-development-report.md`
- 追加桌面日志目标：`C:\Users\24763\Desktop\YunXi Agent开发日志.md`

文件路径：
- 项目开发目录：`D:\YunXi Agent`
- 项目报告目录：`D:\YunXi Agent\docs\reports`
- 项目日志：`D:\YunXi Agent\docs\development-log.md`
- 桌面报告目录：`C:\Users\24763\Desktop\YunXi Agent开发报告`
- 桌面开发日志：`C:\Users\24763\Desktop\YunXi Agent开发日志.md`

验证结果：
- 已完成 v2.0.2 整改开发报告撰写，并同步到桌面开发报告目录。
- 本次仅生成文档和追加日志，未修改 Rust 源码。
- 尚未运行 `cargo fmt`、`cargo check`、`cargo test` 或 `cargo build`。
- 尚未执行编译产物清理、提交、推送或创建 Git tag。
- 后续必须先完成报告要求的整改、统一验证、真实 live provider 取消复核和重新审核；审核通过前不得宣称完成，也不得进入 v2.0.3。

提交和推送状态：本次仅撰写开发报告并追加日志，未提交、未推送、未创建 Git tag；现有 `v2.0.2` tag 不得移动或删除，修复发布版本号/tag 需用户确认。

署名：开发报告撰写者

## 2026-07-19 10:45:34 +08:00

工作目标：严格依据
`C:\Users\24763\Desktop\YunXi Agent开发报告\2026-07-19-100611-yunxi-agent-v2-0-2-streaming-remediation-development-report.md`
实施 v2.0.2 审核失败后的流式状态机整改候选，保留现有 `v2.0.2`
annotated tag，不自行确定修复版本号或 tag，并在真实 live 与重新审核通过前
不宣称整改完成。

执行流程：1. 读取整改报告和对应审核报告，核对 `master`、`origin/master`
与现有 `v2.0.2` tag 基线。2. 使用 CodeGraph 定位 provider 协议映射、runtime
流 future、TUI presentation/timeline、Markdown collector 与 raw-mode Ctrl+C
路径。3. 实施稳定 event ID、可靠/本地序列来源、event ID 幂等去重、debug
重复计数、有界最小归档与 active session 释放。4. 重构 Markdown 行、段落、
fence 与 grapheme 提交边界。5. 将 raw TUI Ctrl+C 接入共享取消 token，并让
runtime 在取消时立即丢弃 provider future。6. 增加 provider、runtime、TUI、
Unicode、回收与取消回归测试。7. 统一执行格式化、检查、全工作区测试、
workspace/release 构建、版本、Evaluation Harness、offline TUI 与 Git diff
检查。8. 尝试真实 DeepSeek live 门禁；因缺少知情后的外部请求授权，被安全
审查阻止，未绕过。9. 同步 README、状态文档、TUI 文档和本整改报告。

修改文件：
- workspace 依赖：`D:\YunXi Agent\Cargo.toml`、`D:\YunXi Agent\Cargo.lock`
- core：`D:\YunXi Agent\crates\yunxi-agent-core\src\cancellation.rs`、
  `event.rs`、`lib.rs`、`tests\event_tests.rs`
- protocol/provider：`D:\YunXi Agent\crates\yunxi-agent-protocol\src\lib.rs`、
  `D:\YunXi Agent\crates\yunxi-agent-provider\src\lib.rs`、
  `tests\provider_tests.rs`
- runtime：`D:\YunXi Agent\crates\yunxi-agent-runtime\src\lib.rs`、
  `tests\runtime_tests.rs`
- TUI：`D:\YunXi Agent\crates\yunxi-agent-tui\Cargo.toml`、
  `src\app.rs`、`host.rs`、`lib.rs`、`presentation.rs`、`streaming.rs`、
  `timeline_store.rs`
- CLI：`D:\YunXi Agent\crates\yunxi-agent-cli\src\interactive.rs`、
  `jsonl_redaction.rs`、`render.rs`、`tui\mod.rs`
- 文档：`D:\YunXi Agent\README.md`、`docs\extraction-status.md`、
  `docs\tui-presentation.md`、`docs\development-log.md`、
  `docs\reports\2026-07-19-100611-yunxi-agent-v2-0-2-streaming-remediation-development-report.md`

验证结果：
- fmt、fmt check、workspace check、workspace tests、workspace build、release
  build 全部通过；release 输出 `yunxi 2.0.2`。
- TUI 76 条、runtime 45 条、provider 44 条测试通过；取消测试验证 provider
  future 在首个 delta 后 100ms 门限内退出，保留 partial 且不接收 late delta。
- Evaluation Harness 31/31、golden 通过，JSON 可解析，JSONL 恰好一行，
  persona/memory/relationship/control 均 1.0，违规/绕过计数为 0。
- release offline TUI 普通输入、PageUp/PageDown 与空闲 Ctrl+C 退出通过。
- 经用户明确授权，在隔离空白目录执行真实 DeepSeek TUI：短流式只产生一个
  `LIVE_OK` assistant cell；长流活跃时 Ctrl+C 成功取消并保留 partial；同一
  TUI 随后接受下一次输入并返回 `NEXT_OK`，空闲 Ctrl+C 退出码为 0。
- 重新审核和发布门禁仍未完成。

清理状态：用户已授权。清理前 `target` 为 12,319 项、约 3.50 GB，CLI
测试状态为 3 项、1,248 字节；`cargo clean` 实际移除 11,373 个文件、
3.3 GiB。随后递归删除 CLI `.yunxi` 状态目录。最终 `target`、
`D:\YunXi Agent\crates\yunxi-agent-cli\.yunxi` 与
`C:\Users\24763\.codex\visualizations\2026\07\17\019f6dd0-e0ac-7fd3-bf9-90654d52170e\yunxi-live-smoke`
均不存在。

提交和推送状态：未提交、未推送、未创建新 tag、未安装整改候选。现有
annotated `v2.0.2` tag 保持不变。待重新审核通过，并由用户
确认修复版本号/tag 后再进入发布流程。

署名：开发者

## 2026-07-19 11:11:51 +08:00

工作目标：根据 `C:\Users\24763\Desktop\YunXi Agent审核报告\2026-07-19-110509-YunXi-Agent-v2.0.2-整改候选源码预审报告.md` 撰写 YunXi Agent v2.0.2 整改候选发布门禁开发报告，并同步项目内开发日志、桌面开发报告与桌面开发日志。

执行流程：1. 读取 v2.0.2 整改候选源码预审报告，确认结论为预审通过，但不构成正式版本审核通过，不允许进入 v2.0.3 开发。2. 核对当前 Git 状态为 `master...origin/master`，且存在多项未提交整改候选源码改动。3. 依据预审报告将下一阶段目标收敛为用户确认修复版本号/tag、统一验证、清理、单一发布提交、新 annotated tag、non-force 推送和正式复审准备。4. 在报告前部写入固定 14 条硬性约束，并明确旧 `v2.0.2` tag 不得移动、删除或覆盖。5. 在项目内新增发布门禁开发报告。6. 追加项目内开发日志，将报告复制到桌面开发报告目录，并追加桌面开发日志。

修改文件：
- 新增开发报告：`D:\YunXi Agent\docs\reports\2026-07-19-111151-yunxi-agent-v2-0-2-remediation-release-gate-development-report.md`
- 追加项目日志：`D:\YunXi Agent\docs\development-log.md`
- 新增/覆盖复制目标：`C:\Users\24763\Desktop\YunXi Agent开发报告\2026-07-19-111151-yunxi-agent-v2-0-2-remediation-release-gate-development-report.md`
- 追加桌面日志目标：`C:\Users\24763\Desktop\YunXi Agent开发日志.md`

文件路径：
- 项目开发目录：`D:\YunXi Agent`
- 项目报告目录：`D:\YunXi Agent\docs\reports`
- 项目日志：`D:\YunXi Agent\docs\development-log.md`
- 桌面报告目录：`C:\Users\24763\Desktop\YunXi Agent开发报告`
- 桌面开发日志：`C:\Users\24763\Desktop\YunXi Agent开发日志.md`

验证结果：
- 已完成 v2.0.2 整改候选发布门禁开发报告撰写，并同步到桌面开发报告目录。
- 本次仅生成文档和追加日志，未修改 Rust 源码。
- 尚未运行 `cargo fmt`、`cargo check`、`cargo test` 或 `cargo build`。
- 尚未执行编译产物清理、提交、推送或创建 Git tag。
- 后续必须先由用户确认修复版本号/tag，再统一验证、清理、发布并进入正式复审；正式审核通过前不得宣称完成，也不得进入 v2.0.3。

提交和推送状态：本次仅撰写开发报告并追加日志，未提交、未推送、未创建 Git tag；现有 `v2.0.2` tag 不得移动或删除。

署名：开发报告撰写者

## 2026-07-19 11:34:41 +08:00

工作目标：依据
`D:\YunXi Agent\docs\reports\2026-07-19-111151-yunxi-agent-v2-0-2-remediation-release-gate-development-report.md`
和用户确认，将流式状态机整改候选发布为 `2.0.2-hotfix.1` / annotated
`v2.0.2-hotfix.1`，严格保留旧 `v2.0.2` tag，并为独立正式复审提供固定发布对象。

执行流程：1. 核对 `master` 与 `origin/master` 基线均为
`ef7f43f97a783b3ee37d47ab034b709180d0c82e`，核对旧 `v2.0.2` tag object
`3f60445680211c27e1f6fe4e3b5c85a471fe5513` 和 peeled commit
`ef7f43f97a783b3ee37d47ab034b709180d0c82e`。2. 按用户确认将 workspace、
Cargo.lock、CLI/TUI、persona context、Evaluation Harness、测试期望、README
与状态文档同步到 `2.0.2-hotfix.1`。3. 统一执行 fmt、check、workspace tests、
workspace/release build、版本冒烟、Evaluation Harness 文本/JSON/JSONL、offline
TUI 与真实 DeepSeek live TUI。4. 用固定短提示核验唯一 `LIVE_OK` canonical cell；
用正常长篇 Rust 教程在 `[assistant*]` 活跃时触发 Ctrl+C，核验 turn 取消、partial
保留、程序不退出，并在同一进程得到 `NEXT_OK`。5. 执行 diff/status 检查。
6. 经用户授权和绝对路径校验，清理编译产物、CLI 测试状态与隔离 smoke 临时目录。
7. 将全部整改、版本和文档纳入单一 hotfix 发布提交；提交后创建新 annotated tag，
以 GitHub API key 临时认证 non-force 推送并核验新旧远程 refs。8. 发布后只通知正式
审核者复审，不宣称正式审核通过，不进入 v2.0.3。

主要修改文件与路径：

- 版本与锁文件：`D:\YunXi Agent\Cargo.toml`、`D:\YunXi Agent\Cargo.lock`。
- CLI：`D:\YunXi Agent\crates\yunxi-agent-cli\src`、
  `D:\YunXi Agent\crates\yunxi-agent-cli\tests\cli_tests.rs`。
- core/protocol/provider/runtime：`D:\YunXi Agent\crates\yunxi-agent-core`、
  `D:\YunXi Agent\crates\yunxi-agent-protocol`、
  `D:\YunXi Agent\crates\yunxi-agent-provider`、
  `D:\YunXi Agent\crates\yunxi-agent-runtime`。
- persona/evaluation：`D:\YunXi Agent\crates\yunxi-agent-persona`、
  `D:\YunXi Agent\crates\yunxi-agent-eval`。
- TUI：`D:\YunXi Agent\crates\yunxi-agent-tui`。
- 文档：`D:\YunXi Agent\README.md`、`D:\YunXi Agent\docs\extraction-status.md`、
  `D:\YunXi Agent\docs\tui-presentation.md`、
  `D:\YunXi Agent\docs\development-log.md`、
  `D:\YunXi Agent\docs\reports\2026-07-19-100611-yunxi-agent-v2-0-2-streaming-remediation-development-report.md`、
  `D:\YunXi Agent\docs\reports\2026-07-19-111151-yunxi-agent-v2-0-2-remediation-release-gate-development-report.md`。

验证结果：`cargo fmt --all`、fmt check、workspace check、workspace tests、
workspace build 与 release build 全部通过；CLI 集成 44、provider 44、runtime 45、
TUI 76 条关键测试全部通过；release 输出 `yunxi 2.0.2-hotfix.1`；Evaluation
Harness 31/31、golden 通过，JSON 可解析，JSONL 恰好一行，质量率均为 1.0，
违规与绕过计数为 0；offline TUI 和真实 DeepSeek live 短流/取消/下一轮/退出门禁通过；
`git diff --check` 通过，仅有 Windows 行尾提示。

清理状态：清理前 `D:\YunXi Agent\target` 有 10,503 项、文件合计
3,133,098,236 字节；`cargo clean` 移除 9,593 个文件、2.9 GiB。最终 `target`、
CLI `.yunxi` 与本次隔离 smoke 目录均不存在。

提交与推送边界：本记录随单一 `2.0.2-hotfix.1` 发布提交入库；随后立即创建
annotated `v2.0.2-hotfix.1` tag，并 non-force 推送 `master` 与新 tag。最终 commit、
tag object、远程 refs 与 API key 推送结果记录在 Git 历史和桌面最终开发日志中。
旧 `v2.0.2` tag 全程保持不变。候选发布不等于正式审核通过。

署名：开发者

## 2026-07-19 13:12:10 +08:00

工作目标：根据 `C:\Users\24763\Desktop\YunXi Agent审核报告\2026-07-19-122912-YunXi-Agent-v2.0.2-hotfix.1-源码与TUI视觉审核报告.md` 撰写 YunXi Agent v2.0.3 重绘调度、滚动与 Resize 稳定开发报告，并同步项目内开发日志、桌面开发报告与桌面开发日志。

执行流程：1. 读取 v2.0.2-hotfix.1 源码与 TUI 视觉审核报告，确认审核通过，可以进入 v2.0.3 开发。2. 核对当前 Git 状态为 `master...origin/master`，存在审核运行生成的未跟踪目录 `crates\yunxi-agent-cli\.yunxi`。3. 使用 CodeGraph MCP 参考 TUI `host.rs`、`app.rs`、`render.rs`、`presentation.rs` 等接入点，确认 v2.0.3 应聚焦重绘调度、viewport anchor、滚动和 resize 稳定。4. 按固定流程在开发报告前部写入 14 条硬性约束。5. 围绕 v2.0.3 范围撰写开发目标、非目标、源码接入点、Codex TUI 参考建议、推荐技术设计、测试要求、真实 TUI 验收和统一验证要求。6. 在项目内新增开发报告。7. 追加项目内开发日志，将报告复制到桌面开发报告目录，并追加桌面开发日志。

修改文件：
- 新增开发报告：`D:\YunXi Agent\docs\reports\2026-07-19-131210-yunxi-agent-v2-0-3-redraw-scroll-resize-development-report.md`
- 追加项目日志：`D:\YunXi Agent\docs\development-log.md`
- 新增/覆盖复制目标：`C:\Users\24763\Desktop\YunXi Agent开发报告\2026-07-19-131210-yunxi-agent-v2-0-3-redraw-scroll-resize-development-report.md`
- 追加桌面日志目标：`C:\Users\24763\Desktop\YunXi Agent开发日志.md`

文件路径：
- 项目开发目录：`D:\YunXi Agent`
- 项目报告目录：`D:\YunXi Agent\docs\reports`
- 项目日志：`D:\YunXi Agent\docs\development-log.md`
- 桌面报告目录：`C:\Users\24763\Desktop\YunXi Agent开发报告`
- 桌面开发日志：`C:\Users\24763\Desktop\YunXi Agent开发日志.md`

验证结果：
- 已完成 v2.0.3 重绘调度、滚动与 Resize 稳定开发报告撰写，并同步到桌面开发报告目录。
- 本次仅生成文档和追加日志，未修改 Rust 源码。
- 尚未运行 `cargo fmt`、`cargo check`、`cargo test` 或 `cargo build`。
- 尚未执行编译产物清理、提交、推送或创建 Git tag。
- 当前未跟踪目录 `D:\YunXi Agent\crates\yunxi-agent-cli\.yunxi` 未被读取或清理；如需清理必须先取得用户确认。
- 后续必须按报告完成 v2.0.3 开发、统一验证、真实 TUI 复核、清理和发布；验证通过前不得宣称完成。

提交和推送状态：本次仅撰写开发报告并追加日志，未提交、未推送、未创建 Git tag；旧 `v2.0.2` 与 `v2.0.2-hotfix.1` tag 均不得移动、删除或覆盖。

署名：开发报告撰写者

## 2026-07-19 14:07:47 +08:00

工作目标：严格依据 v2.0.3 开发报告实施原因感知重绘调度、稳定 transcript
cell/line viewport anchor、resize 安全布局与 Unicode grapheme-safe wrapping，完成
统一验证、真实 offline/live TUI 验收、清理和新版本发布准备。

执行流程：1. 完整读取开发报告和 v2.0.2-hotfix.1 审核报告，锁定 v2.0.3 范围和
旧 tag 不可移动边界。2. 使用 CodeGraph 定位 host tick/draw、app viewport、wrapped
transcript、layout/render 与 CLI TUI bridge。3. 实现 RedrawScheduler 三档优先级和
九类 redraw reason。4. 建立 FollowTail/Pinned/NewOutputBelow cell-line anchor，并把
Host、scrollbar、renderer 接到同一 WrappedTranscript。5. 加固 grapheme wrapping、
极小 terminal layout、composer cursor 和真实 footer 状态。6. 升级 workspace、CLI、
TUI、persona、runtime、evaluation 与文档版本到 2.0.3。7. 统一运行 fmt、check、
workspace tests/build、release build、版本、Evaluation Harness、JSON/JSONL、offline
及经用户授权的 DeepSeek live TUI 验收。8. 核验路径并经用户授权清理 target 与审计
.yunxi 目录。

修改文件：
- 核心实现：`D:\YunXi Agent\crates\yunxi-agent-tui\src\frame.rs`、`viewport.rs`、
  `transcript_layout.rs`、`host.rs`、`app.rs`、`layout.rs`、`render.rs`、`chat.rs`。
- 版本与测试：`D:\YunXi Agent\Cargo.toml`、`Cargo.lock`、CLI/TUI/persona/runtime/
  evaluation 对应源码和测试文件。
- 文档：`D:\YunXi Agent\README.md`、`docs\extraction-status.md`、
  `docs\tui-presentation.md`、`docs\persona-memory.md`、`docs\development-log.md`、
  `docs\reports\2026-07-19-131210-yunxi-agent-v2-0-3-redraw-scroll-resize-development-report.md`。

验证结果：报告规定的 fmt、fmt check、workspace check/test/build、release build 全部
通过；TUI 82 项通过；release 为 `yunxi 2.0.3`；Evaluation Harness 31/31、golden
通过、JSON 正常、JSONL 一行。offline PTY 验证滚动/状态/End；DeepSeek live 验证
短流 canonical cell、长流 pinned history、100x30 到 58x18 动态 resize、active-turn
Ctrl+C partial 保留及下一轮 `RESIZE_NEXT_OK`。`git diff --check` 通过。

清理结果：经路径核验和用户授权，`cargo clean` 移除 17,490 个文件、约 4.9 GiB；
`D:\YunXi Agent\target` 和审计生成的
`D:\YunXi Agent\crates\yunxi-agent-cli\.yunxi` 最终均不存在。

提交和推送状态：本记录随唯一 v2.0.3 发布提交入库，随后立即创建 annotated
`v2.0.3` tag 并使用指定 API key non-force 推送 master 与新 tag；最终 commit、tag
object 和远程 refs 记录在 Git 历史及桌面最终开发日志中。旧 `v2.0.2` 与
`v2.0.2-hotfix.1` tag 全程保持不变。

署名：开发者

## 2026-07-19 15:10:23 +08:00

工作目标：根据 `C:\Users\24763\Desktop\YunXi Agent审核报告\2026-07-19-150645-YunXi-Agent-v2.0.3-源码与TUI视觉审核报告.md` 撰写 YunXi Agent v2.0.3 重绘调度与 TUI Snapshot 整改开发报告，并同步项目内开发日志、桌面开发报告与桌面开发日志。

执行流程：1. 读取 v2.0.3 源码与 TUI 视觉审核报告，确认审核不通过，不可进入 v2.0.4 的既定功能开发。2. 核对当前 Git 状态为 `master...origin/master`，存在在线审核运行生成的未跟踪目录 `crates\yunxi-agent-cli\.yunxi`。3. 使用 CodeGraph MCP 参考 `frame.rs`、`host.rs`、`render.rs` 等接入点，确认缺口集中在 1,000 高频 delta draw/FPS 硬证明、30 FPS 严格边界、80x24 与 120x40 完整 TUI snapshot、真实 online TUI 复测证据。4. 按固定流程在开发报告前部写入 14 条硬性约束。5. 围绕 v2.0.3 整改范围撰写开发目标、版本边界、源码接入点、Codex TUI 参考建议、推荐执行顺序、测试要求、真实 TUI 复核要求和统一验证要求。6. 在项目内新增整改开发报告。7. 追加项目内开发日志，将报告复制到桌面开发报告目录，并追加桌面开发日志。

修改文件：
- 新增开发报告：`D:\YunXi Agent\docs\reports\2026-07-19-151023-yunxi-agent-v2-0-3-redraw-scroll-resize-remediation-development-report.md`
- 追加项目日志：`D:\YunXi Agent\docs\development-log.md`
- 新增/覆盖复制目标：`C:\Users\24763\Desktop\YunXi Agent开发报告\2026-07-19-151023-yunxi-agent-v2-0-3-redraw-scroll-resize-remediation-development-report.md`
- 追加桌面日志目标：`C:\Users\24763\Desktop\YunXi Agent开发日志.md`

文件路径：
- 项目开发目录：`D:\YunXi Agent`
- 项目报告目录：`D:\YunXi Agent\docs\reports`
- 项目日志：`D:\YunXi Agent\docs\development-log.md`
- 桌面报告目录：`C:\Users\24763\Desktop\YunXi Agent开发报告`
- 桌面开发日志：`C:\Users\24763\Desktop\YunXi Agent开发日志.md`

验证结果：
- 已完成 v2.0.3 重绘调度与 TUI Snapshot 整改开发报告撰写，并同步到桌面开发报告目录。
- 本次仅生成文档和追加日志，未修改 Rust 源码。
- 尚未运行 `cargo fmt`、`cargo check`、`cargo test` 或 `cargo build`。
- 尚未执行编译产物清理、提交、推送或创建 Git tag。
- 当前未跟踪目录 `D:\YunXi Agent\crates\yunxi-agent-cli\.yunxi` 未被读取或清理；如需清理必须先取得用户确认。
- 后续必须按报告先完成 v2.0.3 验收整改和重新审核；审核通过前不得宣称完成，也不得进入 v2.0.4。

提交和推送状态：本次仅撰写开发报告并追加日志，未提交、未推送、未创建 Git tag；已发布 `v2.0.3` tag 不得移动、删除或覆盖，整改发布编号与 tag 策略需用户确认。

署名：开发报告撰写者

## 2026-07-19 16:30:48 +08:00

工作目标：严格依据
`C:\Users\24763\Desktop\YunXi Agent开发报告\2026-07-19-151023-yunxi-agent-v2-0-3-redraw-scroll-resize-remediation-development-report.md`
完成 `2.0.3-hotfix.1` 重绘审计整改，补齐严格 30 FPS 的 1,000-delta draw 证明、
80x24 与 120x40 完整 frame snapshot、真实 offline/live TUI 复核，并在不移动任何旧
tag 的前提下准备 annotated `v2.0.3-hotfix.1` 发布。

执行流程：1. 完整读取整改报告并锁定 14 条硬性约束、通用陪伴 Agent 目标和
`v2.0.4` 禁入边界。2. 使用 CodeGraph 复核 `RedrawScheduler`、生产 draw 记录路径、
layout 与 full-frame snapshot 接入点。3. 将最小帧间隔改为 33,334 微秒，增加生产
draw count 和手动推进 `Instant` 的 1,000-delta 确定性测试。4. 扩展 immediate 与
next-frame 回归。5. 新增 80x24、120x40 完整 snapshot 与固定区域、宽度、scrollbar
边界断言。6. 同步 workspace、CLI、TUI、persona、runtime、evaluation、README 和
状态文档到 `2.0.3-hotfix.1`。7. 集中运行 fmt、check、workspace tests/build、
release build、版本、Evaluation Harness、golden、JSON/JSONL、offline one-shot 与
offline PTY。8. 使用 DeepSeek `deepseek-v4-flash` 在真实 ConPTY 中复核短流 canonical
cell、滚轮、PgUp/PgDown、动态 resize、Home/End、active-turn Ctrl+C、partial 保留、
下一轮与退出。9. 将不含凭据的 TUI 证据固化到项目报告目录。10. 执行 diff/status
检查并准备精确清理清单。

主要修改文件与路径：

- 调度与测试：`D:\YunXi Agent\crates\yunxi-agent-tui\src\frame.rs`。
- 布局与渲染：`D:\YunXi Agent\crates\yunxi-agent-tui\src\layout.rs`、
  `D:\YunXi Agent\crates\yunxi-agent-tui\src\render.rs`、
  `D:\YunXi Agent\crates\yunxi-agent-tui\src\app.rs`。
- 完整快照：`D:\YunXi Agent\crates\yunxi-agent-tui\src\snapshots\full_frame_80x24.txt`、
  `D:\YunXi Agent\crates\yunxi-agent-tui\src\snapshots\full_frame_120x40.txt`。
- 版本与测试：`D:\YunXi Agent\Cargo.toml`、`D:\YunXi Agent\Cargo.lock`、CLI、
  persona、runtime、evaluation 对应源码和测试文件。
- 文档：`D:\YunXi Agent\README.md`、`D:\YunXi Agent\docs\extraction-status.md`、
  `D:\YunXi Agent\docs\tui-presentation.md`、`D:\YunXi Agent\docs\persona-memory.md`、
  `D:\YunXi Agent\docs\development-log.md`、原 v2.0.3 开发报告、本整改报告和
  `D:\YunXi Agent\docs\reports\evidence\2026-07-19-v2-0-3-hotfix-1-tui-evidence.md`。

验证结果：`cargo fmt --all`、fmt check、workspace check/test/build、release build
全部通过；TUI 87/87，CLI integration 44、provider 44、runtime 45；release 输出
`yunxi 2.0.3-hotfix.1`。Evaluation Harness 31/31、golden true，JSON 可解析、JSONL
恰好一行，各质量率 1.0，违规和审批绕过为 0。offline one-shot、offline PTY 80x24
和 DeepSeek live TUI 均通过，live 会话 10 个检查点、39,896 bytes、退出码 0；普通
屏幕未发现凭据、协议、thinking、工具参数、memory/context 或内部错误栈泄漏。
`git diff --check` 通过，仅有 Windows 行尾提示。

清理状态：2026-07-19 16:50:18 +08:00 经用户明确授权完成。清理前 `target` 有
16,570 个文件、5,043,980,114 字节，`cargo clean` 报告移除 16,570 个文件、
4.7 GiB；CLI `.yunxi` 有 1 个文件、832 字节；9 个实际存在的精确 `.tmp` 目标
共 59 个文件、9,147,991 字节。一个 `conpty.node` 被本轮证据 Node PID 26592 占用，
经模块路径和启动时间核验后只终止该 PID，再删除残留目录。最终全部清理目标均不
存在，没有删除整个 `.tmp`，没有终止其他 Node/CodeGraph 进程。

提交和推送状态：尚未提交、尚未创建 `v2.0.3-hotfix.1` tag、尚未推送。原
`v2.0.3`、`v2.0.2-hotfix.1`、`v2.0.2` tag 保持不变；本记录将在单一 hotfix 发布
提交中入库，再创建新 annotated tag 并使用指定 GitHub API
key non-force 推送和核验远程 refs。候选发布不等于独立重新审核通过。

署名：开发者
