# YunXi Agent v2.2.0 微信接入合并线代码整改开发报告

- 撰写时间：2026-07-31 17:40:27 +08:00
- 审核依据：`D:\YunXi Agent\docs\reports\audits\2026-07-31-172719-yunxi-agent-v2-2-0-merged-weixin-code-audit-report.md`
- 审核报告 SHA-256：`8D4090AA024D7DEA69BDAD34EBA0A2E31D9ECF55F49E9C0B38ECB6E18BA08239`
- 总纲正本：`D:\YunXi Agent\docs\superpowers\plans\2026-07-22-yunxi-agent-v2-1-1-to-v2-2-0-personal-wechat-roadmap.md`
- 总纲 SHA-256：`2DF30D503F46CFE7496567F5011BF5CBFB8BA73C91C2D2FA3E9F29AF0932EA0F`
- 当前代码基线：Cargo workspace `2.2.0`，HEAD `f0c84d2a4620bbbaee3887b964605d5c8e220def`，提交信息为 `feat: close v2.2.0 weixin p1 runtime blockers`。
- 当前工作树：存在未提交的源码、测试、配置和文档修改；本报告不撤销、不覆盖这些修改。
- 当前 tag：`v2.1.0` 至 `v2.1.6` 历史 tag 保留；`v2.2.0` tag 尚未创建。
- 审核结论转化：当前审核不通过，必须继续留在 `v2.2.0` 合并开发线整改；本报告不是完成声明。
- 报告类型：面向开发者的代码整改开发指令。

## 一、硬性约束

以下约束是本次 `v2.2.0` 代码整改、验证、文档同步和发布的前置要求，不得因为上下文过长、任务拆分、局部修复或环境阻断而忽略、弱化或绕过。

1. 开发目录固定在 `D:\YunXi Agent` 及其工作树内。
2. 核心目标是开发一个通用型陪伴 agent；默认 YunXi 运行路径必须摆脱上游 Codex CLI 源码依赖，不得默认依赖 `vendor/codex-rs`、`codex-*` crate 或 `yunxi-agent-codex`。
3. 涉及源码参考需要先抽取、迁移和构建整体能力，不要在单个点上反复纠结；参考源码不是 Rust 时，只参考其逻辑并使用 Rust 类型、trait、异步任务、显式错误和确定性测试复刻。
4. 中间无需频繁验证；完成一批完整能力后再统一执行构建、测试和验证。
5. 验证通过前不能宣称完成、发布或审核通过。
6. 每次阶段结束后要清理编译中间产物；递归清理必须针对精确绝对路径并取得用户确认。
7. 每次任务结束后，都要在项目日志中追加本次工作的详细记录。
8. 日志必须包含时间戳、工作目标、执行流程、修改文件、文件路径、验证结果、提交和推送状态，并在日志结尾署名“开发报告撰写者”。
9. 后续开发报告、设计文档、索引和状态文档必须与实际代码状态保持一致。
10. 回复用户使用中文。
11. 之后每个版本更迭都必须提交为新的 Git tag；历史版本 tag 不得删除、移动或覆盖，便于回滚。
12. 涉及 shell 命令必须谨慎；递归删除、强制移动、清空目录、系统安装/卸载、PATH/注册表/系统配置修改等操作必须先取得用户确认。
13. 不可因为上下文变长而忽略硬性要求；如果发现约束执行力度减弱，必须向用户报告当前情况。
14. 工作尽量固定在项目文件夹；涉及 `D:\源码` 或 `C:\Users\24763\Desktop` 等其他目录时，必须明确告知用户具体路径和操作内容。

## 二、当前状态与开发目标

当前代码已经不再是空白骨架。会话父子关系、QueueFull Ready pending 扫描、远程控制 hub、delivery spool、session reset、微信 eval 和版本 `2.2.0` 基线已经进入工作树，但代码审核确认这些能力尚未形成可用、可恢复、可验证的完整闭环。

本阶段开发目标不是重新实现已经存在的 session binding 或基础状态 store，而是关闭以下九个 P1，并让实现、测试、文档和证据一致：

1. 远程控制命令和审批/追问 prompt 必须有安全的微信 outbound 回执路径。
2. 远程控制注册失败不得静默忽略，one-shot 必须有确定结果。
3. AgentEvent 必须建立公开文本白名单和安全输出合并边界，不能直接丢弃所有流式事件。
4. delivery 必须区分确定失败和结果不明，不能无条件自动重试造成重复消息。
5. 多段回信必须具备原子提交或可恢复 manifest，不能留下半截 spool。
6. CLI 后台 dispatch 必须有 lease、退出等待和 Running stale recovery。
7. companion、memory、persona、session reset 和通道元数据隔离必须有跨入口证据。
8. 离线 eval 不能用恒真布尔值代替真实门禁，真实证据必须单独脱敏保存。
9. 开发报告、微信文档、索引和状态文档必须修正为当前 `2.2.0`/`f0c84d2` 状态。

`v2.2.0` tag 继续禁止创建。只有整改、统一验证、真实 iLink/Provider/ConPTY 证据和重新审核全部通过后，才允许创建新的 annotated tag。

## 三、已具备的基础与保持范围

以下能力已经具备基础实现或部分测试，后续开发不得通过重复造轮子、删除旧实现或改变既有边界来“解决”当前审核问题：

- `WeixinConversationBinding` 已保存 `root_session_id`、`active_session_id`、`last_completed_session_id` 和兼容 `session_id`。
- pending turn 已保存 `turn_session_id` 与 `parent_session_id`，成功路径已经有真实 Runtime 和 FileSessionStore 重载测试。
- `serve` 已存在 Ready pending 扫描和 QueueFull 退避框架，但后台 CLI wiring、Running 恢复和长任务租约仍未闭环。
- delivery 已有认证加密 state、Unicode grapheme 分段、`delivery_id`、message hash 和 client id 字段，但不能据此假定 iLink 服务端已经提供幂等语义。
- remote control 已有明确 slash command、scope、一次消费、过期和状态持久化模型，但控制结果和 prompt 没有进入微信 transport。
- 现有 TUI、CLI、Provider、Approval、SessionStore、登录、配对、加密 pending、cursor 和 companion 既有行为必须持续保留。

## 四、P1 整改要求

### P1-1：建立远程控制和审批提示的微信 outbound 路径

当前 `D:\YunXi Agent\crates\yunxi-agent-weixin\src\serve.rs:247-265` 解析 slash command 后只调用 `hub.handle_command`，成功返回的 `WeixinRemoteControlOutcome` 没有进入 delivery spool 或 iLink `sendmessage`。`D:\YunXi Agent\crates\yunxi-agent-weixin\src\turn_supervisor.rs:368-389` 能注册 approval/user-input request，但生成的 `WeixinRemoteControlPrompt` 没有发送给微信用户。

必须实现：

- 为控制 outcome、approval prompt、user-input prompt、超时、未匹配、跨 scope、重复消费、服务关闭和 channel closed 建立明确的安全 outbound 接口。
- 复用现有 delivery 认证加密、Unicode 分段、脱敏和幂等模型，不允许绕过 delivery 直接调用 transport。
- `status`、`approve`、`deny`、`answer`、`stop` 都必须产生用户可理解的脱敏结果。
- approval prompt 至少包含不透明 request ID、purpose、safe action、safe reason、脱敏 cwd label 和 expires-at；不得发送原始 provider payload、隐藏推理、token、环境变量、完整路径或原始用户正文。
- prompt 和 outcome 必须绑定 account、peer、direct message、item、session 和 purpose，禁止跨会话投递。

必须测试：

- 使用 `RecordingMessageTransport` 或等价可观测 transport，证明 prompt 和 outcome 实际进入 outbound 队列。
- 正常 status/approve/deny/answer/stop。
- 未匹配、过期、重复、跨账户、跨对端、跨 turn、channel closed、服务关闭和 delivery 失败。
- 已确认成功的控制结果不重复发送；结果不明不得虚报已送达。

目标文件：

- `D:\YunXi Agent\crates\yunxi-agent-weixin\src\serve.rs`
- `D:\YunXi Agent\crates\yunxi-agent-weixin\src\turn_supervisor.rs`
- `D:\YunXi Agent\crates\yunxi-agent-weixin\src\remote_control.rs`
- `D:\YunXi Agent\crates\yunxi-agent-weixin\src\delivery.rs`
- `D:\YunXi Agent\crates\yunxi-agent-storage\src\weixin_state.rs`

### P1-2：远程控制注册失败必须进入显式错误路径

当前 `turn_supervisor.rs` 对 `register_cancellation` 使用 `let _ =`，approval/user-input 使用 `if let Ok(prompt)`；状态 store、锁或持久化失败时，Runtime 可能继续等待，但微信用户没有 request ID，也没有收到不可用提示。

必须实现：

- 删除静默忽略；注册失败必须返回带脱敏分类的错误。
- cancellation 注册失败时，本轮必须明确降级或失败，不能让 `/stop` 看似可用而实际没有 token。
- approval 注册失败必须向既有 one-shot 发送拒绝；user-input 注册失败必须发送 `None`，同时记录 `remote_control_register_failed`。
- 失败结果必须通过安全 outbound 路径通知已准入用户；不能把 Rust error、路径或内部状态原文发送给微信。
- 服务关闭、QueueFull、锁冲突、state schema 错误和 response channel closed 都必须有确定终态。

必须测试：

- state store 写入失败、锁失败、内存锁失败、response channel closed。
- approval、user-input、cancellation 注册失败后 one-shot 的值。
- 注册失败消息脱敏、终态持久化和重启后不重复注册。

### P1-3：实现公开 AgentEvent 过滤和安全输出合并

当前 `turn_supervisor.rs:368-389` 对 `AgentRunStreamReceiver` 的事件使用 `Some(_event) => {}`，所有流式事件被忽略。生产回信只在 `turn_supervisor.rs:194-216` 读取 `result.final_response`，delivery 层只有最终文本分段 spool。

开发者必须先明确本阶段策略：

- 最小安全路径可以先采用 `final-text-only`，但必须消费事件流、过滤非公开事件并记录确定的终态；不能以丢弃所有事件作为“安全过滤”。
- 如果实现受控分段流式，必须建立公开文本事件白名单，只允许面向用户的公开文本，过滤 reasoning、tool event、provider wire、路径、环境变量和秘密。
- typing/processing 状态必须与最终回答分离；typing 失败不能使最终回答失败。
- 同一公开文本不能因为事件重放、final_response 收口或重启恢复而重复进入 delivery spool。
- 设计决策必须写入 `docs/weixin.md`、开发报告和 eval 场景，不能在代码中隐式选择。

必须测试：

- reasoning、tool、provider wire、路径、秘密事件全部不出现在微信文本。
- 多个文本事件合并、重复事件、最终事件、取消、失败和重启收口。
- final-only 或受控分段策略的顺序、边界和 delivery 状态一致。

### P1-4：区分 delivery 确定失败与结果不明

当前 `delivery.rs:308-318` 对 retryable 错误安排重试，`delivery.rs:415-429` 将 Timeout、Network、408/425/429/5xx 和部分 API 错误统一视为可重试。`delivery_id` 被放入 `client_id`，但当前没有官方 iLink 证据证明该字段具有服务端幂等语义。

必须实现：

- 将错误分类至少拆分为 `DefiniteFailure`、`OutcomeUnknown`、`RetryableWithVerifiedIdempotency`、`Succeeded`。
- 只有官方协议明确提供并已验证的幂等键，或明确确定未送达，才允许自动重试。
- Timeout、响应体断连、网络中断等可能发生在服务端已接受消息之后的情况，必须进入 `OutcomeUnknown`，不能盲目自动重发。
- `OutcomeUnknown` 必须可在 status/doctor/support bundle 中观察，但不得把秘密或原文写入日志。
- 不能把 `client_id` 字段未经证据验证地宣称为“恰好一次”保障。

必须测试：

- 4xx、5xx、timeout、断连、响应解析失败、重复调用、重启待投递。
- 已确认未送达、已确认成功、结果不明三种状态分别验证。
- 未证明幂等时不自动重试；已确认幂等时重试次数和 message hash 可追溯。

### P1-5：多段 delivery spool 必须原子或可恢复

当前 `delivery.rs:83-125` 逐段调用 `enqueue_pending_delivery`。第 N 段失败时，前 N-1 段已经持久化；`turn_supervisor.rs:194-216` 随后可能把 turn 标为失败，但前序 delivery 仍可能继续发送。

必须实现以下一种完整方案：

- 在 `WeixinStateStore` 增加单次原子 batch enqueue，整条消息 manifest 和所有 segment 一次提交；或
- 建立 message manifest、segment 状态、turn 收口状态和恢复规则，使部分写入在重启后能够确定性补齐、回滚或标记待诊断。

硬性要求：

- 不得出现“微信收到半截回答、Runtime 标记失败、恢复时无法判断是否补发”的不确定状态。
- 已确认成功的 segment 不得重复发送。
- segment 顺序、total_segments、message_hash、turn_id、delivery_id 必须一致。
- 第 N 段写入失败、进程重启、transport 结果不明和局部发送失败必须有独立测试。

### P1-6：关闭后台 dispatch 的 lease 和 Running 恢复缺口

生产 CLI 在 `D:\YunXi Agent\crates\yunxi-agent-cli\src\weixin.rs:240-277` 设置 `background_runtime_dispatch=true`。当前 `serve.rs:352-375` 会记录 dispatch inflight，但 `weixin_state.rs:624-655` 主要记录 retry time 和诊断；Ready 扫描在 `weixin_state.rs:504-521`，Running 转换在 `weixin_state.rs:969-982`，没有进程异常退出后的 stale Running 恢复路径。

必须实现：

- pending runtime turn 增加 lease owner、attempt token、started-at、expires-at、last error 和 retry count。
- serve 启动时扫描过期 Running；只能把确认属于本进程旧租约的 pending 原子恢复为 Ready，不得把仍在运行的任务误判为 stale。
- 服务关闭时等待或取消后台 task，并把未完成任务转为可恢复状态。
- lease 过期、后台 task panic、进程异常退出、QueueFull 和正常完成必须有不同状态迁移。
- CLI 的 `background_runtime_dispatch=true` 必须由集成测试覆盖，不能只用 foreground test dispatcher 证明。

必须测试：

- 后台长任务超过 lease 不重复执行。
- 进程在 begin turn 后退出，重启后恰好一次恢复。
- 取消、服务关闭、QueueFull、task panic 和 transport 结果不明。
- 同一 account/peer/dm 的并发串行与不同对端并行。

### P1-7：补齐 companion 连续性和 session reset 证据

当前 session history 已有主要成功路径测试，但尚未证明总纲 G-2.1.9 的跨入口连续性。

必须实现和测试：

- 本地 CLI 先写入允许的偏好，微信随后读取；微信形成允许记忆，本地 CLI 随后读取。
- `--companion` enabled/disabled、quiet hours、tool request 权限和 memory extraction mode 在 CLI/微信入口一致。
- `weixin session reset --account ... --peer ... --confirm` 只影响指定 conversation；不得遗留 binding、pending turn 或 remote request。
- reset/archive 必须沿用 YunXi 现有 memory 语义，不静默删除长期记忆。
- account、peer、channel、nickname、context token 等通道元数据不能进入可控 prompt。
- 不得新增第二套 persona、memory schema、relationship graph、Runtime 或 SessionStore。

目标文件：

- `D:\YunXi Agent\crates\yunxi-agent-companion`
- `D:\YunXi Agent\crates\yunxi-agent-persona`
- `D:\YunXi Agent\crates\yunxi-agent-storage\src\lib.rs`
- `D:\YunXi Agent\crates\yunxi-agent-runtime\tests\general_companion_tests.rs`
- `D:\YunXi Agent\crates\yunxi-agent-cli\src\weixin.rs`

### P1-8：离线 eval 不得把真实门禁写成恒真声明

当前 `crates/yunxi-agent-eval/src/lib.rs:770-777` 对 restart recovery 和 real integration 直接返回 `true`，`1137-1152` 主要验证离线场景结构和安全字段；`evals/weixin/README.md` 的真实联调清单仍是待执行门禁。

必须实现：

- 离线 eval 只验证协议 Mock、状态迁移、配对、远程控制、回信分段、重启模拟和脱敏规则。
- 真实 iLink/Provider/ConPTY 必须使用单独的 live harness 或人工门禁，不得写入恒真指标。
- 真实证据必须记录测试账户哈希、时间、退出码、状态、是否触网和失败分类，但不得保存 token、context token、原始用户 ID、原始正文或 provider wire。
- `real_integration`、`restart_recovery` 等字段应区分 `not_run`、`passed`、`failed`、`blocked`，不能用布尔 true 代替执行结果。
- eval 输出必须能被审核报告和 support bundle 追溯。

### P1-9：修正文档基线和状态一致性

当前旧开发报告仍写 `Cargo workspace=2.1.6`、`HEAD=9e1fc57`，与实际 `2.2.0`、`f0c84d2` 不一致；`docs/weixin.md` 仍把 slash control 和 final delivery 描述成已接入能力，但当前审核确认控制回执、审批提示、流式合并和真实回信尚未通过。

开发者必须同步：

- `D:\YunXi Agent\docs\reports\development\`：以当前整改报告和实际代码状态为准。
- `D:\YunXi Agent\docs\README.md`：更新当前版本和开发状态。
- `D:\YunXi Agent\docs\weixin.md`：明确“已实现骨架”“待验证”“未通过审核”的边界。
- `D:\YunXi Agent\docs\reports\README.md`：索引指向真实报告，历史报告不移动、不覆盖。
- `D:\YunXi Agent\docs\development-log.md`：记录每一批实现、验证和提交状态。
- `C:\Users\24763\Desktop\YunXi Agent开发报告\` 和 `C:\Users\24763\Desktop\YunXi Agent开发日志.md`：由项目正本单向同步并校验哈希。

## 五、推荐开发顺序

必须在同一 `v2.2.0` 合并开发线中按批次推进，不通过创建 tag 跳过门禁：

1. 先完成 P1-1 和 P1-2：remote control outbound、审批/追问 prompt、控制 outcome、超时和注册失败 one-shot。
2. 再完成 P1-4 和 P1-5：delivery 结果分类、官方幂等语义确认、原子 manifest/batch enqueue 和重启恢复。
3. 再完成 P1-3：AgentEvent 公共文本白名单、final-only 或受控分段策略、typing 生命周期和事件去重。
4. 再完成 P1-6：后台 dispatch lease、Running stale recovery、服务退出等待/取消和真实 CLI wiring 测试。
5. 再完成 P1-7：companion、memory、persona、session reset 和元数据隔离跨入口测试。
6. 完成 P1-8：离线 eval 与真实 live evidence 分离，补齐脱敏证据。
7. 完成 P1-9：同步代码、文档、报告、索引、日志和状态，随后统一执行验证并申请重新审核。

每一批源码完成后，提交应保持边界清晰；不得把无关目录整理、历史报告迁移、用户数据清理和功能实现混在同一个不可回滚的大提交中。

## 六、统一验证门禁

审核报告确认 `cargo fmt --all -- --check` 已通过；以下验证必须在整改批次完成后统一补跑，当前不得把未执行项记为通过：

```text
cargo fmt --all -- --check
cargo check --workspace
cargo test --workspace -- --test-threads=1
cargo build --workspace --release
target\release\yunxi.exe --version
git diff --check
git fsck --full --no-dangling
```

还必须补充：

- remote control outbound、approval prompt、timeout、scope、一次消费测试。
- delivery 4xx/5xx/timeout/disconnect/unknown outcome/幂等和重启测试。
- multi-segment atomic manifest、局部失败和已确认 segment 不重复测试。
- background runtime dispatch、lease、Running stale recovery 和真实 CLI wiring 测试。
- companion/memory/session reset/元数据隔离跨入口测试。
- `evals/weixin` 全部离线场景和单独 live gate。
- v205、v206、v207、v207-hotfix、v208、v209、v210 ConPTY verifier。
- 指定测试账户的真实 iLink 扫码、私聊、配对、真实 Provider、审批拒绝、批准安全动作、answer、stop、重启恢复和 sendmessage 回执。
- 证据脱敏、support bundle、迁移说明、rollback 说明和 tag object/target/remote refs 检查。

验证失败、执行环境阻断或证据缺失时，必须记录为 `failed`、`blocked` 或 `not_run`，不得改写为通过。

## 七、参考源码与 Rust 化要求

### YunXi 当前代码接入点

- `D:\YunXi Agent\crates\yunxi-agent-weixin\src\serve.rs`：远程命令准入、Ready pending drain、后台 dispatch 和关闭流程。
- `D:\YunXi Agent\crates\yunxi-agent-weixin\src\turn_supervisor.rs`：AgentEvent、approval/user-input/cancellation 注册、Runtime 结果和 sink。
- `D:\YunXi Agent\crates\yunxi-agent-weixin\src\remote_control.rs`：scope、request ID、一次消费、过期和持久化生命周期。
- `D:\YunXi Agent\crates\yunxi-agent-weixin\src\delivery.rs`：Unicode 分段、delivery payload、spool、sendmessage、重试分类和状态收口。
- `D:\YunXi Agent\crates\yunxi-agent-storage\src\weixin_state.rs`：remote control request、pending runtime、delivery metadata、lease、原子状态迁移。
- `D:\YunXi Agent\crates\yunxi-agent-cli\src\weixin.rs`：生产 CLI wiring、background dispatch、Provider/sandbox/approval/cwd 配置。
- `D:\YunXi Agent\crates\yunxi-agent-eval\src\lib.rs`、`D:\YunXi Agent\evals\weixin\`：离线 eval 和真实门禁分界。

### 外部参考

- `D:\源码\reasonix\internal\bot\weixin\weixin.go`、`gateway.go`、`internal\botruntime\runtime.go`：参考长轮询、控制路由、重启恢复和通道到 Runtime 的桥接。
- `D:\源码\openclaw-weixin\src\messaging`、`src\storage\sync-buf.ts`：参考 sendmessage、context token、错误分类、sync buffer 和投递恢复。
- `D:\源码\CowAgent\channel\weixin`：参考个人陪伴通道到核心交接，不引入 Python channel factory、第二套人格或第二套 Runtime。
- 审核报告点名但本机仍缺失的 `D:\源码\OpenAkita`、`D:\源码\Leon\core\context\LEON.md`、`D:\源码\Leon\core\context\ARCHITECTURE.md`、`D:\源码\Letta Code`、`D:\源码\Project N.E.K.O.`、`D:\源码\Tencent\openclaw-weixin`，必须在日志中记录 canonical URL、HEAD、补齐状态或替代依据；审核报告未提供 canonical URL 时不得按项目名猜测并静默拉取。

所有非 Rust 参考只迁移整体逻辑和能力边界，必须使用 YunXi 现有 Rust 模块、状态 store、Runtime、AgentRunControl、serde、tokio、reqwest 和确定性测试重建，不复制 Go/Python/TypeScript Runtime。

## 八、清理、提交、推送与发布

- 每阶段验证结束后清理编译中间产物；`D:\YunXi Agent\target` 等递归清理必须取得用户对精确绝对路径的确认。
- 不得使用 `git clean`、强制移动、清空目录、gc/prune 或系统配置修改绕过问题。
- 当前工作树存在源码、测试、配置和文档修改，开发者必须先识别并保留相关改动，不得以恢复干净工作树为名撤销用户或前序开发改动。
- 代码整改完成后必须先创建清晰的开发 commit；在重新审核通过前不得创建或推送 `v2.2.0` tag。
- 重新审核通过后，创建新的 annotated `v2.2.0` tag，记录 tag message、tag object、peeled target、远端 push 状态和回滚方式。
- `v2.1.0` 至 `v2.1.6` 历史 tag、历史审核报告、证据和日志不得删除、移动、覆盖或清理。

## 九、完成判定

只有当 P1-1 至 P1-9 全部关闭，workspace 全量验证、真实 iLink/Provider/ConPTY 证据、文档状态同步和重新审核全部通过后，才允许将 `v2.2.0` 标记为完成。

在此之前，所有报告、状态文档和用户回复只能使用“整改中”“待验证”“不通过”“阻塞”或“未运行”等准确状态，不得写“已完成”“已发布”“审核通过”。

署名：开发报告撰写者
