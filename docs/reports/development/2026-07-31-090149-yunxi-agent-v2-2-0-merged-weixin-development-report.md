# YunXi Agent v2.2.0 微信接入合并版本开发报告

- 撰写时间：2026-07-31 09:01:49 +08:00
- 审核依据：`C:\Users\24763\Desktop\YunXi Agent审核报告\2026-07-31-002150-YunXi-Agent-v2.1.0-to-v2.2.0-合并开发范围复审报告.md`
- 项目内审核报告归档：`D:\YunXi Agent\docs\reports\audits\2026-07-31-002150-yunxi-agent-v2-1-0-to-v2-2-0-merged-development-reaudit-report.md`
- 审核报告 SHA-256：`E767DB1A40319165C5A5DDE4FB641D9ABB97EF3256137AD78E73DF0B64D46A57`
- 总纲正本：`D:\YunXi Agent\docs\superpowers\plans\2026-07-22-yunxi-agent-v2-1-1-to-v2-2-0-personal-wechat-roadmap.md`
- 总纲 SHA-256：`2DF30D503F46CFE7496567F5011BF5CBFB8BA73C91C2D2FA3E9F29AF0932EA0F`
- 当前代码基线：`Cargo workspace=2.1.6`，`HEAD=9e1fc57`，当前已存在 annotated `v2.1.6` tag，`v2.2.0` tag 不存在。
- 审核结论转化：全范围审核不通过；合并开发只改变开发组织方式，不减少原 `v2.1.7`、`v2.1.8`、`v2.1.9`、`v2.2.0` 的任何功能、测试、边界或证据要求。
- 目标版本：合并大版本 `v2.2.0`
- 必须创建的发布 tag：仅在全部内部门禁和真实验证通过后创建新的 annotated `v2.2.0`
- 开发目录：`D:\YunXi Agent`
- 报告类型：面向开发者的合并版本开发指令；本报告不是完成声明。

## 一、硬性约束

以下约束是 `v2.2.0` 合并开发、验证、发布和文档同步的前置要求，不得因为上下文过长、任务拆分、实现困难、局部修复或合并开发口径而忽略、弱化或绕过。

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

## 二、合并开发口径

本轮从原总纲的分版本推进，调整为一个 `v2.2.0` 合并开发线。开发者必须准确理解三点：

1. 合并开发不是跳过 `v2.1.7`、`v2.1.8`、`v2.1.9`，而是把它们转为 `v2.2.0` 内部强制门禁。
2. 当前 `v2.1.6` 仍有两个 P1 阻塞点；在这两个阻塞点关闭并重新审核通过前，不得进入远程审批、可靠回信、陪伴记忆和最终真实联调的大范围实现。
3. `v2.2.0` 发布 tag 只能在所有内部门禁、全量回归、真实 iLink/Provider 联调、ConPTY 回归和文档证据通过后创建；不得提前打 tag，不得移动历史 tag。

建议开发者把合并线组织为一个主分支上的连续实现批次，但每个批次必须保留清晰提交、测试证据和日志。内部门禁编号仅用于任务追踪，不代表可以创建旧小版本发布 tag。

合并线的最终产品边界必须与总纲保持一致：微信入口只通过本机 YunXi CLI 显式启动的前台 `weixin serve` 承载，使用 iLink 长轮询；不开放公网回调地址，不新增入站端口，不新增后台常驻守护进程，不允许远程微信消息改变 workspace、cwd、provider、model、sandbox 或 approval 配置。首期硬指标仍是个人微信私聊可靠性，群聊、企业微信、公众号、通用多渠道网关、联系人抓取、自动加好友、群发和未经真实验证的主动推送均不属于完成能力。

## 三、当前阻塞点

### P1-1：会话历史没有按 parent/history 规则恢复

当前 `WeixinTurnSupervisor` 已能把已准入文本送入既有 Runtime，但实现只给 `AgentConfig` 传入 `session_id`。`AgentConfig` 本身有 `parent_session_id` 字段，`yunxi-agent-runtime` 的 `restore_parent_history` 只在 `parent_session_id` 存在时加载历史。本地 CLI 的 interactive 路径会把上一轮 active session 作为下一轮 parent，这是当前既有会话历史规则。

开发要求：

- 明确定义微信绑定保存的是 root session、active session、last completed child session 还是 pending turn session；不得只把同一个 deterministic `session_id` 反复传给每一轮。
- 建议把 `WeixinConversationBinding` 扩展为能表达 `root_session_id`、`active_session_id`、`last_completed_session_id` 或等价字段；每个 pending turn 需要有稳定的 `turn_session_id` 与 `parent_session_id`，以支持重试幂等。
- 同一私聊第一轮可以无 parent 创建初始 session；后续轮次必须把上一轮成功完成的 session 作为 `parent_session_id`，并在当前轮完成后原子更新 active session。
- Runtime 调用必须继续走 `Agent::run_with_backend_stream` 和 `AgentInput::text`，不得新建第二套 Runtime、第二套 history loader 或第二套 session store。
- pending 重试时必须复用同一 `turn_session_id`，不能因为 QueueFull、进程重启或网络错误重复写入多份历史。

必须增加测试：

- 使用真实 `YunXiRuntimeBackend` 与可观测 fake provider，连续运行同一微信私聊两轮，证明第二轮 provider messages 包含第一轮 user/assistant 历史。
- 关闭并重新加载 `FileSessionStore` 或模拟新进程，证明 history、persona、memory 和 session parent 链仍可恢复。
- 覆盖不同 account、相同 peer 的隔离；同一 account、不同 peer 的隔离；同一 conversation 的并发串行。
- 覆盖 pending 失败、重试和恢复时不重复写 history、不重复写 memory、不重复写最终 sink。

主要接入点：

- `D:\YunXi Agent\crates\yunxi-agent-weixin\src\turn_supervisor.rs`
- `D:\YunXi Agent\crates\yunxi-agent-storage\src\weixin_state.rs`
- `D:\YunXi Agent\crates\yunxi-agent-runtime\src\lib.rs`
- `D:\YunXi Agent\crates\yunxi-agent-core\src\config.rs`
- `D:\YunXi Agent\crates\yunxi-agent-cli\src\interactive.rs`
- `D:\YunXi Agent\crates\yunxi-agent-weixin\tests\turn_supervisor_tests.rs`

### P1-2：QueueFull 后 Ready pending 没有恢复路径

当前 `serve` 在 `commit_inbound_batch` 后调度 Runtime。若 dispatcher 返回 QueueFull，cursor 和 Ready pending 已经持久化，但错误只进入运行时计数，没有 Ready pending 的 drain、退避重试或重启恢复。

开发要求：

- 在 `weixin serve` 启动时扫描 Ready pending，并在每轮 poll 前或 poll 后执行可恢复 drain。
- QueueFull 不得把 Ready pending 标为失败；应记录脱敏诊断、重试次数、下次重试时间和最后错误分类。
- 实现 bounded retry，避免无限热循环；同一 pending 的调度必须以 pending_id/turn_session_id 幂等。
- cursor 已提交后，后续 poll、重启或容量释放必须能继续处理旧 Ready pending。
- 生产路径和测试路径都必须使用同一 drain 逻辑，不能只在测试中直接调用 supervisor 绕过 serve。

必须增加测试：

- 真实 serve 集成测试：队列满、cursor 已提交、pending 保持 Ready、解除容量后下一轮 poll 完成。
- 重启恢复测试：QueueFull 后关闭 serve，重新启动后 drain Ready pending，最终只产生一次 Runtime 结果。
- 幂等测试：同一 pending 反复调度只写一次 session history、一次 memory、一次 sink 或一次 delivery 记录。
- 脱敏诊断测试：错误信息不得包含 token、context token、用户原文、原始 user ID 或 provider wire。

主要接入点：

- `D:\YunXi Agent\crates\yunxi-agent-weixin\src\serve.rs`
- `D:\YunXi Agent\crates\yunxi-agent-weixin\src\turn_supervisor.rs`
- `D:\YunXi Agent\crates\yunxi-agent-storage\src\weixin_state.rs`
- `D:\YunXi Agent\crates\yunxi-agent-weixin\tests\pending_inbound_recovery_tests.rs`

## 四、v2.2.0 内部门禁

### G-2.1.7：远程文字控制、审批、追问与取消

目标：微信私聊可以通过明确命令控制正在运行的 Agent turn，但所有控制必须桥接既有 `AgentRunControl`，不得创建第二套审批系统。

必须实现：

- 命令路由：`/status`、`/stop`、`/approve <id>`、`/deny <id> [reason]`、`/answer <id> <text>`。
- request ID 必须不透明、绑定 account、peer、conversation、turn 和用途，一次消费，可过期。
- 未匹配 ID、跨对端、重复消费、过期、断连响应必须拒绝并写入脱敏诊断。
- 自然语言消息不得被解释为审批；例如“好的”“同意”不能视为 approve。
- 微信远程控制不得提升 sandbox、approval、cwd、provider 或 tool 权限；只能响应已存在的 AgentRunControl 请求。
- `/stop` 只能取消当前 account/peer/conversation/turn 对应的既有 cancellation token，不能停止整个微信服务，不能取消其他私聊、其他账户或本地 CLI/TUI 会话。
- 审批展示只允许包含安全的 action、reason、cwd 标签、请求类型、过期时间和脱敏上下文；不得展示隐藏推理、原始 provider payload、环境变量、token、未脱敏工具输出、本地完整路径或用户原文。
- approval 与 user-input 必须设置本地可配置但有上限的等待期限；到期后要向既有 one-shot response 明确发送拒绝或 `None`，并记录脱敏原因，不能让 turn 因手机无回应永久卡死。
- 服务关闭、队列满和 cancel 竞态必须有确定状态。

技术建议：

- 在 `yunxi-agent-weixin` 内新增远程控制状态机和命令 parser，输出强类型 enum。
- 审批、追问和取消桥接到 `AgentRunControl` 或现有 runtime control facade；不要让微信模块直接改 Runtime 内部状态。
- request ID 和 pending control request 持久化在 `WeixinStateStore` 或等价微信 state 中，状态含 `Pending`、`Consumed`、`Expired`、`Rejected`、`Cancelled`。
- 所有命令响应只输出脱敏摘要；内部错误保留分类，不暴露路径、secret、provider wire。

必须测试：

- approve、deny、answer、stop、status 正常路径。
- ID 不存在、跨账号、跨私聊、重复、过期、断连、服务关闭、QueueFull。
- `/stop` 作用域隔离：只取消目标私聊 turn，不影响其他私聊、其他账户、本地 CLI/TUI 或整个 `weixin serve` 前台服务。
- 审批展示脱敏：安全 action/reason/cwd 标签存在，provider wire、环境变量、token、完整路径和未脱敏工具输出不存在。
- approval/user-input 超时：到期后 one-shot 收到明确拒绝或 `None`，turn 不永久悬挂。
- sandbox/approval 不被远程提升。
- 自然语言不触发审批。

参考路径：

- `D:\YunXi Agent\crates\yunxi-agent-core\src\stream.rs`
- `D:\YunXi Agent\crates\yunxi-agent-runtime\src\lib.rs`
- `D:\YunXi Agent\crates\yunxi-agent-cli\src\interactive.rs`
- `D:\源码\reasonix\internal\bot\gateway.go`
- `D:\源码\reasonix\internal\botruntime\runtime.go`
- `D:\源码\OpenAkita` 当前本机缺失；实现本门禁前必须补齐或在日志中记录无法引用的原因与替代依据。

### G-2.1.8：可靠回信与安全流式输出合并

目标：把既有 Runtime 的公开文本输出安全投递回微信，建立可靠 delivery spool；不得泄露 reasoning、tool event、provider wire、路径或秘密。

必须实现：

- 只消费公开文本类 `AgentEvent` 或等价最终文本，不消费 reasoning、tool event、内部 trace、provider 原始响应。
- 使用既有流式事件和 `AgentRunControl`，不得复刻 Provider streaming。
- 接入 iLink `sendmessage`，支持最终文本发送；逐 token 或分段流式必须等可靠投递边界成熟后再打开。
- 按 Unicode、段落、代码块和微信长度限制安全分段，避免截断 surrogate、组合字符和代码块围栏。
- 建立 delivery spool，至少记录 delivery id、turn id、message hash、segment index、retry count、last status、last redacted error、next retry time。
- stale context 只在已知可恢复语义下清理并有限重试；超时或断连结果不明时不得盲目重复发送。
- typing 或预提示失败不得把最终回答误报为失败。

必须测试：

- 4xx、5xx、stale context、超时、断连、重复响应、重启待投递、Unicode 长文本、代码块长文本、局部失败。
- 最终文本不包含 reasoning/tool/provider wire。
- delivery spool 重启后继续投递，且不会重复发送已确认成功的 segment。
- 日志和证据脱敏。

参考路径：

- `D:\YunXi Agent\crates\yunxi-agent-protocol\src\lib.rs`
- `D:\YunXi Agent\crates\yunxi-agent-runtime\src\lib.rs`
- `D:\YunXi Agent\crates\yunxi-agent-storage\src\weixin_state.rs`
- `D:\源码\openclaw-weixin\src\messaging`
- `D:\源码\openclaw-weixin\src\storage\sync-buf.ts`
- `D:\源码\reasonix\internal\bot\weixin\weixin.go`
- `D:\源码\CowAgent\channel\weixin`
- `D:\源码\Tencent\openclaw-weixin` 当前本机缺失；需确认是否与 `D:\源码\openclaw-weixin` 为同一来源，不能擅自用未确认路径替代审计点名路径。

### G-2.1.9：陪伴、人格与记忆连续性

目标：微信入口复用 YunXi 现有 persona、memory、relationship graph、companion control 和同一 `SessionStore`，形成本地 CLI 与微信入口一致的陪伴连续性。

必须实现：

- `yunxi weixin serve --companion` 只显式启用既有 companion 路径，不创建第二套人格、第二套 memory schema 或第二套 relationship store。
- 微信消息正文与通道元数据分离；account、peer、nickname、channel identity 不得作为可控 prompt 注入。
- 本地 CLI 与微信连续交互必须共享已允许的 persona、偏好、关系和长期记忆。
- 增加 `yunxi weixin session reset --account ... --peer ... --confirm` 或等价命令，严格限定目标会话。
- reset/archive 必须沿用既有 YunXi 语义，不静默删除长期记忆，不绕过用户确认。
- 未完成真实验证前，不允许后台主动微信推送。

必须测试：

- CLI 先写入偏好，微信后续可按既有 memory 语义 recall。
- 微信先形成允许记忆，CLI 后续可恢复。
- companion enabled/disabled、quiet hours、tool request 权限和 memory extraction mode 与本地 CLI 一致。
- session reset 只影响指定 account/peer/conversation，不影响其他对端或本地 session。
- 通道元数据 prompt injection 被隔离。

参考路径：

- `D:\YunXi Agent\crates\yunxi-agent-companion`
- `D:\YunXi Agent\crates\yunxi-agent-persona`
- `D:\YunXi Agent\crates\yunxi-agent-storage\src\lib.rs`
- `D:\YunXi Agent\crates\yunxi-agent-runtime\tests\general_companion_tests.rs`
- `D:\源码\Leon\core\context\LEON.md` 当前本机缺失。
- `D:\源码\Leon\core\context\ARCHITECTURE.md` 当前本机缺失。
- `D:\源码\Letta Code` 当前本机缺失。
- `D:\源码\Project N.E.K.O.` 当前本机缺失。

### G-2.2.0：完整 CLI 接入、真实联调与发布收口

目标：把微信接入从开发闭环收口为可发布 `v2.2.0`，完成 CLI、evals、真实联调、ConPTY 回归、文档和证据。

必须实现：

- 收口 `login`、`status`、`doctor`、`serve`、`pair`、`session reset`、`logout` 的帮助、onboarding、扫码恢复、状态文本、JSON、JSONL 和错误码边界。
- 新增 `D:\YunXi Agent\evals\weixin\`，覆盖协议 Mock、状态迁移、配对、审批、回信、重启恢复、安全诊断和真实联调清单。
- 真实 iLink 测试只能读取系统凭证或受保护环境；不得把 token、context token、原始用户 ID、原始正文写入日志或证据。
- 使用指定测试账户完成真实扫码、私聊、配对、真实 Provider、拒绝审批、批准安全测试动作、取消、重启恢复和投递回执。
- 文档必须明确说明 iLink 扫码绑定的是 Bot 身份，不等同于控制扫码个人号；当前完成范围只包含已配对个人私聊，不包含群聊、企业微信、公众号、客服、通用多渠道网关、联系人抓取、自动加好友、群发或未验证主动推送。
- 更新 support bundle、迁移说明、rollback 说明、onboarding、扫码恢复说明、文档索引和状态文档。
- Windows ConPTY 证明前台微信服务没有破坏既有 TUI/CLI 行为。
- 全部通过后创建新的 annotated `v2.2.0` tag；已有 `v2.1.0` 至 `v2.1.6` tag 不删除、不移动、不覆盖。

必须测试：

- `cargo fmt --all -- --check`
- `cargo check --workspace`
- `cargo test --workspace -- --test-threads=1`
- `cargo build --workspace --release`
- `target\release\yunxi.exe --version`
- `git diff --check`
- `git fsck --full --no-dangling`
- v205、v206、v207、v207-hotfix、v208、v209、v210 ConPTY verifier。
- `evals/weixin` 全部离线和真实条件测试。
- CLI 帮助、onboarding、扫码恢复、status/doctor JSON/JSONL、群聊排除和 iLink Bot 身份说明均与实际实现一致。
- tag 类型、tag target、远端状态和历史 tag 保持性检查。

参考路径：

- `D:\YunXi Agent\evals\weixin` 当前不存在，需在合并开发中新增。
- `D:\YunXi Agent\scripts\conpty\v210`
- `D:\YunXi Agent\docs\reports\evidence`
- `D:\YunXi Agent\crates\yunxi-agent-cli`
- `D:\源码\Tencent\openclaw-weixin` 当前本机缺失；需要补齐或确认等价来源。
- `D:\源码\reasonix\internal\bot\weixin`

## 五、既有能力保留门禁

合并开发期间，以下能力必须持续通过。任何一项回归都阻塞 `v2.2.0`：

1. `v2.1.0` TUI 主视图、Details、Approval、取消、流式文本、CJK/Emoji、窄屏和终端恢复。
2. plain、JSON、JSONL、no-TUI、pipe、CI 和 forced fallback 输出契约。
3. Provider、model、cwd、sandbox、approval、context window 和现有 companion 配置。
4. persona、memory schema、recall、relationship graph、SessionStore 和旧 session JSON 兼容。
5. QR 登录、安全凭证、WeixinStateStore、原子写入、schema migration、账户锁、stale lock 和 logout 边界。
6. getupdates cursor、pairing、重复消息幂等、加密 pending inbound、重启恢复和脱敏诊断。
7. 群聊关闭、秘密不进入日志、陌生发送者不进入 Runtime、微信不获取 shell 旁路权限，不开放公网回调地址，不新增入站端口。
8. 所有已有 annotated tag、历史报告、证据和回滚路径。
9. 前台 `weixin serve`、扫码、长轮询、远程控制和回信投递不得破坏本地 CLI、JSON/JSONL、TUI、ConPTY、Provider、Approval 和 companion 的既有行为。

禁止通过删除旧测试、降低断言、改变历史 tag、清理历史证据、移动功能到测试范围外或把未完成能力写成已完成来制造通过。

## 六、参考源码状态与使用规则

本次撰写开发报告时只读核对了审核报告点名的参考源码路径。结果如下：

| 参考路径 | 本机状态 | 使用要求 |
| --- | --- | --- |
| `D:\源码\reasonix\internal\bot\weixin\weixin.go` | 存在 | 参考长轮询、重启恢复和通道状态恢复逻辑，Rust 化到 YunXi state/serve。 |
| `D:\源码\reasonix\internal\bot\gateway.go` | 存在 | 参考消息网关和控制路由，不引入 Go runtime。 |
| `D:\源码\reasonix\internal\botruntime\runtime.go` | 存在 | 参考通道到 Runtime 的桥接思路，必须映射为 YunXi 既有 Runtime。 |
| `D:\源码\openclaw-weixin\src\messaging` | 存在 | 参考 sendmessage、context token 和错误分类，Rust 化实现。 |
| `D:\源码\openclaw-weixin\src\storage\sync-buf.ts` | 存在 | 参考 sync buffer、游标和待投递恢复边界，Rust 化实现。 |
| `D:\源码\CowAgent\channel\weixin` | 存在 | 参考个人陪伴通道到核心交接，不引入 Python channel factory 或第二套 Runtime。 |
| `D:\源码\OpenAkita` | 缺失 | G-2.1.7 前必须补齐 canonical 源码、记录 URL/HEAD，或明确说明无法引用并给出替代依据。 |
| `D:\源码\Leon\core\context\LEON.md` | 缺失 | G-2.1.9 前必须补齐或记录无法引用原因。 |
| `D:\源码\Leon\core\context\ARCHITECTURE.md` | 缺失 | G-2.1.9 前必须补齐或记录无法引用原因。 |
| `D:\源码\Letta Code` | 缺失 | G-2.1.9 前必须补齐或记录无法引用原因。 |
| `D:\源码\Project N.E.K.O.` | 缺失 | G-2.1.9 前必须补齐或记录无法引用原因。 |
| `D:\源码\Tencent\openclaw-weixin` | 缺失 | G-2.2.0 前必须确认是否与 `D:\源码\openclaw-weixin` 等价；不得擅自替代审计点名路径。 |

开发者补齐参考源码时必须写入日志，记录仓库 URL、clone 方式、HEAD、是否 shallow、关键文件是否存在。若涉及网络拉取到 `D:\源码`，必须明确告知用户；若审核报告没有给出 canonical URL，不得凭名称猜测并静默拉取。

## 七、文档与状态同步

完成每个合并开发批次后，必须同步以下文档：

- `D:\YunXi Agent\docs\reports\development\`：新增或更新开发报告正本。
- `D:\YunXi Agent\docs\reports\audits\`：仅审核者新增审核报告，开发者不得伪造审核通过。
- `D:\YunXi Agent\docs\reports\README.md`：报告索引必须指向真实存在的正本。
- `D:\YunXi Agent\docs\README.md`：如 CLI、微信命令或状态文档入口变化，必须更新。
- `D:\YunXi Agent\docs\weixin.md` 或等价微信文档：必须准确反映 sendmessage、远程审批、群聊和真实联调状态。
- `D:\YunXi Agent\docs\reports\evidence\`：真实联调、ConPTY、manifest、脱敏证据必须可追溯。
- `C:\Users\24763\Desktop\YunXi Agent开发报告\`：桌面开发报告副本必须由项目正本单向复制并校验 SHA-256。
- `C:\Users\24763\Desktop\YunXi Agent开发日志.md`：桌面日志必须由项目日志单向同步并校验 SHA-256。

验证通过前，文档只能写“实现中”“待验证”“不通过”“禁止宣称完成”，不得写“完成”“已发布”“已通过”。

## 八、统一验证与清理

开发者应按批次实现，完成一批后统一验证。建议最低验证集合：

```text
cargo fmt --all -- --check
cargo check --workspace
cargo test --workspace -- --test-threads=1
cargo build --workspace --release
target\release\yunxi.exe --version
git diff --check
```

进入 `G-2.2.0` 发布收口时，还必须补充：

- `evals/weixin` 离线和真实条件测试。
- 真实 iLink 扫码、私聊、配对、审批、取消、投递回执和重启恢复。
- v205、v206、v207、v207-hotfix、v208、v209、v210 ConPTY verifier。
- tag object、tag target、远端 refs 和历史 tag 保持性核验。
- 证据脱敏检查，禁止 token、context token、用户原文、原始用户 ID、Provider wire、系统路径泄露。

清理要求：

- 每次阶段验证完成后，应清理编译中间产物，避免硬盘占用。
- 对 `D:\YunXi Agent\target` 这类递归删除必须先向用户明确请求确认，确认内容必须包含精确绝对路径。
- 不得执行未经确认的 `git clean`、递归删除、强制移动、清空目录、gc/prune、系统安装/卸载、PATH/注册表修改。

## 九、发布与回滚门禁

`v2.2.0` 发布前必须同时满足：

1. P1-1 和 P1-2 重新审核通过。
2. G-2.1.7、G-2.1.8、G-2.1.9、G-2.2.0 全部内部门禁完成。
3. 既有能力保留门禁无回归。
4. 全量测试、release build、ConPTY、真实 iLink/Provider 联调和证据脱敏全部通过。
5. 开发报告、设计文档、索引、状态文档、日志与实际代码一致。
6. 工作树中没有未解释的源码、测试、配置或文档变更。

满足以上条件后，创建新的 annotated `v2.2.0` tag，并记录 tag message、tag object、peeled commit、远端 push 状态和回滚说明。历史 `v2.1.0` 至 `v2.1.6` tag 不得删除、移动或覆盖。

## 十、交付清单

开发者完成合并版本后，至少交付：

- 源码变更：storage、weixin、runtime/core/cli 接入、delivery spool、remote control、companion 连续性和 evals。
- 测试变更：turn supervisor、serve recovery、remote control、sendmessage、delivery、companion、session reset、evals/weixin、ConPTY。
- 文档变更：开发报告、审核申请说明、README/report index、微信用户文档、迁移/rollback、support bundle 说明。
- 证据变更：脱敏真实联调证据、ConPTY 证据、测试输出摘要、tag 和远端核验证据。
- 日志变更：本次工作目标、执行流程、修改文件、文件路径、验证结果、提交和推送状态、署名。

署名：开发报告撰写者
