# YunXi Agent v2.1.3-hotfix.1 微信登录闭环整改开发报告

- 撰写时间：2026-07-27 16:59:54 +08:00
- 审核依据：`C:\Users\24763\Desktop\YunXi Agent审核报告\2026-07-23-125451-YunXi-Agent-v2.1.3-微信二维码登录审核报告.md`
- 项目内审核报告归档：`D:\YunXi Agent\docs\reports\audits\2026-07-23-125451-yunxi-agent-v2-1-3-weixin-qr-login-audit-report.md`
- 审核报告 SHA-256：`14462BD5D7E92E854D11F025CA61A3105644DD4BD6E247C9DC1132964CE71043`
- 总纲正本：`D:\YunXi Agent\docs\superpowers\plans\2026-07-22-yunxi-agent-v2-1-1-to-v2-2-0-personal-wechat-roadmap.md`
- 桌面总纲副本：`C:\Users\24763\Desktop\2026-07-22-yunxi-agent-v2-1-1-to-v2-2-0-personal-wechat-roadmap.md`
- 总纲 SHA-256：`2DF30D503F46CFE7496567F5011BF5CBFB8BA73C91C2D2FA3E9F29AF0932EA0F`
- 当前工作树基线：`HEAD=f36387069c1d63078812b2ae85c9a5c8f39ef2d3`，`git describe=v2.1.3-3-gf363870-dirty`
- 审核结论转化：`v2.1.3` 审核不通过，禁止进入总纲图中的 `v2.1.4`。
- 整改目标版本：`v2.1.3-hotfix.1`
- 必须创建的 tag：新的 annotated `v2.1.3-hotfix.1`
- 开发目录：`D:\YunXi Agent`
- 报告类型：面向开发者的整改开发指令；本报告不是完成声明。

## 一、硬性约束

以下约束是 `v2.1.3-hotfix.1` 整改、验证、发布和文档同步的前置要求，不得因为上下文过长、任务拆分、实现困难或局部修复而忽略、弱化或绕过。

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

## 二、审核结论与整改目标

`v2.1.3` 审核结论是不通过。当前版本已有 QR 状态机、fake secret store、Windows Credential Manager 代码、脱敏边界、CLI 基础命令和既有回归证据，但缺少两个总纲级闭环：

1. 真实微信扫码确认缺口：没有真实账户从二维码获取、扫码、确认到 Windows 系统凭证和 `.yunxi/weixin/` 非机密账户元数据落盘的端到端证据。
2. CLI 登录 Mock 集成验收缺口：总纲要求 CLI 定向测试覆盖 login Mock 成功、过期、取消、凭证不可用；当前测试主要覆盖状态机和持久化函数，没有让 `yunxi weixin login` 命令层走可控 Mock transport / fake store。

因此，下一步不是进入 `v2.1.4` 的状态持久化，也不是做长轮询、配对、消息入站或 Runtime 绑定。开发者必须先完成 `v2.1.3-hotfix.1`，补齐真实登录验证和 CLI Mock 集成验收，复审通过后才允许进入 `v2.1.4`。

由于 `v2.1.3` tag 已经存在，整改版本必须创建新的 annotated `v2.1.3-hotfix.1` tag；不得移动、覆盖或删除原 `v2.1.3` tag。

## 三、阻塞点拆解

### P1-1：真实扫码确认缺口

当前源码中 `run_login` 已具备真实网络路径，但审核报告确认没有真实扫码后写入 Credential Manager、写入 `.yunxi/weixin/*.json`、重启后读取的证据。

整改目标：

- 在 Windows 环境完成一次真实 `yunxi weixin login --account <name>`。
- 从真实二维码获取、扫码、确认到 token 写入系统凭证存储。
- 只保存非机密 metadata 到工作区 `.yunxi/weixin/`。
- 使用 `status --json`、`doctor --json` 和受控重启后读取核对状态。
- 输出中不得出现 token、二维码 payload、原始用户 ID、数据密钥或原始响应 body。

如果外部账号、腾讯接口或网络条件仍不允许真实确认，开发者必须继续停留在 `v2.1.3` 整改候选状态，不能以“代码已实现”替代真实验证。

### P1-2：CLI 登录 Mock 集成验收缺口

当前测试分布说明核心状态机可测试，但 CLI 层没有可控注入路径：

- `crates\yunxi-agent-weixin\tests\login_store_tests.rs` 使用 `ScriptedTransport` 测试状态机。
- `crates\yunxi-agent-cli\src\weixin.rs` 单元测试只测试 `persist_login`。
- `crates\yunxi-agent-cli\tests\cli_tests.rs` 没有让 `yunxi weixin login` 走 Mock transport。
- 生产 `run_login` 固定创建 `IlinkHttpClient` 与 `SystemWeixinSecretStore`，缺少仅测试可用的私有执行 seam。

整改目标：

- 保持生产 endpoint 固定为官方 iLink endpoint。
- 保持 fake store 仅测试可用，不能进入生产 CLI 路径。
- 在 CLI 内抽取私有、可测试的登录执行辅助函数。
- 生产入口继续绑定真实 `IlinkHttpClient` 与 `SystemWeixinSecretStore`。
- 测试入口注入 `ScriptedTransport` 与 fake store。
- 补齐 CLI 层成功、过期、取消、凭证不可用、metadata 写失败回滚、秘密不出 stdout/stderr/JSON 的测试。

## 四、代码接入点

本次整改应聚焦以下路径，不要扩大到 `v2.1.4` 功能：

| 路径 | 当前职责 | 整改要求 |
| --- | --- | --- |
| `D:\YunXi Agent\crates\yunxi-agent-cli\src\weixin.rs` | `run_login` 生产路径、status/doctor/logout CLI 输出 | 抽取私有 login 执行辅助；增加测试 seam；保留生产固定 endpoint 和 system store。 |
| `D:\YunXi Agent\crates\yunxi-agent-weixin\src\login.rs` | `WeixinLoginStateMachine`、QR 状态转换、取消、超时 | 复用现有状态机；不要把 CLI 测试逻辑塞进状态机；必要时补齐事件/错误分类。 |
| `D:\YunXi Agent\crates\yunxi-agent-weixin\src\secret_store.rs` | `WeixinSecretStore`、fake store、Windows Credential Manager 后端 | 增加真实写入/读取/删除诊断辅助和测试覆盖；禁止明文降级。 |
| `D:\YunXi Agent\crates\yunxi-agent-weixin\src\account_store.rs` | `.yunxi/weixin/` 非机密账户 metadata | 覆盖 metadata 写失败回滚、半成品清理和秘密搜索；原子写入完整性留给 v2.1.4，但本阶段不得产生明文秘密。 |
| `D:\YunXi Agent\crates\yunxi-agent-cli\tests\cli_tests.rs` | CLI 黑盒集成测试 | 增加 login mock 成功、过期、取消、凭证不可用、秘密脱敏测试。 |
| `D:\YunXi Agent\docs\weixin.md` | 微信能力边界说明 | 更新 hotfix 状态、真实登录验证步骤、失败条件和仍未进入长轮询的边界。 |
| `D:\YunXi Agent\docs\reports\README.md` | 报告索引 | 增加审核报告与本整改开发报告入口。 |
| `D:\YunXi Agent\docs\development-log.md` | 开发日志 | 记录整改目标、验证、提交、推送和 tag。 |

禁止触碰或扩大范围：

- 不新增 `WeixinStateStore` 完整状态机；那是 `v2.1.4` 范围。
- 不实现长轮询、配对准入、消息入站、会话绑定、远程审批或流式回信。
- 不修改 `vendor/`、`extracted/`、历史 evidence 或 ConPTY 版本脚本。
- 不把微信状态塞入 `SessionRecord`。

## 五、推荐实现方案

### 1. CLI 私有登录执行辅助

建议在 `crates\yunxi-agent-cli\src\weixin.rs` 中抽取类似：

```text
run_login_with_dependencies(
  account,
  config,
  output_mode,
  login_transport,
  secret_store,
  account_store,
  cancellation,
)
```

该函数保持 crate-private 或测试专用，不提供用户可控 base URL 参数。生产 `run_login` 只负责构造：

- `IlinkHttpClient::new(...)`
- `SystemWeixinSecretStore::new(...)`
- 真实 `WeixinAccountStore`
- Ctrl-C cancellation
- 当前 CLI 输出模式

测试只在模块内或测试 cfg 下构造：

- scripted login transport
- fake secret store
- 临时工作区 account store
- 可控 cancellation token

### 2. CLI Mock 集成覆盖

至少补齐以下测试：

- login mock confirmed：输出安全提示，写入 fake store 和 metadata，`status --json` 可读取脱敏状态。
- login mock expired：返回明确失败，不写 token，不写 metadata。
- login mock cancelled：取消后不写 token，不写 metadata。
- secure store unavailable：不写 metadata，不落明文秘密。
- metadata write failure：安全 store 写入后必须回滚，不留下孤儿凭证引用。
- stdout/stderr/JSON 搜索：二维码 payload、bot token、数据 key、原始 user id、secret pair id 均不可见。

测试必须覆盖命令层或 CLI 私有执行辅助，而不是只调用 `WeixinLoginStateMachine`。

### 3. 真实扫码验证闭环

真实验证建议步骤：

1. 使用真实账号执行 `target\release\yunxi.exe weixin login --account <test-name>`。
2. 扫码并确认。
3. 检查 `D:\YunXi Agent\.yunxi\weixin\` 只包含非机密 metadata。
4. 使用 `target\release\yunxi.exe weixin status --account <test-name> --json`。
5. 使用 `target\release\yunxi.exe weixin doctor --account <test-name> --json`。
6. 重启 shell 或重新执行二进制，确认凭证引用仍可诊断读取。
7. 如测试 logout，必须使用 `--confirm`，且只删除指定微信账户凭证和 metadata，不触碰 YunXi session、persona memory 或其他账户。

真实验证的 evidence 或日志只能记录脱敏账户、哈希、状态码、命令退出码和是否存在安全引用。不得记录二维码、token、原始 user id 或系统凭证明文。

## 六、参考源码状态与使用边界

本机参考源码已具备：

| 参考输入 | 当前状态 | 使用方式 |
| --- | --- | --- |
| `D:\源码\openclaw-weixin` | 已存在，remote `https://github.com/Tencent/openclaw-weixin.git`，HEAD `cef0bfc390393f716903e16d50408118047f87e0` | 参考官方 iLink 登录、账户和状态处理逻辑。 |
| `D:\源码\openclaw-weixin\src\auth\login-qr.ts` | 已存在 | 参考 QR 状态、轮询节奏、过期、确认。 |
| `D:\源码\openclaw-weixin\src\auth\accounts.ts` | 已存在 | 参考账户索引、凭证引用和非机密 metadata 边界。 |
| `D:\源码\openclaw-weixin\src\auth\account-store.test.ts` | 已存在 | 参考账户存储测试思路。 |
| `D:\源码\reasonix\internal\bot\weixin\weixin_login.go` | 已存在 | 参考二维码状态机、限时 HTTP、取消和确认逻辑。 |
| `D:\源码\reasonix\internal\bot\weixin\weixin.go` | 已存在 | 仅作为后续轮询/context token 边界参考，本阶段不要实现长轮询。 |

所有 TypeScript 和 Go 参考源码只能提取逻辑，必须用 Rust 重新建模。不得复制 Node/OpenClaw 宿主、Go gateway/controller、媒体上传、消息处理或后续版本能力。

## 七、验收清单

整改完成后至少通过：

| 验证项 | 必须结论 |
| --- | --- |
| `cargo fmt --all -- --check` | 通过。 |
| `cargo check --workspace` | 通过。 |
| `cargo test --workspace` | 通过。 |
| `cargo test -p yunxi-agent-weixin` | QR 状态机、secret store、account store、脱敏测试全部通过。 |
| CLI login mock success | 经过命令层或 CLI 私有执行辅助，写 fake store 与 metadata，输出无秘密。 |
| CLI login mock expired | 明确失败，不写 token，不写 metadata。 |
| CLI login mock cancelled | 明确取消，不写 token，不写 metadata。 |
| CLI login mock credential unavailable | 明确失败，不降级明文，不生成 metadata。 |
| metadata failure rollback | 安全 store 写入后 metadata 失败时回滚凭证引用。 |
| 真实微信扫码确认 | 完成二维码获取、扫码、确认、系统凭证写入、metadata 写入、status/doctor 和受控重启读取。 |
| 搜索式脱敏 | stdout/stderr/JSON/日志/metadata 不含 token、二维码 payload、原始 user id、数据 key。 |
| 旧 CLI 回归 | `run`、sessions、persona、memory、companion、JSON、JSONL、TUI 不回归。 |
| 真实 Provider smoke | 证明既有 Provider 路径未破坏，不冒充微信联调。 |
| ConPTY v210 verifier | `ok=true`、`read_only=true`，不得覆盖旧 evidence。 |
| `git diff --check` | 通过。 |
| Git tag | 创建新的 annotated `v2.1.3-hotfix.1`，历史 tag 不变。 |

如果真实扫码仍无法完成，报告必须明确停留在 `v2.1.3` 整改候选，不能进入 `v2.1.4`。

## 八、文档更新要求

必须更新：

- `D:\YunXi Agent\docs\weixin.md`
- `D:\YunXi Agent\docs\README.md`
- `D:\YunXi Agent\docs\reports\README.md`
- `D:\YunXi Agent\docs\development-log.md`

文档必须明确：

- `v2.1.3-hotfix.1` 只关闭登录验证和 CLI Mock 验收缺口。
- 当前仍不进入 `v2.1.4` 状态持久化，不做长轮询、消息入站、会话绑定、远程审批或群聊。
- 真实验证证据的脱敏格式和禁止记录字段。
- 安全凭证不可用时必须失败。
- 回滚通过选择旧 tag 完成，不重写 tag。

## 九、清理、日志与发布纪律

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

- `v2.1.3-hotfix.1` 完成后必须创建新的 annotated `v2.1.3-hotfix.1` tag。
- 不得移动、删除、覆盖 `v2.1.3`、`v2.1.2` 或任何历史 tag。
- 推送必须非强制。
- 复审通过前不得进入 `v2.1.4`。

## 十、当前任务状态

2026-07-27 17:28:52 +08:00 更新：`v2.1.3-hotfix.1` 整改候选已完成代码实现和自动化验证，但真实微信扫码确认仍需要用户在本机扫码配合，当前不得宣称完整闭环完成，不得进入 `v2.1.4`。

已完成的整改：

- `D:\YunXi Agent\Cargo.toml` 将 workspace 版本更新为 `2.1.3-hotfix.1`，`Cargo.lock` 同步包版本。
- `D:\YunXi Agent\crates\yunxi-agent-cli\Cargo.toml` 新增测试专用 `async-trait` dev-dependency。
- `D:\YunXi Agent\crates\yunxi-agent-cli\src\weixin.rs` 抽取私有 `run_login_with_dependencies`，生产入口仍固定构造 `IlinkHttpClient`、`SystemWeixinSecretStore` 和真实 `WeixinAccountStore`；测试入口注入 scripted transport、fake store、临时 account store、可控 cancellation 和输出缓冲区。
- `D:\YunXi Agent\crates\yunxi-agent-cli\src\weixin.rs` 增加 CLI 层 Mock 登录验收：成功、过期、取消、凭证不可用、metadata 写失败回滚、`--json` 登录拒绝、stdout/status JSON 脱敏。
- `D:\YunXi Agent\crates\yunxi-agent-cli\src\main.rs`、`crates\yunxi-agent-cli\tests\cli_tests.rs`、`crates\yunxi-agent-runtime\tests\general_companion_tests.rs`、`crates\yunxi-agent-tui\src\app.rs`、`crates\yunxi-agent-tui\src\render.rs`、`crates\yunxi-agent-tui\src\snapshots\*.txt`、`crates\yunxi-agent-weixin\tests\ilink_client_tests.rs` 同步当前版本口径。
- `D:\YunXi Agent\README.md`、`docs\README.md`、`docs\weixin.md`、`docs\reports\README.md` 更新 hotfix 状态、真实验证门禁、脱敏格式和仍未实现边界。

自动化验证结果：

- `cargo fmt --all -- --check`：通过。
- `cargo check --workspace`：通过。
- `cargo test --workspace`：通过；CLI 主集成 48/48、JSONL 10/10、TUI 161/161。
- `cargo test -p yunxi-agent-weixin`：通过；iLink client 3/3、models 2/2、login/store 6/6、redaction 2/2。
- `cargo test -p yunxi-agent-cli weixin::tests`：通过；CLI 微信 helper/persistence 8/8。
- `cargo build --workspace --release`：通过；release 输出 `yunxi 2.1.3-hotfix.1`。
- 10 组 `weixin` help：通过。
- 未配置 `status --json`、`doctor --json`：通过，账户与秘密字段脱敏。
- 陪伴评测：31/31，`golden_passed=true`，审批绕过 0，主动边界违规 0。
- Provider 单元测试：46/46。
- 真实 DeepSeek Provider smoke：`credential_index=1`，exit code 0，JSONL 53 行，`secret_leak_detected=False`；该结果只证明 Provider 路径未回归，不冒充真实微信联调。
- TUI 单元测试：161/161。
- ConPTY v210 verifier：`ok=true`、`read_only=true`，evidence SHA-256 `a291a66cf91ee788bba9944edbafbf4c8dfce43bcd2d6c7b2cc78bc6c2990fc6`。
- YunXi 自有 Markdown：156 个文件、65 个本地链接、失效 0。
- 默认 CLI 依赖树：425 行，`yunxi-agent-codex`、`codex-*`、`vendor/codex-rs` 匹配 0。
- 受保护目录 `vendor`、`extracted`、`docs\reports\evidence`、`scripts\conpty` 变更 0。
- `git diff --check`：通过。
- `git fsck --full`：返回 0；仓库存在历史 dangling 对象输出，但没有对象完整性失败。

真实微信扫码确认状态：

- 尚未完成。当前没有真实账号从二维码扫码确认到 Windows Credential Manager 写入、`.yunxi/weixin/` 非机密 metadata 落盘、`status --json`/`doctor --json` 与受控重启读取的端到端证据。
- 因此当前状态是 `v2.1.3-hotfix.1` 整改候选，不能宣称真实微信登录闭环完成，不能进入 `v2.1.4`，也不能把 Mock 通过替代真实验证。

清理与发布状态：

- 本阶段未执行删除、递归清理、移动、重命名、`git clean`、gc、prune、系统安装、卸载、PATH/注册表/系统配置修改或用户目录清理。
- 产生的清理候选包括 `D:\YunXi Agent\target` 和 `D:\YunXi Agent\.tmp\v213-hotfix1-status-doctor`。这些路径未取得本次明确清理确认前不得删除。
- 尚未创建 release commit，尚未创建 annotated `v2.1.3-hotfix.1` tag，尚未推送 GitHub；历史 tag 未移动、删除或覆盖。

署名：开发者

### 2026-07-27 18:03:24 +08:00 发布前最终验证

- 真实扫码成功后已更新 `D:\YunXi Agent\README.md`、`D:\YunXi Agent\docs\README.md`、`D:\YunXi Agent\docs\weixin.md`、`D:\YunXi Agent\docs\reports\README.md`、本开发报告和 `D:\YunXi Agent\docs\development-log.md`，当前口径为“真实扫码已通过，但仅代表登录与安全凭证引用链路”。
- `cargo fmt --all -- --check`：通过。
- `cargo test -p yunxi-agent-weixin`：通过；iLink client 3/3、models 2/2、login/store 6/6、redaction 2/2。
- `cargo test -p yunxi-agent-cli weixin::tests`：通过；CLI 微信 helper/persistence 8/8。
- YunXi 自有 Markdown：156 个文件、65 个本地链接、失效 0。
- `git diff --check`：exit code 0。
- `cargo check --workspace`：通过。
- `cargo test --workspace`：通过；CLI 单元 23/23、CLI 集成 48/48、JSONL 10/10、TUI 161/161，微信相关测试全通过。
- `cargo build --workspace --release`：通过。
- `D:\YunXi Agent\target\release\yunxi.exe --version`：`yunxi 2.1.3-hotfix.1`。
- release `status --json`：`state=ready`、`credential_state=present`、`credential_backend=windows-credential-manager`、`credential_reference_present=true`、`secrets_included=false`。
- release `doctor --json`：`account_metadata=true`、`credential_store=present`、`credentials_configured=true`、消息接收/发送/群聊均 false、`network_request_performed=false`、`secrets_included=false`。
- 结论：`v2.1.3-hotfix.1` 发布前验证通过，可以创建 release commit 和新的 annotated `v2.1.3-hotfix.1` tag；该结论不代表复审已通过，不代表进入 `v2.1.4`，也不代表微信消息闭环完成。

署名：开发者

### 2026-07-27 17:59:51 +08:00 真实扫码成功记录

- 第三次在可见 PowerShell 窗口运行 `D:\YunXi Agent\target\release\yunxi.exe weixin login --account default`，二维码仅显示在本机终端，没有写入聊天、报告、日志或文件。
- 状态标记文件 `D:\YunXi Agent\.tmp\v213-hotfix1-real-login-20260727-175729.status.json` 只记录退出码和完成时间，不包含二维码、token、原始账号或数据 key。
- 标记结果为 `exit_code=0`，完成时间 `2026-07-27T17:57:44.1993830+08:00`。
- `D:\YunXi Agent\target\release\yunxi.exe --json weixin status --account default` 返回 `state=ready`、`credential_state=present`、`credential_backend=windows-credential-manager`、`credential_reference_present=true`、`secrets_included=false`、metadata 路径 `D:\YunXi Agent\.yunxi/weixin\account-933b5bde.json`、exit code 0。
- `D:\YunXi Agent\target\release\yunxi.exe --json weixin doctor --account default` 返回 `account_metadata=true`、`credential_store=present`、`credentials_configured=true`、`fixed_production_endpoint=true`、消息接收/发送/群聊均为 false、`network_request_performed=false`、`secrets_included=false`、exit code 0。
- 新 PowerShell 进程重新执行 `status --json` 仍返回 `state=ready` 和 `credential_state=present`，完成受控重启读取验证。
- 只读解析 `D:\YunXi Agent\.yunxi\weixin\account-933b5bde.json`：字段仅为 `account_id`、`connection_state`、`created_at_millis`、`credential`、`endpoint`、`schema_version`、`updated_at_millis`、`workspace_id`；账户为 `account#933b5bde`，workspace 为 `workspace#73521066`，凭证目标为 Windows Credential Manager 引用名，未命中 QR、token、原始 user id 或 secret payload 值。
- 结论：真实扫码登录验证门禁已完成；该证据只证明登录与安全凭证引用链路，不代表微信消息接收、发送、长轮询、Runtime 绑定、远程审批或群聊能力完成。复审通过前仍不得进入 `v2.1.4`。
- 新增运行状态 `D:\YunXi Agent\.yunxi\weixin\account-933b5bde.json` 和 Windows Credential Manager 中的 `default` 账户凭证引用是本次真实验收结果，未取得明确授权前不得删除；新增清理候选为 `D:\YunXi Agent\.tmp\v213-hotfix1-real-login-20260727-175729.status.json`。

署名：开发者

### 2026-07-27 17:54:00 +08:00 第二次真实扫码尝试记录

- 已重新打开可见 PowerShell 窗口运行 `D:\YunXi Agent\target\release\yunxi.exe weixin login --account default`；二维码仅显示在本机终端窗口，没有写入聊天、报告、日志或文件。
- 状态标记文件 `D:\YunXi Agent\.tmp\v213-hotfix1-real-login-20260727-175309.status.json` 只记录退出码和完成时间，不包含二维码、token、原始账号或数据 key。
- 标记结果为 `exit_code=70`，完成时间 `2026-07-27T17:53:24.8354063+08:00`；PowerShell 窗口仍在运行，PID 为 `15960`，窗口中应保留人类可读错误文本。
- 只读复核 `D:\YunXi Agent\target\release\yunxi.exe --json weixin status --account default`：`state=not_configured`、`credential_state=not_configured`、`secrets_included=false`、exit code 0。
- 只读复核 `D:\YunXi Agent\target\release\yunxi.exe --json weixin doctor --account default`：`account_metadata=false`、`credential_store=missing`、`credentials_configured=false`、`network_request_performed=false`、`secrets_included=false`、exit code 0。
- 未发现 `default` 账户凭证或 `.yunxi/weixin` metadata 遗留；新增清理候选为 `D:\YunXi Agent\.tmp\v213-hotfix1-real-login-20260727-175309.status.json`。
- 结论保持不变：真实扫码验证仍未完成；当前仍是 `v2.1.3-hotfix.1` 整改候选，必须等待窗口安全错误文本定位根因，不得创建 release commit、annotated tag 或推送 GitHub。

署名：开发者

### 2026-07-27 17:49:29 +08:00 真实扫码尝试记录

- 已在可见 PowerShell 窗口运行 `D:\YunXi Agent\target\release\yunxi.exe weixin login --account default`；二维码仅显示在本机终端窗口，没有写入聊天、报告、日志或文件。
- 状态标记文件 `D:\YunXi Agent\.tmp\v213-hotfix1-real-login-20260727-174525.status.json` 只记录退出码和完成时间，不包含二维码、token、原始账号或数据 key。
- 标记结果为 `exit_code=70`，完成时间 `2026-07-27T17:45:41.3020625+08:00`。CodeGraph 复核后确认 70 对应 CLI 未分类 `InternalError`，具体根因需要终端窗口中的安全错误文本辅助判断。
- 只读复核 `D:\YunXi Agent\target\release\yunxi.exe --json weixin status --account default`：`state=not_configured`、`credential_state=not_configured`、`secrets_included=false`、exit code 0。
- 只读复核 `D:\YunXi Agent\target\release\yunxi.exe --json weixin doctor --account default`：`account_metadata=false`、`credential_store=missing`、`credentials_configured=false`、`network_request_performed=false`、`secrets_included=false`、exit code 0。
- 未发现 `default` 账户凭证或 `.yunxi/weixin` metadata 遗留；新增清理候选为 `D:\YunXi Agent\.tmp\v213-hotfix1-real-login-20260727-174525.status.json`，清理前必须重新列出精确绝对路径并取得确认。
- 结论保持不变：真实扫码验证仍未完成，当前仍是 `v2.1.3-hotfix.1` 整改候选；不得创建 release commit、不得创建 annotated `v2.1.3-hotfix.1` tag、不得推送 GitHub、不得进入 `v2.1.4`。

署名：开发者
