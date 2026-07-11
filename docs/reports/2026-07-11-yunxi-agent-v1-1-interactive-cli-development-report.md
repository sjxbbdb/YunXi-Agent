# YunXi Agent v1.1 Interactive CLI Development Report

生成时间：2026-07-11 10:54:13 +08:00

## 核心结论

YunXi Agent v1.0 已经完成终端命令封装，并且默认运行路径保持 YunXi-owned
runtime，不依赖上游 Codex runtime、`vendor/codex-rs`、`codex-*` 或
`yunxi-agent-codex`。但 v1.0 的产品形态仍是 headless one-shot CLI：

- `yunxi --version` 可以输出 `yunxi 1.0.0`。
- `yunxi "你的任务"` 可以执行一次性 prompt。
- 直接运行 `yunxi` 会报 `a prompt is required`。
- 默认非 live provider 仍会输出静态离线响应，例如
  `YunXi autonomous runtime accepted prompt: ...`。

这就是当前不能像 Codex CLI 端一样持续互动回答的直接原因：v1.0 封装的是已经自主化的
Agent runtime 和一次性 CLI 入口，还没有实现交互式 terminal host、REPL 会话循环、实时输出
renderer、Ctrl+C 取消、slash command、跨轮 history 注入和交互式 approval UX。

v1.1 的目标是补齐这个产品层缺口：让 `yunxi` 在无 prompt 时进入 Codex CLI 风格的交互式终端，
在有 prompt 时继续保持 one-shot 模式；同时把当前已经具备的 provider、session、stream、
tool、storage、MCP、multi-agent、JSONL 能力接到交互式 host 上。模型/provider 层继续保留可替换接口，
后续可以接入其他 Agent 的模型接口。

## 当前基线

当前主目录 `D:\YunXi Agent` 已具备：

- v1.0 主命令：`yunxi`。
- 兼容命令：`yunxi-agent-cli`。
- 安装位置：`C:\Users\admin\AppData\Local\YunXi Agent\bin\yunxi.exe`。
- 默认 backend：`yunxi`。
- 已验证 DeepSeek live provider stream 与 non-stream smoke。
- `yunxi-agent-runtime` 已有真实 runtime parity 链路：
  provider -> context -> approval cache -> sandbox decision -> tools -> MCP ->
  skills -> multi-agent child scoped stream -> storage -> JSONL。
- `yunxi-agent-cli` 已有 one-shot、JSON、JSONL、sessions、parity map 等命令面。
- `.codegraph/` 已存在，后续理解和定位代码时继续优先使用 CodeGraph。

当前主要差距：

- CLI host 只有 one-shot run，没有交互式输入循环。
- 无参数 `yunxi` 仍视为错误，而不是进入新会话。
- 终端输出只打印最终结果，没有按 stream event 增量渲染模型回复。
- 交互式 session id、turn id、history、resume 还没有串成用户可感知的对话流。
- Ctrl+C cancellation 和 child runtime cancellation 还没有接到 terminal host。
- approval/escalation 事件存在，但缺少交互式确认 UI。
- live provider 可以通过参数和环境使用，但还没有成为交互式模式里的清晰默认体验。

## 硬性约束

- 默认 YunXi runtime 不得依赖 `vendor/codex-rs`。
- 默认 CLI 依赖图不得包含 `codex-*` crate。
- 默认 `yunxi` 不得依赖 `yunxi-agent-codex`。
- `vendor/codex-rs` 和 `extracted/codex-core-agent-sources` 只能作为行为参考和对照输入。
- v1.1 只补 terminal interactive CLI，不引入 TUI、desktop app、cloud tasks、doctor、update、
  completion、marketplace 或复杂 installer。
- 模型/provider 层继续保持可替换接口，不绑定到 OpenAI 或 DeepSeek。
- API key 只能从环境变量或测试 harness 临时读取，不打印、不写入日志、不写入提交。
- 构建阶段先整体接线，不在单点反复验证；完成后统一运行最终验证门。
- 任务结束必须同步桌面开发日志：
  `C:\Users\admin\Desktop\YunXi Agent开发日志.md`。
- 最终验证后必须清理 `target`、根目录 `.yunxi` 和临时 smoke/fixture 产物。

## v1.1 总目标

v1.1 要把 YunXi Agent 从“可安装的一次性 CLI”推进到“可持续对话的交互式 CLI”。
完成后应满足：

1. `yunxi` 无参数启动时进入交互式 REPL，而不是报错。
2. `yunxi "任务内容"` 继续保持 one-shot 行为和脚本兼容性。
3. 交互式模式支持真实 provider 通路，优先使用显式参数或环境变量配置。
4. 每轮对话复用同一个 session，history 会进入下一轮 provider request。
5. 模型 stream event 可以增量渲染到终端。
6. shell、patch、MCP、skill、multi-agent 事件可以在终端里可读展示。
7. Ctrl+C 可以取消当前 turn，并保持 shell 状态和 session 状态干净。
8. approval/escalation 可以在交互式模式中等待用户确认。
9. slash commands 可以管理会话、模型、provider、cwd、显示帮助和退出。
10. JSON/JSONL one-shot 继续保持稳定，下游自动化不被交互式模式破坏。

## 构建面 1：交互式 CLI 入口

目标文件：

- `crates/yunxi-agent-cli/src/main.rs`
- 新建 `crates/yunxi-agent-cli/src/interactive.rs`
- `crates/yunxi-agent-cli/tests/cli_tests.rs`

构建内容：

- 当没有 subcommand、没有 prompt、没有 `--json`、没有 `--jsonl` 时，进入 interactive mode。
- 保留当前 one-shot 判断：只要存在 prompt，就执行原有 `run_agent_backend`。
- `--json` 和 `--jsonl` 不进入 REPL，缺少 prompt 时仍返回结构化错误，避免脚本被交互式阻塞。
- 交互式入口打印简短 banner、当前 cwd、provider/model 摘要和 session id。
- 支持 stdin 非 TTY 时的 batch interactive smoke：逐行读取命令，遇到 `/exit` 退出。

完成口径：

- `yunxi` 可以进入提示符。
- `yunxi "你好"` 行为不变。
- `yunxi --jsonl` 无 prompt 不会挂住等待输入。

## 构建面 2：REPL 循环与 Slash Commands

目标文件：

- `crates/yunxi-agent-cli/src/interactive.rs`
- 新建 `crates/yunxi-agent-cli/src/commands.rs`
- `crates/yunxi-agent-cli/tests/cli_tests.rs`

构建内容：

- 实现基础提示符：`yunxi> `。
- 支持空输入跳过。
- 支持 `/exit`、`/quit` 退出。
- 支持 `/help` 输出命令列表。
- 支持 `/clear` 清理当前屏幕或输出分隔提示。
- 支持 `/cwd` 查看当前工作目录。
- 支持 `/model` 查看或切换模型字段。
- 支持 `/provider` 查看或切换 provider 字段。
- 支持 `/session` 显示当前 session id、turn 数、cwd、model、provider。
- 支持 `/resume <session_id>` 将后续 turn 绑定到已有 session。

完成口径：

- slash command 不进入 provider。
- 普通文本输入会作为新 turn 发送给 runtime。
- 非法 slash command 返回清晰错误并继续 REPL。

## 构建面 3：交互式 Session 与 History

目标文件：

- `crates/yunxi-agent-cli/src/interactive.rs`
- `crates/yunxi-agent-runtime/src/session_driver.rs`
- `crates/yunxi-agent-storage/src/lib.rs`
- `crates/yunxi-agent-cli/tests/cli_tests.rs`

构建内容：

- REPL 启动时创建或恢复一个 session id。
- 每轮 turn 调用 runtime 时携带 parent/session 配置。
- 复用现有 file-backed session store，保存 user prompt、assistant response、events、metadata。
- `/resume <session_id>` 后，下一轮使用 storage history 重建 context。
- 交互式模式退出时输出 session id，便于下一次 resume。

完成口径：

- 同一 REPL 内连续两轮对话可以看到 history 注入。
- 退出后可以通过 `yunxi sessions show <id>` 查看记录。
- `/resume` 后新 prompt 不再拼接旧 prompt 文本，而是走 runtime history restore。

## 构建面 4：真实 Provider 默认体验

目标文件：

- `crates/yunxi-agent-cli/src/interactive.rs`
- `crates/yunxi-agent-provider/src/lib.rs`
- `README.md`
- `scripts/provider/deepseek-live-smoke.ps1`

构建内容：

- 交互式模式沿用 `--provider-live`、`--model`、`--provider`、环境变量配置。
- 如果显式启用 live provider 且缺少 key，进入 REPL 前给出可读错误，不泄露环境变量值。
- 如果未显式启用 live provider，则使用当前 offline provider，但 banner 要明确显示 `offline provider`。
- 支持 `YUNXI_PROVIDER_BASE_URL`、`YUNXI_PROVIDER_API_KEY`、`YUNXI_PROVIDER_MODEL`
  这类通用环境变量路径。
- DeepSeek 仍作为 OpenAI-compatible profile，不写死为唯一 provider。

完成口径：

- `yunxi --provider-live` 可以在交互式模式中调用真实 provider。
- 缺少 key 时不 panic、不输出 key 名以外的敏感内容。
- offline 模式输出明确，不让用户误以为模型已真实回答。

## 构建面 5：Stream Renderer

目标文件：

- 新建 `crates/yunxi-agent-cli/src/render.rs`
- `crates/yunxi-agent-runtime/src/lib.rs`
- `crates/yunxi-agent-provider/src/lib.rs`
- `crates/yunxi-agent-cli/tests/jsonl_tests.rs`

构建内容：

- 将 `AgentEvent` 转成 terminal-friendly render event。
- 对 assistant message delta 增量打印。
- 对 reasoning、tool started、tool completed、warning、provider error 使用紧凑状态行。
- 对 command stdout/stderr delta 保持顺序输出。
- 对 final response 避免重复打印。
- JSONL 模式继续走原有 `protocol_events_from_agent_events`，不混入 terminal renderer。

完成口径：

- stream 模型回复能边到达边显示。
- 工具事件可读但不刷屏。
- JSONL 输出仍是纯 JSON line。

## 构建面 6：Ctrl+C Cancellation

目标文件：

- `crates/yunxi-agent-cli/src/interactive.rs`
- `crates/yunxi-agent-runtime/src/turn_driver.rs`
- `crates/yunxi-agent-exec/src/lib.rs`
- `crates/yunxi-agent-multi-agent/src/lib.rs`

构建内容：

- REPL 运行 turn 时注册 cancellation token。
- 首次 Ctrl+C 取消当前 turn，不退出整个 REPL。
- 第二次 Ctrl+C 或空闲状态 Ctrl+C 退出 REPL。
- cancellation 传播到 provider stream、exec manager、MCP call 和 child runtime。
- cancellation 结果保存到 session events，并输出 `cancelled` 状态。

完成口径：

- 正在执行长 shell 或长 provider 请求时 Ctrl+C 可以停止当前 turn。
- 取消后可以继续输入下一轮 prompt。
- JSONL cancellation event 仍保持稳定。

## 构建面 7：Approval / Escalation 交互 UX

目标文件：

- `crates/yunxi-agent-cli/src/interactive.rs`
- `crates/yunxi-agent-cli/src/render.rs`
- `crates/yunxi-agent-tools/src/lib.rs`
- `crates/yunxi-agent-sandbox/src/lib.rs`

构建内容：

- 当 runtime 产生 approval requested 或 escalation requested 时，交互式 host 暂停当前工具执行并询问用户。
- 支持 `y`、`n`、`always`、`once` 等基础输入。
- `always` 写入 session approval cache，不写入全局配置。
- 非交互 one-shot 按现有 policy 自动处理，不弹交互输入。
- approval 文案显示 tool name、cwd、sandbox、network、目标路径和简短原因。

完成口径：

- 危险 shell/patch 请求能在 REPL 中等待用户确认。
- 选择拒绝后当前 turn 继续以 tool declined 结果反馈给模型。
- approval cache 复用不会泄露命令环境变量值。

## 构建面 8：MCP / Skills / Multi-Agent 交互可观察性

目标文件：

- `crates/yunxi-agent-cli/src/render.rs`
- `crates/yunxi-agent-mcp/src/lib.rs`
- `crates/yunxi-agent-skills/src/lib.rs`
- `crates/yunxi-agent-multi-agent/src/lib.rs`

构建内容：

- MCP lifecycle 在终端里显示 configured、initialized、reused、tool_started、tool_completed。
- skill invocation 显示 skill 名称和结构化结果摘要。
- multi-agent child scoped stream 以缩进或前缀显示，避免和父回复混在一起。
- 对长生命周期 MCP session 复用给出简短状态，不重复打印大段 schema。

完成口径：

- 用户能看出 agent 正在调用哪个 MCP/skill/child agent。
- child scoped stream 细粒度事件在终端可读。
- JSONL 事件保持原状。

## 构建面 9：配置、文档与安装升级

目标文件：

- `Cargo.toml`
- `Cargo.lock`
- `README.md`
- `docs/extraction-status.md`
- `scripts/install/install-yunxi.ps1`

构建内容：

- 版本提升到 `1.1.0`。
- README 增加交互式使用说明：
  - `yunxi`
  - `yunxi --provider-live`
  - `yunxi --model <model>`
  - slash commands
  - Ctrl+C 行为
- 安装脚本继续复制 `yunxi.exe` 和兼容 `yunxi-agent-cli.exe`。
- 不新增复杂 installer，不持久化 API key。

完成口径：

- `yunxi --version` 输出 `yunxi 1.1.0`。
- 新打开 PowerShell 输入 `yunxi` 即进入交互式模式。
- README 明确 v1.1 仍是 terminal CLI，不是 TUI/桌面应用。

## 最终统一验证门

v1.1 构建完成后一次性运行：

```powershell
cargo fmt
cargo fmt -- --check
cargo test
cargo check --workspace
cargo build -p yunxi-agent-cli --release --bins
target\release\yunxi.exe --version
target\release\yunxi.exe --backend yunxi "YunXi Agent v1.1 one-shot smoke"
@("你好"; "/session"; "/exit") | target\release\yunxi.exe --backend yunxi
@("/help"; "/exit") | target\release\yunxi.exe --backend yunxi
target\release\yunxi.exe --jsonl
cargo run -p yunxi-agent-cli -- parity map
cargo tree -p yunxi-agent-cli
# Run dependency keyword scan for codex/vendor/yunxi-agent-codex in default CLI tree.
# Run owned-source secret-pattern scan without printing secret values.
git diff --check
.\scripts\provider\deepseek-live-smoke.ps1 -Model deepseek-v4-flash
.\scripts\provider\deepseek-live-smoke.ps1 -Model deepseek-v4-flash -NoStream
codegraph sync "D:\YunXi Agent"
cargo clean
Remove-Item -Recurse -Force .\.yunxi -ErrorAction SilentlyContinue
```

验收时必须记录：

- 每条命令退出码。
- one-shot smoke 输出。
- interactive stdin smoke 是否进入并退出 REPL。
- `/help`、`/session`、`/exit` 是否可用。
- `--jsonl` 无 prompt 是否保持结构化错误，不进入 REPL。
- live provider stream/non-stream 是否通过；失败时记录 classification，不记录密钥。
- 默认 CLI dependency tree 是否命中上游依赖关键词。
- owned-source secret scan 是否无匹配。
- `target` 和根目录 `.yunxi` 是否已清理。

## 完成定义

v1.1 完成时必须同时满足：

- `yunxi` 无参数进入交互式 CLI。
- `yunxi "任务"` one-shot 兼容行为不破。
- 交互式模式可以使用 offline provider，也可以通过 `--provider-live` 使用真实 provider。
- REPL 内多轮对话复用 session，并保存到现有 storage。
- streaming output、tool lifecycle、MCP lifecycle、child scoped stream 在终端中可读。
- Ctrl+C 可以取消当前 turn 并继续下一轮。
- approval/escalation 在交互式模式中有用户确认路径。
- 默认依赖图继续不含 `vendor/codex-rs`、`codex-*` 和 `yunxi-agent-codex`。
- 文档、状态页、CodeGraph、桌面开发日志全部同步。
- 编译产物和临时 session/smoke 产物清理完成。

## 下一步执行指令

下一阶段直接按本报告实施 YunXi Agent v1.1。执行时先整体构建交互式 CLI host、
REPL、session/history、stream renderer、cancellation、approval UX 和 provider 配置接线；
构建过程中不做频繁测试，不在单点卡住。全部构建完成后统一运行最终验证门，
再更新状态文档、桌面开发日志、安装产物和 GitHub 同步状态。
