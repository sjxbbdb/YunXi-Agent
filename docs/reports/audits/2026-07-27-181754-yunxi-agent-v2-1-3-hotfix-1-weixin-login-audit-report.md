# YunXi Agent v2.1.3-hotfix.1 微信登录复审核报告

- 审核时间：2026-07-27 18:17:54 +08:00
- 审核版本：`v2.1.3-hotfix.1`
- 审核性质：v2.1.3 微信二维码登录与系统安全凭证存储整改复审
- 审核目录：`D:\YunXi Agent`
- 总纲正本：`D:\YunXi Agent\docs\superpowers\plans\2026-07-22-yunxi-agent-v2-1-1-to-v2-2-0-personal-wechat-roadmap.md`
- 总纲 SHA-256：`2DF30D503F46CFE7496567F5011BF5CBFB8BA73C91C2D2FA3E9F29AF0932EA0F`
- 开发报告：`D:\YunXi Agent\docs\reports\development\2026-07-27-165954-yunxi-agent-v2-1-3-hotfix-1-weixin-login-verification-remediation-development-report.md`

## 一、审核结论

**审核通过，允许进入总纲图中的 `v2.1.4` 开发。**

本次复审只对照总纲图中 v2.1.3 的要求及上一轮 v2.1.3 审核报告列出的整改项，不把 v2.1.4 的功能倒算为当前版本缺陷。确认结果如下：

- QR 登录状态机、终端安全显示、过期/取消/重定向失败路径已具备并通过测试。
- CLI 登录已有仅测试可用的依赖注入入口，Mock 成功、过期、取消、凭证不可用、元数据写入失败回滚和脱敏测试通过。
- 已独立复核真实 Windows 扫码登录结果：Credential Manager 凭证引用存在，`.yunxi/weixin/` 只保存非机密元数据，新进程重读为 ready。
- 既有 CLI、陪伴 Agent、Provider、TUI/ConPTY 和仓库边界未发现当前版本阻塞点。
- 微信消息接收、发送、长轮询、配对、Runtime 会话绑定、远程审批、流式回信和群聊仍未实现，不得把本版本通过解释为微信聊天闭环完成。

## 二、总纲逐项核对

| v2.1.3 要求 | 审核结果 | 证据 |
| --- | --- | --- |
| QR 获取、ANSI/二维码终端显示、安全 URL 兜底 | 通过 | `crates/yunxi-agent-weixin/src/login.rs`；CLI `write_login_event`；Mock 登录输出脱敏测试 |
| 等待、已扫码、确认、重定向、过期、超时、取消状态 | 通过 | `WeixinLoginStateMachine` 与 `login_store_tests.rs`；CLI helper 状态测试 |
| Windows 系统安全凭证存储，禁止明文降级 | 通过 | `crates/yunxi-agent-weixin/src/secret_store.rs`；Credential Manager 实际状态为 present；fake store 不可用测试通过 |
| `.yunxi/weixin/` 仅保存非机密账户元数据 | 通过 | `crates/yunxi-agent-weixin/src/account_store.rs`；现场字段审计无 token、二维码 payload、原始 user ID 或秘密值 |
| `yunxi weixin login --account` 可用，过期后需显式重试 | 通过 | 真实 release CLI 登录成功；过期 Mock 明确失败且不写入凭证/元数据 |
| status/doctor 脱敏诊断 | 通过 | release `status --json`、`doctor --json`；均为 `secrets_included=false`，不发起网络请求 |
| fake store 覆盖重启取回秘密、元数据不含秘密 | 通过 | CLI 与微信 crate 测试；真实新进程通过 Credential Manager 引用复核 |
| 当前版本不提前实现消息服务 | 通过 | `serve`、pair 等边界仍明确未实现；文档明确未开放消息、长轮询和群聊 |

## 三、源码审核

### 1. 登录与凭证链路

- `D:\YunXi Agent\crates\yunxi-agent-weixin\src\login.rs`
  - `WeixinLoginStateMachine` 负责二维码获取、轮询、取消、超时和类型化失败。
  - 生产 transport 仍使用固定官方 iLink endpoint；测试 transport 只通过私有测试 seam 注入。
  - 确认状态要求 bot token 非空，过期、重定向和验证失败不会继续写入凭证。
- `D:\YunXi Agent\crates\yunxi-agent-weixin\src\secret_store.rs`
  - Windows 使用系统 Credential Manager；不可用时返回错误，不创建明文文件回退。
  - fake store 仅用于测试，并覆盖账户隔离、不可用和凭证读写。
- `D:\YunXi Agent\crates\yunxi-agent-weixin\src\account_store.rs`
  - 元数据包含 schema、脱敏账户 ID、连接状态、官方 endpoint、凭证引用、workspace 标识和时间戳。
  - 本版本未把 v2.1.4 的原子状态存储要求提前混入；当前直接写入协议属于下一版本需替换的明确边界。
- `D:\YunXi Agent\crates\yunxi-agent-cli\src\weixin.rs`
  - 生产入口固定构造真实 `IlinkHttpClient`、`SystemWeixinSecretStore` 和 workspace account store。
  - `run_login_with_dependencies` 只作为 crate 内测试 seam，注入 scripted transport、fake store 和缓冲输出，不开放用户自定义 endpoint 或凭证后端。
  - `persist_login` 在 token/data-key 写入后遇到元数据失败时执行回滚；登录输出不打印秘密。

### 2. 当前版本回归边界

- 未发现对既有 `run`、sessions、persona、memory、companion、JSON/JSONL 或 TUI 生产路径的破坏性改动。
- TUI 本版本没有新增路线图范围；`cargo test -p yunxi-agent-tui` 全部通过，ConPTY v210 基线通过，未发现布局、交互或输出协议回归。
- 依赖树中无 `yunxi-agent-codex`、`codex-*` 或 `vendor/codex-rs` 生产依赖；根 Cargo 的 exclude 仅保留既有源码边界声明。

## 四、测试与运行验证

| 验证项 | 结果 |
| --- | --- |
| `cargo fmt --all -- --check` | 通过 |
| `cargo check --workspace` | 通过 |
| `cargo test --workspace` | 通过；CLI 23/23，CLI 集成 48/48，JSONL 10/10，TUI 161/161，微信相关测试全部通过 |
| `cargo test -p yunxi-agent-weixin` | 通过；iLink 3/3，模型 2/2，登录/存储 6/6，脱敏 2/2 |
| `cargo test -p yunxi-agent-cli weixin::tests` | 通过；8/8 |
| `cargo build --workspace --release` | 通过 |
| release `yunxi --version` | `yunxi 2.1.3-hotfix.1` |
| release `status --json` | `state=ready`，`credential_state=present`，Windows Credential Manager，`secrets_included=false` |
| release `doctor --json` | 元数据和凭证均 present，无网络请求，`secrets_included=false` |
| 新进程重读 status | 通过，仍为 `ready/present` |
| 元数据字段/秘密检查 | 通过；仅有允许的账户、状态、endpoint、凭证引用、workspace 和时间字段 |
| 真实 Provider smoke | 通过；DeepSeek 返回 `YUNXI_V213_HOTFIX1_PROVIDER_OK`，exit code 0；仅证明既有 Provider 未回归，不冒充微信联调 |
| `yunxi eval companion --json` | 31/31，`golden_passed=true`，审批绕过 0，禁止记忆写入 0，主动边界违规 0 |
| ConPTY v210 verifier | `ok=true`，`read_only=true`，evidence SHA-256 `a291a66cf91ee788bba9944edbafbf4c8dfce43bcd2d6c7b2cc78bc6c2990fc6` |
| `git diff --check` | 通过 |
| tag | `v2.1.3-hotfix.1` 为 annotated tag，tag object `fe1cce2f95099c1c2cddbfe4a3d993b4bd75b310`，目标提交 `7518dfb8d8d8e069906c3bcbafcf36765b3e8808` |
| 远端发布 | `origin/master` 已包含该 tag，远端 tag object 一致；未使用 force，历史 tag 未移动/删除/覆盖 |

## 五、参考源码核对

本版本实际对应并应在实现中保留的参考来源如下：

- `D:\源码\openclaw-weixin\src\auth\login-qr.ts`：参考二维码登录状态、轮询、过期和确认逻辑；已按 Rust 状态机重新建模。
- `D:\源码\openclaw-weixin\src\auth\accounts.ts`：参考账户索引与凭证引用边界；当前只落盘脱敏元数据。
- `D:\源码\openclaw-weixin\src\auth\account-store.test.ts`：参考账户存储和秘密不落盘测试思路。
- `D:\源码\reasonix\internal\bot\weixin\weixin_login.go`：参考二维码登录、限时 HTTP、取消和确认流程；未复制 Go 宿主实现。
- `D:\源码\reasonix\internal\bot\weixin\weixin.go`：作为 iLink 账户/context 边界参考；本版本没有提前实现长轮询和消息 Runtime。
- 总纲指定的 Tencent iLink 协议行为、CowAgent 过期/重连策略：当前只吸收登录状态和显式重试的逻辑，不把后续常驻消息服务提前并入。

## 六、允许进入 v2.1.4 的开发建议

下一版本必须围绕“状态持久化、诊断与安全账户生命周期”展开，不能直接跳到消息 Runtime：

1. 在 `crates/yunxi-agent-storage` 内新增独立、版本化的 `WeixinStateStore`，不要向 `SessionRecord` 塞微信字段，也不要继续复用普通 `FileSessionStore` 的直接截断写入。
2. 实现同目录临时文件、flush/sync、同卷原子替换和启动恢复检查；覆盖损坏记录、未完成临时文件、未来 schema 和中断写入测试。
3. 持久化游标、回执、会话绑定、加密待处理项、配对请求和状态转换时间；明确 `accepted -> ready -> running -> terminal`，不承诺跨 Provider/工具副作用的绝对一次执行。
4. 实现账户粒度系统排他锁，覆盖跨 workspace、跨进程竞争和陈旧锁恢复；`logout --confirm` 遇到活动服务锁必须拒绝，且不得删除 YunXi session、persona memory 或其他账户数据。
5. 完善 `status --json`、`doctor`、`pair list/approve/deny` 的脱敏机器输出，所有 request ID 必须不透明、限时并校验账户归属。

建议参考：

- `D:\源码\reasonix\internal\bot\weixin\weixin.go`：账户/context 与通道状态的持久化边界，只迁移逻辑，不复制 Go gateway。
- `D:\源码\reasonix\internal\bot\weixin\weixin_test.go`：状态、过期和隔离测试组织方式。
- `D:\YunXi Agent\crates\yunxi-agent-storage\src\lib.rs`：现有 `SessionStore`、schema 和兼容规则；微信状态应独立扩展。
- `D:\源码\openclaw-weixin\src\auth\accounts.ts`、`account-store.test.ts`：账户索引、引用、删除和测试边界。
- 总纲中 Letta 的可审阅状态理念：只借鉴状态可诊断、可解释和生命周期清晰的产品思路，不引入新的运行时依赖。

v2.1.4 完成前不得实现长轮询、消息入站、Runtime 会话绑定、远程审批、流式回信或群聊；这些仍按总纲进入后续版本。

## 七、清理、日志与发布状态

- 本次审核未执行删除、递归清理、移动、重命名、`git clean`、gc、prune、系统安装/卸载、PATH/注册表/系统配置修改。
- `D:\YunXi Agent\target`、`D:\YunXi Agent\.tmp`、`D:\YunXi Agent\.yunxi` 及 ConPTY 依赖/工作目录均未清理；如需清理，必须另行确认精确绝对路径。
- 本次审核新增项目内审核报告并将在桌面审核目录同步副本；追加项目内和桌面开发日志。
- 本次审核未创建审核提交、未修改生产源码、未创建/移动/删除 tag；当前工作树在写入本报告和日志前保持干净。

**最终结论：v2.1.3-hotfix.1 审核通过，允许进入 v2.1.4；后续开发必须严格遵守总纲范围和上述边界。**

署名：审核者
