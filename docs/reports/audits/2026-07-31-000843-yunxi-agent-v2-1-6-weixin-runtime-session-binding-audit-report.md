# YunXi Agent v2.1.6 微信会话绑定与既有 Runtime 审核报告

- 审核时间：2026-07-31 00:08:43 +08:00
- 审核版本：v2.1.6
- 审核依据：v2.1.1 至 v2.2.0 微信端接入总纲图中的 v2.1.6 要求
- 审核范围：当前仓库源码、测试、构建产物、版本标记及文档状态
- 审核者：审核者

## 一、审核结论

**审核不通过。当前版本禁止进入 v2.1.7 开发，必须先完成 v2.1.6 整改并重新审核。**

本版本的会话绑定数据结构、Provider 与本地 CLI Runtime 配置复用、测试 sink 边界和基础构建质量均已具备；但总纲要求的“复用既有 companion session”尚未形成可证明的连续历史链路，且微信 serve 在队列满时会推进网络游标后留下无法自动恢复的 Ready pending。两项均属于当前版本的核心行为阻塞点，不是发布顺序或版本号问题。

## 二、当前版本基线

| 项目 | 审核结果 |
| --- | --- |
| Cargo 工作区版本 | 2.1.6 |
| 当前 HEAD | 9e1fc57，docs: record v2.1.6 release closure |
| v2.1.6 tag | 存在，tag object 为 5fb58d4fc91ba5636f871d3474df7ed14679567b |
| tag target | 6b16d4cab6257424cf9b3d8d5b6d297c3e9beaad |
| 工作树 | 审核开始时干净 |
| 审核源代码改动 | 本轮未修改 Rust 源码 |
| 审核对象 | 只对照 v2.1.6，不横向比较多个版本 |

当前 HEAD 是 v2.1.6 发布闭环的文档提交，源码主体位于 v2.1.6 tag target。版本标记未发现异常，但版本标记正确不能替代功能验收。

## 三、总纲要求核验

### 3.1 WeixinConversationBinding

通过部分要求：

- 已在 crates/yunxi-agent-storage/src/weixin_state.rs 增加 WeixinConversationBinding。
- 绑定键包含 account_id、peer_id_hash、direct_message_key。
- 绑定值包含 workspace_id、session_id、来源标签及创建、活动、更新时间。
- begin_pending_runtime_turn 在 pending 状态转为 Running 时同步建立或更新绑定。
- upsert_conversation_binding 会在同一账户、同一对端、同一私聊键下复用已有 session_id。
- 状态快照校验和原子保存路径已覆盖绑定数据。

但是，仅在存储层复用同一个字符串 session_id，不等于已经复用本地 CLI 的连续会话历史。该关键行为见第四节阻塞项 P1-1。

### 3.2 WeixinTurnSupervisor

通过部分要求：

- 已实现 crates/yunxi-agent-weixin/src/turn_supervisor.rs。
- 使用既有 AgentInput::text。
- 使用既有 Agent::run_with_backend_stream。
- 没有复制第二套 Runtime。
- 成功结果只写入 WeixinRuntimeSinkRecord。
- 生产 serve 使用 NoopWeixinRuntimeSink，v2.1.6 没有提前接入 sendmessage，也没有提前做逐 token 微信回信。
- 当前代码边界没有把远程审批、远程追问、远程取消、群聊和 persona/memory 扩展提前混入本版本。

但 supervisor 只设置 session_id，没有按照当前本地交互运行时的 parent_session_id 与历史恢复规则建立可验证的 turn 链路。该问题与第四节 P1-1直接相关。

### 3.3 本地 CLI 与微信 serve 的 Runtime 配置复用

通过：

- crates/yunxi-agent-cli/src/weixin.rs 的 run_serve 使用 prepare_runtime_invocation。
- ProviderMode、model、cwd、sandbox、approval 和 companion 相关配置沿用已准备的本地 Runtime 配置。
- YunXi backend 通过 build_yunxi_runtime_backend 构造，未另建一套独立 Runtime。
- v2.1.6 的测试覆盖了 provider、model、cwd、approval、sandbox、context 和 session_id 的传递。

这一项满足总纲的架构方向。仍需在整改后的真实 YunXiRuntimeBackend 集成测试中证明配置复用和历史恢复同时成立。

### 3.4 按 WeixinConversationKey 串行执行与有界队列

通过部分要求：

- supervisor 使用由 account_id、peer_id_hash、direct_message_key 组成的会话队列键。
- 同一键使用互斥执行器，队列状态包含运行中、等待中和全局等待计数。
- QueueFull 发生在 begin_pending_runtime_turn 之前，第二条 pending 在测试中保持 Ready，没有被错误推进。
- 不同对端可以使用不同队列键。

未通过完整要求：

- 当前测试没有证明两个同一会话的真实并发 turn 会严格串行并且 memory/history 写入不交叉。
- 当前测试没有覆盖不同账户、相同对端标识下的严格隔离。
- max_global_queue 主要限制等待项，不包含运行中的会话；需要在整改中明确容量语义并用测试锁定。
- 更关键的是，serve 已先 commit_inbound_batch，再调用 dispatcher。QueueFull 后 serve 仅累计运行时错误，没有扫描、重试或恢复 Ready pending。结果是网络 cursor 已前移，消息仍停在 Ready，后续轮询没有通用的恢复入口。

因此该部分目前不能按总纲宣称完整通过。

### 3.5 v2.1.6 功能边界

通过：

- 没有要求本版本实现逐 token 微信发送。
- 没有要求本版本实现 sendmessage。
- 没有要求本版本实现远程审批、远程追问和取消。
- 没有要求本版本实现群聊。
- 没有要求本版本增加 persona、memory、companion 的微信端扩展。

本版本的 sink 作为最终文本测试出口符合阶段边界，但不能掩盖会话历史和待处理任务恢复缺失。

## 四、阻塞项

### P1-1：复用 session_id 但没有恢复连续会话历史

证据：

- crates/yunxi-agent-weixin/src/turn_supervisor.rs 在构造 turn 配置时调用 with_session_id，但没有设置 parent_session_id。
- crates/yunxi-agent-runtime/src/lib.rs 的 build_initial_messages 会通过 restore_parent_history 恢复历史，而 restore_parent_history 在 parent_session_id 缺失时直接返回空历史。
- crates/yunxi-agent-cli/src/interactive.rs 的本地交互路径会把 active_session_id 作为下一轮的 parent_session_id，这正是本版本总纲要求复用的既有规则。
- 当前 supervisor 测试中的 CapturingBackend 只检查传入配置和 prompt，没有使用真实 YunXiRuntimeBackend 验证第二轮 provider 输入是否包含第一轮用户消息和 assistant 输出。

影响：

同一对端虽然在存储层得到相同 session_id，但第二轮运行可能以没有上一轮上下文的初始消息启动。这样形成的是“同 ID 标识复用”，不是可用的持久 companion session。用户在个人微信端无法获得连续陪伴体验，也不能证明 memory/history 写入没有脱节。

整改要求：

- 采用与本地 interactive 一致的 session/parent/history 语义，明确绑定的是根会话还是当前活动会话；不要只把 parent_session_id 机械设置为同一个 session_id。
- 对每个私聊 turn 明确当前活动 session、父 session 和绑定更新时机，确保重启后仍能从持久存储恢复。
- 增加真实 YunXiRuntimeBackend 集成测试，使用可观测的 fake provider 和 InMemorySessionStore 或 FileSessionStore：连续执行同一对端两轮，断言第二轮 provider 消息包含第一轮用户消息和 assistant 输出，并断言重新加载存储后链路仍成立。
- 增加不同账户、相同对端标识的隔离测试，确认不会共享 session、history 或 memory store。

### P1-2：QueueFull 后 Ready pending 没有恢复路径

证据：

- crates/yunxi-agent-weixin/src/serve.rs 先执行 commit_inbound_batch，再按 accepted_item_ids 调用 runtime dispatcher。
- dispatcher 返回 QueueFull 时 serve 只增加 runtime_error_count，没有将 pending 放回可重试队列，也没有在启动或下一次轮询时 drain Ready pending。
- turn_supervisor_tests.rs 的 queue full 测试只证明第二条 pending 保持 Ready，不能证明生产 serve 会再次调度它。

影响：

网络 cursor 已提交后，如果当次 dispatcher 因队列容量返回 QueueFull，Ready pending 可能长期滞留。消息没有丢失，但也没有自动完成，用户会看到微信端消息无响应，且仅靠继续拉取新消息不能保证旧任务恢复。这违反本版本对“有界队列且不丢消息”的完成性要求。

整改要求：

- 在 commit 与 dispatch 的职责之间增加明确的可恢复机制：例如启动时及每轮 poll 前扫描 Ready pending，按会话键重新入队；或使 dispatcher/serve 在 QueueFull 时保留可重试任务并提供退避重试。
- 设计幂等调度：同一个 pending_id 只能有一个 Running turn，重试不能重复写 history、sink 或 receipt。
- 增加真实 serve 集成测试：制造队列满、提交网络 cursor、确认 pending 仍为 Ready，解除容量后再次轮询或重启 serve，最终该 pending 完成且只产生一次结果。
- 将 QueueFull、重试次数、最后错误和下一次重试时间写入脱敏诊断字段，不能只保留内存计数。

## 五、测试与验证结果

已执行并通过：

- cargo fmt --all -- --check
- cargo check --workspace
- cargo test --workspace -- --test-threads=1
- cargo test -p yunxi-agent-weixin --test turn_supervisor_tests -- --test-threads=1
- cargo test -p yunxi-agent-storage --test weixin_state_tests -- --test-threads=1
- cargo test -p yunxi-agent-weixin --lib -- --test-threads=1
- cargo build --workspace --release
- target/release/yunxi.exe --version，输出 yunxi 2.1.6
- git diff --check
- git fsck --full --no-dangling
- 依赖边界检查未发现 codex、vendor 或 yunxi-agent-codex 的正常依赖匹配
- v2.1.6 tag 对象和 tag target 可解析

这些结果证明当前代码格式、编译、既有自动化测试和发布标记基本稳定，但现有测试没有覆盖本报告列出的两个生产级行为缺口，因此不能作为通过依据。

本轮未执行真实微信网络会话和真实模型服务 smoke test。由于两个阻塞项可以通过源码路径和针对性测试缺口直接确认，不影响“不通过”结论。

## 六、通过项与残余风险

通过项：

- v2.1.6 workspace 版本和 tag 已形成。
- 微信状态快照已支持会话绑定。
- turn supervisor 已复用 Agent 与既有 backend stream 入口。
- CLI 与微信 serve 的 Runtime 配置来源统一。
- v2.1.6 没有越界实现后续版本的 sendmessage、逐 token 回信、远程审批、群聊和 companion 扩展。
- TUI、CLI、Provider、companion 和 storage 的工作区回归测试通过。

残余风险：

- 当前 supervisor 测试使用 CapturingBackend，未覆盖真实 Runtime 的 history 组装。
- 真实 serve 的提交、队列、重试和重启恢复链路缺少集成测试。
- 不同账户隔离和同会话并发顺序没有被充分证明。
- target 目录因本轮构建重新生成，审核结束时保留；未执行递归删除。

## 七、参考源码与 Rust 化要求

本版本整改必须继续参考以下路径，只抽取逻辑和测试思想，不复制宿主框架：

- D:\源码\reasonix\internal\bot\types.go
- D:\源码\reasonix\internal\bot\gateway.go
- D:\源码\reasonix\internal\botruntime\runtime.go
- D:\源码\reasonix\internal\bot\weixin\weixin.go
- D:\源码\reasonix\internal\bot\weixin\weixin_test.go
- D:\源码\CowAgent\channel\channel_factory.py
- D:\源码\CowAgent\channel\weixin\
- D:\源码\openclaw-weixin\src\storage\sync-buf.ts
- D:\源码\openclaw-weixin\src\storage\state-dir.ts

本项目内应优先复用和对齐：

- D:\YunXi Agent\crates\yunxi-agent-runtime\src\lib.rs
- D:\YunXi Agent\crates\yunxi-agent-storage\src\lib.rs
- D:\YunXi Agent\crates\yunxi-agent-storage\src\weixin_state.rs
- D:\YunXi Agent\crates\yunxi-agent-cli\src\interactive.rs
- D:\YunXi Agent\crates\yunxi-agent-cli\src\weixin.rs
- D:\YunXi Agent\crates\yunxi-agent-weixin\src\turn_supervisor.rs
- D:\YunXi Agent\crates\yunxi-agent-weixin\src\serve.rs
- D:\YunXi Agent\crates\yunxi-agent-weixin\tests\turn_supervisor_tests.rs
- D:\YunXi Agent\crates\yunxi-agent-storage\tests\weixin_state_tests.rs

Rust 化约束：

- reasonix、CowAgent、openclaw-weixin 的逻辑只能转译为 Rust 的领域类型、trait、异步任务和持久状态机。
- 不引入第二套 Agent Runtime、第二套 SessionStore 或独立 memory 实现。
- 所有队列、状态转换和重试都必须可测试、可观测、可恢复。
- 先完成一批构建后统一验证，验证通过前不得宣称完成。

## 八、进入下一版本的判定

当前判定：**不允许进入 v2.1.7。**

必须完成 P1-1 和 P1-2，并至少补齐以下证据后重新审核：

1. 真实 runtime 两轮历史恢复测试通过。
2. 真实 serve QueueFull 后 Ready pending 恢复测试通过。
3. 同会话并发串行测试通过。
4. 不同账户隔离测试通过。
5. 全工作区 fmt、check、test、release build 通过。
6. 不提前实现 v2.1.7 的远程审批、追问、取消或 sendmessage/逐 token 回信。

本报告只决定 v2.1.6 是否可以进入总纲中的 v2.1.7，不对 v2.1.7 之后的版本作提前验收。

## 九、提交、推送与清理状态

- 本轮没有修改 Rust 源码。
- 本轮没有创建新的 Git commit、push 或 tag。
- 既有 v2.1.6 tag 未删除、未移动。
- 审核文档和日志属于审核记录，不代替开发者的版本提交。
- target 目录由构建命令重新生成；因未获得本轮针对该目录的递归删除确认，保持原样。

署名：审核者
