# YunXi Agent v1.2.1 Interactive Provider Recovery Design

生成时间：2026-07-11 +08:00

## 背景

YunXi Agent v1.2 的 interactive REPL 在单轮 provider 请求失败时使用 `?` 将错误传到
CLI 顶层，导致整个进程退出并返回 PowerShell。一次 DeepSeek HTTP 400 同时暴露出两个缺口：
provider 丢弃了服务端错误正文，且 v1.2 live interactive 验收只检查启动状态，没有真正发送
prompt。

同目录、同安装版、同“你好”随后在 one-shot 和 interactive 均成功，因此该 400 不是稳定可复现
的 interactive schema 差异。本版不假定单一服务端根因，而是在兼容、安全和交互生命周期边界修复
可确认的问题。

## 目标

- 单轮 provider、tool 或 network 错误只结束当前 turn，REPL 保持运行并重新显示 `yunxi>`。
- 保留最后一次成功的 active session；失败 turn 不递增 turn count，不替换 session id。
- DeepSeek HTTP 400/422 首次失败时，移除非必要 `metadata` 后兼容重试一次。
- provider 工具循环保留 assistant `tool_calls`，并让 tool result 使用匹配的 `tool_call_id`。
- fallback 仍失败时，显示 provider/status/classification 和限长、脱敏的服务端结构化详情。
- live interactive 自动验收必须真正发送 prompt 并验证 assistant 内容。
- 发布为新版本 `v1.2.1`；`v1.0.0`、`v1.1.0`、`v1.2.0` 不删除、不移动。

## 非目标

- 不把任意 400 加入普通三次 transient retry。
- 不永久删除所有 provider 的 request metadata。
- 不改变 one-shot 的失败退出语义。
- 不引入 TUI、desktop、cloud task、update、doctor 或 marketplace 表面。
- 不引入上游 Codex runtime 默认依赖。

## 设计

### REPL 错误边界

`InteractiveSession::read_eval_loop` 在普通 prompt 分支捕获 `run_turn` 错误，使用与 CLI 顶层一致的
脱敏格式输出 `[error] ...`，随后继续读取下一行。初始化错误、stdin 读取失败和 interactive command
自身的不可恢复错误仍可终止 REPL；`/exit`、`/quit` 和 EOF 保持现有退出语义。

失败 turn 不调用 `update_session_state`，因此不会污染 active session。下一条输入重新解析 provider
selection，允许环境或 `/provider`、`/model` 变化生效。

### DeepSeek schema fallback

`OpenAiTransportProvider` 在普通 retry 策略结束后检查最终响应。仅当：

- provider profile 为 `deepseek`；
- status 为 400 或 422；
- request body 存在 `metadata`；

才克隆请求、删除顶层 `metadata`，并通过现有 `send_with_retries` 再发送一次。fallback 最多一次，
不会递归，也不会删除 `tools`、messages 或 model。其他 provider 和其他 status 完全保持原行为。

### Tool call history

`ProviderMessage` 携带可选 `tool_call_id` 和 assistant tool call 列表。runtime 收到模型工具调用后先
确保每个调用具有稳定 id，再把 assistant tool call 消息写入本轮 history；执行结果使用相同 id 构造
`role=tool` 消息。这样第二次 chat completion 请求符合 OpenAI/DeepSeek 的工具消息协议。

### 安全错误详情

错误映射从最终 `ProviderTransportResponse.body` 中只提取 JSON 的 `error.message`、`error.type`、
`error.code`，兼容顶层 `message`。详情先折叠控制字符和空白，再对 `Bearer`、`sk-`、`ghp_`、
`github_pat_` 等 token-like 片段脱敏，最后限制为 240 个字符。无法解析或没有允许字段时不回显原始
body。

错误分类仍由 status 决定，避免服务端文本改变脚本可依赖的 `classification`。

## 测试与验收

- provider 单元测试验证 DeepSeek 400 首次 body 含 metadata、fallback body 不含 metadata、fallback
  成功后返回正常 assistant 内容。
- provider 单元测试验证非 DeepSeek 400 不 fallback。
- provider 单元测试验证最终错误包含允许的 message/type/code，且 API key、Authorization/Bearer 和
  超长尾部不泄漏。
- provider 单元测试验证 assistant tool call 与 tool result 序列化为相同 call id。
- CLI 集成测试启动本地 mock HTTP server，按 400、400、200 返回；向 interactive stdin 发送两条
  prompt 和 `/exit`，验证第一轮错误后第二轮仍成功且进程正常退出。
- 统一执行 `cargo fmt -- --check`、`cargo test`、`cargo check --workspace`、release 双二进制、离线
  smoke、真实 DeepSeek stream/non-stream/one-shot/interactive prompt、依赖和秘密扫描。
- 安装后再次执行真实 interactive prompt，确认错误不会导致 REPL 非预期退出。

## 发布与清理

- workspace 版本提升到 1.2.1。
- 创建新 annotated tag `v1.2.1`，旧 tag SHA 必须在发布前后保持一致。
- GitHub 所有远端读写和核验只使用 GitHub REST API。
- 更新 CodeGraph 和桌面开发日志。
- 发布后执行 `cargo clean`，删除根目录 `.yunxi` 和临时 smoke 文件。
