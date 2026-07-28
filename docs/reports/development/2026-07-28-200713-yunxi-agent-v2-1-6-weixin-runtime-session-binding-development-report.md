# YunXi Agent v2.1.6 微信会话绑定既有 Runtime 开发报告

- 撰写时间：2026-07-28 20:07:13 +08:00
- 审核依据：`C:\Users\24763\Desktop\YunXi Agent审核报告\2026-07-28-195752-YunXi-Agent-v2.1.6-微信会话绑定Runtime审核报告.md`
- 项目内审核报告归档：`D:\YunXi Agent\docs\reports\audits\2026-07-28-195752-yunxi-agent-v2-1-6-runtime-session-binding-audit-report.md`
- 审核报告 SHA-256：`305F3F9B0E67E57FE0E8BABDF2C0359DF8733FACEBBDFBEC615F1618660CE4F3`
- 总纲正本：`D:\YunXi Agent\docs\superpowers\plans\2026-07-22-yunxi-agent-v2-1-1-to-v2-2-0-personal-wechat-roadmap.md`
- 总纲 SHA-256：`2DF30D503F46CFE7496567F5011BF5CBFB8BA73C91C2D2FA3E9F29AF0932EA0F`
- 当前工作树基线：`HEAD=e4f00e1a1f4581b390957934e63d1ca877090fa4`，`git describe=v2.1.5-hotfix.2-dirty`
- 审核结论转化：`v2.1.6` 审核不通过；当前仓库实际仍为 `v2.1.5-hotfix.2`，`v2.1.6` 尚未形成可审核实现。
- 目标版本：`v2.1.6`
- 必须创建的 tag：新的 annotated `v2.1.6`
- 开发目录：`D:\YunXi Agent`
- 报告类型：面向开发者的下一阶段开发指令；本报告不是完成声明。

## 一、硬性约束

以下约束是 `v2.1.6` 开发、验证、发布和文档同步的前置要求，不得因为上下文过长、任务拆分、实现困难或局部修复而忽略、弱化或绕过。

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

## 二、审核结论与阶段目标

本次 `v2.1.6` 审核不通过的原因不是“已有 Runtime 绑定实现存在缺陷”，而是当前仓库仍停留在 `v2.1.5-hotfix.2`。审核报告确认：

- workspace 版本仍是 `2.1.5-hotfix.2`。
- 当前 HEAD 指向 `v2.1.5-hotfix.2`。
- 没有 `v2.1.6` 开发报告或发布 tag。
- 没有 `WeixinConversationBinding`、`WeixinTurnSupervisor`、微信到既有 Runtime 的调用链、共享配置、会话串行有界队列或 fake backend 集成测试。

因此开发者必须实际完成总纲 `v2.1.6`：让已准入微信私聊使用与本地 CLI 相同的持久 companion session，而不是创建第二套聊天或第二套 Runtime。

本阶段必须交付：

1. `WeixinConversationBinding`：将 `account + peer + dm` 映射到既有 `SessionId`。
2. `WeixinTurnSupervisor`：从已解密、已准入的 pending inbound 构造 `AgentInput::text`，调用既有 `Agent::run_with_backend_stream`。
3. 共享运行配置：`yunxi run` 与 `yunxi weixin serve` 复用同一 Provider、model、cwd、sandbox、approval、context window 和 companion 配置构造路径。
4. per-conversation 串行与有界队列：同一微信私聊不得交叉 turn 或交叉记忆写入，不同已准入私聊可独立运行。
5. 测试 sink：本版本只把 Runtime 最小最终文本投递到测试 sink，不向微信 sendmessage，不做逐 token 流式回信。
6. fake backend 集成测试：证明同一对端复用一个 `SessionId`，不同账户/对端严格隔离，Runtime 配置与本地 CLI 一致。

本阶段禁止实现或宣称：

- `v2.1.7` 远程文字控制、审批、追问、取消。
- `v2.1.8` 微信 sendmessage、逐 token 流式输出或合并回信。
- `v2.1.9` 新 persona、memory、companion 体系。
- 群聊、企业微信、公众号、多渠道网关、第二套 Agent、第二套 Runtime 或第二套审批系统。

## 三、代码接入点

| 路径 | 当前职责 | `v2.1.6` 开发要求 |
| --- | --- | --- |
| `D:\YunXi Agent\crates\yunxi-agent-storage\src\weixin_state.rs` | 微信账户、游标、pair、pending inbound、锁和原子 state。 | 新增 `WeixinConversationBinding`，包含 account、peer、dm key、`SessionId`、source label、last activity、schema、原子更新和恢复。 |
| `D:\YunXi Agent\crates\yunxi-agent-storage\src\lib.rs` | 既有 `SessionStore`、`SessionRecord`、`SessionId` 和 history/parent 兼容边界。 | 复用既有 session 体系，不向 `SessionRecord` 硬塞微信字段；绑定记录应独立保存并引用既有 `SessionId`。 |
| `D:\YunXi Agent\crates\yunxi-agent-weixin\src\serve.rs` | `getupdates` 轮询、加密 pending inbound、pair 和状态提交。 | 在已准入 pending 解密后交给 `WeixinTurnSupervisor`；未准入消息仍不得进入 Runtime。 |
| `D:\YunXi Agent\crates\yunxi-agent-weixin\src\payload_cipher.rs` | pending inbound 认证加密和解密恢复。 | 复用 `decrypt_pending_inbound`，只把恢复出的安全文本交给 supervisor，不写入日志或普通状态。 |
| `D:\YunXi Agent\crates\yunxi-agent-core\src\runner.rs` | `Agent::run_with_backend_stream`。 | 作为唯一 Runtime 调用入口；不得复制 Runtime 或新建第二套执行引擎。 |
| `D:\YunXi Agent\crates\yunxi-agent-core\src\input.rs` | `AgentInput::text`。 | 微信已准入文本应转换成同一 `AgentInput::text` 形态，保持 CLI 输入兼容。 |
| `D:\YunXi Agent\crates\yunxi-agent-cli\src\main.rs` | 本地 CLI `run` 的 Provider、sandbox、approval 和 cwd 构造。 | 抽取最小共享构造 helper，供 `run` 与 `weixin serve` 使用同一路径。 |
| `D:\YunXi Agent\crates\yunxi-agent-weixin\tests\` 或 CLI 集成测试 | 微信模块集成测试。 | 增加 fake backend、测试 sink、SessionId 复用、隔离、串行队列和配置一致性验收。 |
| `D:\YunXi Agent\docs\weixin.md`、`README.md`、`docs\README.md` | 用户和开发者能力边界。 | 更新为 `v2.1.6` 会话绑定实际状态；不得写成远程审批、回信或群聊完成。 |

## 四、推荐实现方案

### 1. `WeixinConversationBinding`

建议在 `WeixinStateSnapshot` 中新增独立绑定记录，或在同一 state store facade 下新增绑定集合：

```text
WeixinConversationBinding {
  schema_version,
  account_id,
  peer_id_hash,
  direct_message_key,
  workspace_id,
  session_id,
  source_label,
  created_at_millis,
  last_activity_millis,
  updated_at_millis,
}
```

约束：

- 主键必须包含 account、peer 和 dm key，避免跨账户或跨对端串话。
- `session_id` 必须引用既有 YunXi session 体系。
- 不得把微信字段塞进旧 `SessionRecord`。
- 绑定创建、更新 last activity、pending 状态转换应通过 `WeixinStateStore` 原子写入。
- 旧 state schema 和未来 schema 仍要安全迁移或拒绝。

### 2. `WeixinTurnSupervisor`

建议在 `crates\yunxi-agent-weixin\src\turn_supervisor.rs` 或等价模块新增 supervisor：

```text
WeixinTurnSupervisor {
  bind_or_load_session(...)
  enqueue_turn(...)
  run_ready_turn(...)
  write_test_sink(...)
}
```

行为：

- 只处理已配对、已解密、非终态 pending inbound。
- 使用 `AgentInput::text` 构造输入。
- 使用既有 `Agent::run_with_backend_stream` 调用 Runtime。
- 使用测试 sink 接收最终文本，暂不调用微信 sendmessage。
- Runtime 失败、取消或 unknown 结果必须写入 pending 状态和脱敏错误。
- 不展示隐藏推理、原始工具输出、环境变量、token 或未脱敏本地路径。

### 3. 共享运行配置

开发者需要从 CLI `run` 路径抽取最小共享配置构造，形成可测试 facade：

```text
build_yunxi_runtime_invocation(config, provider_args, sandbox_args, approval_args)
```

验收点：

- 本地 `yunxi run` 和 `yunxi weixin serve` 对同一输入获得同一 Provider、model、cwd、sandbox、approval、context window 和 companion 开关。
- 不新增第二套 Provider 选择逻辑。
- 不绕过现有 approval/sandbox 规则。
- 不为微信自然语言隐式提高权限。

### 4. 串行与有界队列

必须以 `WeixinConversationKey` 为边界实现串行：

- 同一 account + peer + dm key 同时只能运行一个 turn。
- 同一会话多个 pending inbound 按顺序执行。
- 不同已准入对端可以独立运行，但需要全局容量上限。
- 队列满时写入脱敏诊断，不能丢消息后推进状态。
- 不得交叉写 memory、persona 或 session history。

## 五、测试与验收要求

开发完成后至少通过：

| 类别 | 验收要求 |
| --- | --- |
| 绑定测试 | 同一账户同一对端复用同一 `SessionId`；不同账户或不同对端严格隔离。 |
| Runtime 调用 | fake backend 证明微信输入通过 `AgentInput::text` 和 `Agent::run_with_backend_stream` 进入既有 Runtime。 |
| 配置一致性 | 微信 Runtime 获得的 Provider、sandbox、approval、cwd、model、companion 与对应本地 CLI 一致。 |
| 串行队列 | 同一会话不交叉 turn，不交叉 memory/session 写入；队列容量和背压有测试。 |
| 测试 sink | Runtime 最小最终文本进入测试 sink；本版本不调用 sendmessage，不做逐 token 微信输出。 |
| pending 状态 | ready/running/terminal 状态转换可恢复，崩溃和 fake backend 失败写入脱敏错误。 |
| 安全边界 | 未准入消息、群消息、自消息、附件、未知消息不得触发 Runtime/session/persona/memory/tool。 |
| 回归 | `cargo fmt --all -- --check`、`cargo check --workspace`、`cargo test --workspace -- --test-threads=1`、storage/weixin/CLI/provider/TUI 定向测试、companion eval、ConPTY、Provider smoke、`git diff --check` 全部通过。 |
| 发布 | release build 输出 `yunxi 2.1.6`，创建并推送新的 annotated `v2.1.6` tag，远端 tag object 和 peeled target 一致。 |

验证通过前不能宣称 `v2.1.6` 完成，也不能进入 `v2.1.7`。

## 六、源码参考状态与使用边界

本次核对的参考源码状态如下：

| 参考输入 | 本机状态 | 使用方式 |
| --- | --- | --- |
| `D:\YunXi Agent\crates\yunxi-agent-runtime\src\lib.rs` | 已存在 | 复用既有 backend stream 和 Runtime 边界，不复制 Runtime。 |
| `D:\YunXi Agent\crates\yunxi-agent-storage\src\lib.rs` | 已存在 | 参考 `SessionStore`、`SessionId`、history/parent 兼容边界。 |
| `D:\YunXi Agent\crates\yunxi-agent-storage\src\weixin_state.rs` | 已存在 | 增加微信绑定状态，继续独立于 `SessionRecord`。 |
| `D:\YunXi Agent\crates\yunxi-agent-cli\src\main.rs` | 已存在 | 抽取 `run` 与 `weixin serve` 共用配置构造。 |
| `D:\源码\reasonix\internal\bot\types.go` | 已存在 | 参考消息归一化、会话键和绑定结构。 |
| `D:\源码\reasonix\internal\bot\gateway.go` | 已存在 | 参考通道到核心的路由和串行边界。 |
| `D:\源码\reasonix\internal\botruntime\runtime.go` | 已存在 | 参考会话映射和 Runtime 交接思路，必须 Rust 化。 |
| `D:\源码\openclaw-weixin\src\storage\sync-buf.ts`、`state-dir.ts` | 已存在 | 参考 pending、状态恢复和队列边界。 |
| `D:\源码\CowAgent\channel\channel_factory.py`、`D:\源码\CowAgent\channel\weixin\` | 已存在，浅克隆；remote `https://github.com/zhayujie/CowAgent.git`，HEAD `6185c73f8d2764f78a135f8fd6df35d4e246a6b7` | 参考个人陪伴通道到核心的交接和所有权；不得引入 Python channel factory 或第二套 Runtime。 |

所有 Go、Python 和 TypeScript 源码只能提取架构、状态机、会话绑定和测试思路，必须用 Rust 2024、trait、明确错误类型和 Tokio 同步原语重新实现。不得引入 Go Runtime、Python channel factory、OpenClaw 宿主、第二套 Agent、第二套记忆或第二套审批系统。

## 七、发布、清理与日志要求

开发完成后必须：

1. 完成一批实现后统一验证，不要在单点上反复纠结。
2. 更新 `Cargo.toml` workspace 版本为 `2.1.6`，并确保 release CLI 输出 `yunxi 2.1.6`。
3. 更新 `README.md`、`docs/README.md`、`docs/weixin.md`、报告索引、状态文档和开发日志，使文档与实际代码一致。
4. 创建新的发布 commit。
5. 创建新的 annotated `v2.1.6` tag。
6. 推送 commit 和 tag，并核验远端 master、tag object 和 peeled target。
7. 不移动、覆盖或删除 `v2.1.5-hotfix.2` 或任何历史 tag。
8. 阶段结束后清理编译中间产物；涉及递归删除、清空目录或 `git clean` 前必须得到用户对精确绝对路径的确认。
9. 日志必须记录时间戳、工作目标、执行流程、修改文件、文件路径、验证结果、提交和推送状态，并以“开发报告撰写者”署名。

`v2.1.6` 审核通过前，任何文档、日志、发布说明或回复都不得宣称远程审批、sendmessage、流式回信、群聊或完整微信聊天闭环已经完成。

署名：开发报告撰写者
