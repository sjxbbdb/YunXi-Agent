# YunXi Agent v2.1.1 至 v2.2.0 个人微信接入与目录治理总纲图

制定时间：2026-07-22 10:34:40 +08:00
重构时间：2026-07-22 11:42:21 +08:00

## 一、总目标与发布约束

本路线图以 CLI 为产品核心，为 YunXi Agent 接入**个人微信私聊入口**。目录治理是
微信接入的前置工程：先让源码、参考输入、文档、验证证据和可再生产物各有稳定边界，
再增加新的通道 crate，避免在后续十个版本中持续放大根目录混乱和路径耦合。

到 v2.2.0 时，用户可在本机通过 YunXi CLI 扫码绑定微信 iLink Bot 身份；已配对的个人
微信用户可与同一个 YunXi 陪伴 Agent 持续对话，重启后保持会话、人格和记忆连续，并
始终沿用 YunXi 现有的工具审批机制。目录治理不得以破坏 Cargo workspace、Codex 参考
映射、历史审计证据或 Git tag 为代价。

本次发布线共十个独立审核版本：

| 版本 | 必须创建的 Tag | 当期核心成果 |
| --- | --- | --- |
| v2.1.1 | `v2.1.1` | 项目目录治理、Git 忽略边界与文档索引基线 |
| v2.1.2 | `v2.1.2` | 微信模块/CLI 骨架、iLink 协议客户端与确定性 Mock 测试 |
| v2.1.3 | `v2.1.3` | 二维码登录与系统安全凭证存储 |
| v2.1.4 | `v2.1.4` | 账户/通道状态持久化与诊断命令 |
| v2.1.5 | `v2.1.5` | 长轮询收消息、配对、去重与幂等 |
| v2.1.6 | `v2.1.6` | 微信会话绑定既有 YunXi Runtime |
| v2.1.7 | `v2.1.7` | 文字审批、追问、取消与会话队列 |
| v2.1.8 | `v2.1.8` | 可靠回信与安全的流式输出合并 |
| v2.1.9 | `v2.1.9` | 复用既有人格、记忆与陪伴能力 |
| v2.2.0 | `v2.2.0` | 完整 CLI 接入、真实联调与发布收口 |

每个版本均须提交为新的 annotated Git tag。已有 tag 不得移动、覆盖或删除。未通过
本版本验收前，不得宣称版本完成或进入下一版本。

## 二、范围、边界与非目标

### v2.2.0 范围内能力

- 只接入腾讯个人微信 iLink Bot 官方路径，由本机 YunXi CLI 发起扫码。
- 以显式启动的本地前台服务承载微信通道，使用长轮询；不开放公网回调地址，不新增
  入站端口。
- 首期只支持私聊；数据模型保留账户 ID，以避免后续多账户扩展时发生破坏性迁移。
- 复用 YunXi 既有的 session、persona、memory、relationship graph、companion、
  provider、sandbox、stream、取消、审批和用户追问能力。
- 实现本地凭证保护、发送者配对/白名单、会话绑定、消息投递诊断和真实 Provider
  验证。

### v2.2.0 明确不做的内容

- 不做企业微信、公众号、微信客服、企业群机器人、群聊机器人或通用多渠道网关。
- 不做新的 TUI 重构、视觉改版或与微信无关的交互修改；新增界面只限 CLI 帮助、二维
  码、状态和诊断文本。
- 不建立第二套 Agent、第二套记忆、第二套人格提示词或第二套审批系统。
- 不使用 iPad/PC Hook、逆向协议、联系人抓取、群发、自动加好友或自动拉群。
- 不宣称已完成未经过真实验证的后台主动推送。既有 companion 建议可在用户下一次
  主动私聊时被召回，但后台主动推送必须等待腾讯接口语义和真实测试均通过后才可开启。

iLink 扫码登录获得的是 Bot 身份，不等同于对扫码个人号的无限制控制。普通微信群
消息下发并不稳定，因此本发布线在代码和配置上均关闭群聊；v2.2.0 的硬指标是私聊
可靠性。

## 三、现有 YunXi 模块与合并原则

微信只作为新的个人输入/输出入口，不能替换现有 CLI Runtime。以下模块必须按现有
边界复用：

| 现有模块 | 当前职责 | 微信接入方式 |
| --- | --- | --- |
| `crates/yunxi-agent-cli/src/main.rs` | CLI 路由、Provider 选择、JSON/JSONL、会话/人格/记忆/陪伴命令 | 增加窄化的 `weixin` 命令族；仅抽取 `run` 与 `weixin serve` 共用的内部执行辅助函数。 |
| `crates/yunxi-agent-core/src/input.rs` | `AgentInput { prompt }` | v2.2.0 内保持兼容，不为微信破坏现有输入结构。微信适配层生成同样的文本输入。 |
| `crates/yunxi-agent-core/src/stream.rs` | `AgentRunControl`、流式事件、审批、用户追问、取消 | 作为唯一远程交互桥接，不另造微信审批协议。 |
| `crates/yunxi-agent-runtime/src/lib.rs` | `YunXiRuntimeBackend` 流式运行时 | 通过 `Agent::run_with_backend_stream` 原样复用。 |
| `crates/yunxi-agent-storage/src/lib.rs` | Session 与持久化存储边界 | 增加兼容的微信账户、绑定和投递状态记录；不污染 `SessionRecord` 的旧数据格式。 |
| `crates/yunxi-agent-companion`、`crates/yunxi-agent-persona` | 陪伴控制、人格和记忆上下文 | 微信使用同一套配置、同一条记忆链路，不重复抽取或写入记忆。 |
| `crates/yunxi-agent-tui` | 终端 TUI | 本路线图不修改；现有 TUI 是回归验证目标。 |

仅新增一个生产 crate：`crates/yunxi-agent-weixin`。它负责 iLink HTTP 协议、二维码
登录状态机、微信消息归一化和微信特有投递策略。当前只有一个成熟通道，不提前建立
泛化 `yunxi-agent-channel` 框架；未来出现第二个可验证通道后，再由已验证的微信实现
抽取通用部分。

根目录 `Cargo.toml` 只增加 `yunxi-agent-weixin` workspace member 及最少 Rust 依赖。
`vendor/` 与 `extracted/` 均是参考材料，不得直接修改。

在创建微信 crate 前，仓库根目录遵循以下稳定边界：`crates/` 为 YunXi 自有 Rust 源码，
`evals/` 为可重复验证资产，`scripts/` 为工具与 ConPTY 验证脚本，`docs/` 为长期文档和
可复核证据，`vendor/` 为外部固定参考快照，`extracted/` 为迁移输入。`target/`、
`.codegraph/`、`.tmp/`、`.yunxi/`、`.worktrees/` 和 ConPTY `node_modules/` 仅为本地
产物或状态，必须被 Git 忽略且不作为源码分类的一部分。

目录治理期间不得将 `vendor/codex-rs` 或 `extracted/codex-core-agent-sources` 移入新的
目录名：它们仍被 `Cargo.toml`、迁移脚本、README 和提取索引引用。历史
`docs/superpowers/` 与 `docs/reports/` 也先保留路径兼容；先创建索引和新文档落位规则，
再由单独版本化提交逐批迁移历史文档。

## 四、目标 CLI 与运行链路

最终命令族如下：

```text
yunxi weixin login [--account default]
yunxi weixin status [--account default] [--json]
yunxi weixin doctor [--account default] [--json]
yunxi weixin serve [--account default] [--workspace <path>] [现有 provider/sandbox/approval 参数]
yunxi weixin pair list|approve|deny ...
yunxi weixin session reset [--account default] --peer <request-or-alias> --confirm
yunxi weixin logout [--account default] --confirm
```

`weixin serve` 必须复用 `yunxi run` 的 Provider 选择、模型、工作目录、sandbox 和审批
配置；不得启动另一套模型 Runtime。它是可 Ctrl-C 取消的前台 CLI 服务。现有 one-shot、
JSON、JSONL、交互 CLI 和 TUI 行为必须保持不变。

`weixin serve` 的工作目录必须由本地操作者在启动时决定：复用既有工作目录参数；若现有
CLI 没有等价参数，则仅为 `weixin serve` 提供 `--workspace <path>`，默认当前目录，并在
启动前规范化为绝对路径。YunXi session 与非机密微信状态位于该工作区的
`.yunxi/sessions/` 和 `.yunxi/weixin/`；iLink 凭证和状态加密密钥位于系统凭证存储。远程
微信消息绝不能改变工作目录、模型、Provider、sandbox 或审批模式。

最终数据流：

```text
个人微信 iLink Bot
  -> Weixin iLink 长轮询客户端
  -> 已认证发送者的配对/白名单闸门
  -> 消息回执、游标、会话绑定的持久化
  -> 既有 YunXi Agent + YunXiRuntimeBackend 流
  -> 既有审批 / 追问 / 取消控制
  -> 投递队列、文本合并器、iLink sendmessage
  -> 个人微信私聊
```

通道会话键固定由 `channel=weixin + account_id + chat_type=dm + peer_id` 组成。它只用于
查找既有 YunXi `SessionId`，不替代当前 Session 格式，因此既有历史、persona-memory、
关系图和 companion 状态能够继续工作。

可靠性语义必须诚实区分三层：iLink 入站和出站网络传输只能按官方协议做到**至少一次**；
YunXi 在单一 `WeixinConversationKey` 内保证同一消息 ID 不会并发执行两次；网络结果不明
或崩溃位于外部副作用边界时，记录为待诊断状态，绝不宣称全链路“恰好一次”。

## 五、安全、隐私与兼容性硬约束

以下任一项违反均视为版本阻塞：

1. iLink token、二维码 payload、`context_token`、原始用户 ID 和原始消息内容不得出现
   在普通日志、JSON/JSONL、错误文本或诊断包中；日志仅使用脱敏账户名和单向哈希。
2. token 只能写入系统凭证存储。账户元数据可本地持久化，但不得提供明文 token 降级
   方案；安全存储不可用时，登录必须明确失败。
3. 私聊默认采用配对/白名单模式。发送者被准入前，其文本、附件、工具请求、记忆写入
   和 Session 创建均不得进入 Agent Runtime。群聊在代码和配置层均关闭。
4. 微信文字命令只能回答既有 `AgentRunControl` 产生的审批/追问，绝不能自动批准工具；
   仅已准入发送者可取消、回答或决定审批。
5. 微信不得展示隐藏推理、原始工具输出、本地路径、环境变量、凭证或 Provider 私密
   payload；仅展示面向用户的简短状态与最终回答。
6. 微信适配器没有 shell 执行路径，不得放宽 YunXi sandbox 或 approval mode。
7. 存储更新必须原子且可重启恢复。崩溃后允许按幂等键重试，但不得悄然丢消息，也不得
   为同一会话创建第二个 YunXi Session。
8. 现有 CLI 命令、TUI、persona/memory 数据、`SessionRecord` 和 Provider 行为必须保持
   向后兼容。
9. 当前 `FileSessionStore` 的 session JSON 不能承担微信通道状态：微信必须使用独立的
   `WeixinStateStore`，提供临时文件写入、刷新、同卷原子替换、schema migration、损坏
   检测和账户级跨进程锁；不得假设普通 `std::fs::write` 具备上述性质。
10. 已准入但尚未执行完成的入站消息只可作为短期加密队列项存储；数据加密密钥由系统
    凭证存储保护，终态后按可审计保留策略删除正文。陌生发送者只保存脱敏配对请求，不
    保存消息正文。加密能力不可用时 `weixin serve` 必须拒绝启动。
11. 项目内 `docs/superpowers/plans/` 的路线图是唯一可编辑正本；桌面文件只作为带
    SHA-256 校验的只读分发副本。版本审核以项目内正本为准，副本不同步时必须在报告中
    明示，不得让两份文件分别演进。

## 六、参考项目、参考路径与 Rust 化原则

所有非 Rust 源码只参考架构、协议与测试思路，必须以 idiomatic Rust 重写：使用
`serde` 数据模型、`tokio` 任务、`reqwest` 请求、显式错误类型和 Rust 测试；不得逐行
搬运 Go、Python 或 TypeScript 实现。

| 参考项目/路径 | 本路线图中的用途 | 不得引入的范围 |
| --- | --- | --- |
| `D:\源码\reasonix` 的根目录、`.gitignore`、`REASONIX.md`、`docs/`、`scripts/` | 仓库治理基准：稳定源码/前端/工具/文档分层，构建与本地状态统一忽略，项目记忆保持简短而持久。 | 复制 Go 的 `cmd/internal` 外形；YunXi 保持 Cargo workspace 与 crate 边界。 |
| [Tencent/openclaw-weixin](https://github.com/Tencent/openclaw-weixin) | iLink 官方协议行为基准：扫码、账户 ID、长轮询、`get_updates_buf`、`context_token`、消息/媒体接口、重连；重点研究 `src/` 下 auth、api、messaging、monitor、storage 逻辑。 | Node/OpenClaw 宿主耦合。 |
| `D:\源码\reasonix\internal\bot\weixin\weixin_login.go`、`weixin.go` | 二维码状态机、限时 HTTP、轮询游标、自消息过滤、context token 重试。 | 明文凭证落盘和 Go 专属写法。 |
| `D:\源码\reasonix\internal\bot\types.go`、`gateway.go`、`internal\botruntime\runtime.go` | 消息归一化、会话串行、文字决策、Session 映射、通道到 Runtime 路由。 | 独立 Bot Gateway/Controller 产品层；YunXi 已有 Runtime。 |
| [CowAgent](https://github.com/zhayujie/CowAgent) 的 `channel/channel_factory.py`、`channel/weixin/` | 通道所有权、二维码恢复、多媒体演进和长期运行的个人陪伴产品经验。 | Python Runtime、泛化 channel factory、第二套记忆。 |
| [OpenAkita](https://github.com/openakita/openakita) | 个人 iLink 绑定、IM 中断语义、工具/插件分级权限。 | 大范围多平台产品和 UI/插件栈。 |
| [Leon](https://github.com/leon-ai/leon) 的 `core/context/LEON.md`、`core/context/ARCHITECTURE.md` | 本地优先、分层记忆、稳定自我模型、有限主动脉冲。 | Node/Python Runtime 和无限制自治。 |
| [Letta Code](https://github.com/letta-ai/letta-code) | 长期身份、可审阅记忆、反思质量、本地优先状态。 | 用新内存引擎替换 YunXi 既有人格/记忆。 |
| [Project N.E.K.O.](https://github.com/Project-N-E-K-O/N.E.K.O) | 人格连续性与“不同入口仍是同一个陪伴者”的产品原则。 | 虚拟形象、游戏、移动端范围。 |

## 七、逐版本开发路线

### v2.1.1：项目目录治理、Git 忽略边界与文档索引基线

**目标**：不移动、不删除、不重命名任何既有源码或证据，在不改变运行行为的前提下建立
可持续的项目分类与可复现边界。此版本是微信接入的工程地基，不实现微信网络、登录或新
crate。

**实现内容**：

- 以 `D:\YunXi Agent` 为唯一工作根目录，生成根目录资产清单：稳定源码、参考输入、
  文档、验证脚本、本地状态和可再生产物必须分别标注用途、是否应跟踪、是否可清理与
  当前引用方。
- 保持 `crates/`、`evals/`、`scripts/`、`docs/`、`vendor/`、`extracted/` 的既有路径；
  特别禁止移动 `vendor/codex-rs`、`extracted/codex-core-agent-sources`、现有
  `scripts/conpty/v205` 至 `v210`、`v207-hotfix` 与 `docs/reports/evidence/`。
- 在确认 `git ls-files` 不匹配任何已跟踪文件后，补齐 `.gitignore` 的目录级规则：
  `/.tmp/`、`/scripts/conpty/**/node_modules/`、`/scripts/conpty/**/.work/`。不删除
  已存在的 Node 依赖、`.work/`、`target/`、`.codegraph/` 或临时采集结果。
- 建立 `docs/README.md` 或 `docs/index.md`，提供架构、路线图、操作手册、提取索引、
  审核报告、开发报告和 ConPTY 证据的稳定导航；根 `README.md` 只增加到该索引的入口，
  不把所有新说明继续堆入根 README。
- 明确 `docs/superpowers/plans/` 为可编辑路线图正本；桌面同名文件只在同步时由正本生成，
  使用 SHA-256 一致性检查。若当前权限无法同步桌面副本，记录“副本滞后”，但不在桌面和
  项目内分别修改，以免产生双真相。
- 在 `docs/reports/` 说明报告命名与新增落位规则：后续审核报告进入
  `docs/reports/audits/`，开发报告进入 `docs/reports/development/`，证据保留在
  `docs/reports/evidence/`。本版本不迁移历史报告，以避免断开数百处 Markdown 引用。
- 为 `scripts/conpty/` 建立总览，列出每个版本脚本目录、验证场景、依赖、输出目录和
  evidence 链接；不抽取共享代码、不改已有 `npm run verify` 路径。
- 记录可再生产物的清理候选与空间占用，但所有清理操作必须等待用户对精确绝对路径的
  再次授权；不得使用 `git clean`、递归删除或宽泛通配符。

**参考**：`D:\源码\reasonix` 的根目录、`.gitignore`、`docs/`、`scripts/` 与
`REASONIX.md`；YunXi 现有 `README.md`、`docs/extraction-index/`、
`docs/reports/evidence/`、`scripts/README.md`。

**验收**：

1. 所有本阶段修改只限 `.gitignore`、README/索引/报告等文档；`Cargo.toml`、Rust 源码、
   `vendor/`、`extracted/`、历史证据和 ConPTY 版本脚本均不移动、不删除。
2. `git check-ignore -v` 能证明所有现存 ConPTY `node_modules/` 和 `.work/` 已被统一
   规则忽略，且 `git ls-files` 证明无已跟踪文件被新规则隐藏。
3. 从根 README 可在两步内进入当前路线图、报告索引、ConPTY 说明和提取索引；现有
   Markdown 链接抽查无失效。
4. `git diff --check` 通过；`git status --short` 中不再出现 ConPTY `node_modules/`，但
   不得用删除实现该结果。
5. 项目内路线图与桌面分发副本已同步时，SHA-256 必须一致；无法同步时，项目内正本的
   路径、哈希和副本滞后原因必须记录在开发/审核报告中。
6. 创建包含目录治理变更的发布提交与 annotated `v2.1.1` tag；发布日志必须记录未执行
   任何清理及其原因。

### v2.1.2：微信模块/CLI 骨架、iLink 协议客户端与确定性 Mock 测试

**目标**：在 v2.1.1 建立的稳定目录和文档边界中，同时完成窄化的 Rust 微信模块、CLI
命令骨架和可离线验证的 iLink 协议层；本版本仍不要求真实扫码、凭证持久化或常驻服务。

**实现内容**：

- 新增 `crates/yunxi-agent-weixin`，定义最小领域类型：`WeixinAccountId`、
  `WeixinPeerId`、`WeixinMessageId`、`WeixinConversationKey`、
  `WeixinConnectionState`、非机密 `WeixinAccountMetadata`。
- 在 `crates/yunxi-agent-cli/src/` 新增内部 `weixin` 命令模块，并挂载到既有
  `CliCommand`；创建 `login`、`status`、`doctor`、`serve`、`pair`、`logout` 的帮助和
  参数校验骨架，不重写现有命令分发。
- 从当前 `run` 路径抽取私有共享执行辅助函数，使未来 `weixin serve` 必然复用同一个
  Provider 选择、Runtime 构造、错误脱敏和 JSON/JSONL 约定；此版本不得实际调起
  YunXi Runtime。
- 在 `yunxi-agent-weixin` 中实现私有 `IlinkHttpClient`，复用 workspace 的
  `reqwest + rustls`，负责超时、响应大小上限、请求头、请求 ID 和腾讯错误归一化。
- 为二维码获取/状态、`getupdates`、`sendmessage`、`sendtyping`、上传 URL 建立 serde
  请求/响应模型；为“数字或字符串 message ID”实现显式反序列化，缺失关键字段必须失败。
- 生产地址固定为官方 iLink endpoint；只有测试构造器可注入 mock endpoint，不能暴露为
  无约束用户 CLI 参数。使用维护良好的 Rust HTTP mock 库，覆盖请求头、授权脱敏、游标、
  API 错误、错误 JSON、超时和数字 ID；不得手写 HTTP 解析器。
- 将官方端点、必要 header 和响应字段固定为版本化协议 fixture；协议字段出现未知破坏性
  变化时客户端必须安全失败并在 `doctor` 中输出脱敏诊断，而不是猜测字段含义继续运行。
- 在 `docs/` 索引中增加 `weixin` 文档入口，明确本版本仅为协议与命令骨架，尚未可登录、
  不能收发消息，且不支持群聊。

**Rust 结构**：

```text
yunxi-agent-cli::weixin
  -> yunxi-agent-weixin::{domain, IlinkHttpClient, WeixinApiError}
  -> IlinkQrApi / IlinkPollApi / IlinkSendApi
```

**参考**：Tencent 官方插件协议行为、Reasonix `internal/bot/weixin/weixin.go` 的字段容错
和限时 HTTP、Reasonix adapter/gateway 分层、Hermes 对 iLink 私聊限制的说明、现有 YunXi
CLI 与 stream Runtime。

**验收**：全部 fixture 不需要腾讯凭证和公网；测试证明 token 不会进入 `Debug`、
`Display`、错误输出或 JSON snapshot；新 crate、既有 `run`、`sessions`、`persona`、
`memory`、`companion`、TUI、JSON、JSONL 回归均通过；`yunxi weixin --help` 可用，其余
操作命令必须明确说明尚未完成；协议 fixture 覆盖缺失/未知关键字段的安全失败；
`cargo fmt --all -- --check`、`cargo check --workspace`、`cargo test --workspace` 和
`git diff --check` 通过。

### v2.1.3：二维码登录与系统安全凭证存储

**目标**：在不落盘明文秘密的条件下，将本机 YunXi 与一个 iLink 账户绑定。

**实现内容**：

- 实现 `WeixinLoginStateMachine`：获取二维码、在终端渲染 ANSI/二维码、输出安全 URL
  兜底、轮询扫描/确认/重定向/过期状态，产出类型化登录结果。
- 实现窄化 `WeixinSecretStore`，Windows 首期接入系统凭证存储。秘密键由 YunXi 安装
  标识与微信账户 ID 组成，并为后续通道状态生成独立数据加密密钥；两者不得出现在
  账户 JSON 中。
- 在已规范化 workspace 的 `.yunxi/weixin/` 保存非机密账户信息：账户 ID、连接状态、
  官方 base URL、创建/更新时间、凭证引用和 workspace 标识；不得硬编码 C 盘路径，
  不得将 token、加密密钥或二维码 payload 写入文件。
- 使 `yunxi weixin login --account <name>` 真正可用；`status` 仅显示脱敏状态。二维码
  过期后必须由用户再次显式登录，禁止隐藏无限重试。
- 用 fake `WeixinSecretStore` 覆盖测试，证明重启可经引用重新取回秘密，而账户元数据中
  没有秘密。

**参考**：Tencent 扫码流程、Reasonix `weixin_login.go`、CowAgent 的二维码过期和重连
策略。

**验收**：Mock 覆盖等待、已扫码、重定向、确认、过期、超时、取消、凭证不可用和不支持
平台；对日志、诊断和账户元数据运行搜索式脱敏测试；无微信命令时原 CLI 启动完全不变。

### v2.1.4：状态持久化、诊断与安全账户生命周期

**目标**：在接受任何用户消息前，使账户状态可检查、可重启恢复。

**实现内容**：

- 扩展 `crates/yunxi-agent-storage`，建立独立的、版本化 `WeixinStateStore`。不得向
  `SessionRecord` 直接塞微信字段，以保证旧 session JSON 与既有命令兼容；也不得复用
  当前 `FileSessionStore` 的普通 JSON 直接写入实现。
- `WeixinStateStore` 使用“临时同目录文件 -> flush -> 同卷原子替换”的写入协议，并在
  启动时识别未完成临时文件、损坏记录和不支持的 schema。持久化账户元数据、
  `get_updates_buf` 游标、入站/出站回执、会话绑定、secret-backed reply context、待投递
  元数据、过期配对请求和加密待处理入站项；每条记录必须有 schema version、更新时间和
  状态转换时间。
- 待处理入站项采用 `accepted -> ready -> running -> terminal` 状态机。接纳消息与游标推进
  必须在同一持久化提交中完成；重启后只恢复非终态项，并对同一消息 ID 施加单会话串行锁。
  不得承诺外部 Provider/工具副作用跨崩溃的绝对一次执行。
- 使用系统范围、账户粒度的真实排他锁防止同一 iLink 账户在不同 workspace 或不同进程中
  同时运行两个 `weixin serve`。第二进程必须退出并诊断；陈旧锁只能在验证持有进程已退出
  后恢复，禁止仅按锁文件存在与否强行覆盖。
- 配对请求只保存不透明 request ID、账户/对端哈希、过期时间和状态；`pair list` 默认不
  输出原始对端标识，`pair approve|deny` 只能接受未过期 request ID。
- 实现 `yunxi weixin doctor` 和 `status --json`：检查存储访问、凭证引用、账户锁、私聊
  策略和最后一次脱敏错误。不得显示凭证或联系人数据。
- `logout --confirm` 在该账户存在活动 `serve` 锁时必须拒绝执行并要求先本地停止服务；服务
  停止后只删除指定账户的微信凭证引用、加密队列与微信状态，不得触及 YunXi session、
  persona memory、工作区或其他账户。文档必须明确 logout 不是“删除聊天历史/长期记忆”。

**参考**：Reasonix 的账户/context 持久化、Letta 的可审阅状态理念、YunXi
`SessionStore` 兼容规则。

**验收**：旧 v2.1.0 数据目录迁移测试通过；原子写入/中断测试不产生截断的有效账户记录；
锁竞争、陈旧锁恢复、加密待处理项恢复、schema 降级拒绝和配对 request ID 过期均有测试；
`status`、`doctor`、`logout --confirm` 的机器输出无秘密，且 logout 不影响无关状态。

### v2.1.5：私聊长轮询、配对和幂等

**目标**：安全地接收个人微信私聊；对同一消息 ID 保证不并发重复 dispatch，并以可恢复的
状态机处理崩溃与重试，不对外部网络或模型/工具副作用虚假承诺“恰好一次”。

**实现内容**：

- 实现前台 `yunxi weixin serve`：按 iLink `getupdates` 长轮询契约运行，尊重服务端
  timeout hint，使用带 jitter 的有界指数退避，并在 Ctrl-C 后及时停止。
- 只有当消息批次与加密 `accepted/ready` 队列项已被同一持久化提交接纳后才保存
  `get_updates_buf`；在 Agent dispatch 前记录 receipt 与状态转换，以便重启恢复。终态
  receipt 采用有上限的 TTL/容量清理策略；若崩溃发生在模型或工具副作用边界，只能标记
  不确定并交由诊断，不能静默发起第二个新 turn。
- 将文本归一化为 `WeixinInboundEnvelope`：账户、对端、私聊键、消息 ID、时间戳、
  context token。日志只打印哈希。首期附件只回复“暂不支持”，不得伪造文本 prompt。
- 默认 `pairing`：首次陌生私聊仅生成短时配对请求，在本地 CLI 中通过
  `yunxi weixin pair approve|deny` 控制。未知发送者不得接触 Session、记忆或工具。群
  消息忽略，仅保留脱敏诊断。
- 增加自消息过滤、轮询健康状态、队列饱和处理；轮询任务必须在其他会话执行时持续接收
  新消息。`/stop`、审批和追问的专用路由在 v2.1.7 才启用，在此之前控制形态文本按普通
  私聊文本处理，不得偷偷改变运行状态。

**参考**：Tencent `getupdates` 游标、Reasonix 轮询和去重概念、OpenAkita 的配对/IM
中断产品模式。

**验收**：本地 mock 服务覆盖批次中断重启、重复 ID、游标推进、加密队列恢复、模型边界
不确定状态、token 过期、空轮询、退避、Ctrl-C、群聊拒绝和未进入 Agent 的配对流程；
陌生消息不得创建 YunXi session 或任何 persona/memory 写入；既有 CLI 取消和 TUI 测试
通过。

### v2.1.6：将微信会话接入既有 YunXi Runtime

**目标**：让准入私聊使用与本地 CLI 相同的持久 companion session，而非创建另一套聊天。

**实现内容**：

- 在 `yunxi-agent-storage` 增加 `WeixinConversationBinding`：将
  `account + peer + dm` 映射到既有 `SessionId`，记录最后活动和稳定来源标签，继续使用
  当前 session history/parent 规则。
- 在 `yunxi-agent-weixin` 实现 `WeixinTurnSupervisor`。它取得正常 CLI 选出的
  `AgentConfig` 与 backend，构造 `AgentInput::text`，通过 `AgentRunControl` 调用
  `Agent::run_with_backend_stream`。
- 只抽取必要 CLI 内部构造代码，确保 `run` 与 `weixin serve` 对 Provider、模型、
  sandbox、cwd、context window 和 companion 开关使用同一路径。
- 按 `WeixinConversationKey` 串行执行、使用有界队列。不同已准入私聊可独立运行；同一
  陪伴会话不得交叉 turn 或交叉记忆写入。
- 首先使用测试 sink 投递最小最终文本；不在本版本逐 token 发送，流式呈现留给 v2.1.8。

**参考**：YunXi `Agent::run_with_backend_stream`、`SessionStore`、Reasonix 会话绑定、
CowAgent 通道到核心的交接。

**验收**：fake backend 集成测试证明同一对端复用一个 `SessionId`，不同账户/对端严格
隔离；Runtime 获得的 Provider、sandbox、approval 与对应本地 CLI 相同；标准 `yunxi run`
和交互/TUI 路径无变化。

### v2.1.7：远程文字控制、审批、追问与取消

**目标**：让个人陪伴 Agent 在多步骤任务中安全可控，不降低既有审批规则。

**实现内容**：

- 将 `AgentRunControl` 桥接到私聊文本，只支持 `/status`、`/stop`、
  `/approve <id>`、`/deny <id> [reason]`、`/answer <id> <text>` 等明确命令。
- Runtime 审批或追问产生唯一、不透明、会过期的远程请求 ID。仅匹配账户/会话中的已准入
  用户可回应，并完成既有 one-shot response；请求记录必须绑定 turn ID、请求类型、创建/
  过期时间和单次消费状态，不能只按短 ID 匹配。
- `/stop` 仅取消该私聊的既有 cancellation token，不能停止整个微信服务或其他会话。
- 审批展示 action、reason 和安全的 cwd 标签，不展示隐藏推理、原始 Provider payload、
  环境变量、token 或未脱敏工具输出。
- 必须继续使用 YunXi 当前 approval/sandbox 配置。微信不得提升权限、保存广泛命令
  前缀，或把自然语言“好的”理解为批准。
- 为 approval 与 user-input 设置本地可配置但有上限的等待期限；到期后由 host 向既有
  one-shot 发送明确拒绝或 `None`，并记录脱敏原因，避免某个 turn 因手机无回应而永久卡死。
  覆盖过期、重复回应、陈旧回应、断连、队列满和服务关闭。

**参考**：YunXi `AgentRunControl`、Reasonix 的文本决策/会话串行、OpenAkita 的明确
权限层级。

**验收**：端到端 fake Runtime 覆盖批准、拒绝、回答、取消、超时、重复、跨对端拒绝和
断连等待；远程工具请求在收到匹配的 `/approve <id>` 前不得执行；既有本地 TUI 审批行为
无回归。

### v2.1.8：可靠回信与安全流式合并

**目标**：让个人微信用户及时获得真实回信，不泄露内部流事件，也不虚报投递成功。

**实现内容**：

- 构建微信展示层消费既有 `AgentEvent`：发送简短 processing/typing 状态，合并可公开
  文本为逻辑段落，绝不转发 reasoning 或 tool event。
- 按会话安全保存 `context_token`。发生已知 stale-context 发送失败时清除 token，并以
  新 client message ID 仅重试一次；其他失败走有界、可观察重试策略。
- 加入 outbound delivery spool：稳定的本地 delivery ID、消息 hash、turn ID、次数、API
  返回 message ID、最后一次确定结果与不确定状态。腾讯返回成功回执后才标记为 delivered。
  只有官方接口提供且已验证的幂等键，或明确确定未送达时，才可自动重试；超时/断连等
  结果不明的发送不得盲目重发，必须显示为待诊断，不能虚报送达或制造重复私聊。
- 实现 Unicode 安全长度限制、段落/代码块边界分段、顺序投递与节奏控制；任一段失败均不
  得把后续段误报为成功。
- 在接口支持时发送/停止 `sendtyping`。typing 失败不应使最终回答失败，但必须脱敏记录。

**参考**：Tencent send/context 接口、Reasonix context-token 重试、CowAgent 恢复经验、
Hermes 多段消息投递问题作为负向测试用例。

**验收**：mock 覆盖 stale context、超时/断连结果不明、4xx/5xx、确定失败重试、重复发送、
重启待投递、长 Unicode、代码块、局部失败和状态脱敏；结果不明的段不得自动重复发送；
失败段必须在 status/diagnostic 可见且绝不向用户或 Runtime 虚报已送达；公开微信文本无
推理、工具轨迹和秘密。

### v2.1.9：陪伴、人格与记忆连续性

**目标**：微信成为同一个 YunXi 陪伴者的私密入口，而非一次性外部 Bot。

**实现内容**：

- `yunxi weixin serve --companion` 显式进入现有 companion 配置路径，复用
  `PersonaSettings`、persona-memory、relationship graph、memory extraction 和
  companion control；禁止创建重复 schema。
- 让稳定微信来源映射到同一个 YunXi session identity，使重启、本地 CLI 续聊和微信
  续聊中的人格/偏好/关系召回一致。
- 新增通道安全的 companion status/session reset 处理。`weixin session reset` 必须由本地
  CLI 以账户、对端与 `--confirm` 三者精确确认；已配对用户只能发起请求并收到本地确认
  提示，不能直接执行。reset 的作用范围、是否归档 session、是否删除记忆必须逐项沿用
  YunXi 已有语义，不得静默清除人格或长期记忆。
- 通道元数据是受信任应用上下文，不能成为用户可控 prompt；用户消息仍走普通 prompt
  路径，身份标签和通道状态不得变成模型指令。
- 主动行为保持克制：companion 观察可在下一次用户主动私聊中召回。未完成真实验证前，
  禁止后台定时微信推送。

**参考**：Leon 有界主动脉冲/自我模型、Letta Code 的长期身份与可审阅记忆、Project
N.E.K.O 的跨入口连续性、YunXi 已有 companion/persona crate。

**验收**：集成测试证明已记住的人格/偏好通过相同绑定 session 可用，且没有创建第二套
memory store；session reset/archive 保持 YunXi 当前 memory 语义且只作用于目标会话；
不带 `--companion` 的 `weixin serve` 不得静默启用 companion 或工具。

### v2.2.0：完整 CLI 接入、真实联调与发布收口

**目标**：发布可靠的个人微信陪伴入口，并提供诚实、可复现的真实验证证据。

**实现内容**：

- 收口 CLI 帮助、onboarding、`status`、`doctor`、配对控制、扫码恢复和文档；在既有
  支持范围内保持普通文本、JSON、JSONL 输出约定。
- 新增 `evals/weixin/`，包含 mock 协议场景和单独门控的 live harness。真实测试只能从
  系统 secret store 或受保护环境读取凭证，不得使用 fixture 或日志。
- 文档必须说明：仅私聊、iLink Bot 身份、群聊排除、安全凭证、远程审批、logout 范围、
  投递行为、恢复步骤，以及未验证主动推送不属于已完成能力。
- 补齐从 v2.1.0 数据迁移检查、无秘密 support bundle 格式和 rollback 说明。回滚通过选择
  旧 Git tag 完成，不重写 tag。
- 更新项目状态/索引/设计文档，使其与真实代码状态一致。

**创建 `v2.2.0` tag 前必须全部通过**：

1. `cargo fmt --all -- --check`
2. `cargo check --workspace`
3. `cargo test --workspace`
4. `yunxi-agent-weixin`、存储迁移、CLI、审批、投递和 companion 连续性定向测试。
5. 既有 CLI/TUI 回归；Windows 下在适用时完成真实 ConPTY 验证，证明前台微信服务没有
   破坏终端行为。
6. 使用指定测试账户完成真实个人微信 iLink 扫码和私聊往返：入站、配对、真实 YunXi
   Provider 回答、拒绝审批、批准安全测试动作、取消、重启恢复和投递回执。
7. `git diff --check`、版本元数据、`yunxi --version` 输出、发布提交与 annotated
   `v2.2.0` tag 一致。
8. 发布/审核报告必须记录项目内总纲图的当前 SHA-256；桌面分发副本能够同步时必须与
   正本一致。因权限无法同步时，报告必须明确项目内正本路径、哈希和桌面副本状态。

真实 iLink 或真实 Provider 测试未通过时，v2.2.0 不得发布；项目必须停留在最近一个
已通过的 v2.1.x，并在该版本完成整改。

## 八、全发布线开发纪律

- 所有源码工作固定在 `D:\YunXi Agent` 及其 worktree。不得把凭证、生成源码、编译
  产物或微信临时追踪数据写入无关目录。
- 不复制 Go/Python/TypeScript 源码；提取协议逻辑并以 Rust 模型、测试和实现重建，
  在设计/开发报告中保留参考来源。
- 先完成完整功能切片，再统一运行约定测试。未通过测试与真实验证前，禁止宣称完成。
- 清理编译/采集产物属于受控操作：仅在用户授权下，清理已核实位于
  `D:\YunXi Agent` 内的明确路径；禁止宽泛删除、禁止触碰用户数据。
- 每阶段结束后，开发日志必须追加带时间戳的目标、流程、修改文件/路径、验证、提交/推送、
  清理和署名“开发者”。每次审核必须对照本总纲图，并完成源码、测试、真实 Provider、
  真实 TUI/CLI 与版本 tag 检查，署名“审核者”。
- 任何微信状态迁移、目录迁移或清理均需在独立提交中完成；提交前后必须记录受影响的
  绝对路径、存储 schema、引用扫描结果和回滚方式，禁止把它们与不相关功能混合提交。

## 九、v2.2.0 完成定义

只有在以下闭环全部成立时，v2.2.0 才算完成：项目根目录已具备可验证的源码、参考输入、
文档、验证资产与本地再生产物边界，文档索引可定位路线图/报告/证据且历史引用未断裂；
项目内路线图为唯一正本，桌面副本状态与 SHA-256 可追溯；已配对个人微信私聊进入既有
YunXi companion Runtime；复用可持久的私密 session 和记忆；加密待处理状态和会话绑定在
受控重启后可恢复；回信具有真实投递依据但不对结果不明的网络发送虚假承诺；工具仍位于
明确远程审批之后；默认 CLI 与既有 TUI 保持稳定；秘密不离开本地安全存储；群聊和未经
验证的主动推送不被宣传为已完成能力。
