# YunXi Agent v2.1.4 微信状态持久化、诊断与安全账户生命周期开发报告

- 撰写时间：2026-07-27 18:30:50 +08:00
- 审核依据：`C:\Users\24763\Desktop\YunXi Agent审核报告\2026-07-27-181754-YunXi-Agent-v2.1.3-hotfix.1-微信登录复审核报告.md`
- 项目内审核报告归档：`D:\YunXi Agent\docs\reports\audits\2026-07-27-181754-yunxi-agent-v2-1-3-hotfix-1-weixin-login-audit-report.md`
- 审核报告 SHA-256：`5DAD97B2AFFD071D4AD100605A31ED7A2EBA559F21A60145B5F1D9FC1D4B7061`
- 总纲正本：`D:\YunXi Agent\docs\superpowers\plans\2026-07-22-yunxi-agent-v2-1-1-to-v2-2-0-personal-wechat-roadmap.md`
- 桌面总纲副本：`C:\Users\24763\Desktop\2026-07-22-yunxi-agent-v2-1-1-to-v2-2-0-personal-wechat-roadmap.md`
- 总纲 SHA-256：`2DF30D503F46CFE7496567F5011BF5CBFB8BA73C91C2D2FA3E9F29AF0932EA0F`
- 当前工作树基线：`HEAD=94cd83f642bcb1e1551a22ebe96f609f05afb67f`，`git describe=v2.1.3-hotfix.1-1-g94cd83f-dirty`
- 审核结论转化：`v2.1.3-hotfix.1` 审核通过，允许进入总纲图中的 `v2.1.4` 开发。
- 目标版本：`v2.1.4`
- 必须创建的 tag：新的 annotated `v2.1.4`
- 开发目录：`D:\YunXi Agent`
- 报告类型：下一阶段开发指令报告，主要面向开发者；本报告不是完成声明。

## 一、硬性约束

以下约束是 `v2.1.4` 开发、验证、发布和文档同步的前置要求，不得因为上下文过长、任务拆分、实现困难或局部修复而忽略、弱化或绕过。

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

## 二、阶段目标与边界

`v2.1.3-hotfix.1` 已补齐二维码登录、系统安全凭证存储、CLI Mock 验收和真实 Windows 登录复审证据，审核报告结论为通过。开发者现在可以进入 `v2.1.4`，但本阶段只能围绕“状态持久化、诊断与安全账户生命周期”展开。

`v2.1.4` 的目标是在接受任何微信用户消息前，让账户状态、通道游标、配对请求、待处理项和安全退出语义可检查、可恢复、可诊断。该版本是后续 `v2.1.5` 长轮询和 `v2.1.6` Runtime 绑定的状态底座，不是消息闭环版本。

本阶段必须交付：

1. 在 `crates/yunxi-agent-storage` 内新增独立、版本化的 `WeixinStateStore`。
2. 用原子写入协议替代微信通道状态的直接截断写入。
3. 为微信账户、游标、回执、会话绑定占位、secret-backed reply context 引用、待投递元数据、配对请求和加密待处理入站项建立可审阅记录。
4. 建立账户粒度、跨 workspace、跨进程的系统排他锁。
5. 完善 `yunxi weixin status --json`、`doctor`、`pair list|approve|deny` 和 `logout --confirm` 的脱敏机器输出与生命周期规则。
6. 补齐迁移、损坏记录、未完成临时文件、未来 schema、锁竞争、陈旧锁恢复、加密队列恢复和 request ID 过期测试。

本阶段禁止实现或宣称：

- 长轮询收消息、消息入站 dispatch、消息发送、流式回信、远程审批、Runtime 会话绑定、群聊、主动推送或完整微信聊天闭环。
- 第二套 Agent、第二套 Runtime、第二套人格、第二套记忆或第二套审批系统。
- 明文 token、二维码 payload、原始联系人标识、原始消息正文、Provider 私密 payload 或本地路径泄露到日志、JSON、错误文本、Markdown 证据或普通状态文件。

## 三、当前源码状态判断

开发前需要理解现有边界，避免把 `v2.1.4` 做成对旧 session 或登录 metadata 的局部补丁。

| 路径 | 当前状态 | `v2.1.4` 处理原则 |
| --- | --- | --- |
| `D:\YunXi Agent\crates\yunxi-agent-storage\src\lib.rs` | `SessionRecord` 与 `FileSessionStore` 管理 YunXi session；`FileSessionStore` 是普通 JSON 读写模型。 | 新增微信专属 store，不向 `SessionRecord` 塞微信字段，不把普通 session store 当微信状态通道。 |
| `D:\YunXi Agent\crates\yunxi-agent-weixin\src\account_store.rs` | `.yunxi/weixin/` 已保存非机密账户 metadata；当前 `save` 使用 truncate 写入并 `sync_all`。 | 保留账户 metadata 语义，但把后续通道状态交给独立 `WeixinStateStore` 的原子协议承载。 |
| `D:\YunXi Agent\crates\yunxi-agent-weixin\src\secret_store.rs` | `WeixinSecretStore` 已封装系统凭证存储和 fake store。 | 继续只保存凭证和数据密钥；状态 store 只能保存 secret reference 或加密 payload，不能塞秘密明文。 |
| `D:\YunXi Agent\crates\yunxi-agent-cli\src\weixin.rs` | `login/status/doctor/logout/pair/serve` 已有命令骨架；`serve` 仍明确不启动长轮询或 Runtime。 | 接入状态 store、锁、诊断和配对生命周期；`serve` 只做启动前状态/锁/安全能力检查，不做消息轮询。 |
| `D:\YunXi Agent\docs\weixin.md` | 已说明登录能力边界。 | 更新 `v2.1.4` 的状态持久化、诊断、logout、锁和“仍未接收消息”的边界。 |
| `D:\YunXi Agent\docs\reports\README.md` | 报告索引需要包含新审核和本开发报告。 | 本阶段结束时与实际文件保持一致。 |

## 四、核心实现方案

### 1. 新增独立 `WeixinStateStore`

建议在 `crates/yunxi-agent-storage` 内新增微信状态模块，例如：

```text
crates/yunxi-agent-storage/src/
  lib.rs
  weixin_state.rs
```

`lib.rs` 只暴露稳定 facade 类型，不泄露内部 JSON 文件布局。推荐类型包括：

- `WeixinStateStore`
- `FileWeixinStateStore`
- `WeixinStateSnapshot`
- `WeixinStateRecord`
- `WeixinStateSchemaVersion`
- `WeixinStateError`
- `WeixinStateMigration`
- `WeixinStateWriteOptions`
- `WeixinAccountLock`
- `WeixinPairRequest`
- `WeixinPendingInbound`
- `WeixinDeliveryRecord`

最低 schema 规则：

- 每个顶层文件和每条关键记录必须包含 `schema_version`、`created_at_millis`、`updated_at_millis`。
- 任何状态转换必须记录 `transitioned_at_millis` 或等价的转换时间。
- 当前版本只能读取当前 schema 和明确支持的旧 schema。
- 遇到未来 schema 必须拒绝并给出脱敏诊断，不能静默降级或覆盖。
- 遇到损坏 JSON、字段缺失、类型错误或未知终态要返回结构化错误，不得截断重写。

### 2. 原子写入协议

`WeixinStateStore` 不得复用 `FileSessionStore` 的普通 JSON 直接写入。写入协议必须至少满足：

1. 在目标文件同目录创建唯一临时文件。
2. 序列化完整状态到临时文件。
3. `write_all` 后执行 flush 和文件级 sync。
4. 使用同卷原子替换更新目标文件。
5. 可行时同步父目录，确保目录项落盘。
6. 启动时识别未完成临时文件，并按规则忽略、诊断或清理候选，但不能把半成品当作有效状态。
7. 写入失败必须保留旧有效状态，不得产生截断的目标文件。

Windows 下需要注意替换语义和文件句柄占用；测试应覆盖临时文件残留、目标文件存在、目标文件不存在、写入中断、目标只读或目录不可写等路径。

### 3. 状态模型

`v2.1.4` 需要建立后续消息 Runtime 的持久状态底座，但不得真正拉取或派发消息。建议状态域如下：

| 状态域 | 必要字段 | 边界 |
| --- | --- | --- |
| 账户状态 | account id、workspace id、endpoint、credential reference、connection state、last redacted error | 不保存 token、data key 或原始用户 ID。 |
| 游标 | `get_updates_buf`、更新时间、来源账户 | 仅为后续长轮询准备；本阶段不主动调用 `getupdates`。 |
| 入站回执 | message id hash、peer hash、接纳时间、状态 | 不保存陌生发送者正文。 |
| 出站回执 | delivery id、状态、错误摘要、更新时间 | 不实际调用 sendmessage。 |
| 会话绑定占位 | account hash、peer hash、workspace id、session id optional | 不创建新的 Runtime session。 |
| reply context 引用 | secret reference、过期时间、用途 | 只保存引用，不保存明文 context token。 |
| 待投递元数据 | delivery id、状态、重试计数、更新时间 | 不对结果不明网络发送宣称成功。 |
| 配对请求 | request id、account hash、peer hash、expires_at、state | request id 必须不透明、限时、校验账户归属。 |
| 加密待处理入站项 | item id、message id hash、encrypted payload reference、state、transition times | 加密能力不可用时后续 `serve` 必须拒绝启动。 |

### 4. 待处理入站项状态机

即使 `v2.1.4` 不做真实消息接收，也必须把状态机建模清楚，供 `v2.1.5` 直接接入。

状态只允许：

```text
accepted -> ready -> running -> terminal
```

约束：

- 接纳消息和游标推进必须属于同一持久化提交。
- 重启后只恢复非终态项。
- 同一 message id 在同一账户、同一会话内必须串行处理。
- `terminal` 应区分 succeeded、failed、cancelled、expired 或 unknown 等可审计原因。
- 不得承诺跨 Provider、工具调用、外部网络发送的绝对一次执行；只能保证本地状态机的可恢复与可诊断。

### 5. 账户粒度系统排他锁

实现 `WeixinAccountLock` 或等价抽象，作用范围必须是账户粒度，而不是单 workspace 粒度。

必须覆盖：

- 同一 iLink 账户在同一 workspace 启动第二个 `weixin serve` 时失败。
- 同一 iLink 账户跨 workspace 启动第二个 `weixin serve` 时失败。
- 不同账户可独立持锁。
- 第二进程退出时给出脱敏诊断，不能继续运行。
- 陈旧锁只能在验证持有进程已经退出后恢复，禁止仅按锁文件存在与否覆盖。
- `logout --confirm` 遇到活动服务锁必须拒绝，并提示先停止本地服务。

如果实现需要平台差异，必须用 trait 封装，并用 fake lock backend 覆盖测试。不得通过删除锁文件的方式规避竞争。

## 五、CLI 与文档接入点

### `status --json`

`yunxi weixin status --account <name> --json` 至少应包含：

- account id 或 account hash。
- connection state。
- credential state 和 credential backend。
- state store schema version。
- account lock state。
- pending inbound count。
- pending delivery count。
- pair request count。
- last redacted error。
- `secrets_included=false`。

禁止输出凭证、联系人、原始消息、二维码 payload、context token、data key、系统用户名、完整本地路径或 Provider payload。

### `doctor`

`yunxi weixin doctor --account <name> [--json]` 至少检查：

- `.yunxi/weixin/` 或微信状态目录可读写。
- state store schema 可读且非未来 schema。
- 系统凭证引用存在且可诊断，不读取或打印秘密。
- 账户锁状态。
- 私聊策略为开启，群聊策略为关闭。
- 加密待处理队列能力可用。
- 最后一次脱敏错误。

`doctor` 可以说明问题和修复建议，但不得自动删除、迁移或清理用户数据。

### `pair list|approve|deny`

配对请求生命周期必须由 `WeixinStateStore` 承载：

- `pair list` 默认只显示不透明 request ID、脱敏账户、对端哈希、状态和过期时间。
- `pair approve` 只能接受未过期、未消费、账户归属匹配的 request ID。
- `pair deny` 同样只能处理未过期 request ID，并记录终态。
- 重复 approve/deny、跨账户 request ID、过期 request ID 必须失败且不改变无关状态。
- 不得输出原始对端标识或陌生消息正文。

### `logout --confirm`

`logout --confirm` 必须变成安全生命周期操作：

- 如果存在该账户活动 `serve` 锁，必须拒绝执行。
- 服务停止后，只删除指定账户的微信凭证引用、加密待处理队列、微信状态和微信 metadata。
- 不得删除 YunXi session、persona memory、工作区文件、其他账户、其他 Provider 配置或历史报告。
- 文档必须明确：logout 不是删除聊天历史、不是删除长期记忆、不是清理整个项目。
- 失败时必须给出可诊断状态，不能留下“凭证删了但状态显示 ready”的不一致。

## 六、参考源码状态与使用边界

本次审核报告和总纲提到的关键参考源码均已在本机存在，本轮无需新增拉取：

| 参考输入 | 本机状态 | 使用方式 |
| --- | --- | --- |
| `D:\源码\reasonix` | 已存在，remote `https://github.com/esengine/DeepSeek-Reasonix.git`，当前短提交 `db4be5e6` | 参考项目分层、微信账户/context 状态边界和测试组织。 |
| `D:\源码\reasonix\internal\bot\weixin\weixin.go` | 已存在 | 参考账户、context token、轮询游标和通道状态边界；只迁移逻辑，不复制 Go gateway。 |
| `D:\源码\reasonix\internal\bot\weixin\weixin_test.go` | 已存在 | 参考状态、过期、隔离和重启场景测试组织。 |
| `D:\源码\openclaw-weixin` | 已存在，remote `https://github.com/Tencent/openclaw-weixin.git`，当前短提交 `cef0bfc` | 参考 Tencent iLink 官方协议行为和存储模型。 |
| `D:\源码\openclaw-weixin\src\auth\accounts.ts` | 已存在 | 参考账户索引、凭证引用、状态读取和删除边界。 |
| `D:\源码\openclaw-weixin\src\auth\account-store.test.ts` | 已存在 | 参考账户存储测试和秘密不落盘测试。 |
| `D:\源码\openclaw-weixin\src\auth\pairing.ts`、`pairing.test.ts` | 已存在 | 参考 request ID、过期、approve/deny 和归属校验逻辑。 |
| `D:\源码\openclaw-weixin\src\storage\state-dir.ts`、`sync-buf.ts` | 已存在 | 参考状态目录、同步缓冲和恢复测试思路，Rust 侧必须重新建模。 |
| `D:\YunXi Agent\crates\yunxi-agent-storage\src\lib.rs` | 已存在 | 参考现有 `SessionStore` facade 和兼容规则；微信状态必须独立扩展。 |

所有 TypeScript 和 Go 参考源码只能作为逻辑参考，必须用 Rust 复刻。不得引入 OpenClaw 宿主、Node runtime、Reasonix Bot Gateway、明文凭证落盘、媒体上传、群聊、插件系统或第二套 Runtime。

总纲中 Letta 的价值只限“状态可审阅、可解释、生命周期清晰”的产品理念，不得引入新的 Letta 运行时依赖，也不得替换 YunXi 既有人格、记忆或 session 体系。

## 七、推荐测试与验收

开发完成后，至少执行以下验证。未通过前不能宣称 `v2.1.4` 完成。

| 类别 | 验收要求 |
| --- | --- |
| 格式与编译 | `cargo fmt --all -- --check`、`cargo check --workspace`。 |
| 全量测试 | `cargo test --workspace`。 |
| 存储定向测试 | `cargo test -p yunxi-agent-storage weixin` 或等价定向测试，覆盖原子写入、损坏记录、临时文件、未来 schema、中断写入和迁移。 |
| 微信定向测试 | `cargo test -p yunxi-agent-weixin`，覆盖 secret reference、account metadata、加密待处理项和脱敏错误。 |
| CLI 定向测试 | 覆盖 `status --json`、`doctor --json`、`pair list|approve|deny`、`logout --confirm`、活动锁拒绝和无秘密输出。 |
| 锁测试 | 覆盖同账户同 workspace、同账户跨 workspace、不同账户、第二进程失败、陈旧锁恢复。 |
| 迁移测试 | 覆盖旧 `v2.1.0` 数据目录、既有 `.yunxi/weixin/*.json` metadata、缺失 state store、schema 降级拒绝。 |
| 安全搜索 | 对 stdout、stderr、JSON、日志、报告证据和状态文件搜索 token、data key、二维码 payload、context token、原始 user id、原始 peer id。 |
| 回归 | 既有 `yunxi run`、sessions、persona、memory、companion、provider、JSON、JSONL、TUI/ConPTY 回归不得失败。 |

真实微信环境不是本阶段强制消息联调目标。若开发者执行真实登录或真实状态诊断，只能记录脱敏状态、哈希、退出码和能力可用性，不得记录任何凭证明文或联系人数据。

## 八、发布、清理与日志要求

`v2.1.4` 开发完成后必须执行：

1. 统一验证通过后，才能更新状态文档、设计文档、报告索引和开发日志。
2. 每个阶段结束后清理编译中间产物，避免 `target`、临时测试目录和 evidence 生成物占用过多硬盘；涉及递归删除、清空目录或 `git clean` 前必须得到用户对精确绝对路径的确认。
3. 提交新的发布 commit。
4. 创建新的 annotated `v2.1.4` tag。
5. 推送 commit 和 tag。
6. 不移动、覆盖或删除 `v2.1.3-hotfix.1`、`v2.1.3` 或任何历史 tag。
7. 在 `D:\YunXi Agent\docs\development-log.md` 追加详细记录，并同步到 `C:\Users\24763\Desktop\YunXi Agent开发日志.md`。
8. 报告、日志、设计文档、索引和状态文档必须与实际代码状态一致；不能把未实现的长轮询、Runtime 绑定、远程审批、流式回信或群聊写成已完成能力。

## 九、开发者执行顺序建议

建议按批次推进，不要在单个点反复纠结：

1. 先建立 `WeixinStateStore` 的类型、schema、路径规划和原子写入基础。
2. 再接入 account metadata、secret reference、pair request、pending inbound、delivery record 和 lock record。
3. 然后把 `status`、`doctor`、`pair`、`logout` 接入状态 store 和锁。
4. 最后集中补齐存储、CLI、安全、迁移和回归测试。

本阶段完成后，审核者应能从代码和测试直接确认：微信状态已经有独立可恢复底座，诊断输出无秘密，账户生命周期不破坏 YunXi 既有 session/persona/memory，且后续 `v2.1.5` 可以在此基础上接入长轮询而不重写持久化协议。

## 十、开发完成记录（发布前）

- 完成时间：2026-07-27 19:10:01 +08:00
- 执行者署名：开发者
- 当前目标版本：`v2.1.4`
- 当前发布状态：开发、验证和用户确认范围内清理已完成；下一步创建 release commit、新 annotated `v2.1.4` tag，并使用 GitHub CLI 与桌面 API key 非强制推送。

### 1. 实现摘要

本阶段已在 `D:\YunXi Agent` 内完成 `v2.1.4` 微信状态持久化、诊断与安全账户生命周期开发：

1. 在 `crates\yunxi-agent-storage` 新增独立版本化 `WeixinStateStore`，没有向 `SessionRecord` 添加微信字段。
2. 新增同目录临时文件、flush/sync、同卷替换、父目录 best-effort sync、未完成临时文件候选识别和写失败保留旧状态的原子写入路径。
3. 建立账户状态、credential reference、游标、回执、会话绑定占位、reply context 引用、pending delivery、pair request、pending inbound 和 last redacted error 的可审阅 schema。
4. 建立 pending inbound 本地状态机：`accepted -> ready -> running -> terminal`，终态不参与重启恢复。
5. 实现账户粒度跨 workspace 锁、活动锁诊断、陈旧锁仅在进程退出后恢复、同账户竞争失败和 `logout --confirm` 活动锁拒绝。
6. 将 `yunxi weixin status --json`、`doctor`、`pair list|approve|deny`、`serve` 和 `logout --confirm` 接入状态 store 与锁；输出只含脱敏账户、schema、锁状态、pending/pair 计数、credential state/backend 和 `secrets_included=false`。
7. 保持 `v2.1.4` 边界：未实现、未启动、未宣称长轮询、消息接收、消息发送、Runtime 绑定、远程审批、群聊、主动推送或完整微信聊天闭环。

### 2. 修改文件与路径

- `D:\YunXi Agent\Cargo.toml`
- `D:\YunXi Agent\Cargo.lock`
- `D:\YunXi Agent\README.md`
- `D:\YunXi Agent\crates\yunxi-agent-storage\Cargo.toml`
- `D:\YunXi Agent\crates\yunxi-agent-storage\src\lib.rs`
- `D:\YunXi Agent\crates\yunxi-agent-storage\src\weixin_state.rs`
- `D:\YunXi Agent\crates\yunxi-agent-storage\tests\weixin_state_tests.rs`
- `D:\YunXi Agent\crates\yunxi-agent-cli\src\main.rs`
- `D:\YunXi Agent\crates\yunxi-agent-cli\src\weixin.rs`
- `D:\YunXi Agent\crates\yunxi-agent-cli\tests\cli_tests.rs`
- `D:\YunXi Agent\crates\yunxi-agent-runtime\tests\general_companion_tests.rs`
- `D:\YunXi Agent\crates\yunxi-agent-tui\src\app.rs`
- `D:\YunXi Agent\crates\yunxi-agent-tui\src\render.rs`
- `D:\YunXi Agent\crates\yunxi-agent-tui\src\snapshots\full_frame_100x30.txt`
- `D:\YunXi Agent\crates\yunxi-agent-tui\src\snapshots\full_frame_120x40.txt`
- `D:\YunXi Agent\crates\yunxi-agent-tui\src\snapshots\full_frame_200x50.txt`
- `D:\YunXi Agent\crates\yunxi-agent-tui\src\snapshots\full_frame_58x18.txt`
- `D:\YunXi Agent\crates\yunxi-agent-tui\src\snapshots\full_frame_80x24.txt`
- `D:\YunXi Agent\crates\yunxi-agent-weixin\tests\ilink_client_tests.rs`
- `D:\YunXi Agent\docs\README.md`
- `D:\YunXi Agent\docs\weixin.md`
- `D:\YunXi Agent\docs\reports\README.md`
- `D:\YunXi Agent\docs\development-log.md`
- `D:\YunXi Agent\docs\reports\audits\2026-07-27-181754-yunxi-agent-v2-1-3-hotfix-1-weixin-login-audit-report.md`
- `D:\YunXi Agent\docs\reports\development\2026-07-27-183050-yunxi-agent-v2-1-4-weixin-state-store-diagnostics-lifecycle-development-report.md`

### 3. 验证结果

- `cargo fmt --all -- --check`：通过。
- `cargo check --workspace`：通过。
- `cargo test -p yunxi-agent-storage`：通过；原有 storage 测试 22/22，新 `weixin_state` 测试 6/6，覆盖原子写入、损坏 JSON、未来 schema、旧 schema 内存迁移、pair 生命周期、pending inbound 状态机、账户锁竞争和陈旧锁恢复。
- `cargo test -p yunxi-agent-weixin`：通过；iLink client 3/3、models 2/2、login/store 6/6、redaction 2/2。
- `cargo test -p yunxi-agent-cli`：通过；CLI 单元 23/23、兼容二进制单元 23/23、CLI 集成 50/50、JSONL 10/10，覆盖 status/doctor/pair、pair approve/deny、活动锁拒绝 logout 和无秘密输出。
- `cargo test --workspace`：通过；TUI 161/161，Provider 回归包含在 workspace 测试中。
- `cargo build --workspace --release`：通过；release `yunxi --version` 输出 `yunxi 2.1.4`。
- release 微信帮助 10 组命令 exit code 0；`status --json` 与 `doctor --json` 均含 `secrets_included=false`，敏感字符串命中数 0。
- `cargo test -p yunxi-agent-provider`：Provider 46/46 通过。
- `cargo test -p yunxi-agent-tui`：TUI 161/161 通过。
- `D:\YunXi Agent\target\release\yunxi.exe eval companion --json`：31/31，`golden_passed=true`，审批绕过 0，主动边界违规 0。
- 真实 Provider smoke：release CLI 返回 `YUNXI_V214_REAL_PROVIDER_OK`，exit code 0，敏感 key 泄漏 0；该结果只证明通用 Provider 路径未回归，不代表微信真实消息联调。
- ConPTY v210 只读 verifier：`ok=true`、`read_only=true`，正式 evidence SHA-256 为 `a291a66cf91ee788bba9944edbafbf4c8dfce43bcd2d6c7b2cc78bc6c2990fc6`。
- YunXi 自有 Markdown 检查：130 个 Markdown，54 个本地链接，失效 0。
- 默认 CLI 正常依赖树：427 行，`yunxi-agent-codex`、`codex-*`、`vendor/codex-rs` 匹配 0。
- 受保护范围 `vendor`、`extracted`、`docs\reports\evidence`、`scripts\conpty` 变更 0。
- `git diff --check` exit code 0。
- `git fsck --full` exit code 0；仓库仍存在历史 dangling 对象输出，未执行 gc/prune。

### 4. 清理与安全状态

经用户明确确认后，仅删除以下项目内构建/临时产物，并逐项核验已不存在：

- `D:\YunXi Agent\target`
- `D:\YunXi Agent\.tmp\v213-hotfix1-status-doctor`
- `D:\YunXi Agent\.tmp\v213-hotfix1-real-login-20260727-174525.status.json`
- `D:\YunXi Agent\.tmp\v213-hotfix1-real-login-20260727-175309.status.json`
- `D:\YunXi Agent\.tmp\v213-hotfix1-real-login-20260727-175729.status.json`

未触碰 `D:\YunXi Agent\.git`、`D:\YunXi Agent\.yunxi`、源码、正式 evidence、用户目录、Windows Credential Manager 凭证或任何 Git tag。未使用 `git clean`、gc、prune、force、系统安装/卸载、PATH/注册表修改或用户目录清理。

### 5. 提交、推送和 tag 状态

发布前本地 `v2.1.4` tag 不存在，历史 tag 未移动、删除或覆盖。下一步将创建 release commit、新 annotated `v2.1.4` tag，并使用 GitHub CLI 与 API key 非强制推送。推送完成后将追加发布后 docs-only 收口记录；该收口只推进 `master`，不会移动 `v2.1.4` 或任何历史 tag。

署名：开发者
