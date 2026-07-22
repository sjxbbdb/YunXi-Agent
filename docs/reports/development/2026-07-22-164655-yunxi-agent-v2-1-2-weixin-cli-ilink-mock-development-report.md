# YunXi Agent v2.1.2 微信模块、CLI 骨架与 iLink Mock 开发报告

- 撰写时间：2026-07-22 16:46:55 +08:00
- 审核依据：`C:\Users\24763\Desktop\YunXi Agent审核报告\2026-07-22-164039-YunXi-Agent-v2.1.1-hotfix.1-独立复审审核报告.md`
- 项目内审核报告归档：`D:\YunXi Agent\docs\reports\audits\2026-07-22-164039-yunxi-agent-v2-1-1-hotfix-1-independent-reaudit-report.md`
- 审核报告 SHA-256：`21F052C2C2604D836BCD363CA5C2C706CDFEF659318468C519D07A31ADA24E20`
- 总纲正本：`D:\YunXi Agent\docs\superpowers\plans\2026-07-22-yunxi-agent-v2-1-1-to-v2-2-0-personal-wechat-roadmap.md`
- 桌面总纲副本：`C:\Users\24763\Desktop\2026-07-22-yunxi-agent-v2-1-1-to-v2-2-0-personal-wechat-roadmap.md`
- 总纲 SHA-256：`2DF30D503F46CFE7496567F5011BF5CBFB8BA73C91C2D2FA3E9F29AF0932EA0F`
- 当前工作树基线：`HEAD=74da1c4e32fe47942edbaca59a0bd85ed166cb90`，`git describe=v2.1.1-hotfix.1-3-g74da1c4-dirty`
- 复审结论转化：`v2.1.1-hotfix.1` 独立复审通过，可以进入总纲图中的 `v2.1.2` 开发。
- 目标版本：`v2.1.2`
- 必须创建的 tag：`v2.1.2`
- 开发目录：`D:\YunXi Agent`
- 报告类型：下一阶段开发指令报告，主要面向开发者；本报告不是完成声明。

## 一、硬性约束

以下约束是 `v2.1.2` 开发、验证、发布和文档同步的前置要求，不得因为上下文过长、任务拆分、实现困难或局部修复而忽略、弱化或绕过。

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

## 二、阶段目标

`v2.1.2` 的目标是在已经通过复审的目录治理基线中，建立微信接入的最小 Rust 工程骨架、CLI 命令骨架、iLink HTTP 协议客户端和确定性 Mock 测试。

本阶段必须交付：

1. 新增唯一生产 crate：`D:\YunXi Agent\crates\yunxi-agent-weixin`。
2. 根 `Cargo.toml` 增加 `crates/yunxi-agent-weixin` workspace member 和 default member，最少引入必要依赖。
3. 在 `crates\yunxi-agent-cli\src\` 增加窄化 `weixin` 命令模块，并挂载到现有 `CliCommand`。
4. 提供 `yunxi weixin login|status|doctor|serve|pair|logout` 的帮助和参数校验骨架。
5. 实现私有 `IlinkHttpClient`，覆盖超时、响应大小上限、请求 ID、错误归一化和日志脱敏。
6. 建立二维码、轮询、发送、typing、上传 URL 等 iLink serde 请求/响应模型。
7. 完成确定性离线 Mock 测试，覆盖 token 脱敏、请求头、游标、API 错误、错误 JSON、超时、数字或字符串 message ID。
8. 在 `docs/` 索引中增加微信文档入口，明确本版本不能真实登录、不能真实收发消息、不支持群聊。

本阶段仍不实现真实扫码登录、系统凭证存储、账户状态持久化、长轮询常驻服务、会话绑定、远程审批、流式回信、真实微信联调或后台主动推送。

## 三、版本与发布边界

有效准入结论是：`v2.1.1-hotfix.1` 通过，可以进入 `v2.1.2`。该结论只关闭目录治理和历史报告路径整改，不代表微信能力已经存在。

开发者必须遵守：

- `v2.1.2` 只能实现微信模块/CLI 骨架、iLink 协议客户端和确定性 Mock 测试。
- 不修改 `vendor/`、`extracted/`、历史 evidence 或 ConPTY 版本脚本。
- 不建立泛化 `yunxi-agent-channel` 框架；当前只新增 `yunxi-agent-weixin`。
- 不复制 Go、Python、TypeScript 或 Node 源码；只参考协议逻辑和测试思路，以 Rust 重写。
- 不把 `vendor/codex-rs`、`codex-*` crate 或 `yunxi-agent-codex` 带回默认运行路径。
- 完成后必须创建新的发布提交和 annotated `v2.1.2` tag；`v2.1.1-hotfix.1`、`v2.1.1` 及所有历史 tag 不得移动、覆盖或删除。

## 四、参考源码状态与补齐要求

本次依据总纲检查了本机参考源码状态：

| 参考输入 | 本机状态 | 本阶段要求 |
| --- | --- | --- |
| `D:\源码\reasonix` | 已存在 | 可参考目录治理、微信登录/轮询 Go 逻辑、bot runtime 分层。 |
| `D:\源码\reasonix\internal\bot\weixin\weixin_login.go` | 已存在 | 参考二维码状态机、限时 HTTP、扫码过期与确认流程。 |
| `D:\源码\reasonix\internal\bot\weixin\weixin.go` | 已存在 | 参考轮询游标、自消息过滤、context token 重试。 |
| `D:\源码\reasonix\internal\bot\weixin\weixin_test.go` | 已存在 | 参考 Mock 与边界测试思路。 |
| `D:\源码\openclaw-weixin` | 已补齐 | `v2.1.2` 的 iLink 官方协议行为基准；重点参考 `src\api\api.ts`、`src\api\types.ts`、`README.md`、`README.zh_CN.md`、`package.json`。 |

初稿撰写时曾尝试执行 `git clone --depth 1 https://github.com/Tencent/openclaw-weixin.git D:\源码\openclaw-weixin`，但 GitHub 连接失败。随后按用户要求重试 GitHub 直连，已成功拉取到 `D:\源码\openclaw-weixin`：

- remote：`https://github.com/Tencent/openclaw-weixin.git`
- HEAD：`cef0bfc390393f716903e16d50408118047f87e0`
- 关键路径核对：`src\api\api.ts`、`src\api\types.ts`、`README.md`、`README.zh_CN.md`、`package.json` 均存在。

开发者可以基于该本地快照提取 iLink 协议行为和测试思路，但仍不得复制 TypeScript 源码；必须用 idiomatic Rust、serde、tokio、reqwest 和显式错误类型重建协议客户端。

CowAgent、OpenAkita、Leon、Letta Code、Project N.E.K.O 等是总纲后续版本的参考输入，不属于 `v2.1.2` 必须补齐的阻塞参考。若开发者主动扩大参考范围，必须先核对本机路径并记录，不得把后续版本范围混入本阶段实现。

## 五、现有 YunXi 接入点

CodeGraph 复核得到当前关键接入点：

| 路径 | 当前职责 | `v2.1.2` 接入方式 |
| --- | --- | --- |
| `D:\YunXi Agent\Cargo.toml` | workspace members、default-members、workspace deps、exclude | 增加 `crates/yunxi-agent-weixin`，保持 `exclude = ["external/codex-rs", "vendor/codex-rs", "crates/yunxi-agent-codex"]`。 |
| `D:\YunXi Agent\crates\yunxi-agent-cli\src\main.rs` | `CliCommand`、`run_cli`、Provider/sandbox/approval/cwd 构造、命令分发 | 增加 `CliCommand::Weixin`，调用新 `weixin` 模块；抽取私有共享配置辅助函数，避免未来 `weixin serve` 走第二套配置。 |
| `D:\YunXi Agent\crates\yunxi-agent-cli\src\provider_mode.rs` | live/offline Provider 选择与 `AgentConfig` 应用 | `weixin serve` 后续必须复用，不得在微信模块内重新判断 Provider。 |
| `D:\YunXi Agent\crates\yunxi-agent-core\src\input.rs` | `AgentInput::text` | 本阶段不改；后续微信适配层只生成同样文本输入。 |
| `D:\YunXi Agent\crates\yunxi-agent-core\src\runner.rs` | `Agent::run_with_backend_stream` | 本阶段只保留未来桥接边界，不实际调用 Runtime。 |
| `D:\YunXi Agent\crates\yunxi-agent-core\src\backend.rs` | `AgentBackend::run_stream`、`AgentRunControl` 接口边界 | 本阶段测试不得伪造审批协议；后续 `v2.1.7` 再桥接远程文字控制。 |
| `D:\YunXi Agent\crates\yunxi-agent-storage\src\lib.rs` | `SessionStore`、`SessionRecord` 历史兼容边界 | 本阶段不把微信字段塞入 `SessionRecord`；状态存储推迟到 `v2.1.4`。 |

## 六、建议 Rust 结构

新增 crate 建议结构：

```text
crates/yunxi-agent-weixin/
  Cargo.toml
  src/
    lib.rs
    domain.rs
    error.rs
    redaction.rs
    ilink/
      mod.rs
      client.rs
      models.rs
      qr.rs
      poll.rs
      send.rs
  tests/
    ilink_client_tests.rs
    ilink_models_tests.rs
    redaction_tests.rs
```

CLI 侧建议结构：

```text
crates/yunxi-agent-cli/src/
  main.rs
  weixin.rs
```

公开 facade 应尽量窄：

- `WeixinAccountId`
- `WeixinPeerId`
- `WeixinMessageId`
- `WeixinConversationKey`
- `WeixinConnectionState`
- `WeixinAccountMetadata`
- `IlinkHttpClient`
- `WeixinApiError`

内部协议层建议拆分为：

- `IlinkQrApi`：二维码获取与状态模型。
- `IlinkPollApi`：`getupdates`、游标、消息批次模型。
- `IlinkSendApi`：`sendmessage`、`sendtyping`、上传 URL 模型。

`WeixinMessageId` 必须显式支持数字或字符串反序列化；关键字段缺失必须返回结构化错误。token、二维码 payload、`context_token`、原始用户 ID 和原始消息内容不得进入 `Debug`、`Display`、错误链、JSON snapshot 或普通日志。

## 七、CLI 命令骨架要求

新增命令族：

```text
yunxi weixin login [--account default]
yunxi weixin status [--account default] [--json]
yunxi weixin doctor [--account default] [--json]
yunxi weixin serve [--account default] [--workspace <path>] [现有 provider/sandbox/approval 参数]
yunxi weixin pair list|approve|deny ...
yunxi weixin logout [--account default] --confirm
```

`v2.1.2` 命令行为必须诚实：

- `yunxi weixin --help`、各子命令 `--help` 必须可用。
- `login`、`serve`、`pair approve`、`logout` 等未实现真实能力的命令必须返回明确“尚未完成/将在后续版本实现”的错误或状态，不得假装可用。
- `status --json` 和 `doctor --json` 的骨架输出不能包含秘密字段。
- `weixin serve` 在本阶段只做参数校验和配置路径准备，不启动长轮询，不调起 `Agent::run_with_backend_stream`。
- 远程微信消息绝不能改变工作目录、Provider、模型、sandbox 或 approval mode。

推荐从 `run_cli` 中抽取私有辅助函数：

- 解析并规范化 cwd。
- 构造 `AgentConfig`。
- 应用 model/provider/context/memory/persona/companion 选项。
- 解析 `ProviderMode`，但不要提前为微信复制一套 provider 选择逻辑。

抽取应保持私有、小范围，不把 CLI 主流程重写成大规模框架。

## 八、iLink HTTP 客户端要求

`IlinkHttpClient` 必须：

1. 复用 workspace `reqwest = 0.12`、`rustls-tls`、`serde`、`serde_json`、`tokio`、`thiserror`。
2. 对每个请求设置明确超时、响应大小上限和请求 ID。
3. 生产 endpoint 固定为官方 iLink endpoint；只有测试构造器可注入 mock endpoint。
4. 不暴露任意 base URL CLI 参数，避免用户误把微信通道变成通用 HTTP 代理。
5. 将 HTTP status、腾讯错误码、错误 JSON、超时、反序列化失败归一为 `WeixinApiError`。
6. 错误输出只包含脱敏账户名、请求 ID、接口名、分类和状态码，不包含 token、payload、原始用户 ID 或原始消息正文。
7. 协议 fixture 固定官方端点、必要 header 和关键响应字段；未知破坏性变化必须安全失败。

测试必须使用维护良好的 Rust HTTP mock 库，优先 `wiremock` 或 `httpmock`。不得手写 HTTP 解析器。若新增依赖导致离线测试无法解析，必须在日志中记录依赖获取方式和 `Cargo.lock` 变化。

## 九、文档更新要求

需要同步更新：

- `D:\YunXi Agent\docs\README.md`
- `D:\YunXi Agent\docs\reports\README.md`
- `D:\YunXi Agent\docs\development-log.md`
- 建议新增 `D:\YunXi Agent\docs\weixin.md` 或 `D:\YunXi Agent\docs\weixin\README.md`

微信文档必须明确：

- `v2.1.2` 只提供协议与命令骨架。
- 本版本尚不能真实扫码登录、收发消息或常驻服务。
- 首期只支持私聊设计，群聊关闭。
- 凭证存储、账户状态、长轮询、会话绑定、远程审批、流式回信和真实联调属于后续版本。
- 外部源码参考状态必须与实际本机路径一致，不能把拉取失败的项目写成本机已有。

## 十、推荐执行顺序

1. 补齐 `D:\源码\openclaw-weixin` 或取得等价官方源码快照，记录 commit/hash。
2. 新建 `crates/yunxi-agent-weixin` 的 crate 边界、Cargo 依赖和最小 lib facade。
3. 实现 `domain.rs`、`error.rs`、`redaction.rs`，先保护秘密输出边界。
4. 实现 iLink serde 模型，覆盖数字/字符串 message ID、缺失关键字段、未知字段容错。
5. 实现私有 `IlinkHttpClient` 和 QR/Poll/Send API 封装。
6. 增加 Rust HTTP mock 测试，覆盖请求头、授权脱敏、超时、错误 JSON、API 错误和游标。
7. 在 CLI 增加 `weixin` 模块和 `CliCommand::Weixin`，先接入帮助、参数校验和未实现状态。
8. 从 `run_cli` 抽取最小共享配置构造辅助函数，保证未来 `weixin serve` 使用同一套 Provider/sandbox/approval/cwd。
9. 更新文档索引和微信说明，记录本阶段真实能力边界。
10. 完成一批构建后统一运行格式、workspace check/test、CLI 定向测试、TUI 回归、ConPTY 只读 verifier 和文档链接检查。

## 十一、验收清单

完成 `v2.1.2` 后至少通过：

| 验证项 | 必须结论 |
| --- | --- |
| `cargo fmt --all -- --check` | 通过。 |
| `cargo check --workspace` | 通过。 |
| `cargo test --workspace` | 通过。 |
| `cargo test -p yunxi-agent-weixin` | 新 crate 定向测试通过。 |
| CLI help 测试 | `yunxi weixin --help` 与各子命令帮助可用。 |
| 协议 fixture 测试 | 无需腾讯凭证、公网或真实微信即可通过。 |
| 脱敏测试 | token、二维码 payload、`context_token`、原始用户 ID、原始消息正文不进入 Debug/Display/error/JSON snapshot。 |
| 缺失/未知字段测试 | 关键字段缺失安全失败，未知非关键字段不破坏兼容。 |
| 既有 CLI/TUI 回归 | one-shot、JSON、JSONL、sessions、persona、memory、companion、TUI 不回归。 |
| `git diff --check` | 通过。 |
| ConPTY v210 只读 verifier | `ok=true`、`read_only=true`，不得覆盖旧 evidence。 |
| Git tag | 创建新的 annotated `v2.1.2`，旧 tag 不变。 |

若协议字段没有对照 `D:\源码\openclaw-weixin`、Reasonix 或等价官方快照完成复核，不能宣称 `v2.1.2` 的协议客户端完成。若只完成 CLI 帮助但协议 mock 未完成，也不能宣称版本完成。

## 十二、清理、日志与发布纪律

阶段结束后要记录清理候选，但不得直接执行清理。以下路径如需清理，必须另行列出精确绝对路径并取得用户确认：

- `D:\YunXi Agent\target`
- `D:\YunXi Agent\.tmp`
- `D:\YunXi Agent\.yunxi`
- `D:\YunXi Agent\scripts\conpty\*\node_modules`
- `D:\YunXi Agent\scripts\conpty\*\.work`
- 其他构建、采集、缓存或临时状态目录

日志必须追加到：

- `D:\YunXi Agent\docs\development-log.md`
- `C:\Users\24763\Desktop\YunXi Agent开发日志.md`

日志必须包含时间戳、工作目标、执行流程、修改文件、文件路径、验证结果、提交和推送状态，并在结尾署名为开发报告撰写者。

发布纪律：

- `v2.1.2` 完成后必须创建新的 annotated `v2.1.2` tag。
- 不得移动、删除、覆盖 `v2.1.1-hotfix.1`、`v2.1.1` 或任何历史 tag。
- 推送必须非强制。
- 复审通过前不得进入 `v2.1.3`。

## 十三、当前任务状态

`2026-07-22 18:04:39 +08:00`，`v2.1.2` 开发候选已经按照本报告完成：新增唯一正式生产 crate `crates/yunxi-agent-weixin`，接入 `yunxi weixin login|status|doctor|serve|pair|logout` 命令骨架，实现固定生产 endpoint、显式超时、1 MiB 响应上限、请求 ID、统一错误分类、秘密包装与 JSON 脱敏，以及 QR、状态轮询、更新游标、发信、输入状态和上传 URL 的 serde/HTTP 封装。真实扫码登录、消息收发、长轮询服务、凭证持久化、群聊、远程审批和 Agent runtime bridge 均未实现，CLI 对这些能力保持明确失败，不伪装可用。

协议字段已对照 `D:\源码\openclaw-weixin` 的 `cef0bfc390393f716903e16d50408118047f87e0` 快照和 `D:\源码\reasonix` 本地参考文件复核。测试采用 `wiremock 0.6.5`，没有手写 HTTP 解析器；依赖通过获授权的 Cargo 联网流程获取并写入 `Cargo.lock`。定向微信测试 7/7、CLI 主集成测试 48/48、JSONL 10/10、Provider 46/46、TUI 161/161 通过；`cargo fmt --all -- --check`、`cargo check --workspace`、`cargo test --workspace`、workspace release build、10 组微信帮助、release 版本检查和 `git diff --check` 全部通过。陪伴评测 31/31、失败 0、`golden_passed=true`、审批绕过 0、主动边界违规 0。ConPTY v210 只读 verifier 返回 `ok=true`、`read_only=true`，正式 evidence SHA-256 保持 `a291a66cf91ee788bba9944edbafbf4c8dfce43bcd2d6c7b2cc78bc6c2990fc6`；历史 v210 golden、历史 evidence、`vendor/`、`extracted/` 和 ConPTY 脚本变更数为 0。YunXi 自有文档检查 42 个本地链接，失效 0；默认 CLI 正常依赖树没有 `codex-*`、`yunxi-agent-codex` 或 `vendor/codex-rs`。

当前尚未创建发布 commit 或 tag，也尚未推送 GitHub。发布前基线为本地/远程 master `74da1c4e32fe47942edbaca59a0bd85ed166cb90`、历史 tag 52 个、远程不存在 `v2.1.2`。下一步只允许创建固定作者/签名者 `开发者 <developer@yunxi-agent.local>` 的发布 commit 和全新 annotated `v2.1.2`，并进行非强制推送；不得移动、删除或覆盖任何历史 tag。复审通过前不得进入 `v2.1.3`。

清理候选仅记录为 `D:\YunXi Agent\target`、`D:\YunXi Agent\.tmp`、`D:\YunXi Agent\.yunxi`、`D:\YunXi Agent\scripts\conpty\*\node_modules` 和 `D:\YunXi Agent\scripts\conpty\*\.work`。本阶段没有执行删除、递归清理、目录移动、`git clean`、gc、prune 或用户目录清理，任何后续清理仍须列出精确绝对路径并另行取得确认。

署名：开发报告撰写者

## 十四、发布结果

`2026-07-22 18:09:14 +08:00`，发布提交 `b09f442adeaebc854f0ec00fb4c497bc6ec90e41` 已创建并通过原子、非强制推送发布到 GitHub master。提交 tree 为 `08f65e02562c7123d8a6dde3dc3c83b2915605b8`，parent 为 `74da1c4e32fe47942edbaca59a0bd85ed166cb90`，作者为 `开发者 <developer@yunxi-agent.local>`。

全新 annotated `v2.1.2` tag object 为 `7aa184e4b58fddad050d9affb64a5ce27121489b`，目标为发布提交 `b09f442adeaebc854f0ec00fb4c497bc6ec90e41`。GitHub master 与本地发布提交一致，远程 tag 总数由 52 增至 53；发布前 52 个历史 tag 对象 SHA 变化数为 0，`v2.1.1-hotfix.1` tag object 继续保持 `12262fa6a19cd444403414606810077d6dfc81f3`。未使用 force，未移动、删除或覆盖任何历史 tag。

本节作为发布后 docs-only 收口只允许继续推进 master，不得移动 `v2.1.2`。发布完成不等于独立复审通过；在 v2.1.2 审核通过前不得进入 v2.1.3。

署名：开发报告撰写者
