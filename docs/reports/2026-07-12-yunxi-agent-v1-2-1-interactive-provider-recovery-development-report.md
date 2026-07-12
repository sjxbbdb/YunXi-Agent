# YunXi Agent v1.2.1 Interactive Provider Recovery Development Report

生成时间：2026-07-12 +08:00

## 问题

v1.2 interactive 在某一轮 provider 请求失败时把错误传播到 CLI 顶层，导致整个 REPL 退出并
直接返回 PowerShell。DeepSeek HTTP 400 同时只显示泛化的 `unsupported_schema`，因为 transport
读取到的服务端错误正文没有进入 provider error mapping。v1.2 的 live interactive 验收也只检查
banner 和 `/session`，没有真正发送 prompt。

## 修复

- interactive 普通 turn 错误在 REPL 边界捕获，输出 `[error]` 后继续读取下一条输入。
- 失败 turn 不更新 active session 或 turn count；成功 turn 的存储和 resume 行为不变。
- DeepSeek HTTP 400/422 在普通 retry 完成后执行一次去除顶层 `metadata` 的兼容请求。
- fallback 保留 tools/messages/model，不扩展为任意 400 的普通重试。
- `ProviderMessage` 增加 assistant tool calls 和 tool result call id；runtime 为缺失 id 的调用生成稳定 id。
- 后续 provider 请求保留 assistant `tool_calls`，并为每条 `role=tool` 消息写入匹配的 `tool_call_id`。
- 最终 provider 错误只提取 message/type/code，折叠空白、脱敏 token-like 片段并限制 240 字符。
- provider 测试覆盖 fallback、provider scoping 和错误脱敏。
- provider 请求 JSON 测试覆盖 assistant tool call 与 tool result id 一致性。
- CLI 集成测试通过本地顺序 HTTP server 覆盖第一轮两次 400、第二轮 200，验证 REPL 不退出。
- live smoke 增加真实 interactive prompt 模式，要求 assistant marker 和 `/exit` 正常结束。
- workspace 版本提升到 1.2.1，计划新建 `v1.2.1` tag；旧 tag 保持不可变。

## 硬性约束

- 构建阶段不运行测试、检查、格式化、编译或 live 请求。
- 所有代码、测试、脚本、版本和文档接线完成后只执行一次统一验证门。
- API key/PAT 不打印、不写日志、不写源码、不提交。
- 默认 CLI 不依赖 `vendor/codex-rs`、`codex-*` 或 `yunxi-agent-codex`。
- GitHub 远端读取、写入和核验只使用 REST API。
- `v1.0.0`、`v1.1.0`、`v1.2.0` 不删除、不移动。

## 统一验证结果

- `cargo fmt`、`cargo fmt -- --check`：通过。
- `cargo test`：通过；workspace 共 196 项测试，0 失败。
- `cargo check --workspace`：通过，无 warning。
- `cargo build -p yunxi-agent-cli --release --bins`：通过。
- 两个 release 二进制均报告 `yunxi 1.2.1`。
- 本地顺序 HTTP 回归：第一轮 400、metadata fallback 400、第二条 prompt 200；REPL 正常继续并退出。
- provider JSON 回归：assistant `tool_calls` 与 `role=tool` 的 `tool_call_id` 一致。
- forced-offline one-shot/interactive、参数冲突、JSONL error、parity map：通过。
- 默认 CLI dependency tree 356 行，上游 Codex runtime 依赖命中 0。
- 自有发布范围 108 个文件，secret pattern 和个人 API 路径命中均为 0。
- DeepSeek stream：47 行 JSONL，退出码 0，泄漏检测 False。
- DeepSeek non-stream：19 行 JSONL，退出码 0，泄漏检测 False。
- DeepSeek interactive prompt：51 行输出，banner、assistant marker、`/exit` 正常结束均通过，泄漏检测 False。
- 用户实际目录 release/安装版“你好”交互：assistant 完成、session 写入、仅 `/exit` 结束，无 provider 400。
- 安装版 SHA-256 与最终 release 构建一致，版本为 1.2.1。

统一门中先发现测试 fixture 的长 `sk-` 假字符串触发秘密形状扫描，缩短 fixture 后扫描归零并完整
重跑测试。安装验收首次保留的服务端 detail 暴露了真实协议根因：工具结果消息缺少
`tool_call_id`。随后补齐 assistant tool call history、匹配 id 和请求 JSON 回归，并完整重跑 Rust、
release、真实 DeepSeek 与安装版验收。

## 发布要求

- 本报告所在最终提交必须通过 GitHub Git Data REST API 发布到 `master`。
- annotated tag `v1.2.1` 必须指向该最终提交；`v1.0.0`、`v1.1.0`、`v1.2.0` 不得删除或移动。
- GitHub master/ref/commit/tree/tag 必须通过 REST API 独立回读核验；不得使用 Git transport 远端命令。
- 发布前必须同步 CodeGraph，并将本阶段详细记录追加到桌面开发日志。
- 发布后必须执行 `cargo clean`，删除仓库内 `.yunxi` 和临时 smoke 产物。
