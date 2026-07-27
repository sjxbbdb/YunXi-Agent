# YunXi Agent v2.1.4 微信状态持久化与账户生命周期审核报告

- 审核时间：2026-07-27 20:11:43 +08:00
- 审核版本：`v2.1.4`
- 审核目录：`D:\YunXi Agent`
- 总纲正本：`D:\YunXi Agent\docs\superpowers\plans\2026-07-22-yunxi-agent-v2-1-1-to-v2-2-0-personal-wechat-roadmap.md`
- 总纲 SHA-256：`2DF30D503F46CFE7496567F5011BF5CBFB8BA73C91C2D2FA3E9F29AF0932EA0F`
- 开发报告：`D:\YunXi Agent\docs\reports\development\2026-07-27-183050-yunxi-agent-v2-1-4-weixin-state-store-diagnostics-lifecycle-development-report.md`

## 一、审核结论

**审核不通过，禁止进入总纲图中的 `v2.1.5`。**

当前源码已经具备 v2.1.4 的大部分状态存储、诊断、锁和配对生命周期能力，但发现一个总纲级阻塞点：**从已有 v2.1.3 登录账户升级到 v2.1.4 后，没有把旧 `.yunxi/weixin/account-*.json` 登录元数据初始化/迁移为新的 `WeixinStateStore` 状态记录。**

本机真实账户的复核结果为：

- 旧账户非机密 metadata 存在，凭证引用存在且状态为 `ready/present`。
- `D:\YunXi Agent\.yunxi\weixin\state\` 不存在状态文件。
- release `status --json` 返回 `state_store_configured=false`、`state_store_schema_version=null`。
- release `doctor --json` 返回 `state_store=missing`、`state_store_schema_current=false`。
- 新进程仍然只能重复报告缺失，不能恢复出 v2.1.4 状态。

这不只是测试数据差异，而是升级后的真实账户无法获得本版本要求的“状态可检查、可重启恢复”底座。必须留在 v2.1.4 完善并重新审核，不能以“重新扫码登录才会创建状态文件”替代升级迁移。

## 二、阻塞点

### P1：旧账户状态 Store 迁移/初始化缺失

总纲 v2.1.4 明确要求旧数据目录迁移测试通过，并要求账户状态可检查、可重启恢复。当前实现的实际行为如下：

- `D:\YunXi Agent\crates\yunxi-agent-cli\src\weixin.rs:295-307` 只在成功执行新的 `run_login` 后调用 `upsert_account_state`。
- `D:\YunXi Agent\crates\yunxi-agent-cli\src\weixin.rs:355-419` 的 `status` 在状态文件不存在时只把 `state_store_configured` 设为 `false`，没有从已有账户 metadata 创建初始状态。
- `D:\YunXi Agent\crates\yunxi-agent-cli\src\weixin.rs:477-526` 的 `doctor` 只报告 `missing`，没有迁移、初始化或明确的可执行修复路径。
- `D:\YunXi Agent\crates\yunxi-agent-storage\src\weixin_state.rs:840-895` 的 `WeixinStateMigration` 只迁移已经存在的状态 JSON schema；它不读取旧的 `.yunxi/weixin/account-*.json`，因此不能完成账户目录迁移。
- `D:\YunXi Agent\crates\yunxi-agent-storage\tests\weixin_state_tests.rs:96-115` 测试的是“已有 state JSON 缺少 schema 时在内存中补齐”，不是从 v2.1.0/v2.1.3 账户 metadata 迁移到 state store。
- CLI 集成测试主要在临时 workspace 预先写入新 `WeixinStateStore` 后测试 pair/status，没有覆盖“已有登录 metadata、没有 state 文件、升级后首次 status/doctor”的真实升级路径。

**整改要求：**

1. 为旧账户 metadata 增加一次性、幂等、可验证的 state store 初始化/迁移：从允许的非机密字段和凭证引用构造当前 schema 的最小账户状态，不读取或复制 token、二维码 payload、原始用户 ID 或其他秘密。
2. 明确迁移触发点。建议在 `status/doctor` 只读诊断前执行安全的初始化，或提供明确的本地迁移步骤；无论采用哪种方式，都不得要求用户重新扫码才能恢复已有登录状态。
3. 迁移必须使用 v2.1.4 原子写入协议，失败时保留旧 metadata 和凭证引用，不留下截断 state 文件或半成品秘密数据。
4. 增加真实目录形状测试：旧账户 metadata 存在、state 文件不存在、凭证引用为 fake/present，迁移后 state 为 current；重复执行不重复写坏时间/状态；损坏 metadata、未来 state schema 和凭证不可用均需安全诊断。
5. 在 Windows release 环境对当前已有账户执行一次升级后 `status --json`、`doctor --json` 和新进程读取，必须得到 `state_store_configured=true`、当前 schema 和无秘密输出。
6. 更新 `README.md`、`docs/weixin.md`、开发报告和日志，准确说明迁移结果；复审通过前不得进入 v2.1.5。

## 三、总纲逐项核对

| v2.1.4 要求 | 结果 | 审核说明 |
| --- | --- | --- |
| 独立、版本化 `WeixinStateStore`，不修改 `SessionRecord` | 通过 | `crates/yunxi-agent-storage/src/weixin_state.rs` 独立实现；旧 session 结构未混入微信字段 |
| 临时同目录文件、flush/sync、同卷原子替换 | 通过 | Windows `MoveFileExW(REPLACE_EXISTING|WRITE_THROUGH)`；失败前保留旧状态测试通过 |
| 识别临时文件、损坏 JSON、未来 schema | 通过 | `temp_file_candidates`、JSON/schema 错误路径及专项测试通过 |
| 账户、游标、回执、绑定、reply context、pending delivery、pair、pending inbound schema | 通过 | `WeixinStateSnapshot` 已包含对应记录类型，字段具备 schema/时间戳边界 |
| `accepted -> ready -> running -> terminal`，终态不恢复 | 通过 | `WeixinPendingInbound::transition` 和 6 项状态存储测试通过 |
| 接纳与游标同一持久化提交、同消息 ID 串行锁 | 部分通过 | 本版本建立记录模型和状态约束；真实消息接纳/长轮询留在 v2.1.5，当前不应宣称消息闭环 |
| 跨 workspace/进程账户排他锁与陈旧锁恢复 | 通过 | 账户级 lock root、进程探测、活动锁拒绝和陈旧锁恢复测试通过 |
| pair request ID 脱敏、过期、单次消费 | 通过 | pair list/approve/deny 集成测试和存储测试通过 |
| status/doctor 检查状态 store、凭证、锁、私聊策略和脱敏错误 | 部分通过 | 命令能报告 missing，但旧账户未自动初始化，导致真实升级账户不满足 state store 可恢复要求 |
| logout 活动锁拒绝、只删除指定微信状态和凭证 | 通过 | 活动锁拒绝测试通过；真实账户未执行确认 logout，避免破坏审核用凭证 |
| 旧数据目录迁移 | **不通过** | 仅有已有 state JSON 的旧 schema 内存迁移，没有旧账户 metadata 到新 state store 的迁移 |

## 四、源码与边界审核

- `D:\YunXi Agent\crates\yunxi-agent-storage\src\weixin_state.rs`：独立 state store、原子写入、schema、pair、pending inbound 和账户锁实现完整，未发现当前实现中的秘密明文落盘。
- `D:\YunXi Agent\crates\yunxi-agent-cli\src\weixin.rs`：新登录能创建 state，但升级旧账户缺少迁移入口；这是本次唯一 P1 阻塞点。
- `D:\YunXi Agent\crates\yunxi-agent-cli\tests\cli_tests.rs`：50 项 CLI 集成测试通过，覆盖 pair、锁、status/doctor 脱敏，但未覆盖旧登录 metadata 升级迁移。
- `D:\YunXi Agent\crates\yunxi-agent-storage\tests\weixin_state_tests.rs`：6 项新测试通过，但“legacy schema”是已有 state JSON 的内存补字段，不等价于旧账户目录迁移。
- 未发现 v2.1.4 提前实现长轮询、消息入站、消息发送、Runtime 绑定、远程审批、流式回信或群聊；这些边界保持正确。
- TUI 本版本没有新的路线图范围；TUI 测试和既有 ConPTY 只读基线通过，未发现本版本视觉/交互回归。

## 五、测试与运行验证

| 验证项 | 结果 |
| --- | --- |
| `cargo fmt --all -- --check` | 通过 |
| `cargo check --workspace` | 通过 |
| `cargo test --workspace -- --test-threads=1` | 通过；无失败测试，TUI 161/161，CLI 集成 50/50，JSONL 10/10 |
| `cargo test -p yunxi-agent-storage -- --test-threads=1` | 通过；原有 22/22，新 state 6/6，控制/会话 6/6 |
| `cargo test -p yunxi-agent-weixin -- --test-threads=1` | 通过；iLink 3/3，模型 2/2，login/store 6/6，redaction 2/2 |
| `cargo test -p yunxi-agent-cli -- --test-threads=1` | 通过；CLI 单元 23/23、兼容二进制 23/23、集成 50/50、JSONL 10/10 |
| `cargo build --workspace --release` | 通过 |
| release `yunxi --version` | `yunxi 2.1.4` |
| release status | 安全输出；真实账户 `credential_state=present`，但 `state_store_configured=false` |
| release doctor | 安全输出；真实账户 `credential_store=present`，但 `state_store=missing` |
| 新进程 status | 仍为 state store 缺失，确认不是单次进程缓存问题 |
| JSON 登录/未确认 logout | 均按边界拒绝，无网络和删除动作 |
| 真实 Provider smoke | 通过；返回 `YUNXI_V214_REAL_PROVIDER_OK`，exit code 0；仅证明通用 Provider 未回归 |
| `yunxi eval companion --json` | 31/31，`golden_passed=true`，禁止记忆写入 0，审批绕过 0，主动边界违规 0 |
| ConPTY v210 | `ok=true`、`read_only=true`，evidence SHA-256 `a291a66cf91ee788bba9944edbafbf4c8dfce43bcd2d6c7b2cc78bc6c2990fc6` |
| `git diff --check` | 通过 |
| tag | annotated `v2.1.4`，tag object `cc6e8ba3fd21311330e55bdcae120c907c3e3cdc`，target `72bbc8084313f2b2e417126c691838edf203417e` |
| 远端发布 | 远端 master 已包含 tag，tag object 一致；未使用 force，历史 tag 未移动/删除/覆盖 |

第一次合并批处理因工具 124 秒时限超时，未返回失败测试名称；随后拆分并串行重新执行完整 workspace 测试，确认全部通过。因此该工具超时不是审核阻塞，真实旧账户 state store 缺失才是阻塞点。

## 六、参考源码核对

本版本总纲要求的参考来源已经在开发报告中列明，本次复核重点确认其 Rust 化边界：

- `D:\源码\reasonix\internal\bot\weixin\weixin.go`：参考账户/context 持久化和通道状态边界；本版本应继续借鉴其迁移时的账户隔离逻辑，但用 Rust state store 实现。
- `D:\源码\reasonix\internal\bot\weixin\weixin_test.go`：参考账户状态、过期和隔离测试组织方式；本次整改应增加旧 metadata 到新 state 的升级 fixture。
- `D:\YunXi Agent\crates\yunxi-agent-storage\src\lib.rs`：参考现有 `SessionStore` 的 schema/兼容规则；微信状态必须保持独立，不能把迁移字段塞入 `SessionRecord`。
- `D:\源码\openclaw-weixin\src\auth\accounts.ts` 与 `account-store.test.ts`：可继续参考账户索引、凭证引用、删除和迁移测试边界；只抽取逻辑，不能引入 Node 宿主。
- Letta 的可审阅状态理念仅作为产品设计参考，不引入新的运行时依赖。

## 七、后续开发限制

当前不能进入 v2.1.5。开发者必须先完成本报告 P1 整改并重新审核，至少补齐：

1. 旧账户 metadata 到 `WeixinStateStore` 当前 schema 的幂等迁移/初始化。
2. 迁移失败保留旧状态、原子写入和无秘密输出测试。
3. 使用当前真实账户或等价脱敏 Windows fixture 完成升级后 `status`、`doctor`、新进程读取证据。
4. 重新执行 workspace、微信、CLI、Provider、陪伴、ConPTY、tag 和文档门禁。

在 v2.1.4 复审通过前，不得开始或宣称 v2.1.5 的长轮询、消息入站、配对准入、幂等 dispatch 或任何微信聊天闭环。

## 八、清理、日志与发布状态

- 本次审核未执行删除、递归清理、移动、重命名、`git clean`、gc、prune、系统安装/卸载、PATH/注册表/系统配置修改。
- `D:\YunXi Agent\target`、`.tmp`、`.yunxi`、ConPTY 依赖和正式 evidence 均未清理；状态文件缺失是审核发现，不执行补写或删除来掩盖问题。
- 本次审核未修改生产源码、测试源码、Cargo 配置、vendor、extracted、ConPTY 脚本或历史 tag。
- 本次审核新增项目内审核报告并同步桌面审核目录，追加项目内和桌面开发日志。

**最终结论：v2.1.4 审核不通过，禁止进入 v2.1.5；完成旧账户状态 Store 迁移/初始化后必须重新审核。**

署名：审核者
