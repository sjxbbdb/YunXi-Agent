# YunXi Agent v2.2.0 微信接入合并线代码审核报告

- 审核时间：2026-07-31 17:27:19 +08:00
- 审核者：审核者
- 审核范围：当前工作树相对 `v2.1.0` 至 `v2.2.0` 合并开发总纲的全部要求
- 审核基准：`D:\YunXi Agent\docs\superpowers\plans\2026-07-22-yunxi-agent-v2-1-1-to-v2-2-0-personal-wechat-roadmap.md`
- 当前开发报告：`D:\YunXi Agent\docs\reports\development\2026-07-31-090149-yunxi-agent-v2-2-0-merged-weixin-development-report.md`
- 当前版本：Cargo workspace `2.2.0`
- 当前 HEAD：`f0c84d2a4620bbbaee3887b964605d5c8e220def`，提交信息为 `feat: close v2.2.0 weixin p1 runtime blockers`
- 工作树状态：存在未提交源码、测试、配置和文档修改
- tag 状态：`v2.1.0` 至 `v2.1.6` 历史 tag 仍存在；`v2.2.0` tag 尚未创建

## 一、审核结论

**审核不通过。禁止宣称 v2.2.0 完成，禁止创建或推送 v2.2.0 发布 tag。当前必须继续留在 v2.2.0 合并开发线整改，整改完成后重新审核。**

本次审核不是只检查版本号，也不是只检查离线状态。源码中已经有会话历史、QueueFull Ready pending 恢复、远程控制 hub、delivery spool、session reset 和微信 eval 的实现骨架，但关键运行闭环尚未满足总纲的“可用、可恢复、可验证”要求。尤其是微信端控制命令目前没有回信路径，审批提示也没有发送路径；这会使远程控制在真实个人微信上不可用，属于当前版本的 P1 阻塞点。

`v2.1.7`、`v2.1.8`、`v2.1.9` 被合并为 `v2.2.0` 的内部强制门禁，不代表这些内容可以跳过。本报告只对照当前合并线与总纲要求，不把后续未要求的功能倒算为当前缺陷；但当前合并线的所有内部强制门禁、既有能力保留门禁和真实发布证据都必须关闭后，才能审核通过。

## 二、已确认通过或部分通过的内容

### 1. 版本与历史回滚边界

- Cargo workspace 当前为 `2.2.0`。
- 已有 `v2.1.0` 至 `v2.1.6` tag 未被删除、移动或覆盖。
- `v2.2.0` tag 尚不存在，符合“审核通过前不得创建发布 tag”的要求；但这也意味着当前不能被视为已发布版本。

### 2. 会话父子关系和历史恢复

这部分相较此前 P1 已有实质整改，当前不能再简单认定为“同一 session_id 反复复用”。

- `D:\YunXi Agent\crates\yunxi-agent-storage\src\weixin_state.rs:1351-1370` 保存 `root_session_id`、`active_session_id`、`last_completed_session_id` 和兼容 `session_id`。
- `D:\YunXi Agent\crates\yunxi-agent-storage\src\weixin_state.rs:969-1005` 为 pending turn 固定 `turn_session_id`、保存 `parent_session_id`，并将 pending 转为 Running 后原子保存。
- `D:\YunXi Agent\crates\yunxi-agent-storage\src\weixin_state.rs:2464-2518` 依据上一轮成功 session 设置 parent，并在成功或失败后更新/恢复 binding。
- `D:\YunXi Agent\crates\yunxi-agent-weixin\src\turn_supervisor.rs:131-144` 将当前 turn session 和上一轮 completed session 传入既有 `AgentConfig`。
- `D:\YunXi Agent\crates\yunxi-agent-weixin\tests\turn_supervisor_tests.rs:295-354` 使用真实 `YunXiRuntimeBackend`、`FileSessionStore` 重载和可观测 provider，验证第二轮请求包含第一轮 user/assistant 历史。

因此，P1-1 的核心实现和主要测试证据已具备。但仍需要把失败、取消、delivery 部分失败、进程在 Running 状态退出等场景纳入最终回归，不能以当前两轮成功路径替代完整恢复证据。

### 3. QueueFull 的 Ready pending 恢复

- `D:\YunXi Agent\crates\yunxi-agent-weixin\src\serve.rs:343-445` 已在 serve 循环中扫描 Ready pending，QueueFull 时记录重试次数、退避时间和脱敏错误；生产 CLI 在 `D:\YunXi Agent\crates\yunxi-agent-cli\src\weixin.rs:240-277` 已注入 runtime dispatcher、delivery dispatcher 和 remote control hub。
- `D:\YunXi Agent\crates\yunxi-agent-weixin\src\serve.rs:1047-1114` 已有 QueueFull 后容量恢复测试。
- `D:\YunXi Agent\crates\yunxi-agent-weixin\src\serve.rs:1115-1200` 已有关闭后重新启动并 drain Ready pending 的测试。

该项只能判定为**部分通过**：现有 serve 测试使用测试 dispatcher，默认 `background_runtime_dispatch=false`，没有覆盖 CLI 真实设置为 `true` 的后台 task 生命周期、服务关闭等待、长运行任务租约和 Running 崩溃恢复。见本报告阻塞点 5。

### 4. 基础安全和分段能力

- 入站正文和回信 payload 使用认证加密 state 存储；现有 delivery 测试验证普通 state JSON 不出现正文、原始 user ID 和 context token。
- `D:\YunXi Agent\crates\yunxi-agent-weixin\src\delivery.rs:340-369` 按 Unicode grapheme 分段，现有测试覆盖组合字符/Emoji 不被拆断。
- `D:\YunXi Agent\crates\yunxi-agent-weixin\src\remote_control.rs:33-64` 只识别显式 slash command，自然语言不会直接触发审批。
- `cargo fmt --all -- --check` 已执行并通过。

## 三、当前阻塞点

### P1-1：远程控制没有微信回执和审批提示发送路径

源码已经把 hub 注入生产 CLI，但运行闭环没有完成：

- `D:\YunXi Agent\crates\yunxi-agent-weixin\src\serve.rs:247-265` 解析 `/status`、`/stop`、`/approve`、`/deny`、`/answer` 后只调用 `hub.handle_command`，成功返回的 `WeixinRemoteControlOutcome` 被丢弃，没有写入 delivery spool，也没有调用 iLink `sendmessage`。
- `D:\YunXi Agent\crates\yunxi-agent-weixin\src\turn_supervisor.rs:368-389` 收到 approval/user-input 后只注册 request；`WeixinRemoteControlPrompt` 被用于生成超时 task，prompt 本身没有被发送给微信用户。
- `D:\YunXi Agent\crates\yunxi-agent-cli\src\weixin.rs:240-277` 的生产 wiring 只能证明对象被注入，不能证明微信用户能看到 request ID、审批内容、控制结果或超时结果。

实际后果：用户无法从微信获得 `/status` 结果，无法看到待审批的安全摘要，无法确认 `/approve`、`/deny`、`/answer`、`/stop` 是否生效。总纲 G-2.1.7 要求的是可用的远程文字控制，不是只有内存 hub 和单测。

整改要求：为控制响应和 approval/user-input prompt 建立明确的安全 outbound 接口，复用当前 delivery 认证、分段、脱敏和幂等设计；不能直接打印原始 request、provider payload、完整路径或用户正文。为成功、未匹配、跨 scope、过期、重复消费、服务关闭和 channel closed 分别发送可理解且脱敏的结果，并测试消息确实进入 transport。

### P1-2：远程控制注册失败被静默忽略

- `D:\YunXi Agent\crates\yunxi-agent-weixin\src\turn_supervisor.rs:154-160` 对 `register_cancellation` 使用 `let _ =`，状态持久化或锁错误时不会阻止运行，也不会向用户报告 stop 不可用。
- `D:\YunXi Agent\crates\yunxi-agent-weixin\src\turn_supervisor.rs:378-388` 对 approval/user-input 注册只使用 `if let Ok(prompt)`，注册失败时静默跳过；Agent 仍可能等待既有 one-shot，用户却没有 request ID。

这违反“审批/追问超时不能永久悬挂”和“服务关闭、队列满、cancel 竞态必须有确定状态”的要求。整改必须：注册失败进入可观察的脱敏错误路径，确保 one-shot 得到拒绝或 `None`；对取消注册失败要让本轮明确降级或失败，不能假装远程控制已开启。

### P1-3：流式 AgentEvent 被直接丢弃，尚未实现安全输出合并

- `D:\YunXi Agent\crates\yunxi-agent-weixin\src\turn_supervisor.rs:368-389` 使用 `Some(_event) => {}`，所有流式事件都被忽略。
- 生产回信只在 `D:\YunXi Agent\crates\yunxi-agent-weixin\src\turn_supervisor.rs:194-216` 读取 `result.final_response` 后调用 sink。
- `D:\YunXi Agent\crates\yunxi-agent-weixin\src\delivery.rs:62-127` 只实现最终文本分段入 spool；没有公开文本事件过滤器、事件去重/合并器、typing 生命周期或事件到最终文本的可验证边界。

总纲 G-2.1.8 要求只消费公开文本类事件，过滤 reasoning/tool/provider wire，并使用既有 stream/control 形成安全输出。当前实现既没有泄露这些事件，也没有完成安全合并，因此只能判定为**未完成**，不能以“最终文本可发送”替代该门禁。整改需要明确当前版本采用“最终文本优先”还是“受控分段流式”，并实现公开事件白名单、去重、最终收口和 typing 失败不影响最终回答的测试。

### P1-4：delivery 对结果不明的错误直接自动重试，存在重复微信消息风险

- `D:\YunXi Agent\crates\yunxi-agent-weixin\src\delivery.rs:308-318` 对 retryable 错误安排再次发送。
- `D:\YunXi Agent\crates\yunxi-agent-weixin\src\delivery.rs:415-429` 将 Timeout、Network、HTTP 408/425/429/5xx 以及部分 API 错误统一视为可重试。
- `D:\YunXi Agent\crates\yunxi-agent-weixin\src\delivery.rs:88-102` 和 `193-213` 将 `delivery_id` 放入 `client_id`，但当前项目没有官方 iLink 证据证明该字段对 sendmessage 具有服务端幂等语义。

HTTP 超时、响应体断连和网络错误可能发生在服务端已接受消息之后。没有明确“确定未送达”或已确认的服务端幂等键时，自动重试会重复发送个人微信回信。整改必须把 definite failure 与 outcome unknown 分开；只有官方协议明确支持的幂等条件才能自动重试，未知结果应进入人工/受控恢复状态，并增加 4xx、5xx、超时、断连、重复调用和重启测试。

### P1-5：多段 spool 不是整体原子提交，部分回信会与 turn 失败状态并存

- `D:\YunXi Agent\crates\yunxi-agent-weixin\src\delivery.rs:83-125` 在循环内逐段调用 `enqueue_pending_delivery`，第 N 段失败时，前 N-1 段已经保存。
- `D:\YunXi Agent\crates\yunxi-agent-weixin\src\turn_supervisor.rs:194-216` 对 sink 返回错误使用 `?`；外层会把 turn 标记为失败，但已落盘的前序 delivery 仍可能继续发送。

这会产生“微信收到半截回答、Runtime turn 失败、后续恢复无法判断是否应该补发”的不一致。整改应提供单次 state-store 原子 batch enqueue，或在 delivery 记录上建立完整 message manifest、可恢复的分段状态和 turn 收口状态；必须覆盖第 N 段写入失败、进程重启和已确认 segment 不重复发送。

### P1-6：CLI 后台 dispatch 的租约和 Running 崩溃恢复没有关闭

- 生产 CLI 在 `D:\YunXi Agent\crates\yunxi-agent-cli\src\weixin.rs:240-277` 明确设置 `background_runtime_dispatch=true`。
- `D:\YunXi Agent\crates\yunxi-agent-weixin\src\serve.rs:352-375` 调度前调用 `record_pending_runtime_dispatch_inflight`，但 `D:\YunXi Agent\crates\yunxi-agent-storage\src\weixin_state.rs:624-655` 只写入 `next_retry_at_millis` 和诊断，pending 仍是 Ready。
- `D:\YunXi Agent\crates\yunxi-agent-storage\src\weixin_state.rs:504-521` 的 `load_ready_pending_inbound` 只扫描 Ready；`D:\YunXi Agent\crates\yunxi-agent-storage\src\weixin_state.rs:969-982` 才把它转换为 Running。当前没有将进程异常退出后遗留的 Running pending 安全恢复为 Ready 的启动路径。

长运行 turn 超过租约时可能被再次扫描；进程在 `begin_pending_runtime_turn` 之后退出时，pending 会永久停在 Running，重启无法继续。现有 `serve_loop_drains_ready_pending_after_queue_full_capacity_recovers` 和 `...after_restart_without_duplicate_runtime` 使用测试 dispatcher 且默认 foreground 模式，不能证明 CLI 真实后台路径安全。整改需要引入明确 lease/attempt token、任务退出等待或取消、stale Running 恢复规则，并增加 CLI wiring 的后台模式测试。

### P1-7：陪伴连续性和 session reset 缺少跨入口可执行证据

当前实现已出现 `--companion` 全局配置、微信 Runtime 复用和 `session reset` CLI 路径，但现有测试证据主要集中在 session history 和状态 store，未覆盖总纲 G-2.1.9 的完整门禁：

- 本地 CLI 先写偏好、微信读取；微信形成允许记忆、本地 CLI 读取。
- companion enabled/disabled、quiet hours、tool request 权限和 memory extraction mode 跨入口一致。
- reset 只影响指定 account/peer/conversation，并不遗留可继续使用的 binding、pending turn 或 remote request。
- 通道元数据不能通过 prompt 注入改变 persona、权限或工作区。

在这些测试和真实运行证据补齐前，不能宣称“通用型陪伴 agent 的个人微信连续性已完成”。参考实现必须继续复用 `D:\YunXi Agent\crates\yunxi-agent-companion`、`persona`、`storage`、`runtime`，禁止建立第二套记忆或人格系统。

### P1-8：离线 eval 把真实门禁写成恒真声明，不能代替证据

- `D:\YunXi Agent\crates\yunxi-agent-eval\src\lib.rs:770-777` 对 restart recovery 和 real integration 直接返回 `true`，只是声明需要人工门禁。
- `D:\YunXi Agent\crates\yunxi-agent-eval\src\lib.rs:1137-1152` 只证明离线场景加载、指标和安全字段可渲染，不能证明真实扫码、Provider、审批、取消、sendmessage 回执或重启恢复。
- `D:\YunXi Agent\evals\weixin\README.md` 中的真实联调清单是待执行门禁，不是已完成证据。

整改要求：离线 eval 可以保留为协议和安全回归，但必须另存脱敏的真实 iLink/Provider/ConPTY 证据，明确测试账户哈希、时间、退出码、状态和是否触网，不保存 token、context token、原始用户 ID、原文或 provider wire。

### P1-9：开发报告的基线与实际代码状态不一致

当前开发报告 `D:\YunXi Agent\docs\reports\development\2026-07-31-090149-yunxi-agent-v2-2-0-merged-weixin-development-report.md` 的“当前代码基线”仍记录 `Cargo workspace=2.1.6`、`HEAD=9e1fc57`、`v2.2.0 tag 不存在`；实际审核时 Cargo 为 `2.2.0`，HEAD 为 `f0c84d2`。报告不是完成声明，但该段已经不能作为当前状态文档。

开发者必须在下一整改批次更新开发报告、`docs/README.md`、`docs/weixin.md`、`docs/reports/README.md` 和日志索引，使“实现中/待验证/不通过”与源码和证据一致。当前 `docs/weixin.md` 仍把 slash control 和 final delivery 以已接入能力描述，但应同时明确控制回执、审批提示、流式合并和真实回信尚未通过审核。

## 四、总纲门禁判定

| 总纲门禁 | 当前判定 | 依据 |
| --- | --- | --- |
| G-2.1.7 远程文字控制、审批、追问、取消 | 不通过 | hub 已有，但没有微信回执/审批提示；注册失败静默；全链路测试不足 |
| G-2.1.8 可靠回信与安全流式输出合并 | 不通过 | final spool 有骨架，但 AgentEvent 被丢弃、没有公开事件合并、结果不明重试和原子多段提交未解决 |
| G-2.1.9 陪伴、人格、记忆连续性 | 部分实现，不通过 | session history 已验证，但跨入口 companion/memory/reset/prompt 隔离证据不足 |
| G-2.2.0 CLI 接入、真实联调、发布收口 | 不通过 | eval 真实门禁为声明；未完成真实 iLink/Provider/ConPTY/全量验证；文档基线过时；tag 不能创建 |
| 既有 TUI/CLI/Provider/Approval/SessionStore/微信登录能力保留 | 未完成验证 | 格式检查通过；完整构建、测试、release、ConPTY 和真实路径本轮因执行环境阻断未运行 |

## 五、开发者整改顺序

必须仍在 `D:\YunXi Agent` 及其工作树内完成，不能创建新的发布 tag 来绕过当前门禁。建议按以下顺序完成一批后统一验证：

1. 先补远程控制 outbound：把 approval/user-input prompt、控制 outcome、超时和错误接入安全 delivery；处理注册失败；将 scope 校验明确绑定 account、peer、dm、turn 和 purpose，并补一次消费、过期、跨账户/对端/turn、服务关闭测试。
2. 再收口 delivery：确认 iLink 官方 `client_id` 是否真的幂等；若不能证明，禁止把 timeout/network 当作自动重试；实现原子 manifest 或 batch enqueue，覆盖局部失败和重启恢复。
3. 再完成流式输出边界：使用既有 `AgentRunStreamReceiver`，建立公开文本白名单和 reasoning/tool/provider wire 过滤；明确 final-only 或受控分段策略；typing 失败不能影响最终回信。
4. 修复后台 dispatch 生命周期：定义 Ready/Running/lease/terminal 的恢复规则，避免租约过期重复调度，服务退出时等待或取消后台 task，增加 CLI `background_runtime_dispatch=true` 的集成测试。
5. 补齐 companion continuity、session reset、元数据隔离和跨 account/peer 测试；不新建第二套 persona、memory 或 Runtime。
6. 更新 `evals/weixin`：离线场景必须实际调用模块行为；真实门禁必须由脱敏证据完成，不能以恒真手工标记替代。
7. 统一更新开发报告、微信文档、报告索引、状态文档和日志；在文档、代码、证据完全一致前不得写“完成”。

## 六、统一验证门禁

本轮审核已执行并通过：

- `cargo fmt --all -- --check`

本轮因 Windows Codex 执行环境提权审批模型未配置，以下命令未能执行，必须由开发者在本机统一补跑并保留脱敏证据：

- `cargo check --workspace`
- `cargo test --workspace -- --test-threads=1`
- `cargo build --workspace --release`
- `target\release\yunxi.exe --version`
- `git diff --check`
- `git fsck --full --no-dangling`
- `evals/weixin` 全部离线和真实条件测试
- v205、v206、v207、v207-hotfix、v208、v209、v210 ConPTY verifier
- 真实 iLink 扫码、私聊、配对、Provider、审批拒绝/批准、answer、stop、重启恢复和 sendmessage 回执

未能执行的命令不是通过，也不是失败；在这些验证补齐前，发布门禁仍为未完成。编译中间产物本轮没有执行递归删除、`git clean` 或清空目录。由于项目开发约束要求对 `D:\YunXi Agent\target` 等递归清理先取得用户确认，本次未擅自清理，需在开发者完成验证后按确认流程处理。

## 七、参考源码与 Rust 化要求

本版本仍必须明确使用下列参考路径，不能只在单点上模仿接口：

- `D:\YunXi Agent\crates\yunxi-agent-core\src\stream.rs`、`runtime\src\lib.rs`、`storage\src\weixin_state.rs`：复用既有 AgentRunControl、公开事件、SessionStore 和状态原子提交，不创建第二套 Runtime。
- `D:\源码\reasonix\internal\bot\weixin\weixin.go`、`gateway.go`、`internal\botruntime\runtime.go`：参考长轮询、网关控制路由、重启恢复和通道到 Runtime 的桥接，Rust 化到 YunXi state/serve/supervisor。
- `D:\源码\openclaw-weixin\src\messaging`、`src\storage\sync-buf.ts`：参考 sendmessage、context token、错误分类、sync buffer 和待投递恢复；不能把未确认字段当作 iLink 幂等语义。
- `D:\源码\CowAgent\channel\weixin`：参考个人陪伴通道与核心交接，不引入 Python channel factory 或第二套人格/Runtime。
- 当前审核基准点名但本机缺失的 `D:\源码\OpenAkita`、`D:\源码\Leon\core\context\LEON.md`、`D:\源码\Leon\core\context\ARCHITECTURE.md`、`D:\源码\Letta Code`、`D:\源码\Project N.E.K.O.`、`D:\源码\Tencent\openclaw-weixin`，必须在开发日志中记录 canonical URL、HEAD、补齐状态或替代依据；不得凭项目名猜测并静默拉取。

参考源码只迁移整体能力和设计逻辑。非 Rust 项目必须按 YunXi 现有模块边界进行 Rust 复刻，不复制第二套依赖、运行时、权限配置或敏感数据模型。

## 八、发布结论

当前版本不能进入“发布收口”状态。开发者必须先整改本报告 P1，统一完成全量测试、真实 iLink/Provider/ConPTY 和脱敏证据，再申请重新审核。重新审核通过后，才允许创建新的 annotated `v2.2.0` tag；历史 tag、报告和证据不得删除、移动、覆盖或清理。

审核报告正本：`D:\YunXi Agent\docs\reports\audits\2026-07-31-172719-yunxi-agent-v2-2-0-merged-weixin-code-audit-report.md`

署名：审核者
