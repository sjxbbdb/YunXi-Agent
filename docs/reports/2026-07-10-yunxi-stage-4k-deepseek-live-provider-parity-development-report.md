# YunXi Stage 4K DeepSeek Live Provider And Real-World Core Parity Development Report

生成时间：2026-07-10 21:18:00 +08:00

## 核心结论

Stage 4J 已经证明主目录 `D:\YunXi Agent` 中的默认 YunXi runtime 可以在不依赖
`vendor/codex-rs`、`codex-*` 或 `yunxi-agent-codex` 默认运行路径的情况下完成离线
headless agent 回合：

```text
provider stream -> tool loop -> multi-agent child runtime -> storage -> JSONL
```

下一版 Stage 4K 的目标不是再证明离线 fixture 能跑，而是把这条自主链路接入真实
OpenAI-compatible provider，并用 `C:\Users\admin\Desktop\api.txt` 中的 DeepSeek API key
做真实网络 smoke。DeepSeek 官方文档显示 DeepSeek API 兼容 OpenAI/Anthropic 格式，
OpenAI format base URL 为 `https://api.deepseek.com`，当前模型为 `deepseek-v4-flash`
和 `deepseek-v4-pro`。

本轮只出报告，不读取或打印任何密钥值。已做只读确认：`api.txt` 存在，包含
DeepSeek 标记和疑似 API key。后续执行 Stage 4K 时，密钥只能进入临时环境变量，
不能写入仓库、日志、JSONL 输出或提交历史。

## 当前基线

主目录当前状态：

- 分支：`master`
- 最新提交：`4377963 Build Stage 4J child runtime parity harness`
- 默认 `yunxi-agent-cli` 依赖树：不包含 `codex`、`vendor`、`yunxi-agent-codex`
- CodeGraph：主目录 `.codegraph` 已建立，索引 up to date
- Stage 4J fixture：23 行 JSONL，8 条 `child_agent` 事件，`storage_state.child_session_ids`
  包含 `agent-1-session`

已具备能力：

- `yunxi-agent-provider`：OpenAI-compatible `/chat/completions` 请求、streaming request、
  SSE parser、tool call 聚合、fixture transport、reqwest transport
- `yunxi-agent-runtime`：provider stream loop、tool loop、child runtime adapter、
  storage state、context injection、history restore、compact entry
- `yunxi-agent-tools`：shell、patch、MCP、skill、multi-agent、tool search、view image、
  request user input facade、sandbox decision event
- `yunxi-agent-mcp`：workspace MCP config loader、session manager、stdio/http JSON-RPC 边界
- `yunxi-agent-storage`：file-backed sessions、history restore、session graph、
  runtime rollout/state snapshot
- `yunxi-agent-cli`：`--backend yunxi`、`--provider-live`、JSONL 输出、sessions
  list/show/history/resume/graph

## 外部资料口径

- DeepSeek API 文档说明其 API 格式兼容 OpenAI/Anthropic。
- DeepSeek Models & Pricing 文档列出 OpenAI format base URL：
  `https://api.deepseek.com`。
- DeepSeek Models & Pricing 文档列出当前模型：
  `deepseek-v4-flash` 和 `deepseek-v4-pro`。

报告依据：

- `https://api-docs.deepseek.com/`
- `https://api-docs.deepseek.com/quick_start/pricing`
- `https://api-docs.deepseek.com/api/list-models`

## 硬性约束

- 默认 YunXi runtime 不得依赖 `vendor/codex-rs`。
- 默认依赖图不得包含 `codex-*` crate。
- 默认 `yunxi-agent-cli` 不得依赖 `yunxi-agent-codex`。
- `vendor/codex-rs` 和 `extracted/codex-core-agent-sources` 只能作为行为参考、源码迁移输入
  和 fixture 对照，不能重新进入默认运行时依赖。
- 模型/provider 层必须继续保持可替换接口，不能把 runtime 绑定死到 OpenAI 或 DeepSeek。
- DeepSeek API key 只能从 `C:\Users\admin\Desktop\api.txt` 读取到临时环境变量。
- 任何命令输出、日志、JSONL、错误消息、测试快照、提交和报告都不得包含 API key。
- 构建阶段先整体接线，不做中途频繁测试；全部构建完成后统一运行最终验证门。
- 遇到复杂单点时先落 facade、fixture 或兼容层并继续推进，不在单点上消耗过多时间。
- 最终验证后必须清理 `target`、`.yunxi` 和临时 smoke 输出文件。
- 每次任务结束必须追加桌面开发日志：
  `C:\Users\admin\Desktop\YunXi Agent开发日志.md`。

## Stage 4K 总目标

Stage 4K 要把 YunXi Agent 从“离线自主 fixture 可运行”推进到“真实 provider 可用且可诊断”。

完成后应满足：

1. 使用 DeepSeek API key 可以通过 `--backend yunxi --provider-live` 完成真实模型调用。
2. DeepSeek base URL、model、streaming 开关通过 provider-neutral 配置进入 runtime。
3. live smoke 覆盖非流式和流式两条路径，失败时输出可诊断但不泄露密钥的错误。
4. live provider 能复用现有 tool registry 和 runtime loop；如果 DeepSeek 对某些 tool schema
   字段不兼容，新增 provider capability 开关或 DeepSeek preset，而不是硬编码到 runtime。
5. 所有新增能力仍能在无真实 API key 的环境下通过 fixture 测试。
6. 默认依赖树继续不包含上游 Codex runtime。

## 构建面 1：Secret-Safe DeepSeek Smoke Harness

目标文件：

- `scripts/provider/deepseek-live-smoke.ps1`
- `docs/extraction-status.md`

构建内容：

- 新增 PowerShell smoke 脚本，只从 `C:\Users\admin\Desktop\api.txt` 提取 DeepSeek API key。
- 脚本必须只输出 key 是否存在、模型、base URL、命令退出码、JSONL 行数和事件计数。
- 脚本不得打印 key、Authorization header、完整请求体或完整响应体。
- 脚本设置以下临时环境变量：

```powershell
$env:YUNXI_PROVIDER_API_KEY = $deepseekKey
$env:YUNXI_PROVIDER_BASE_URL = "https://api.deepseek.com"
$env:YUNXI_AGENT_MODEL = "deepseek-v4-flash"
$env:YUNXI_PROVIDER_STREAM = "1"
```

- 脚本结束时必须清空本进程中的 `YUNXI_PROVIDER_API_KEY`。
- 脚本必须支持 `-Model deepseek-v4-pro` 参数，用于高能力模型 smoke。
- 脚本必须支持 `-NoStream` 参数，用于非流式 fallback。

验收口径：

- 找不到 key 时退出码非 0，错误为 `DeepSeek API key not found in api.txt`。
- 找到 key 时不打印 key 值。
- smoke 输出能被日志安全引用。

## 构建面 2：DeepSeek Provider Compatibility Profile

目标文件：

- `crates/yunxi-agent-provider/src/lib.rs`
- `crates/yunxi-agent-provider/tests/provider_tests.rs`
- `crates/yunxi-agent-runtime/src/lib.rs`

构建内容：

- 保持现有 `YUNXI_PROVIDER_BASE_URL`、`YUNXI_AGENT_MODEL`、`YUNXI_PROVIDER_STREAM`
  作为通用 provider-neutral 配置。
- 新增可选 provider compatibility profile：

```text
YUNXI_PROVIDER_PROFILE=deepseek
```

- profile 只影响 provider adapter 默认值，不改变 runtime 语义。
- `deepseek` profile 默认：

```text
base_url = https://api.deepseek.com
model = deepseek-v4-flash
stream = true
wire_api = chat_completions
```

- 如果 DeepSeek 返回不支持 `parallel_tool_calls` 或某些 tool schema 字段，profile 可以关闭
  对应 capability：

```text
tools = true
parallel_tool_calls = false
stream_usage = false
```

- `--model` 和 `YUNXI_AGENT_MODEL` 必须覆盖 profile 默认 model。
- `YUNXI_PROVIDER_BASE_URL` 必须覆盖 profile 默认 base URL。

验收口径：

- 不设置 profile 时现有默认行为不变。
- 设置 `YUNXI_PROVIDER_PROFILE=deepseek` 时可以不再手动指定 base URL 和 model。
- DeepSeek profile 的 request URL 仍为
  `https://api.deepseek.com/chat/completions`。

## 构建面 3：Live Provider Error Classification And Redaction

目标文件：

- `crates/yunxi-agent-provider/src/lib.rs`
- `crates/yunxi-agent-core/src/error.rs`
- `crates/yunxi-agent-cli/src/main.rs`
- `crates/yunxi-agent-provider/tests/provider_tests.rs`

构建内容：

- 将 provider HTTP 错误分为：

```text
auth_error
rate_limit
timeout
network
server_error
bad_request
unsupported_schema
unknown_provider_error
```

- 错误消息中保留 HTTP status、分类、provider profile、base URL host、model。
- 错误消息不得包含 API key、Bearer token、Authorization header、完整请求 body。
- CLI JSONL 的 error event 使用稳定 shape：

```json
{
  "type": "error",
  "kind": "provider_error",
  "provider": "deepseek",
  "status": 401,
  "classification": "auth_error",
  "message": "provider returned HTTP 401 (auth_error)"
}
```

- 对 DeepSeek 常见失败做 fixture：
  - 401 auth
  - 429 rate limit
  - 400 unsupported schema
  - 5xx server error
  - network timeout

验收口径：

- 所有错误 fixture 均不含密钥。
- CLI 能以非 0 退出码结束 provider error。
- JSONL 逐行可解析。

## 构建面 4：DeepSeek Live Smoke Matrix

目标文件：

- `scripts/provider/deepseek-live-smoke.ps1`
- `crates/yunxi-agent-cli/tests/jsonl_tests.rs`
- `docs/extraction-status.md`

构建内容：

- live smoke 分为四级：

```text
L0: provider key and /models optional probe
L1: non-stream simple prompt
L2: stream simple prompt
L3: stream prompt with tool-call encouragement
```

- L1 命令：

```powershell
$env:YUNXI_PROVIDER_STREAM = "0"
cargo run -p yunxi-agent-cli -- --backend yunxi --provider-live --jsonl --model deepseek-v4-flash "Reply exactly: YUNXI_DEEPSEEK_OK"
```

- L2 命令：

```powershell
$env:YUNXI_PROVIDER_STREAM = "1"
cargo run -p yunxi-agent-cli -- --backend yunxi --provider-live --jsonl --model deepseek-v4-flash "Reply exactly: YUNXI_DEEPSEEK_STREAM_OK"
```

- L3 命令：

```powershell
$env:YUNXI_PROVIDER_STREAM = "1"
cargo run -p yunxi-agent-cli -- --backend yunxi --provider-live --jsonl --model deepseek-v4-flash "If tools are available, use the safest available shell command to print YUNXI_TOOL_OK, then summarize the result."
```

- L3 不是默认通过门，因为真实模型可能选择不调用工具；它作为 diagnostic smoke。

验收口径：

- L1 必须通过。
- L2 必须通过，或产出明确 streaming incompatibility 分类。
- L3 若未调用工具，不判定核心失败；若调用工具，tool loop 必须能正常返回结果。
- 每次 live smoke 后删除 `.yunxi` session 运行痕迹和临时输出文件。

## 构建面 5：真实 Provider 对 Codex 核心缺口的反馈闭环

目标文件：

- `docs/extraction-status.md`
- `docs/reports/2026-07-10-yunxi-stage-4k-deepseek-live-provider-parity-development-report.md`
- 后续缺口对应 crate 的测试文件

构建内容：

- 将 DeepSeek live smoke 暴露出的失败归入以下 bucket：

```text
provider_payload_shape
provider_stream_parser
tool_schema_compatibility
tool_loop_runtime
context_payload_size
jsonl_output_shape
storage_side_effect
auth_and_secret_handling
network_retry_timeout
unknown
```

- 每个 bucket 必须形成一个 fixture 测试或文档化的已知限制。
- 如果真实 DeepSeek tool calling 行为不同于 OpenAI，优先在 provider profile/capability 层适配，
  不得把 DeepSeek 特例扩散到 runtime、tools 或 storage。
- 如果 DeepSeek 对长上下文或 tools 有限制，runtime 应通过 context budget 和 provider capability
  收敛请求，而不是让 live call 直接失败。

验收口径：

- live failure 不再是“看一段报错”，而是能映射到具体代码层。
- 下一轮开发报告可以基于 bucket 精准推进 Codex core parity。

## 最终统一验证门

Stage 4K 构建完成后一次性运行：

```powershell
cargo fmt
cargo fmt -- --check
cargo test
cargo check --workspace
cargo build -p yunxi-agent-cli
cargo run -p yunxi-agent-cli -- parity map
cargo tree -p yunxi-agent-cli
git diff --check
.\scripts\provider\deepseek-live-smoke.ps1 -Model deepseek-v4-flash
.\scripts\provider\deepseek-live-smoke.ps1 -Model deepseek-v4-flash -NoStream
cargo clean
```

额外检查：

- `cargo tree -p yunxi-agent-cli` 输出不得命中 `codex`、`vendor`、`yunxi-agent-codex`。
- `git diff --check` 不得出现空白错误。
- live smoke 输出不得包含 `sk-`、`Bearer`、`Authorization` 或实际 key 片段。
- `.yunxi` 运行痕迹必须删除。
- `target` 必须删除。
- 桌面开发日志必须追加本轮记录。

## 完成定义

Stage 4K 完成时必须同时满足：

- DeepSeek live L1 smoke 通过。
- DeepSeek live L2 smoke 通过，或有明确 streaming incompatibility 诊断和 fixture 回归。
- DeepSeek profile/preset 不破坏 provider-neutral 架构。
- 无 API key 泄露到仓库、日志、JSONL、错误消息或测试快照。
- 默认 YunXi CLI 依赖树仍完全自主。
- 离线 fixture 测试仍完整通过。
- smoke 产物和编译产物清理完成。

## 后续仍未覆盖的 Codex CLI Agent 核心缺口

Stage 4K 解决的是真实 provider 接入和诊断，不会一次性补齐所有 Codex CLI agent 核心差距。
完成 Stage 4K 后仍需继续推进：

- 平台级 sandbox runner 和 escalation 执行边界
- Guardian/request_user_input/session remember/persistent approval
- MCP 长生命周期真实 server 复用和 approval elicitation
- 完整 context manager、token budgeting、rollout truncation、复杂 resume
- parent/child cancellation 和 interrupt propagation
- apply_patch Windows helper/arg0 和更多异常诊断边界
- skills/plugins 生命周期和权限模型
- Codex 全量 protocol/JSONL event shape

## 下一步执行指令

下一阶段开发直接按本报告执行：

```text
先整体构建 Stage 4K 的 DeepSeek live provider smoke harness、provider profile、
错误分类与密钥脱敏、live smoke matrix、失败 bucket 反馈闭环；
构建过程中不做中途频繁测试；
全部构建完成后统一运行最终验证门；
验证后清理 target、.yunxi 和临时 smoke 输出；
写桌面日志，提交并推送。
```
