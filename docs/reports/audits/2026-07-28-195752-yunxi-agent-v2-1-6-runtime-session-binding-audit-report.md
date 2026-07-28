# YunXi Agent v2.1.6 微信会话绑定既有 Runtime 审核报告

- 审核时间：2026-07-28 19:57:52 +08:00
- 审核对象：当前项目实际源码状态，目标核验总纲中的 `v2.1.6`
- 总纲正本：`D:\YunXi Agent\docs\superpowers\plans\2026-07-22-yunxi-agent-v2-1-1-to-v2-2-0-personal-wechat-roadmap.md`
- 开发目录：`D:\YunXi Agent`
- 审核范围：只对照总纲 `v2.1.6`，不进行多个版本横向比较

## 一、审核结论

**审核不通过，禁止进入 `v2.1.7`。**

本次通知要求审核，但仓库实际仍处于 `v2.1.5-hotfix.2`：`Cargo.toml` 的 workspace
版本为 `2.1.5-hotfix.2`，HEAD 为 `e4f00e1`，且 HEAD 仍指向 annotated tag
`v2.1.5-hotfix.2`。当前源码和报告目录没有发现 `v2.1.6` 的实现提交、开发报告或发布
Tag。因此本次不能把 `v2.1.5-hotfix.2` 的既有微信轮询和状态能力认定为 `v2.1.6` 的
Runtime 会话接入完成。

当前不是“已有 v2.1.6 功能有缺陷”，而是“v2.1.6 尚未形成可审核版本”。在开发者完成
下列实现、统一验证并创建新的 annotated `v2.1.6` Tag 前，项目必须停留在
`v2.1.5-hotfix.2`，不得开始 `v2.1.7`。

## 二、版本与源码基线

| 核验项 | 结果 | 证据 |
| --- | --- | --- |
| workspace 版本 | 未进入 v2.1.6 | `Cargo.toml` 为 `2.1.5-hotfix.2` |
| HEAD | 未进入 v2.1.6 | `e4f00e1 (HEAD -> master, tag: v2.1.5-hotfix.2)` |
| v2.1.6 开发报告 | 未发现 | `docs/reports/development/` 无 v2.1.6 文件 |
| v2.1.6 Tag | 未创建 | 当前最近版本 Tag 为 `v2.1.5-hotfix.2` |
| v2.1.6 生产源码 | 未发现 | 全库搜索未发现 `WeixinConversationBinding`、`WeixinTurnSupervisor` |
| 当前基线回归 | 通过 | 格式检查、workspace check、workspace 全量测试均通过 |

当前工作树中的变更仅涉及此前审核报告、报告索引和开发日志等文档；本次未发现
`crates/` 下有对应 v2.1.6 的生产源码或集成测试变更。

## 三、总纲逐项审核

### 1. `WeixinConversationBinding`：未实现，阻塞

总纲要求在 `yunxi-agent-storage` 中增加会话绑定，将 `account + peer + dm` 映射到既有
`SessionId`，记录最后活动时间和稳定来源标签，并继续使用当前 session history/parent
规则。

当前 `yunxi-agent-weixin` 仅有 `WeixinConversationKey`（账户和对端维度），
`yunxi-agent-storage` 仍是 v2.1.5 的账户、游标、pair、pending 和锁状态存储；没有绑定
记录、SessionId 映射、来源标签或 last-activity 更新。不能证明同一对端会复用同一会话，
也不能证明不同账户/对端会严格隔离。

### 2. `WeixinTurnSupervisor`：未实现，阻塞

总纲要求微信端取得已准入消息，使用现有 `AgentInput::text` 和
`Agent::run_with_backend_stream`，通过既有 Runtime 执行，不复制第二套 Agent、memory、
persona 或审批系统，并先以测试 sink 接收最终文本。

当前 `yunxi-agent-weixin` 的 `serve` 只负责轮询、消息分类、配对和加密 pending 入站，
没有 turn supervisor、Agent dispatch、fake backend 接入或最终文本 sink。全库现有
`run_with_backend_stream` 调用仍属于 Runtime/CLI 测试及本地 CLI 路径，未形成微信到既有
Runtime 的调用链。

### 3. CLI 与微信服务共享运行配置：未实现，阻塞

总纲要求 `yunxi run` 与 `weixin serve` 复用同一套 Provider、model、cwd、sandbox、
approval、context window 和 companion 配置构造路径，只抽取必要 CLI 构造代码。

当前 `weixin serve` 尚未接入 Agent Runtime，因此不存在可验收的共享构造路径，也没有
测试证明微信调用和本地 CLI 获得相同 Provider、sandbox、approval 与 cwd。

### 4. 会话串行与有界队列：未实现，阻塞

总纲要求按 `WeixinConversationKey` 串行执行，同一陪伴会话不能交叉 turn 或交叉记忆
写入；不同已准入私聊可以独立运行；队列必须有界。

当前源码没有微信 turn 队列、per-conversation supervisor、串行锁或队列容量测试。
`pending_inbound` 的持久化队列不能替代 Runtime turn 的串行执行证明。

### 5. 测试 sink 与版本边界：未实现，阻塞

总纲要求 v2.1.6 先使用测试 sink 投递最小最终文本，不在本版本逐 token 发送微信。
当前没有微信 Runtime 结果进入测试 sink 的实现或测试，也没有 v2.1.6 的功能边界测试。

本版本仍不得提前实现：

- v2.1.7 的 `/status`、`/stop`、`/approve`、`/deny`、`/answer` 远程控制和远程审批；
- v2.1.8 的 `sendmessage`、逐 token 或合并后的流式微信回信；
- v2.1.9 的新 persona、memory、companion 体系；
- 群聊、企业微信、公众号或第二套通道网关。

### 6. fake backend 集成验收：未实现，阻塞

总纲要求 fake backend 集成测试至少证明：

- 同一账户、同一私聊对端的多次消息复用同一个 `SessionId`；
- 不同账户或不同对端严格隔离；
- 微信 Runtime 得到的 Provider、sandbox、approval 和 cwd 与本地 CLI 一致；
- 既有 `yunxi run`、交互/TUI、Provider、companion 路径无回归。

当前测试目录没有这些 v2.1.6 集成测试，故不能以现有 v2.1.5 测试替代验收。

## 四、已执行验证

本轮对当前实际源码执行：

- `cargo fmt --all -- --check`：通过；
- `cargo check --workspace`：通过；
- `cargo test --workspace -- --test-threads=1`：通过，无失败测试；
- 全库符号和测试搜索：未发现 v2.1.6 会话绑定、turn supervisor、微信 Runtime dispatch
  或 fake backend 集成实现。

上述结果只证明 `v2.1.5-hotfix.2` 基线没有回归，不证明 `v2.1.6` 已完成。由于目标版本
尚未形成源码提交，本轮未执行 v2.1.6 的 release build、真实微信 Runtime 联调或
Session 复用验收。

## 五、开发者整改要求

1. 在 `D:\YunXi Agent\crates\yunxi-agent-storage\src\weixin_state.rs` 或其合适的
   storage 边界实现可持久化的 `WeixinConversationBinding`，明确 `account + peer + dm`
   主键、既有 `SessionId`、last activity、source label、schema 迁移、原子写入和恢复。
2. 在 `D:\YunXi Agent\crates\yunxi-agent-weixin` 实现 `WeixinTurnSupervisor`，只接收
   已配对且恢复成功的私聊，构造 `AgentInput::text`，调用既有
   `Agent::run_with_backend_stream`，将最终文本交给测试 sink；不得复制 Runtime。
3. 从 `D:\YunXi Agent\crates\yunxi-agent-cli\src\main.rs` 抽取最小共享运行配置构造，
   让 `run` 与 `weixin serve` 使用同一 Provider、model、cwd、sandbox、approval、context
   window 和 companion 配置。
4. 用 `WeixinConversationKey` 实现 per-conversation 串行和有界队列，并证明同一会话不会
   交叉 turn 或交叉 memory 写入，不同已准入私聊可以并行。
5. 使用 fake backend 完成账户/对端隔离、SessionId 复用、配置一致性和测试 sink 集成
   测试；同时保留现有 CLI、TUI、Provider、companion、安全凭证和微信状态回归。
6. 完成统一验证后，更新开发报告、索引和日志，创建新的 annotated `v2.1.6` Tag；在
   审核通过前不得宣称完成或进入 `v2.1.7`。不要删除、移动或覆盖历史 Tag。

## 六、参考源码与 Rust 化建议

以下路径是本版本必须明确参考的输入。非 Rust 项目只提取协议、状态机、会话绑定和测试
思路，必须用 Rust 2024、trait、明确错误类型和 Tokio 同步原语重新实现，不得引入宿主
Runtime。

- `D:\YunXi Agent\crates\yunxi-agent-runtime\src\lib.rs`：复用现有 backend stream 和
  `Agent::run_with_backend_stream`，只抽取共享构造，不复制 Runtime。
- `D:\YunXi Agent\crates\yunxi-agent-storage\src\lib.rs`：复用既有 `SessionStore`、
  `SessionId` 和 history/parent 兼容边界。
- `D:\YunXi Agent\crates\yunxi-agent-storage\src\weixin_state.rs`：承载微信绑定状态、
  schema、原子更新和恢复边界。
- `D:\YunXi Agent\crates\yunxi-agent-cli\src\main.rs`：抽取 `run` 与 `weixin serve`
  共用的 Provider、sandbox、approval、cwd 和 companion 配置构造。
- `D:\源码\reasonix\internal\bot\types.go`：参考消息归一化和会话标识；Rust 化为明确
  的微信输入、会话键和绑定结构。
- `D:\源码\reasonix\internal\bot\gateway.go`：参考通道到核心的路由与串行边界；Rust 化
  为显式 supervisor、任务生命周期和错误传播。
- `D:\源码\reasonix\internal\botruntime\runtime.go`：参考已有会话映射和 Runtime 交接；
  只迁移逻辑，不引入 Go Runtime。
- `D:\源码\CowAgent\channel\channel_factory.py` 与 `D:\源码\CowAgent\channel\weixin\`：
  参考个人陪伴通道到核心的交接和所有权；不引入 Python channel factory。
- `D:\源码\openclaw-weixin\src\storage\sync-buf.ts`、`state-dir.ts`：参考 pending、
  状态恢复和队列边界；不得复制 Node/OpenClaw 宿主。

## 七、清理、安全与发布状态

- 本轮未修改生产 Rust 源码、测试源码、凭证、用户微信 state、系统配置或历史 Tag。
- `cargo check` 和全量测试继续使用 `D:\YunXi Agent\target`；按固定约束，删除该目录
  需要用户对精确绝对路径单独确认，本轮未执行任何删除、`git clean` 或清空操作。
- 本轮未创建 commit、未推送、未创建或移动 Tag。
- 本轮需新增的项目报告、索引和日志属于 docs-only 变更；桌面文件为项目正本的分发副本。

署名：审核者
