# YunXi Agent v1.8.6 审核整改开发报告

撰写时间：2026-07-16 21:45:12 +08:00
审核报告：`C:\Users\24763\Desktop\YunXi Agent审核报告\2026-07-16-211045-YunXi-Agent-v1.8.6-源码审核报告.md`
开发目录：`D:\YunXi Agent`
项目内报告位置：`D:\YunXi Agent\docs\reports\2026-07-16-214512-yunxi-agent-v1-8-6-audit-remediation-development-report.md`
桌面报告位置：`C:\Users\24763\Desktop\YunXi Agent开发报告\2026-07-16-214512-yunxi-agent-v1-8-6-audit-remediation-development-report.md`

本报告面向后续开发者，用于把 v1.8.6 源码审核报告中的阻断项转化为可执行的整改开发任务。它不是完成证明；验证通过、文档写回、工作区收敛、提交/tag 状态明确之前，不得宣称 v1.8.6 完成。

## 硬性约束

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
13. 不可因为过长的上下文忽略硬性要求，如果察觉到硬性要求的效力已经减弱，则必须汇报当前情况。
14. 工作尽量固定在项目文件夹，涉及到其他文件夹的操作，要明确告知用户，尤其是 C 盘的用户目录。

补充说明：开发报告和开发日志是用户明确指定的例外输出位置，可写入 `C:\Users\24763\Desktop\YunXi Agent开发报告\` 与 `C:\Users\24763\Desktop\YunXi Agent开发日志.md`；除此之外，开发操作应固定在 `D:\YunXi Agent` 及其工作树内。

## 审核结论转开发目标

审核结论为：v1.8.6 审核不通过，不可进入 v1.8.7 或后续版本开发。

当前源码已具备 v1.8.6 的主要构造，包括 JSON/JSONL 输出脱敏、初步 persona、初步 memory、本地 JSONL 记忆存储、状态流转、召回和 CLI 审查入口。阻断点不在新增大功能，而在闭环：

- P0：`docs/extraction-status.md` 中 v1.8.6 统一验证仍处于 planned/deferred 状态。
- P0：工作区存在未提交修改和未跟踪报告文件，版本成果不可追溯。
- P1：`docs/extraction-status.md` 与 v1.8.6 development report 尚未记录实际验证完成态。

本轮整改目标是完成 v1.8.6 当前基线收敛：运行统一验证、修复验证中暴露的窄问题、写回完成态文档、收敛工作区、明确提交/推送/tag 状态，然后再申请审核。

## 源码参考边界

审核报告明确：总纲图没有为 v1.8.6 当前基线单独指定外部参考源码路径。因此本轮不需要引入新的外部源码，也不应提前实现 v1.8.7 Persona Context Blocks、v1.8.8 Memory Schema v3 或更远版本能力。

本轮实际参考 YunXi 当前源码：

- `D:\YunXi Agent\Cargo.toml`
- `D:\YunXi Agent\crates\yunxi-agent-cli\src\main.rs`
- `D:\YunXi Agent\crates\yunxi-agent-cli\src\jsonl_redaction.rs`
- `D:\YunXi Agent\crates\yunxi-agent-cli\src\render.rs`
- `D:\YunXi Agent\crates\yunxi-agent-cli\tests\cli_tests.rs`
- `D:\YunXi Agent\crates\yunxi-agent-persona\src\profile.rs`
- `D:\YunXi Agent\crates\yunxi-agent-persona\src\compiler.rs`
- `D:\YunXi Agent\crates\yunxi-agent-persona\src\memory.rs`
- `D:\YunXi Agent\crates\yunxi-agent-persona\src\migration.rs`
- `D:\YunXi Agent\crates\yunxi-agent-storage\src\lib.rs`
- `D:\YunXi Agent\crates\yunxi-agent-runtime\src\lib.rs`
- `D:\YunXi Agent\docs\persona-memory.md`
- `D:\YunXi Agent\docs\extraction-status.md`
- `D:\YunXi Agent\docs\reports\2026-07-14-yunxi-agent-v1-8-6-json-output-redaction-development-report.md`

若验证中发现需要参考非 Rust 源码，只能抽取其行为逻辑并在 YunXi Rust crate 内复刻，不得把非 Rust 运行时作为默认依赖引入。

## 技术整改要求

### 1. 保持 v1.8.6 范围

开发者必须把版本范围锁定为：

```text
v1.8.6 当前基线
JSON 输出脱敏 + 初步 persona/memory
```

不要在本轮新增 SQLite、vector search、graph memory、relationship state machine、proactive trigger、TUI memory inspector、云任务、marketplace、SDK packaging 或后续版本 persona/memory 结构化能力。

### 2. 复核 JSON/JSONL 脱敏边界

重点复核 `crates\yunxi-agent-cli\src\main.rs` 中 `print_run_result`：

- `--jsonl` 分支应在 `to_jsonl_line` 前调用结构化脱敏。
- `--json` 分支应在 `serde_json::to_string_pretty` 前对 `AgentRunResult` 做结构化脱敏。
- 普通文本输出不应被误改为机器可读脱敏逻辑。

重点复核 `crates\yunxi-agent-cli\src\jsonl_redaction.rs`：

- 将 `AgentRunResult`、`AgentEvent`、`RuntimeEvent`、嵌套 `serde_json::Value`、`BTreeMap<String, String>` 等机器可读输出统一视为脱敏边界。
- 继续复用 `yunxi_agent_provider::redact_sensitive_text`，不要引入第二套 secret detector。
- 不允许在 JSON 字符串序列化后做整段替换；必须优先处理 typed data。
- memory discard policy 与 CLI 输出脱敏要保持独立，测试也要能分别证明二者仍然有效。

### 3. 复核初步 persona/memory 能力

本轮只需保证当前基线存在并可被 runtime 使用：

- `crates\yunxi-agent-persona\src\profile.rs` 保留 `PersonaProfile`、`PersonaLayers`、`HumanProfile`、`RelationshipState` 和内置 `yunxi_companion_strong()`。
- `crates\yunxi-agent-persona\src\compiler.rs` 保留 persona、human、relationship、memory 到 prompt context 的编译路径。
- `crates\yunxi-agent-runtime\src\lib.rs` 保留 `build_persona_turn_context` 对 settings、profile、memory recall 的加载和注入。
- `crates\yunxi-agent-persona\src\memory.rs` 保留 schema v2、scope、kind、status、recall request/result。
- `crates\yunxi-agent-storage\src\lib.rs` 保留 `FilePersonaMemoryStore` 的 append-only JSONL、迁移、latest records 和 dedup collapse 行为。

不要为了通过本次审核扩展到 schema v3、L0-L3 分层、关系图谱或主动陪伴触发器。

### 4. 完成统一验证

开发者应在一批源码和文档整改完成后统一执行验证，不要在每个小修改后频繁测试。审核报告列出的验证项必须全部闭环：

```powershell
cargo fmt
cargo fmt --check
cargo test -p yunxi-agent-cli json --test cli_tests
cargo test -p yunxi-agent-cli jsonl --test jsonl_tests
cargo test -p yunxi-agent-persona -p yunxi-agent-storage -p yunxi-agent-runtime -p yunxi-agent-cli -p yunxi-agent-tui
cargo test
cargo check --workspace
cargo build -p yunxi-agent-cli --release --bins
target\release\yunxi.exe --version
target\release\yunxi-agent-cli.exe --version
git diff --check
codegraph sync .
codegraph status .
```

还必须补充隔离 `YUNXI_HOME` 的黑盒 JSON 与 JSONL 隐私检查、owned-source secret scan、安装 helper 与 PATH smoke。涉及安装、PATH 或系统配置前必须先获得用户确认。

如果执行 `cargo clean` 清理构建产物，因其会递归清理 `target`，必须先向用户明确说明并取得确认。

### 5. 文档写回完成态

验证通过后，必须把实际结果写回：

- `D:\YunXi Agent\docs\extraction-status.md`
- `D:\YunXi Agent\docs\reports\2026-07-14-yunxi-agent-v1-8-6-json-output-redaction-development-report.md`

写回内容至少包含：

- 实际执行的命令。
- 每条验证的 pass/fail 结果。
- release binary version 输出。
- 黑盒 JSON/JSONL 隐私检查结论。
- secret scan 结论。
- CodeGraph sync/status 结论。
- 构建产物清理结果。
- 提交、推送、tag 状态。

同时检查 `README.md`、`docs\persona-memory.md`、设计文档、索引和状态文档中的版本说明是否仍与 `Cargo.toml` 的 `1.8.6` 不一致；发现不一致必须同步修正。

### 6. 工作区收敛与 Git tag

审核报告记录的工作区未收敛状态必须处理。开发者应：

1. 用 `git status --short` 明确所有改动。
2. 区分 v1.8.6 相关改动与用户/其他任务改动，不得擅自回退不属于本轮的内容。
3. 将 v1.8.6 改动、文档写回和报告记录收敛到可追溯提交。
4. 推送状态必须记录清楚：未推送、已推送或推送失败原因。
5. v1.8.6 阶段完成后必须创建新的 Git tag；如果远端已有对应 tag，必须先核对目标，不得删除、移动或覆盖旧 tag。

## 推荐执行顺序

1. 固定范围：确认本轮仍是 v1.8.6 审核整改，不进入 v1.8.7。
2. 盘点当前工作区：记录现有修改、未跟踪文件和可能属于用户的改动。
3. 按源码参考边界复核 JSON/JSONL、persona、memory、runtime、storage、CLI 入口。
4. 如验证暴露问题，只做窄修复，优先修复机器可读输出脱敏和文档完成态。
5. 完成一批源码/文档构建后统一验证。
6. 验证通过后写回 `docs/extraction-status.md` 和 v1.8.6 development report。
7. 在用户确认后清理编译中间产物。
8. 收敛工作区，提交、推送并创建/确认 v1.8.6 Git tag。
9. 追加开发日志，日志结尾署名为开发者。
10. 再次提交审核。

## 本报告生成状态

本次仅完成审核整改开发报告撰写和落盘，没有运行构建、测试、安装、清理、提交、推送或 tag 操作。因此本报告不能作为 v1.8.6 完成证明。

报告撰写者：开发报告撰写者
