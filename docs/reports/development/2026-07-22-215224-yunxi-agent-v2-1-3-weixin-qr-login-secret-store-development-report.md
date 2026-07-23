# YunXi Agent v2.1.3 微信二维码登录与系统安全凭证存储开发报告

- 撰写时间：2026-07-22 21:52:24 +08:00
- 审核依据：`C:\Users\24763\Desktop\YunXi Agent审核报告\2026-07-22-211826-YunXi-Agent-v2.1.2-微信骨架审核报告.md`
- 项目内审核报告归档：`D:\YunXi Agent\docs\reports\audits\2026-07-22-211826-yunxi-agent-v2-1-2-weixin-skeleton-audit-report.md`
- 审核报告 SHA-256：`E81EAA6E4C8B0FCF5BF17A8855CA7B74F68C05A8DC3AC40287E8BBE953F54484`
- 总纲正本：`D:\YunXi Agent\docs\superpowers\plans\2026-07-22-yunxi-agent-v2-1-1-to-v2-2-0-personal-wechat-roadmap.md`
- 桌面总纲副本：`C:\Users\24763\Desktop\2026-07-22-yunxi-agent-v2-1-1-to-v2-2-0-personal-wechat-roadmap.md`
- 总纲 SHA-256：`2DF30D503F46CFE7496567F5011BF5CBFB8BA73C91C2D2FA3E9F29AF0932EA0F`
- 当前工作树基线：`HEAD=0524dabf7d3c92235ec7256a00b1553d1e4ac18a`，`git describe=v2.1.2-2-g0524dab-dirty`
- 审核结论转化：`v2.1.2` 审核通过，允许进入总纲图中的 `v2.1.3` 开发。
- 目标版本：`v2.1.3`
- 必须创建的 tag：`v2.1.3`
- 开发目录：`D:\YunXi Agent`
- 报告类型：下一阶段开发指令报告，主要面向开发者；本报告不是完成声明。

## 一、硬性约束

以下约束是 `v2.1.3` 开发、验证、发布和文档同步的前置要求，不得因为上下文过长、任务拆分、实现困难或局部修复而忽略、弱化或绕过。

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

`v2.1.3` 的目标是在 `v2.1.2` 微信 crate 与 CLI 骨架基础上，实现二维码登录状态机和系统安全凭证存储，让本机 YunXi 能以受控方式绑定一个 iLink 账户。

本阶段必须交付：

1. `WeixinLoginStateMachine`：获取二维码、终端展示、轮询扫描/确认/重定向/过期/取消/超时状态，并产出类型化登录结果。
2. `WeixinSecretStore`：Windows 首期接入系统凭证存储或等价安全抽象；测试使用 fake store。
3. 非机密账户元数据写入工作区 `.yunxi/weixin/`：账户 ID、连接状态、官方 base URL、创建/更新时间、凭证引用、workspace 标识。
4. `yunxi weixin login --account <name>` 从“未实现”变为可执行登录流程；二维码过期、取消、凭证不可用必须明确失败。
5. `yunxi weixin status` 和 `doctor` 能读取登录后的脱敏状态，但不显示 token、二维码 payload、加密密钥或原始用户标识。
6. 文档说明登录能力边界：本版本只登录和保存安全引用，不启动长轮询、不接收消息、不绑定 Runtime、不发送微信回复。

本阶段仍不实现长轮询、配对准入、消息去重、会话绑定、远程审批、流式回信、真实微信聊天闭环或群聊能力。

## 三、版本与发布边界

`v2.1.2` 已完成并通过审核，发布 tag 为 `v2.1.2`，annotated tag object 为 `7aa184e4b58fddad050d9affb64a5ce27121489b`。`v2.1.3` 应在此基础上新增登录和凭证能力，不回退 `v2.1.2` 骨架。

必须遵守：

- 不移动、覆盖或删除 `v2.1.2`、`v2.1.1-hotfix.1`、`v2.1.1` 或任何历史 tag。
- 不把 token、二维码 payload、`context_token`、原始用户 ID 或原始消息内容写入普通日志、JSON/JSONL、错误文本、Markdown 证据或明文配置文件。
- 不提供明文 token 降级方案；系统安全存储不可用时，登录必须失败。
- 不把微信状态塞进 `SessionRecord`；登录元数据只能进入微信专属状态位置。
- 不复制 TypeScript 或 Go 代码；只参考逻辑并 Rust 复刻。
- 完成后必须提交新的发布 commit，并创建 annotated `v2.1.3` tag。

## 四、参考源码状态

本机参考源码已具备本阶段所需输入：

| 参考输入 | 当前状态 | `v2.1.3` 用途 |
| --- | --- | --- |
| `D:\源码\openclaw-weixin` | 已存在，remote `https://github.com/Tencent/openclaw-weixin.git`，HEAD `cef0bfc390393f716903e16d50408118047f87e0` | 参考官方 iLink 登录、账户、配对和状态处理逻辑。 |
| `D:\源码\openclaw-weixin\src\auth\login-qr.ts` | 已存在 | 重点参考二维码登录状态、轮询节奏、过期和确认处理。 |
| `D:\源码\openclaw-weixin\src\auth\accounts.ts` | 已存在 | 参考账户 ID、账户索引、凭证引用和非机密元数据边界。 |
| `D:\源码\openclaw-weixin\src\auth\account-store.test.ts` | 已存在 | 参考账户存储测试思路，Rust 侧需改为系统 secret store 与 fake store。 |
| `D:\源码\reasonix\internal\bot\weixin\weixin_login.go` | 已存在 | 参考二维码状态机、限时 HTTP、扫码确认、过期和取消逻辑。 |
| `D:\源码\reasonix\internal\bot\weixin\weixin.go` | 已存在 | 参考后续轮询和 context token 边界，但本阶段不实现长轮询。 |

不得引入的范围：

- `openclaw-weixin` 的 OpenClaw 宿主、Node/TypeScript runtime、插件 UI、媒体上传和消息处理链路。
- Reasonix 的 Go gateway/controller 产品层、明文凭证落盘和非 YunXi Runtime 架构。
- CowAgent、OpenAkita、Leon、Letta Code、Project N.E.K.O 的后续能力；它们不属于 `v2.1.3` 必需参考。

## 五、现有代码接入点

当前 `v2.1.2` 已建立微信骨架，`v2.1.3` 应在这些边界内扩展：

| 路径 | 当前状态 | `v2.1.3` 开发要求 |
| --- | --- | --- |
| `D:\YunXi Agent\crates\yunxi-agent-cli\src\weixin.rs` | `login` 当前明确返回未实现；`status/doctor` 只读脱敏输出；`serve` 只校验配置 | 将 `login` 接入二维码状态机；`status/doctor` 读取脱敏登录元数据；`serve` 仍不得启动长轮询。 |
| `D:\YunXi Agent\crates\yunxi-agent-weixin\src\ilink\client.rs` | 固定生产 endpoint、loopback 测试 endpoint、请求 ID、超时、响应上限和脱敏错误 | 增加 QR 登录相关 API 调用组合，不放宽 endpoint 和测试地址约束。 |
| `D:\YunXi Agent\crates\yunxi-agent-weixin\src\ilink\models.rs` | 已有 `GetBotQrCodeRequest/Response`、`QrCodeStatus`、`GetQrCodeStatusResponse` | 基于现有模型实现类型化 `LoginPollState` / `WeixinLoginOutcome`，补齐状态转换测试。 |
| `D:\YunXi Agent\crates\yunxi-agent-weixin\src\domain.rs` | 已有脱敏账户、对端、消息 ID、连接状态和元数据类型 | 增加非机密登录元数据需要的最小字段，不把 secret 放进 Debug/Display。 |
| `D:\YunXi Agent\crates\yunxi-agent-weixin\src\redaction.rs` | 已有 `SecretString` 和快照脱敏 | 所有新错误、诊断、JSON 输出复用该脱敏边界。 |
| `D:\YunXi Agent\crates\yunxi-agent-weixin\Cargo.toml` | 当前依赖不含系统凭证库；测试使用 `wiremock` | 如新增系统凭证依赖，必须最小化、可测试、记录 Cargo.lock 变化；Windows 首期必须可用。 |
| `D:\YunXi Agent\docs\weixin.md` | 已说明 `v2.1.2` 只是骨架 | 更新为 `v2.1.3` 登录能力边界，明确仍不能收发消息。 |

当前 Cargo 清单未检索到 `keyring`、`windows-credentials`、`credential-manager`、`secret-service` 或 `dpapi` 相关依赖。开发者需要为 `WeixinSecretStore` 选择合适的最小安全存储方案，并用 trait + fake 实现隔离平台差异。

## 六、推荐 Rust 结构

建议在 `crates\yunxi-agent-weixin\src\` 内新增或扩展：

```text
login.rs
secret_store.rs
account_store.rs
state_path.rs
ilink/
  qr.rs
  models.rs
```

推荐类型：

- `WeixinLoginStateMachine`
- `WeixinLoginOptions`
- `WeixinLoginEvent`
- `WeixinLoginOutcome`
- `WeixinLoginFailure`
- `WeixinSecretStore` trait
- `SystemWeixinSecretStore`
- `FakeWeixinSecretStore`
- `WeixinCredentialReference`
- `WeixinAccountRecord`
- `WeixinAccountStore`

二维码状态机必须显式处理：

- `Wait`
- `Scanned`
- `Confirmed`
- `ScannedButRedirect`
- `BoundRedirect`
- `Expired`
- `NeedVerifyCode`
- `VerifyCodeBlocked`
- 用户取消
- 网络超时
- 凭证存储不可用

`NeedVerifyCode`、`VerifyCodeBlocked` 和无法确认的 redirect 状态不能猜测继续；必须返回清晰失败或诊断。

## 七、系统安全凭证存储要求

`WeixinSecretStore` 必须是窄化 trait，业务层只依赖接口：

```text
put_token(account_id, token, encryption_key_ref)
get_token(account_id)
delete_token(account_id)
put_data_key(account_id, key)
get_data_key(account_id)
```

实现要求：

1. Windows 首期接入系统凭证存储或等价安全机制；秘密键由 YunXi 安装标识和微信账户 ID 派生。
2. token 与后续加密队列使用的数据加密密钥都不能写入 `.yunxi/weixin/*.json`。
3. 账户 JSON 只能保存凭证引用、脱敏账户、连接状态、官方 base URL、创建/更新时间、workspace 标识和 schema version。
4. 安全存储不可用、写入失败、读取失败、权限错误或平台不支持时，登录必须失败，不得自动降级到明文。
5. `Debug`、`Display`、`serde_json`、错误链和诊断输出都必须经过脱敏测试。
6. fake store 只用于测试，不能被 CLI 生产路径选择。

## 八、CLI 行为要求

`yunxi weixin login --account <name>` 应成为本阶段的主路径：

- 启动时显示脱敏账户名和官方 endpoint。
- 获取二维码后在终端渲染二维码或给出安全 URL/二维码文本兜底；二维码 payload 本身不得进入普通日志。
- 轮询状态要给出简短、安全、可理解的状态文本。
- 过期后必须要求用户重新执行登录，禁止隐藏无限重试。
- Ctrl-C 或取消必须停止轮询，并且不保存半成品 token。
- 登录确认后将 token 写入 `WeixinSecretStore`，将非机密账户元数据写入 `.yunxi/weixin/`。

`status` 与 `doctor`：

- 已登录时显示连接状态、账户哈希、凭证引用是否存在、metadata 路径、schema version 和最后更新时间。
- 未登录时返回明确状态，不报内部错误。
- `--json` 输出必须机器可读且无秘密。

`logout` 在 `v2.1.3` 可以只实现凭证引用删除的安全前置或继续保持未实现；若实现删除，必须要求 `--confirm`，并且只触及指定账户的微信凭证引用和微信元数据，不触及 YunXi session、persona memory、工作区或其他账户。

## 九、测试与验收清单

完成本阶段批量实现后，至少通过：

| 验证项 | 必须结论 |
| --- | --- |
| `cargo fmt --all -- --check` | 通过。 |
| `cargo check --workspace` | 通过。 |
| `cargo test --workspace` | 通过。 |
| `cargo test -p yunxi-agent-weixin` | QR 状态机、secret store、account store、脱敏测试全部通过。 |
| CLI 定向测试 | `login` mock 成功、过期、取消、凭证不可用；`status/doctor --json` 无秘密。 |
| Mock iLink 测试 | 无需真实腾讯凭证或公网即可覆盖等待、已扫码、确认、redirect、过期、verify code、超时。 |
| 搜索式脱敏测试 | token、二维码 payload、bot token、数据加密密钥、原始用户 ID 不出现在 stdout/stderr/JSON/日志/错误。 |
| 旧 CLI 回归 | `run`、sessions、persona、memory、companion、JSON、JSONL、TUI 不回归。 |
| 真实 Provider smoke | 证明新增登录能力不破坏既有 Provider；不得冒充微信真实联调。 |
| ConPTY v210 verifier | `ok=true`、`read_only=true`，不得覆盖旧 evidence。 |
| `git diff --check` | 通过。 |
| Git tag | 创建新的 annotated `v2.1.3`，历史 tag 不变。 |

真实扫码若受腾讯账号或网络条件限制不能完成，不能宣称真实登录已完成；可以提交 mock 完成候选，但审核报告必须明确真实登录缺口。

## 十、文档更新要求

必须更新：

- `D:\YunXi Agent\docs\weixin.md`
- `D:\YunXi Agent\docs\README.md`
- `D:\YunXi Agent\docs\reports\README.md`
- `D:\YunXi Agent\docs\development-log.md`

文档必须明确：

- `v2.1.3` 的新增能力是二维码登录与安全凭证引用。
- 本版本仍不接收微信消息、不发送微信消息、不进行会话绑定、不处理远程审批、不支持群聊。
- token、安全密钥和二维码 payload 的存储位置与禁止输出边界。
- 安全凭证不可用时的失败行为。
- 回滚通过选择旧 tag 完成，不重写 tag。

## 十一、推荐执行顺序

1. 先阅读 `openclaw-weixin\src\auth\login-qr.ts`、`accounts.ts` 和 Reasonix `weixin_login.go`，提取状态机和错误分类，不复制代码。
2. 定义 `WeixinSecretStore` trait、fake store 和生产 store facade。
3. 定义 `WeixinAccountRecord` 与 `.yunxi/weixin/` 元数据 schema。
4. 实现 QR 状态机，使用现有 `IlinkHttpClient` 和 `QrCodeStatus` 模型。
5. 接入 `weixin login` CLI，并实现安全终端输出、取消和错误分类。
6. 扩展 `status/doctor --json` 读取元数据与凭证引用状态。
7. 补齐 mock 测试、脱敏测试和 CLI 黑盒测试。
8. 更新文档、索引和日志。
9. 统一运行验证。
10. 清理编译中间产物前列出精确绝对路径并取得用户确认；随后创建发布 commit 和 annotated `v2.1.3` tag。

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

- `v2.1.3` 完成后必须创建新的 annotated `v2.1.3` tag。
- 不得移动、删除、覆盖 `v2.1.2` 或任何历史 tag。
- 推送必须非强制。
- 复审通过前不得进入 `v2.1.4`。

## 十三、开发执行结果

### 2026-07-23 11:00:41 +08:00

`v2.1.3` 开发候选已按本报告完成。workspace 版本已升至 `2.1.3`；微信 crate 新增二维码登录状态机、取消/超时/过期/redirect/验证码安全失败、Windows Credential Manager 生产凭证存储、按安装和账户派生的凭证引用、随机数据密钥、脱敏账户元数据及账户隔离 fake store。CLI 已接入交互式 `weixin login`、脱敏 `status/doctor` 和需 `--confirm` 的定向 `logout`；`serve`、配对、长轮询、消息收发、Runtime 绑定、远程审批和群聊仍保持关闭。

秘密边界已落实：token 和数据密钥只进入系统凭证存储，`.yunxi/weixin/*.json` 只保存账户哈希、官方 endpoint、连接状态、凭证引用、workspace 哈希、schema version 和时间戳；安全存储不可用或写入失败时不降级到明文，元数据失败会回滚本次凭证写入。二维码 payload 不进入 JSON/JSONL、普通日志、错误链或 Markdown 证据。

验证全部通过：`cargo fmt --all -- --check`、`cargo check --workspace`、`cargo test --workspace`、`cargo test -p yunxi-agent-weixin`、Runtime/TUI 定向测试和 `cargo build --workspace --release` 均成功。CLI 主集成测试 48/48、JSONL 10/10、Provider 46/46、TUI 161/161、微信登录/存储定向测试 6/6；release 输出 `yunxi 2.1.3`，10 组微信帮助命令通过，`status/doctor --json` 只输出账户哈希且 `secrets_included=false`。陪伴评测 31/31，`golden_passed=true`，审批绕过和主动边界违规均为 0。ConPTY v210 verifier 返回 `ok=true`、`read_only=true`，历史 evidence SHA-256 仍为 `a291a66cf91ee788bba9944edbafbf4c8dfce43bcd2d6c7b2cc78bc6c2990fc6`。默认 CLI 依赖树 431 行，Codex 依赖 0；138 个 YunXi 自有 Markdown 文件中的 61 个本地链接失效 0；受保护范围变更 0，`git diff --check` 与 `git fsck --full` 通过。

真实 DeepSeek Provider 单轮 smoke 返回 `YUNXI_V213_REAL_PROVIDER_OK`，exit code 0，没有秘密泄漏或工具调用事件，证明本次微信登录开发未破坏既有 Provider。真实微信扫码确认未完成，未写入真实系统微信凭证或账户元数据；当前只能表述为“Mock 与本地安全存储实现完成，真实扫码仍待独立审核/联调”，不得宣称真实微信聊天闭环。

经用户再次确认后，仅删除了 `D:\YunXi Agent\target`，未使用 `-Force`。`.git`、`.yunxi`、微信源码和正式 evidence 均保留；`.tmp` 与 ConPTY `node_modules/.work` 当时不存在，没有其他清理目标。未触碰用户目录。

当前尚未创建发布 commit、`v2.1.3` tag 或执行 GitHub 推送。下一步只允许创建固定作者和 tagger `开发者 <developer@yunxi-agent.local>` 的发布提交与全新 annotated `v2.1.3`，非强制推送；不得移动、删除或覆盖 53 个历史 tag。发布后仍需独立复审，复审通过前不得进入 `v2.1.4`。

署名：开发报告撰写者
