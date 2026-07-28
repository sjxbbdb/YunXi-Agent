# YunXi Agent v2.1.5 微信私聊长轮询、配对准入与幂等接纳开发报告

- 撰写时间：2026-07-27 23:33:07 +08:00
- 审核依据：`C:\Users\24763\Desktop\YunXi Agent审核报告\2026-07-27-230718-YunXi-Agent-v2.1.4-hotfix.1-微信状态迁移复审审核报告.md`
- 项目内审核报告归档：`D:\YunXi Agent\docs\reports\audits\2026-07-27-230718-yunxi-agent-v2-1-4-hotfix-1-weixin-state-migration-reaudit-report.md`
- 审核报告 SHA-256：`7D66919116B824D594618E77965B03ED184E054F61DE5581E0D4B2F5A18CF0FE`
- 总纲正本：`D:\YunXi Agent\docs\superpowers\plans\2026-07-22-yunxi-agent-v2-1-1-to-v2-2-0-personal-wechat-roadmap.md`
- 总纲 SHA-256：`2DF30D503F46CFE7496567F5011BF5CBFB8BA73C91C2D2FA3E9F29AF0932EA0F`
- 当前工作树基线：`HEAD=664feb3b9be2c8a179a203e297b6e72cf7a7084b`，`git describe=v2.1.4-hotfix.1-1-g664feb3-dirty`
- 审核结论转化：`v2.1.4-hotfix.1` 审核通过，允许进入总纲图中的 `v2.1.5` 开发。
- 目标版本：`v2.1.5`
- 必须创建的 tag：新的 annotated `v2.1.5`
- 开发目录：`D:\YunXi Agent`
- 报告类型：下一阶段开发指令报告，主要面向开发者；本报告不是完成声明。

## 一、硬性约束

以下约束是 `v2.1.5` 开发、验证、发布和文档同步的前置要求，不得因为上下文过长、任务拆分、实现困难或局部修复而忽略、弱化或绕过。

1. 开发目录固定在 `D:\YunXi Agent` 及其工作树内。
2. 核心目标是开发一个通用型陪伴 agent；默认运行路径不得依赖 `vendor/codex-rs`、`codex-*` crate 或 `yunxi-agent-codex`。
3. 涉及到源码参考需要先抽取、迁移和构建整体能力，不要在单个点上反复纠结；如果参考的源码不为 Rust 语言，则需参考其逻辑，进行 Rust 复刻。
4. 中间无需频繁验证，完成一批构建后再统一测试和验证。
5. 验证通过前不能宣称完成。
6. 每次阶段结束后要清理编译中间产物，避免占用大量硬盘空间。
7. 每次任务结束后，都要在日志中追加本次工作的详细记录。
8. 记录必须包含时间戳、工作目标、执行流程、修改文件、文件路径、验证结果、提交和推送状态，以及日志结尾处要进行署名，署名为开发报告撰写者。
9. 后续开发报告、设计文档、索引和状态文档要与实际代码状态保持一致。
10. 回复用户使用中文。
11. 之后每个版本更迭都必须提交为新的 Git tag；以前的版本 tag 不得删除，方便出错后回滚。
12. 涉及到 shell 命令时要万分小心，例如递归删除、强制移动、清空目录、系统级安装/卸载、修改 PATH/注册表/系统配置等操作必须得到用户确认。
13. 不可因为过长的上下文忽略硬性要求；如果察觉到硬性要求的效力已经减弱，则必须汇报当前情况。
14. 工作尽量固定在项目文件夹，涉及到其他文件夹的操作，要明确告知用户，尤其是 C 盘的用户目录。

## 二、阶段目标与版本边界

`v2.1.4-hotfix.1` 已关闭旧账户 metadata 到 `WeixinStateStore` 的迁移阻塞，复审结论为通过。`v2.1.5` 现在可以进入“私聊长轮询、配对和幂等”阶段。

本阶段目标：安全接收个人微信私聊，在不接入 YunXi Runtime 的前提下，完成前台 `yunxi weixin serve` 的 iLink `getupdates` 长轮询、消息准入、加密 pending 队列、游标原子提交、配对请求和重复消息幂等防护。

本阶段必须交付：

1. `yunxi weixin serve` 从“未实现保护入口”变为前台长轮询服务。
2. 接入 iLink `getupdates`，尊重服务端 timeout hint，使用带 jitter 的有界退避，Ctrl-C 后及时退出。
3. 将消息批次、加密 `accepted/ready` 队列项和 `get_updates_buf` 游标写入同一次 `WeixinStateStore` 原子提交。
4. 在进入任何 Agent dispatch 之前写入 receipt 和状态转换；本阶段只完成接纳、排队和幂等，不调起 Runtime。
5. 对同一 account + peer + direct-message 会话施加串行锁，同一 message ID 不得并发执行两次。
6. 默认启用 pairing；陌生私聊只生成短时、不透明 request ID，不创建 YunXi session，不写 persona，不写 memory，不触发工具。
7. 群消息、自消息、附件、token 过期、空轮询、队列饱和、网络错误和轮询健康状态都有明确、脱敏、可恢复处理。

本阶段禁止实现或宣称：

- `v2.1.6` Runtime 会话绑定、YunXi session 创建、Agent dispatch、Provider 调用或工具调用。
- `v2.1.7` 远程审批、追问、取消命令桥接。
- `v2.1.8` 微信回信、sendmessage、流式输出合并。
- 群聊、主动推送、附件解析、媒体上传、联系人抓取、Hook、逆向协议或第二套 Agent。

## 三、现有代码状态

当前代码已具备 `v2.1.5` 的底座，但长轮询仍未启动：

| 路径 | 当前状态 | `v2.1.5` 开发要求 |
| --- | --- | --- |
| `D:\YunXi Agent\crates\yunxi-agent-cli\src\weixin.rs` | `serve` 当前检查 state store、lock、加密 pending queue 后直接报未实现；`status/doctor` 已能初始化旧账户 state。 | 将 `serve` 接入前台长轮询循环；启动前必须调用旧账户初始化路径、获取账户锁、检查凭证和加密队列能力。 |
| `D:\YunXi Agent\crates\yunxi-agent-weixin\src\ilink\models.rs` | 已有 `GetUpdatesRequest`、`GetUpdatesResponse`、`WeixinMessage`、`context_token`、`longpolling_timeout_ms`。 | 基于现有模型补齐入站 envelope、消息类型判定、私聊/群聊/附件边界和安全反序列化测试。 |
| `D:\YunXi Agent\crates\yunxi-agent-weixin\src\ilink\client.rs` | 已有 iLink HTTP client 和协议模型调用基础。 | 增加或完善 `get_updates` 长轮询调用、timeout hint、响应大小上限、错误分类、token 过期和脱敏错误。 |
| `D:\YunXi Agent\crates\yunxi-agent-storage\src\weixin_state.rs` | 已有账户、游标、回执、pending delivery、pair、pending inbound、原子写入和账户锁。 | 增加批次接纳原子 API，确保消息、游标、receipt 和 pending inbound 同一提交。 |
| `D:\YunXi Agent\crates\yunxi-agent-weixin\src\secret_store.rs` | 已有系统凭证和 fake store。 | 为 encrypted pending inbound 提供 secret-backed 数据 key 引用；加密能力不可用时 `serve` 必须拒绝启动。 |
| `D:\YunXi Agent\docs\weixin.md` | 已记录登录、状态迁移和当前边界。 | 更新 `v2.1.5` 长轮询、配对、幂等、未接入 Runtime、群聊关闭和故障恢复说明。 |

## 四、推荐 Rust 结构

建议不要把长轮询逻辑堆进 `weixin.rs`。CLI 层只做参数解析、配置复用、输出和退出码，核心能力下沉到微信 crate 和 storage facade：

```text
crates/yunxi-agent-weixin/src/
  serve.rs
  poller.rs
  inbound.rs
  pairing.rs
  backoff.rs
  envelope.rs
  ilink/
    client.rs
    models.rs

crates/yunxi-agent-storage/src/
  weixin_state.rs
```

推荐类型：

- `WeixinServeOptions`
- `WeixinServeLoop`
- `WeixinPoller`
- `WeixinPollOutcome`
- `WeixinInboundEnvelope`
- `WeixinInboundKind`
- `WeixinPeerPolicy`
- `WeixinPairingDecision`
- `WeixinCursorCommit`
- `WeixinInboundBatchCommit`
- `WeixinMessageDeduper`
- `WeixinBackoff`
- `WeixinServeHealth`

CLI `serve` 应复用既有 `AgentConfig`、workspace、provider/sandbox/approval 参数解析和错误脱敏，但本阶段不得把消息交给 `YunXiRuntimeBackend`。

## 五、核心实现要求

### 1. 长轮询循环

`yunxi weixin serve` 必须是前台服务：

- 启动时初始化旧账户 state，读取系统凭证引用，获取账户粒度锁。
- 使用当前 `WeixinStateStore` 中的 `get_updates_buf` 作为游标；没有游标时使用协议允许的初始值。
- 调用 iLink `getupdates`，尊重 `longpolling_timeout_ms`。
- 网络错误、空轮询和服务端错误使用带 jitter 的有界退避。
- token 过期或凭证失效时暂停轮询，写入脱敏错误和健康状态，不自动删除账户。
- Ctrl-C 或进程退出时释放账户锁，写入服务停止状态。

### 2. 入站消息归一化

所有 iLink 消息先归一化为 `WeixinInboundEnvelope`，字段至少包含：

- account hash。
- peer hash。
- message id hash。
- direct-message key。
- created timestamp。
- context token 的 secret reference。
- message kind：text、unsupported_attachment、group_message、self_message、unknown。

日志、JSON、状态和错误中只能使用 hash、计数、状态和脱敏错误。不得输出原始 user id、peer id、context token、消息正文、附件 URL、Provider payload、本地绝对路径或系统凭证 target。

### 3. 游标和 pending 队列同提交

接纳一个 `getupdates` 批次时，必须在同一次 `WeixinStateStore` 原子保存中完成：

1. 写入或更新 `get_updates_buf`。
2. 写入每条被接纳消息的 inbound receipt。
3. 写入加密 pending inbound 项。
4. 写入状态转换时间。
5. 更新轮询健康状态和最后脱敏错误。

如果加密队列写入失败，游标不得推进。这样才能避免“消息丢失但游标已前进”。

### 4. 配对准入

默认启用 pairing：

- 已准入 peer 的文本消息才允许进入 pending inbound。
- 陌生私聊只生成短时、不透明 request ID 的 pair request。
- pair request 必须绑定 account、peer hash、过期时间和状态。
- 未准入消息不得创建 YunXi session，不得写 persona/memory，不得触发工具。
- 群消息必须在代码和配置层关闭，只记录脱敏诊断计数。

本阶段只实现本地 CLI `pair list|approve|deny` 与 `serve` 的准入协同，不实现远程审批或微信文字命令控制。

### 5. 幂等和不确定状态

必须防止重复执行：

- 同一 account + peer + message id 已存在 receipt 时，不再创建第二个 pending inbound。
- 同一 account + peer + direct-message 会话内，pending inbound 只能串行进入 running。
- 崩溃发生在 Provider、工具或外部发送边界时，状态只能标记 `unknown` 或等价诊断，不能宣称恰好一次。
- `terminal` receipt 应采用容量和 TTL 上限清理策略，清理也必须通过 state store 原子写入。

## 六、源码参考与 Rust 化边界

审核报告与总纲建议的关键参考源码均已在本机存在，本次无需新增拉取：

| 参考输入 | 本机状态 | 使用方式 |
| --- | --- | --- |
| `D:\源码\reasonix\internal\bot\weixin\weixin.go` | 已存在 | 参考轮询游标、自消息过滤、context token 重试、账户状态和通道边界。 |
| `D:\源码\reasonix\internal\bot\weixin\weixin_test.go` | 已存在 | 参考轮询超时、重复消息、状态恢复和账户隔离测试组织。 |
| `D:\源码\openclaw-weixin\src\api\api.ts` | 已存在 | 参考 iLink `getupdates`、协议错误、重连和 sendmessage 边界；本阶段不实现 sendmessage。 |
| `D:\源码\openclaw-weixin\src\api\types.ts` | 已存在 | 参考消息、游标、context token 和未知字段策略。 |
| `D:\源码\openclaw-weixin\src\messaging\inbound.ts` | 已存在 | 参考入站消息归一化、私聊/群聊边界和自消息过滤。 |
| `D:\源码\openclaw-weixin\src\messaging\process-message.ts` | 已存在 | 参考消息准入与处理顺序，不引入 OpenClaw Agent runtime。 |
| `D:\源码\openclaw-weixin\src\auth\pairing.ts` | 已存在 | 参考 pairing request、过期、白名单和 request ID 逻辑。 |
| `D:\源码\openclaw-weixin\src\storage\sync-buf.ts` | 已存在 | 参考游标和同步缓冲恢复边界。 |
| `D:\源码\openclaw-weixin\src\monitor\monitor.ts` | 已存在 | 参考轮询健康、重连和退避观察方式。 |
| `D:\YunXi Agent\crates\yunxi-agent-storage\src\weixin_state.rs` | 已存在 | 复用当前 state store、原子提交、锁、pair 和 pending inbound 模型。 |

所有 TypeScript 和 Go 源码只能提取协议、状态机和测试思路，必须使用 Rust 2024、serde、tokio、reqwest 和现有 YunXi crate 边界重新实现。不得复制 OpenClaw 宿主、Reasonix gateway/controller、明文凭证落盘、群聊、媒体链路、插件系统或第二套 Runtime。

## 七、测试与验收要求

开发完成后至少通过：

| 类别 | 验收要求 |
| --- | --- |
| 格式与编译 | `cargo fmt --all -- --check`、`cargo check --workspace`、`cargo build --workspace --release`。 |
| 全量测试 | `cargo test --workspace -- --test-threads=1`。 |
| 微信定向测试 | iLink mock 覆盖 getupdates 成功、空轮询、timeout hint、网络错误、token 过期、响应过大、未知字段和脱敏错误。 |
| storage 定向测试 | 批次接纳、游标推进、pending inbound、receipt、重复消息、终态 TTL/容量清理、原子失败和重启恢复。 |
| CLI 集成测试 | `serve` 启动、Ctrl-C、账户锁、旧 metadata 初始化、凭证缺失拒绝、加密不可用拒绝、pair list/approve/deny 协同。 |
| 配对测试 | 陌生私聊只生成 request ID；未准入消息不创建 session/persona/memory/tool；过期、重复、跨账户 request ID 均失败。 |
| 安全搜索 | stdout、stderr、JSON、日志、报告证据和状态文件不得出现 token、data key、二维码 payload、context token、原始 user id、原始 peer id 或原始消息正文。 |
| 回归 | 既有 CLI、JSON/JSONL、Provider、companion eval、TUI/ConPTY、sessions、persona、memory 均不得回归。 |

真实微信联调如执行，只能记录脱敏账户 hash、状态、计数、退出码和是否触发网络；不得记录联系人、消息正文、context token、二维码、凭证或系统 secret。

## 八、发布、清理与日志要求

`v2.1.5` 完成后必须：

1. 统一验证通过前不能宣称完成。
2. 更新 `README.md`、`docs/weixin.md`、报告索引、状态文档和开发日志，使文档与实际代码状态一致。
3. 阶段结束后清理编译中间产物，尤其是 `target` 和临时测试目录；涉及递归删除、清空目录或 `git clean` 前必须得到用户对精确绝对路径的确认。
4. 提交新的发布 commit。
5. 创建新的 annotated `v2.1.5` tag。
6. 推送 commit 和 tag。
7. 不移动、覆盖或删除 `v2.1.4-hotfix.1`、`v2.1.4` 或任何历史 tag。
8. 日志记录必须包含时间戳、工作目标、执行流程、修改文件、文件路径、验证结果、提交和推送状态，并以“开发报告撰写者”署名。

`v2.1.5` 审核通过前，任何文档、日志、发布说明或回复都不得宣称 Runtime 会话绑定、远程审批、流式回信、sendmessage、群聊或完整微信聊天闭环已经完成。

署名：开发报告撰写者

## 九、实现与本地发布门禁记录

- 记录时间：2026-07-28 08:58:15 +08:00
- 执行者署名：开发者
- 当前状态：实现、验证和构建产物清理已完成；release commit、annotated tag 和 GitHub 推送待执行。

### 实现摘要

1. `yunxi weixin serve` 已从未实现保护入口改为前台私聊长轮询服务入口。
2. weixin crate 新增入站 envelope、消息分类、带 jitter 的有界 backoff、serve transport trait 和 `run_weixin_serve_loop`。
3. storage crate 新增 `WeixinInboundBatchCommit` 原子提交接口，批次内同时保存游标、receipt、pending inbound、pair request、连接状态和脱敏错误。
4. 陌生私聊只生成短时、不透明 pair request；已准入私聊文本进入 encrypted pending inbound；重复 message id 幂等跳过。
5. token 过期或凭证失效只写入脱敏 suspended/credential health，不删除账户或凭证。
6. CLI `status/doctor/serve` 输出能力口径已更新为 v2.1.5 前台私聊接纳能力，同时继续明确 Runtime dispatch、sendmessage、远程审批、群聊、附件和主动推送未启用。

### 修改路径

- `D:\YunXi Agent\Cargo.toml`
- `D:\YunXi Agent\Cargo.lock`
- `D:\YunXi Agent\README.md`
- `D:\YunXi Agent\crates\yunxi-agent-cli\src\main.rs`
- `D:\YunXi Agent\crates\yunxi-agent-cli\src\weixin.rs`
- `D:\YunXi Agent\crates\yunxi-agent-cli\tests\cli_tests.rs`
- `D:\YunXi Agent\crates\yunxi-agent-runtime\tests\general_companion_tests.rs`
- `D:\YunXi Agent\crates\yunxi-agent-storage\src\lib.rs`
- `D:\YunXi Agent\crates\yunxi-agent-storage\src\weixin_state.rs`
- `D:\YunXi Agent\crates\yunxi-agent-storage\tests\weixin_state_tests.rs`
- `D:\YunXi Agent\crates\yunxi-agent-tui\src\app.rs`
- `D:\YunXi Agent\crates\yunxi-agent-tui\src\render.rs`
- `D:\YunXi Agent\crates\yunxi-agent-tui\src\snapshots\full_frame_58x18.txt`
- `D:\YunXi Agent\crates\yunxi-agent-tui\src\snapshots\full_frame_80x24.txt`
- `D:\YunXi Agent\crates\yunxi-agent-tui\src\snapshots\full_frame_100x30.txt`
- `D:\YunXi Agent\crates\yunxi-agent-tui\src\snapshots\full_frame_120x40.txt`
- `D:\YunXi Agent\crates\yunxi-agent-tui\src\snapshots\full_frame_200x50.txt`
- `D:\YunXi Agent\crates\yunxi-agent-weixin\Cargo.toml`
- `D:\YunXi Agent\crates\yunxi-agent-weixin\src\lib.rs`
- `D:\YunXi Agent\crates\yunxi-agent-weixin\src\backoff.rs`
- `D:\YunXi Agent\crates\yunxi-agent-weixin\src\inbound.rs`
- `D:\YunXi Agent\crates\yunxi-agent-weixin\src\serve.rs`
- `D:\YunXi Agent\crates\yunxi-agent-weixin\tests\ilink_client_tests.rs`
- `D:\YunXi Agent\docs\README.md`
- `D:\YunXi Agent\docs\weixin.md`
- `D:\YunXi Agent\docs\reports\README.md`
- `D:\YunXi Agent\docs\reports\audits\2026-07-27-230718-yunxi-agent-v2-1-4-hotfix-1-weixin-state-migration-reaudit-report.md`
- `D:\YunXi Agent\docs\reports\development\2026-07-27-233307-yunxi-agent-v2-1-5-weixin-long-polling-pairing-idempotency-development-report.md`
- `D:\YunXi Agent\docs\development-log.md`

### 本地验证结果

- `cargo fmt --all -- --check`：通过。
- `cargo check --workspace`：通过。
- `cargo test --workspace -- --test-threads=1`：通过。
- `cargo test -p yunxi-agent-storage -- --test-threads=1`：通过。
- `cargo test -p yunxi-agent-weixin -- --test-threads=1`：通过。
- `cargo test -p yunxi-agent-cli -- --test-threads=1`：通过。
- `cargo test -p yunxi-agent-provider -- --test-threads=1`：通过。
- `cargo test -p yunxi-agent-tui -- --test-threads=1`：通过。
- `cargo build --workspace --release`：通过，`target\release\yunxi.exe --version` 输出 `yunxi 2.1.5`。
- `target\release\yunxi.exe weixin status --account default --json`：通过，脱敏输出，`receive_messages=true`、`foreground_long_polling=true`、`send_messages=false`、`group_chat=false`、`secrets_included=false`。
- `target\release\yunxi.exe weixin doctor --account default --json`：通过，`message_receive_enabled=true`、`message_send_enabled=false`、`group_chat_enabled=false`、`network_request_performed=false`、`secrets_included=false`。
- `YUNXI_WEIXIN_SERVE_MAX_POLLS=1 target\release\yunxi.exe weixin serve --account default --json`：通过，1 次前台长轮询后以 `max_polls` 停止；本机网络结果记录为脱敏 `network_error_count=1`；未启用 Runtime dispatch、sendmessage、远程审批或群聊。
- `target\release\yunxi.exe eval companion --json`：通过，31/31。
- 真实 DeepSeek Provider smoke：通过，marker `YUNXI_V215_REAL_PROVIDER_OK` 检测成功，`secret_leak_detected=false`。
- `npm.cmd run verify --prefix scripts\conpty\v210`：通过，`ok=true`、`read_only=true`。
- 本地 Markdown 入口链接检查、默认 CLI dependency tree Codex 依赖检查、变更文件 secret 扫描、`git diff --check`、`git fsck --full --no-dangling`：均通过。

### 清理与安全状态

已按用户确认清理精确路径 `D:\YunXi Agent\target`，删除前校验其位于项目目录下，删除后确认不存在。未触碰 `D:\YunXi Agent\.yunxi`、C 盘用户目录、Git 历史、历史 tag、正式 evidence、系统凭证或 Windows 配置。未使用 force，未移动或覆盖历史 tag。

### 仍保持禁止声明的能力

本阶段没有实现也不得宣称 Runtime 会话绑定、YunXi session 创建、Agent dispatch、Provider 调用、工具调用、sendmessage/微信回信、远程审批、流式回信、群聊、附件/媒体上传或主动推送已经完成。

### 提交、推送和 tag 状态

截至本记录写入时，release commit、annotated `v2.1.5` tag 和 GitHub 推送尚未执行；下一步执行提交、创建新 tag 和远端核验。历史 tag 不删除、不移动、不覆盖。

署名：开发者

## 十、发布收口记录

- 记录时间：2026-07-28 09:05:08 +08:00
- 执行者署名：开发者

### GitHub 发布结果

- release commit：`44d89488d421748527547f52faa73caad84ef6b2`
- annotated tag：`v2.1.5`
- tag object：`03d9bd6d7985dddb7719da6ae0cd3d25a717dcb9`
- tag target commit：`44d89488d421748527547f52faa73caad84ef6b2`
- 远端仓库：`https://github.com/sjxbbdb/YunXi-Agent`
- 推送方式：GitHub API key 仅进入当前进程 `GH_TOKEN`，通过 GitHub CLI credential helper `gh auth git-credential` 供 git HTTPS 推送使用；未打印、未落盘、未写 git config。
- 推送结果：`master -> master`，`[new tag] v2.1.5 -> v2.1.5`。
- 远端核验：`master_matches=true`、`tag_object_matches=true`、`tag_target_matches=true`。

### 历史 tag 安全状态

本次仅新增 annotated `v2.1.5` tag；未删除、未移动、未覆盖 `v2.1.4-hotfix.1`、`v2.1.4` 或任何历史 tag，未使用 force。

### 发布后文档状态

本条发布收口记录作为 docs-only 追加记录写入项目开发日志和本报告；后续只推送 `master` 上的文档收口提交，不移动已经发布的 `v2.1.5` tag。

署名：开发者
