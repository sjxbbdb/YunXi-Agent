# YunXi Agent v2.1.0 至 v2.2.0 微信接入合并开发范围复审报告

- 审核时间：2026-07-31 00:21:50 +08:00
- 当前代码版本：v2.1.6
- 审核范围：总纲图从 v2.1.0 基线至 v2.2.0 完成定义
- 审核依据：D:\YunXi Agent\docs\superpowers\plans\2026-07-22-yunxi-agent-v2-1-1-to-v2-2-0-personal-wechat-roadmap.md
- 总纲 SHA-256：2DF30D503F46CFE7496567F5011BF5CBFB8BA73C91C2D2FA3E9F29AF0932EA0F
- 审核者：审核者

## 一、重新审核背景与新口径

原总纲把 v2.1.1 至 v2.2.0 划分为十个独立审核版本。现在根据用户新的开发安排，后续不再把 v2.1.7、v2.1.8、v2.1.9 和 v2.2.0 分别作为四个独立发布开发周期，而是合并为一个大版本 v2.2.0 进行开发和最终验收。

合并开发不代表删减小版本内容。原 v2.1.7、v2.1.8、v2.1.9 的全部功能、测试、边界、安全要求和参考源码，均转为 v2.2.0 内部强制门禁；任何一个内部门禁未完成，v2.2.0 都不能通过审核、不能宣称完成、不能创建发布 tag。

已有 v2.1.0 至 v2.1.6 tag、历史审核报告、整改报告和证据全部保留，不删除、不移动、不覆盖。本报告是按新范围重新形成的全范围复审文件，不替换历史报告正文；历史报告继续作为各阶段实施和整改事实的追溯证据。

## 二、审核结论

**当前全范围审核不通过。当前不得宣称 v2.2.0 完成，不得创建 v2.2.0 发布 tag。**

当前 v2.1.0 至 v2.1.5 的既有能力在源码、历史发布证据、当前 workspace 回归和 TUI/ConPTY 只读证据中保持；当前 v2.1.6 已有会话绑定和 Runtime 调度骨架，但仍有两个 P1 阻塞点：

1. WeixinConversationBinding 复用的是 session_id 标识，尚未按本地 CLI 的 parent/history 规则证明连续会话历史真正进入下一轮 Runtime。
2. weixin serve 在提交 cursor 和 Ready pending 后遇到 QueueFull，只累计运行时错误，没有 Ready pending 的 drain、重试或重启恢复路径。

此外，原 v2.1.7、v2.1.8、v2.1.9 和 v2.2.0 的完整能力目前尚未实现。它们不是本轮把未开发内容伪装成 v2.1.6 缺陷的理由，而是合并开发后必须逐项关闭的内部验收门禁。

## 三、当前基线与版本追溯

| 项目 | 结果 |
| --- | --- |
| Cargo workspace 版本 | 2.1.6 |
| 当前 HEAD | 9e1fc57，docs: record v2.1.6 release closure |
| 当前版本 tag | v2.1.6 annotated tag 存在，target 为 6b16d4cab6257424cf9b3d8d5b6d297c3e9beaad |
| v2.1.0 tag | annotated tag，target 为 a8293905af55d659d647515786699ab313a51a07 |
| v2.1.1 至 v2.1.5 tag | 均存在且为 annotated tag |
| v2.2.0 tag | 不存在，符合当前尚未完成的事实 |
| 工作树 | 本轮审核记录产生 docs-only 变更，Rust 源码未修改 |
| 总纲正本 | 项目内 docs/superpowers/plans/，桌面副本由项目正本同步 |
| 审核方式 | 逐项对照总纲，不把版本号、tag 顺序或发布顺序当作源码功能阻塞点 |

## 四、v2.1.0 至 v2.2.0 全范围矩阵

### 4.1 v2.1.0：TUI、流式输出与既有陪伴基线

判定：通过并保持回归。

- v2.1.0 的 TUI、流式输出、终端生命周期、Approval、Details、取消、Provider 和 CLI/plain/JSON/JSONL 边界由历史发布审核通过。
- 当前 workspace 全量测试通过，TUI 测试 161 项通过。
- v205、v206、v207、v207-hotfix、v208、v209、v210 ConPTY verifier 均通过；v209 和 v210 的正式证据以 read_only=true 方式复核。
- 当前 release CLI 输出 yunxi 2.1.6，未发现 v2.1.0 既有 TUI、CLI、Provider、companion 和流式能力回归。
- 当前审核不重新打开已通过的 TUI 视觉改造范围；v2.2.0 只要求保持这些行为，不新增与微信无关的 TUI 重构。

参考路径：

- D:\YunXi Agent\crates\yunxi-agent-tui\
- D:\YunXi Agent\crates\yunxi-agent-core\src\stream.rs
- D:\YunXi Agent\crates\yunxi-agent-cli\src\terminal_mode.rs
- D:\YunXi Agent\scripts\conpty\v210\
- D:\YunXi Agent\docs\reports\evidence\frames\v210-conpty\

### 4.2 v2.1.1：目录治理、忽略边界与文档索引

判定：已完成，当前状态保持。

- crates、evals、scripts、docs、vendor、extracted 的稳定边界仍存在。
- target、.yunxi、.codegraph、.tmp、.worktrees 和 ConPTY node_modules/.work 有忽略边界。
- docs/README.md、scripts/conpty/README.md、docs/reports/README.md 和路线图正本导航仍存在。
- 历史报告没有在本轮被移动或删除。
- git ls-files 检查没有发现已跟踪且被新增忽略规则隐藏的 ConPTY 文件。
- 2.1.1 历史路径整改报告显示原阻塞已通过；当前不重新把历史迁移问题作为功能阻塞。

参考路径：

- D:\YunXi Agent\.gitignore
- D:\YunXi Agent\docs\directory-governance.md
- D:\YunXi Agent\docs\README.md
- D:\YunXi Agent\docs\reports\README.md
- D:\YunXi Agent\scripts\conpty\README.md
- D:\源码\reasonix\REASONIX.md
- D:\源码\reasonix\.gitignore

### 4.3 v2.1.2：微信 crate、CLI 骨架、iLink 协议与 Mock

判定：已完成，当前状态保持。

- crates/yunxi-agent-weixin 已作为唯一生产微信 crate 存在。
- iLink client、poll、qr、send 协议模型和错误归一化仍在。
- CLI weixin 命令族仍挂载到既有分发路径。
- login、status、doctor、serve、pair、logout 的参数和帮助入口仍在。
- 协议模型、数字或字符串 message ID、响应大小限制、超时、错误脱敏和 Mock 测试仍通过。
- 当前 send.rs 只代表协议请求模型，不代表生产消息回信已经接通。

参考路径：

- D:\YunXi Agent\crates\yunxi-agent-weixin\src\domain.rs
- D:\YunXi Agent\crates\yunxi-agent-weixin\src\ilink\client.rs
- D:\YunXi Agent\crates\yunxi-agent-weixin\src\ilink\models.rs
- D:\YunXi Agent\crates\yunxi-agent-weixin\src\ilink\send.rs
- D:\YunXi Agent\crates\yunxi-agent-cli\src\weixin.rs
- D:\源码\reasonix\internal\bot\weixin\
- D:\源码\openclaw-weixin\src\

### 4.4 v2.1.3：二维码登录与系统凭证保护

判定：历史复审已通过，当前回归保持。

- QR 状态机、过期、取消、redirect、终态失败和安全 URL 兜底实现仍在。
- Windows Credential Manager、fake secret store 和不可用时拒绝明文降级的测试仍通过。
- CLI login/status/doctor 的秘密脱敏测试仍通过。
- v2.1.3-hotfix.1 的历史复审已记录真实扫码、CLI Mock、凭证引用和元数据脱敏证据；本轮不重复触发真实账户登录，也不读取用户凭证。
- 当前不能把 v2.1.3 的登录完成解释为微信聊天闭环完成。

参考路径：

- D:\YunXi Agent\crates\yunxi-agent-weixin\src\login.rs
- D:\YunXi Agent\crates\yunxi-agent-weixin\src\secret_store.rs
- D:\YunXi Agent\crates\yunxi-agent-weixin\src\account_store.rs
- D:\YunXi Agent\crates\yunxi-agent-weixin\tests\login_store_tests.rs
- D:\源码\reasonix\internal\bot\weixin\weixin_login.go

### 4.5 v2.1.4：状态持久化、账户锁、诊断与生命周期

判定：历史 hotfix 复审已通过，当前回归保持。

- WeixinStateStore 独立于 SessionRecord。
- 状态 schema、原子同卷替换、损坏/未来 schema 拒绝、账户锁、pair request、logout 和 status/doctor 仍在。
- v2.1.4-hotfix.1 已补齐旧 v2.1.3 account metadata 到 WeixinStateStore 的安全幂等初始化。
- v2.1.5-hotfix.2 已补齐 Windows stale lock 恢复和独立进程 pending 解密证据。
- 当前 storage 微信状态测试通过；没有发现既有状态存储、旧 session、账户锁或配对生命周期回归。

参考路径：

- D:\YunXi Agent\crates\yunxi-agent-storage\src\weixin_state.rs
- D:\YunXi Agent\crates\yunxi-agent-storage\src\lib.rs
- D:\YunXi Agent\crates\yunxi-agent-storage\tests\weixin_state_tests.rs
- D:\YunXi Agent\crates\yunxi-agent-weixin\src\secret_store.rs
- D:\源码\reasonix\internal\bot\types.go
- D:\源码\openclaw-weixin\src\storage\

### 4.6 v2.1.5：私聊长轮询、配对、加密 pending 与幂等

判定：历史 hotfix 复审已通过，当前回归保持。

- iLink getupdates 长轮询、cursor、timeout、退避和 Ctrl-C 路径仍在。
- 陌生私聊只生成脱敏 pair request，不创建 Runtime/session。
- accepted/ready/running/terminal 状态和加密 pending inbound 仍在。
- duplicate message ID、游标和接纳批次原子提交仍在。
- Windows stale lock 复审和独立进程解密测试证据仍可追溯。
- 当前 serve 已具备 runtime dispatcher 接口，但队列满后的恢复缺口属于 v2.1.6 运行闭环，见下一节。

参考路径：

- D:\YunXi Agent\crates\yunxi-agent-weixin\src\serve.rs
- D:\YunXi Agent\crates\yunxi-agent-weixin\src\inbound.rs
- D:\YunXi Agent\crates\yunxi-agent-weixin\src\payload_cipher.rs
- D:\YunXi Agent\crates\yunxi-agent-weixin\tests\pending_inbound_recovery_tests.rs
- D:\源码\reasonix\internal\bot\weixin\weixin.go
- D:\源码\reasonix\internal\bot\weixin\weixin_test.go
- D:\源码\openclaw-weixin\src\storage\sync-buf.ts

### 4.7 v2.1.6：会话绑定既有 Runtime

判定：不通过，存在两个 P1 阻塞点。

已通过部分：

- WeixinConversationBinding 已存在，字段位于 crates/yunxi-agent-storage/src/weixin_state.rs:862-875。
- 绑定键包含 account_id、peer_id_hash、direct_message_key，绑定值包含 workspace_id、session_id、来源标签和时间字段。
- upsert_conversation_binding 位于 weixin_state.rs:1539-1592，同一绑定键会复用已有 binding 的 session_id。
- WeixinTurnSupervisor 已存在于 crates/yunxi-agent-weixin/src/turn_supervisor.rs。
- supervisor 使用 AgentInput::text 和 Agent::run_with_backend_stream，没有建立第二套 Agent Runtime。
- CLI 通过 prepare_runtime_invocation 和 build_yunxi_runtime_backend 复用 Provider、model、cwd、sandbox、approval、context window 和 companion 配置。
- v2.1.6 仍把生产微信 sendmessage、逐 token 回信、远程审批和群聊保持关闭，符合阶段边界。

P1-1：session_id 复用没有证明连续历史恢复。

证据：

- turn_supervisor.rs:121-132 只调用 with_session_id 和 with_session_title，没有设置 parent_session_id。
- runtime/src/lib.rs:1937 调用 restore_parent_history；runtime/src/lib.rs:2004-2026 在 parent_session_id 缺失时直接返回无历史。
- 本地 interactive.rs:583-585 会把 active_session_id 作为下一轮 parent_session_id，这是当前既有会话历史规则。
- tests/turn_supervisor_tests.rs:147-240 使用 CapturingBackend 只检查 AgentConfig 和原始 prompt，未使用真实 YunXiRuntimeBackend 观察第二轮 provider messages。
- storage binding 的现有测试没有充分覆盖不同账户、同一会话跨重启和真实历史链路。

整改要求：

1. 明确绑定的是 root session、active session 还是 child session，并与 interactive 的 parent/history 规则一致；不得只把相同 session_id 机械传入每一轮。
2. 使用真实 YunXiRuntimeBackend 与可观测 fake provider，连续执行同一私聊两轮，证明第二轮包含第一轮 user/assistant 历史。
3. 关闭并重新加载 FileSessionStore 或模拟新进程，证明 history、persona 和 memory 链路仍可恢复。
4. 增加不同 account、相同 peer 标识的隔离测试，以及同一 conversation 的真实并发串行测试。

P1-2：QueueFull 后 Ready pending 没有恢复路径。

证据：

- serve.rs:253-267 先执行 commit_inbound_batch，再调用 dispatcher；dispatcher 失败只增加 runtime_error_count。
- storage weixin_state.rs:311-343 会把新 pending 保存为 Ready，同时 commit 也会保存 next_get_updates_buf。
- turn_supervisor_tests.rs:271-332 只证明 supervisor 直接调用时第二 pending 保持 Ready，未证明生产 serve 在后续 poll 或重启时会再次 drain。
- QueueFull 后当前代码没有 Ready pending 扫描、退避重试、启动恢复或下一轮 poll 恢复逻辑。

整改要求：

1. 在 serve 启动和每轮 poll 前增加 Ready pending 的可恢复调度，或实现等价的有界退避重试。
2. 保证 pending_id 幂等，不能因重试重复写 session history、memory、sink 或最终投递记录。
3. 加入真实 serve 集成测试：队列满、cursor 已提交、pending 保持 Ready、解除容量后下一轮或重启完成且只产生一次结果。
4. 对 QueueFull、重试次数、最后错误和下次重试时间写入脱敏持久化诊断，而不是只保留内存计数。

## 五、合并后的 v2.2.0 内部强制门禁

从本报告起，后续开发统一进入一个 v2.2.0 开发线。下面的 G-2.1.7、G-2.1.8、G-2.1.9 和 G-2.2.0 只是内部门禁编号，不是可以跳过的功能清单，也不替代当前 v2.1.6 P1 整改。

### G-2.1.7：远程文字控制、审批、追问与取消

必须完成：

- /status、/stop、/approve <id>、/deny <id> [reason]、/answer <id> <text> 的明确路由。
- 通过既有 AgentRunControl 桥接 approval、user input、cancel；不创建第二套审批系统。
- 远程请求使用不透明、绑定账户和会话、一次消费、可过期的 request ID。
- 未匹配 request ID、跨对端、重复、过期和断连响应全部拒绝。
- 不得把自然语言“好的”视为批准，不得提升 sandbox 或 approval 权限。
- 测试覆盖批准、拒绝、追问、取消、超时、断连、队列满和服务关闭。

参考和 Rust 化：

- D:\YunXi Agent\crates\yunxi-agent-core\src\stream.rs
- D:\YunXi Agent\crates\yunxi-agent-runtime\src\lib.rs
- D:\YunXi Agent\crates\yunxi-agent-cli\src\interactive.rs
- D:\源码\reasonix\internal\bot\gateway.go
- D:\源码\reasonix\internal\botruntime\runtime.go
- D:\源码\OpenAkita\ 个人 iLink 绑定与 IM 中断/权限逻辑
- 非 Rust 逻辑必须使用 serde、tokio、显式 Rust 状态机和 trait 重写。

### G-2.1.8：可靠回信与安全流式输出合并

必须完成：

- 只消费公开文本 AgentEvent，过滤 reasoning、tool event、Provider wire、路径和秘密。
- 使用既有流式事件和 AgentRunControl，不复刻 Provider streaming。
- 通过 iLink sendmessage 发送最终文本，按 Unicode、段落、代码块安全分段。
- 建立 delivery spool，记录 delivery id、turn id、消息 hash、重试次数、明确成功或不确定状态。
- stale context 只按已知可恢复语义清除并有限重试；超时/断连结果不明时不得盲目重复发送。
- typing 失败不得把最终回答误报为失败；所有错误只写脱敏诊断。
- mock 覆盖 4xx/5xx、stale context、超时、断连、重复、重启待投递、Unicode 长文本和局部失败。

参考和 Rust 化：

- D:\YunXi Agent\crates\yunxi-agent-protocol\src\lib.rs
- D:\YunXi Agent\crates\yunxi-agent-runtime\src\lib.rs
- D:\YunXi Agent\crates\yunxi-agent-storage\src\weixin_state.rs
- D:\源码\openclaw-weixin\src\messaging\
- D:\源码\openclaw-weixin\src\storage\sync-buf.ts
- D:\源码\reasonix\internal\bot\weixin\weixin.go
- D:\源码\CowAgent\channel\weixin\
- 仅抽取通道投递和 context token 逻辑，以 Rust serde/reqwest/tokio 重写，不引入 Node/Python Runtime。

### G-2.1.9：陪伴、人格与记忆连续性

必须完成：

- 微信使用 YunXi 现有 persona、memory、relationship graph、companion control 和同一 SessionStore。
- --companion 只显式启用既有 companion 路径，不创建第二套人格或记忆 schema。
- 证明本地 CLI 与微信连续交互中人格、偏好、关系和已允许记忆一致。
- 增加本地确认的 weixin session reset --account ... --peer ... --confirm，严格限定目标会话。
- reset/archive 必须沿用既有 YunXi 语义，不静默删除长期记忆。
- 用户消息与通道元数据分离，通道身份不能成为可控 prompt。
- 未完成真实验证前，不允许后台主动微信推送。

参考和 Rust 化：

- D:\YunXi Agent\crates\yunxi-agent-companion\
- D:\YunXi Agent\crates\yunxi-agent-persona\
- D:\YunXi Agent\crates\yunxi-agent-storage\src\lib.rs
- D:\YunXi Agent\crates\yunxi-agent-runtime\tests\general_companion_tests.rs
- D:\源码\Leon\core\context\LEON.md
- D:\源码\Leon\core\context\ARCHITECTURE.md
- D:\源码\Letta Code\
- D:\源码\Project N.E.K.O.\
- 非 Rust 项目只吸收本地优先、长期身份、可审阅记忆和跨入口连续性的逻辑，必须 Rust 化并复用 YunXi 现有实现。

### G-2.2.0：完整 CLI 接入、真实联调与发布收口

必须完成：

- 收口 login、status、doctor、serve、pair、session reset、logout 帮助、状态文本和 JSON/JSONL 边界。
- 新增 evals/weixin/，覆盖协议 Mock、状态迁移、配对、审批、回信、重启和安全诊断。
- 真实 iLink 测试只能读取系统凭证或受保护环境，不能把 token、context token、原始用户 ID 或正文写入日志和证据。
- 使用指定测试账户完成真实扫码、私聊、配对、真实 Provider、拒绝审批、批准安全测试动作、取消、重启恢复和投递回执。
- 更新 support bundle、迁移说明、rollback 说明、文档索引和状态文档。
- Windows ConPTY 证明前台微信服务没有破坏既有 TUI/CLI 行为。
- 只有所有内部门禁和全量回归通过后，才创建新的 annotated v2.2.0 tag；已有 tag 不删除、不移动。

参考和 Rust 化：

- D:\YunXi Agent\evals\weixin\，当前不存在，需在合并开发中新增
- D:\YunXi Agent\scripts\conpty\v210\
- D:\YunXi Agent\docs\reports\evidence\
- D:\YunXi Agent\crates\yunxi-agent-cli\
- D:\源码\Tencent\openclaw-weixin\
- D:\源码\reasonix\internal\bot\weixin\
- 所有网络协议、状态机、投递和测试逻辑均以 Rust 类型、trait、异步任务、显式错误和确定性测试重写。

## 六、已有能力保留门禁

合并开发期间，每一批源码完成后，以下能力必须持续通过，任何回归都阻塞 v2.2.0：

1. v2.1.0 TUI 的主视图、Details、Approval、取消、流式文本、CJK/Emoji、窄屏和终端恢复。
2. plain、JSON、JSONL、no-TUI、pipe、CI 和 forced fallback 输出契约。
3. Provider 选择、model、cwd、sandbox、approval、context window 和现有 companion 配置。
4. persona、memory schema、recall、relationship graph、SessionStore 和旧 session JSON 兼容。
5. QR 登录、安全凭证、WeixinStateStore、原子写入、schema migration、账户锁、stale lock 和 logout 边界。
6. getupdates cursor、pairing、重复消息幂等、加密 pending inbound、重启恢复和脱敏诊断。
7. 群聊关闭、秘密不进入日志、陌生发送者不进入 Runtime、微信不获取 shell 旁路权限。
8. 所有已有 annotated tag、历史报告、证据和回滚路径。

合并开发不得通过删除旧测试、降低断言、改变历史 tag、清理历史证据或把功能移出测试范围来制造通过。

## 七、当前验证结果

本轮已通过：

- cargo fmt --all -- --check。
- cargo check --workspace。
- cargo test --workspace -- --test-threads=1；无失败。
- cargo build --workspace --release。
- target/release/yunxi.exe --version，输出 yunxi 2.1.6。
- git diff --check。
- v205、v206、v207、v207-hotfix、v208、v209、v210 ConPTY 只读 verifier；全部返回 ok。
- CLI 微信帮助和 pair 帮助检查；当前明确把 sendmessage、远程审批和群聊标记为不可用。
- v2.1.0 至 v2.1.6 tag 类型和 target 检查；全部为 annotated tag，未发现 tag 缺失或漂移。
- v2.1.0 至 v2.1.6 的 crates diff summary 未发现源码删除；当前 workspace 回归没有发现既有能力回归。
- git ls-files -ci --exclude-standard -- scripts/conpty 返回 none。
- 总纲正本 SHA-256 可核对。

测试边界：

- 当前全量测试包含一个 Windows pending 解密子测试被默认标记 ignored；独立进程解密测试已通过，完整 Windows ignored 测试仍应在开发者后续统一验证中按其适用条件补跑。
- 本轮未执行真实微信网络往返或真实 Provider v2.2.0 联调，因为当前 v2.2.0 功能尚未完成；不能用离线 Mock 代替总纲要求的最终真实门禁。
- 本轮没有执行桌面凭证读取、二维码登录、发送消息或任何用户数据操作。

## 八、进入合并开发的判定与顺序

由于当前 v2.1.6 的两个 P1 尚未关闭，按照固定审核流程：

**当前不得进入 v2.2.0 合并开发的正式下一阶段，必须先完善 v2.1.6 并重新审核。**

v2.1.6 重新审核通过后，开发者可以在同一个 v2.2.0 worktree/开发线内按以下顺序实现内部门禁：

1. 先关闭 P1-1：真实 session history、parent 规则、重启恢复、账户隔离和并发串行。
2. 再关闭 P1-2：QueueFull 的 Ready pending drain、重试、幂等和真实 serve 重启测试。
3. 进入 G-2.1.7，完成远程审批、追问和取消。
4. 进入 G-2.1.8，完成 sendmessage、可靠回信、流式文本安全合并和投递状态。
5. 进入 G-2.1.9，完成 persona、memory、relationship、companion 和 session reset 连续性。
6. 进入 G-2.2.0，完成 CLI 收口、evals/weixin、真实 iLink/Provider 联调、ConPTY 回归、文档和发布证据。
7. 全部通过后，只创建新的 annotated v2.2.0 tag；不删除或移动 v2.1.0 至 v2.1.6 历史 tag。

这意味着“合并开发”改变的是后续开发组织方式，不改变审核门禁和功能清单；它也不允许把四个内部阶段一次性混成没有可追溯证据的大提交。

## 九、提交、推送与清理状态

- 本轮未修改 Rust 源码、Cargo 配置、测试源码、vendor 或 extracted。
- 本轮只新增本审核报告、更新报告索引并追加审核日志。
- 本轮未创建 commit、未 push、未创建 v2.2.0 tag。
- v2.1.0 至 v2.1.6 历史 tag 未删除、未移动、未覆盖。
- target、.codegraph、.tmp、.yunxi、.worktrees 等目录未删除；本轮构建重新使用 D:\YunXi Agent\target，未获得对该绝对路径的递归删除确认，因此保留。
- 未执行 git clean、递归删除、强制移动、清空目录、系统安装/卸载、PATH/注册表修改或用户凭证清理。

署名：审核者
