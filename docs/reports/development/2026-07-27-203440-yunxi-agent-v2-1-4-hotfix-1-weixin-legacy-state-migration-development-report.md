# YunXi Agent v2.1.4-hotfix.1 微信旧账户状态迁移整改开发报告

- 撰写时间：2026-07-27 20:34:40 +08:00
- 审核依据：`C:\Users\24763\Desktop\YunXi Agent审核报告\2026-07-27-201143-YunXi-Agent-v2.1.4-微信状态生命周期审核报告.md`
- 项目内审核报告归档：`D:\YunXi Agent\docs\reports\audits\2026-07-27-201143-yunxi-agent-v2-1-4-weixin-state-lifecycle-audit-report.md`
- 审核报告 SHA-256：`17B2DD31E569628B0402D6B6B8D1B75F65E2C5DFAF11CE9A3AC9852C65B2A054`
- 总纲正本：`D:\YunXi Agent\docs\superpowers\plans\2026-07-22-yunxi-agent-v2-1-1-to-v2-2-0-personal-wechat-roadmap.md`
- 总纲 SHA-256：`2DF30D503F46CFE7496567F5011BF5CBFB8BA73C91C2D2FA3E9F29AF0932EA0F`
- 当前工作树基线：`HEAD=2ff35a5872a52f1bb15bff6ca929c0a5dc58eee4`，`git describe=v2.1.4-1-g2ff35a5-dirty`
- 审核结论转化：`v2.1.4` 审核不通过，禁止进入总纲图中的 `v2.1.5`。
- 整改目标版本：`v2.1.4-hotfix.1`
- 必须创建的 tag：新的 annotated `v2.1.4-hotfix.1`
- 已存在且不得移动的 tag：annotated `v2.1.4`，tag object `cc6e8ba3fd21311330e55bdcae120c907c3e3cdc`，target `72bbc8084313f2b2e417126c691838edf203417e`
- 开发目录：`D:\YunXi Agent`
- 报告类型：面向开发者的整改开发指令；本报告不是完成声明。

## 一、硬性约束

以下约束是 `v2.1.4-hotfix.1` 整改、验证、发布和文档同步的前置要求，不得因为上下文过长、任务拆分、实现困难或局部修复而忽略、弱化或绕过。

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

`v2.1.4` 审核结论是不通过，唯一 P1 阻塞点是：已有 `v2.1.3` 登录账户升级到 `v2.1.4` 后，旧 `.yunxi/weixin/account-*.json` 非机密登录元数据没有被初始化或迁移为新的 `WeixinStateStore` 当前 schema 状态记录。

这意味着真实 Windows 账户仍能通过旧 metadata 和系统凭证引用被识别为 `ready/present`，但 `status --json` 只能报告：

- `state_store_configured=false`
- `state_store_schema_version=null`
- `doctor --json` 中 `state_store=missing`

该问题破坏了总纲 `v2.1.4` 的核心目标：“在接受任何用户消息前，使账户状态可检查、可重启恢复”。因此开发者必须留在 `v2.1.4` 整改线，先完成 `v2.1.4-hotfix.1`，复审通过后才允许进入 `v2.1.5`。

由于 `v2.1.4` tag 已经创建并发布，整改不得移动、覆盖或删除原 `v2.1.4` tag；整改完成、统一验证和复审通过后，必须创建新的 annotated `v2.1.4-hotfix.1` tag。

## 三、P1 阻塞点拆解

### P1：旧账户 metadata 到 `WeixinStateStore` 的迁移/初始化缺失

审核报告定位到当前源码路径：

- `D:\YunXi Agent\crates\yunxi-agent-cli\src\weixin.rs`：新的 `run_login` 成功后会通过 `persist_login -> upsert_account_state` 创建状态，但旧登录账户不会走这条路径。
- `D:\YunXi Agent\crates\yunxi-agent-cli\src\weixin.rs`：`status` 在 state 文件不存在时只输出 `state_store_configured=false`，不会从旧 metadata 创建最小状态。
- `D:\YunXi Agent\crates\yunxi-agent-cli\src\weixin.rs`：`doctor` 在 state 文件不存在时只报告 `missing`，没有迁移、初始化或可执行修复路径。
- `D:\YunXi Agent\crates\yunxi-agent-storage\src\weixin_state.rs`：`WeixinStateMigration` 只处理已经存在的 state JSON schema，不读取旧 `.yunxi/weixin/account-*.json`。
- `D:\YunXi Agent\crates\yunxi-agent-storage\tests\weixin_state_tests.rs`：旧 schema 测试只是已有 state JSON 补字段，不等价于旧账户目录迁移。

整改目标：

1. 已有旧账户 metadata 且 state 文件不存在时，能一次性、幂等、原子地创建当前 schema 的最小 `WeixinStateSnapshot`。
2. 初始化只使用旧 metadata 中允许的非机密字段和凭证引用，不读取、不复制、不打印 token、二维码 payload、原始 user ID、原始 peer ID、data key 或 context token。
3. 初始化完成后，`status --json` 和 `doctor --json` 必须显示 `state_store_configured=true`、当前 schema version、无秘密输出。
4. 新进程再次读取必须得到同样结果，不能依赖当前进程缓存。
5. 失败时必须保留旧 metadata、系统凭证引用和旧有效 state；不得留下截断 state 文件、半成品 JSON 或秘密数据。

## 四、推荐实现方案

### 1. 增加安全迁移入口

建议在 CLI 微信模块增加一个私有 helper，用于把旧账户 metadata 初始化为 state store：

```text
ensure_weixin_state_initialized_from_metadata(
  account_id,
  account_record,
  state_store,
  now_millis,
)
```

推荐行为：

- 如果当前 schema 的 state 已存在：直接返回现有 state，不覆盖 pair、pending inbound、delivery、cursor、last error 等状态。
- 如果 state 缺失：从 `WeixinAccountRecord` 构造最小 `WeixinStateSnapshot`，并通过 `FileWeixinStateStore` 的原子写入协议保存。
- 如果 state 是未来 schema：拒绝迁移并输出脱敏诊断，不得覆盖。
- 如果 state 损坏：拒绝静默覆盖，输出脱敏诊断，要求人工处理或后续专门修复命令。
- 如果旧 metadata 损坏：保持现状，`doctor` 报告结构化错误。
- 如果凭证引用存在但系统凭证不可用：仍可创建带 credential reference 的状态，但 `doctor` 必须标记 credential 不可用；不得读取或复制秘密。

实现位置建议放在 `D:\YunXi Agent\crates\yunxi-agent-cli\src\weixin.rs` 或 `yunxi-agent-weixin` 的非秘密账户生命周期模块中，不建议让 `yunxi-agent-storage` 直接依赖 `yunxi-agent-weixin`，避免 crate 依赖倒置。`yunxi-agent-storage` 可以继续只提供通用 `WeixinStateSnapshot` 构造、校验和原子写入能力。

### 2. 明确触发点

审核报告允许两种路线：`status/doctor` 只读诊断前执行安全初始化，或提供明确本地迁移步骤。为了满足真实升级账户无需重新扫码的要求，建议采用默认懒初始化：

- `yunxi weixin status --account <name> --json`：如果 metadata 存在、state 缺失，则执行一次非破坏性初始化，并在 JSON 中记录 `state_store_migration="initialized_from_legacy_metadata"` 或等价字段。
- `yunxi weixin doctor --account <name> --json`：同样尝试初始化；若失败，输出结构化、安全、脱敏的失败原因。
- 普通文本输出也应说明“state store initialized from existing metadata”，但不得输出完整本地路径、凭证 target 或秘密。

不得要求用户重新扫码才能恢复已有登录状态。重新扫码只能作为凭证失效或用户主动重绑的路径，不能作为旧状态迁移方案。

### 3. 保留当前 `v2.1.4` 已通过能力

整改范围必须窄而完整。保留并复用当前已通过的能力：

- 独立 `WeixinStateStore`。
- 临时同目录文件、flush/sync、同卷原子替换。
- 损坏 JSON、未来 schema、未完成临时文件识别。
- account lock、stale lock 恢复、活动锁拒绝。
- pair request 不透明 ID、过期、approve/deny 单次消费。
- `accepted -> ready -> running -> terminal` 状态模型。
- `logout --confirm` 活动锁拒绝与指定账户删除边界。

禁止为了修复迁移缺口而简化或绕过原子写入、schema 检查、锁规则、脱敏输出或 logout 边界。

## 五、代码接入点

| 路径 | 当前职责 | 整改要求 |
| --- | --- | --- |
| `D:\YunXi Agent\crates\yunxi-agent-cli\src\weixin.rs` | `status`、`doctor`、`persist_login`、pair、logout 的 CLI 入口 | 新增旧 metadata 到 state store 的安全初始化 helper，并接入 `status/doctor`；保持 `run_login` 现有 state upsert 路径。 |
| `D:\YunXi Agent\crates\yunxi-agent-storage\src\weixin_state.rs` | `FileWeixinStateStore`、`WeixinStateSnapshot`、schema、原子写入、pair、pending inbound、lock | 必要时增加 `create_from_account_fields` 或等价构造辅助；不得读取 `account-*.json`，不得依赖 `yunxi-agent-weixin`。 |
| `D:\YunXi Agent\crates\yunxi-agent-weixin\src\account_store.rs` | `.yunxi/weixin/account-*.json` 非机密账户 metadata | 保持 metadata 读取语义；必要时提供只读枚举/校验辅助；不要在这里保存秘密或直接承担通道 state。 |
| `D:\YunXi Agent\crates\yunxi-agent-cli\tests\cli_tests.rs` | CLI 集成测试 | 增加“旧 metadata 存在、state 缺失、首次 status/doctor 后初始化成功”的真实目录形状测试。 |
| `D:\YunXi Agent\crates\yunxi-agent-storage\tests\weixin_state_tests.rs` | state store 定向测试 | 增加构造最小账户状态、原子初始化失败保留旧状态、未来 schema 拒绝覆盖等测试。 |
| `D:\YunXi Agent\docs\weixin.md` | 微信能力边界说明 | 说明 v2.1.4-hotfix.1 的升级迁移结果、status/doctor 行为和仍未进入长轮询。 |
| `D:\YunXi Agent\README.md` | 项目能力入口说明 | 如已记录 v2.1.4 微信状态能力，需要补充旧账户升级迁移边界。 |
| `D:\YunXi Agent\docs\reports\README.md` | 报告索引 | 增加本次审核报告和本整改开发报告入口。 |
| `D:\YunXi Agent\docs\development-log.md` | 开发日志 | 记录整改目标、执行流程、验证、清理、提交、推送和 tag。 |

禁止扩大范围：

- 不实现 `v2.1.5` 长轮询、`getupdates` 常驻服务、消息入站、消息发送、配对准入 dispatch 或幂等 dispatch。
- 不实现 `v2.1.6` Runtime 会话绑定。
- 不实现远程审批、流式回信、群聊或主动推送。
- 不修改 `SessionRecord`，不把微信字段塞进 YunXi session JSON。
- 不修改 `vendor/`、`extracted/`、历史 evidence、历史报告事实记录或历史 tag。

## 六、测试与验收要求

整改完成后至少补齐并通过：

| 验收项 | 要求 |
| --- | --- |
| 旧 metadata 初始化 | 临时 workspace 中写入旧 `account-*.json`，不创建 state；首次 `status --json` 后 state 自动初始化为当前 schema。 |
| `doctor` 初始化 | 同样目录形状下，`doctor --json` 能初始化或报告明确修复状态；成功后 `state_store=present/current`。 |
| 幂等性 | 重复执行 `status/doctor` 不重复破坏状态，不清空 pair、pending、delivery、cursor 或 last error。 |
| 原子失败 | 注入写入失败时保留旧 metadata 和旧有效 state，不产生截断文件。 |
| 未来 schema | 已存在未来 schema 时拒绝覆盖，输出脱敏诊断。 |
| 损坏 metadata | metadata JSON 损坏或字段缺失时不创建错误 state，`doctor` 给出结构化错误。 |
| 凭证不可用 | metadata 凭证引用存在但 secure store 缺失时，状态初始化不泄密，`doctor` 标记 credential unavailable。 |
| 真实 Windows 复核 | 对当前已有真实账户执行 release `status --json`、`doctor --json` 和新进程重读，结果为 `state_store_configured=true`、当前 schema、无秘密输出。 |
| 回归 | `cargo fmt --all -- --check`、`cargo check --workspace`、`cargo test --workspace -- --test-threads=1`、CLI/微信/storage 定向测试、真实 Provider smoke、陪伴评测、ConPTY v210、`git diff --check` 全部通过。 |

验证通过前不能宣称 `v2.1.4-hotfix.1` 完成，也不能进入 `v2.1.5`。

## 七、参考源码状态与使用边界

审核报告提到的关键参考源码均已在本机存在，本轮不需要新增拉取：

| 参考输入 | 本机状态 | 使用方式 |
| --- | --- | --- |
| `D:\源码\reasonix\internal\bot\weixin\weixin.go` | 已存在 | 参考账户/context 持久化、通道状态边界和账户隔离逻辑；Rust 侧只迁移逻辑。 |
| `D:\源码\reasonix\internal\bot\weixin\weixin_test.go` | 已存在 | 参考账户状态、过期、隔离和重启场景测试组织方式。 |
| `D:\源码\openclaw-weixin\src\auth\accounts.ts` | 已存在 | 参考账户索引、凭证引用、删除和迁移边界。 |
| `D:\源码\openclaw-weixin\src\auth\account-store.test.ts` | 已存在 | 参考账户存储、秘密不落盘和迁移测试思路。 |
| `D:\YunXi Agent\crates\yunxi-agent-storage\src\lib.rs` | 已存在 | 参考现有 `SessionStore` 的 schema/兼容规则；微信状态继续独立。 |

所有 Go 和 TypeScript 源码只能作为逻辑参考，必须用 Rust 复刻。不得引入 Node/OpenClaw 宿主、Reasonix gateway/controller、明文凭证落盘、群聊、媒体链路、插件系统或第二套 Runtime。Letta 只作为“状态可审阅、生命周期清晰”的理念参考，不引入运行时依赖。

## 八、发布、清理与日志要求

整改完成后必须：

1. 完成一批构建后统一测试和验证，不要在单点上反复纠结。
2. 验证通过后更新 `README.md`、`docs/weixin.md`、报告索引、状态文档和开发日志，使文档与实际代码一致。
3. 阶段结束后清理编译中间产物；涉及递归删除、清空目录、`git clean` 或其他危险 shell 操作前必须得到用户对精确绝对路径的确认。
4. 创建新的发布 commit。
5. 创建新的 annotated `v2.1.4-hotfix.1` tag。
6. 推送 commit 和 tag。
7. 不移动、覆盖或删除 `v2.1.4`、`v2.1.3-hotfix.1` 或任何历史 tag。
8. 在日志中记录时间戳、工作目标、执行流程、修改文件、文件路径、验证结果、提交和推送状态，并以“开发报告撰写者”署名。

`v2.1.4-hotfix.1` 复审通过前，任何文档、日志、发布说明或回复都不得宣称 `v2.1.5` 已开始、长轮询已完成、微信消息入站已接入、Runtime 已绑定、远程审批可用、流式回信可用或群聊可用。

署名：开发报告撰写者

## 九、实施记录（2026-07-27 21:07:23 +08:00）

本轮已按本报告执行 `v2.1.4-hotfix.1` 整改，实现范围保持在旧账户 metadata 到 `WeixinStateStore` 的安全初始化，不进入 `v2.1.5`，不实现长轮询、消息入站、消息发送、Runtime 绑定、远程审批、流式回信、群聊或主动推送。

### 1. 实现摘要

- 在 `D:\YunXi Agent\crates\yunxi-agent-cli\src\weixin.rs` 增加 `ensure_weixin_state_initialized_from_metadata`，由 `status` 和 `doctor` 在旧 metadata 存在、state 缺失时触发。
- 初始化只使用 `WeixinAccountRecord` 中的脱敏账户标识、workspace hash、官方 endpoint 和 credential reference；不读取、不复制、不输出 token、二维码 payload、原始 user ID、原始 peer ID、data key 或 context token。
- 已存在当前 state 时直接返回 `already_current`，不覆盖 pair、pending inbound、delivery、cursor、last error 等运行状态。
- 缺失 state 时通过 `FileWeixinStateStore::upsert_account_state` 创建当前 schema 最小状态，继续复用同目录临时文件、flush/sync 和同卷替换边界。
- 未来 schema、损坏 state 继续拒绝覆盖；损坏 metadata 由 `doctor --json` 返回结构化安全错误，不创建错误 state。
- 凭证引用存在但系统凭证不可用时，state 仍可由非机密 metadata 初始化；doctor 以 `credential_store` 标记状态，不写明文降级。
- `status --json` 和 `doctor --json` 增加 `state_store_migration` 字段：首次初始化为 `initialized_from_legacy_metadata`，后续新进程读取为 `already_current`。
- `D:\YunXi Agent\crates\yunxi-agent-weixin\src\lib.rs` 导出 `WeixinAccountStoreError`，供 CLI 安全映射 metadata 错误。

### 2. 测试补充

- `D:\YunXi Agent\crates\yunxi-agent-cli\tests\cli_tests.rs` 增加旧 metadata 真实目录形状测试：
  - `cli_weixin_status_initializes_legacy_metadata_state_store_idempotently`
  - `cli_weixin_doctor_initializes_legacy_metadata_state_store`
  - `cli_weixin_status_refuses_future_state_schema_without_overwrite`
  - `cli_weixin_doctor_reports_damaged_metadata_without_state_creation`
- 测试覆盖首次 status 初始化当前 schema、doctor 初始化、重复 status 不清空 pair、未来 schema 拒绝覆盖、损坏 metadata 不创建错误 state、输出不包含原始账户或凭证 target。
- 现有 `D:\YunXi Agent\crates\yunxi-agent-storage\tests\weixin_state_tests.rs` 的原子写入失败保留旧状态、损坏/未来 schema 不重写、legacy state schema 内存迁移、pair 生命周期和 pending inbound 状态机测试继续通过。

### 3. 文档同步

- `D:\YunXi Agent\README.md`：当前版本改为 `v2.1.4-hotfix.1`，说明本版本只补齐旧 metadata 到 state store 的安全初始化。
- `D:\YunXi Agent\docs\README.md`：文档索引增加 hotfix 当前入口和 v2.1.4 审核不通过的前置关系。
- `D:\YunXi Agent\docs\weixin.md`：补充 status/doctor 懒初始化、幂等、未来/损坏 state 拒绝覆盖、损坏 metadata 安全诊断和仍未开放的能力边界。
- `D:\YunXi Agent\docs\reports\README.md`：更新 hotfix 报告入口口径。
- `D:\YunXi Agent\docs\development-log.md`：追加本轮开发、验证、清理与发布前状态记录。

### 4. 验证结果

- `cargo fmt --all -- --check`：通过。
- `cargo check --workspace`：通过。
- `cargo test -p yunxi-agent-storage -- --test-threads=1`：通过，含 state store 6/6。
- `cargo test -p yunxi-agent-weixin -- --test-threads=1`：通过，iLink 3/3、模型 2/2、login/store 6/6、redaction 2/2。
- `cargo test -p yunxi-agent-cli -- --test-threads=1`：通过，CLI 单元 23/23、兼容二进制 23/23、集成 54/54、JSONL 10/10。
- `cargo test --workspace -- --test-threads=1`：通过，含 TUI 161/161。
- `cargo build --workspace --release`：通过。
- `target\release\yunxi.exe --version`：输出 `yunxi 2.1.4-hotfix.1`。
- `target\release\yunxi.exe weixin status --account default --json`：首次返回 `state_store_configured=true`、`state_store_schema_version=1`、`state_store_migration=initialized_from_legacy_metadata`、`credential_state=present`、`secrets_included=false`。
- `target\release\yunxi.exe weixin doctor --account default --json`：返回 `state_store=current`、`state_store_schema_current=true`、`state_store_migration=already_current`、`credentials_configured=true`、`secrets_included=false`。
- 新进程重读 `status --json`：返回 `state_store_migration=already_current`，schema 仍为 1。
- 微信 help 10 组通过：`weixin`、`login`、`status`、`doctor`、`serve`、`pair`、`pair list`、`pair approve`、`pair deny`、`logout`。
- `cargo test -p yunxi-agent-provider -- --test-threads=1`：Provider 46/46 通过。
- `cargo test -p yunxi-agent-tui -- --test-threads=1`：TUI 161/161 通过。
- `target\release\yunxi.exe eval companion --json`：31/31 通过，审批绕过 0、禁止记忆写入 0、主动边界违规 0。
- 真实 Provider smoke：返回 `YUNXI_V214_HOTFIX1_REAL_PROVIDER_OK`，exit code 0。
- `npm.cmd run verify --prefix scripts\conpty\v210`：`ok=true`、`read_only=true`，SHA-256 为 `a291a66cf91ee788bba9944edbafbf4c8dfce43bcd2d6c7b2cc78bc6c2990fc6`。
- Markdown 本地链接检查：132 个 md 文件，缺失链接 0。
- 默认 CLI 依赖树 Codex 匹配 0。
- 受保护目录 `vendor`、`extracted`、`docs\reports\evidence`、`scripts\conpty` 改动 0。
- `git diff --check`：通过，仅有 CRLF 工作区提示。
- `git fsck --full`：退出码 0，输出为既有 dangling 对象。

### 5. 清理与发布状态

- 已在用户确认后递归删除唯一编译中间产物目录 `D:\YunXi Agent\target`，删除前解析并校验目标路径等于项目内 `target`，删除后 `ExistsAfter=false`。
- 未删除、移动或清空 `D:\YunXi Agent\.yunxi`、用户目录、Git 历史、tag、源码、文档、报告或 evidence；未执行 `git clean`、gc、prune、系统安装/卸载、PATH/注册表/系统配置修改或用户目录清理。
- 真实账户验证在项目内创建/确认了 `D:\YunXi Agent\.yunxi\weixin\state\account#933b5bde.json` 非机密状态文件；该文件属于运行状态，不纳入编译产物清理目标。
- 已创建 release commit `ef06b87a2e4cc1328d0ed1516d98e5e046d986b5`，提交信息为 `release: v2.1.4-hotfix.1 weixin legacy state migration`，作者/提交者为 `开发者 <developer@yunxi-agent.local>`。
- 已创建新的 annotated tag `v2.1.4-hotfix.1`，tag object 为 `9285eb9c455b82c0dcdce3b4ba95e8af0ed1aed1`，target 为 `ef06b87a2e4cc1328d0ed1516d98e5e046d986b5`，tagger 为 `开发者 <developer@yunxi-agent.local>`。
- 已使用 GitHub CLI 和 API key 做远端预检查与发布后核验；API key 只放入进程环境或临时内存变量，未输出、未提交、未写入 Git 配置。
- 已原子推送 `master` 和 `refs/tags/v2.1.4-hotfix.1` 到 `https://github.com/sjxbbdb/YunXi-Agent`，未使用 force。
- 发布后远端 master 为 `ef06b87a2e4cc1328d0ed1516d98e5e046d986b5`，远端 `v2.1.4-hotfix.1` tag object 为 `9285eb9c455b82c0dcdce3b4ba95e8af0ed1aed1`，target 为 `ef06b87a2e4cc1328d0ed1516d98e5e046d986b5`，远端 tag 数由 56 增至 57。
- 历史 `v2.1.4` tag object `cc6e8ba3fd21311330e55bdcae120c907c3e3cdc` 和 target `72bbc8084313f2b2e417126c691838edf203417e` 未移动、删除或覆盖。

署名：开发者
