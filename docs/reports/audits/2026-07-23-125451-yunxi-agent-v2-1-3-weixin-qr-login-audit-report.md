# YunXi Agent v2.1.3 微信二维码登录与系统安全凭证存储审核报告

- 审核时间：2026-07-23 12:54:51 +08:00
- 审核版本：`v2.1.3`
- 审核目录：`D:\YunXi Agent`
- 审核依据：`D:\YunXi Agent\docs\superpowers\plans\2026-07-22-yunxi-agent-v2-1-1-to-v2-2-0-personal-wechat-roadmap.md`
- 总纲 SHA-256：`2DF30D503F46CFE7496567F5011BF5CBFB8BA73C91C2D2FA3E9F29AF0932EA0F`
- 发布提交：`f9f7dbbffb9f35e2a88769c0e7a1642f691522f3`
- 发布 tag：`v2.1.3`
- annotated tag object：`7f97abefc14b1309c39d76ad9fb974c482d7a09d`
- 当前 HEAD：`f363870`，`git describe=v2.1.3-3-gf363870`
- 报告类型：基于总纲图的当前版本独立源码审核

## 一、审核结论

**审核不通过，禁止进入总纲图中的 v2.1.4 开发。必须先完善 v2.1.3，并重新审核。**

当前版本的 Rust 编译、workspace 回归、QR 状态机单元测试、fake secret store、脱敏边界、Windows Credential Manager 代码、CLI 基础命令和既有陪伴/TUI/Provider 回归均有实现或通过证据。但是，v2.1.3 的版本闭环仍不成立：

1. 开发报告明确记录“真实微信扫码确认未完成”，当前没有真实账户从二维码获取、扫码、确认到系统凭证和账户元数据落盘的端到端证据。总纲允许提交 Mock 完成候选，但要求审核报告明确真实登录缺口；在本项目固定流程下，这意味着当前版本不能宣称完成，也不能进入下一版本。
2. 总纲验收明确要求 CLI 定向测试覆盖 login Mock 成功、过期、取消和凭证不可用。当前测试只直接测试状态机和持久化函数，CLI 生产路径硬编码 `IlinkHttpClient::new` 与 `SystemWeixinSecretStore::new`，没有 CLI 层的可控 Mock 注入测试，因此该验收项没有完成。

### 阻塞点 P1-1：真实扫码确认缺口

开发报告 `D:\YunXi Agent\docs\reports\development\2026-07-22-215224-yunxi-agent-v2-1-3-weixin-qr-login-secret-store-development-report.md` 明确写明：真实微信扫码确认尚未完成，未写入真实系统微信凭证或账户元数据。该声明与源码审查一致：`run_login` 具备真实网络路径，但当前证据只证明 fake transport 和 fake store，不证明真实扫码后的 `CredWriteW`、`.yunxi/weixin/*.json` 和重启读取链路。

整改要求：在 Windows 环境完成一次真实扫码确认，保存真实账户的非机密 metadata 和系统凭证引用，并通过 `status --json`、`doctor --json` 和受控重启读取核对。输出中不得出现 token、二维码 payload、原始用户 ID 或密钥。若外部账号或网络条件仍不允许完成，必须继续停留在 v2.1.3，不能以“代码已实现”替代真实验证。

### 阻塞点 P1-2：CLI 登录 Mock 集成验收缺口

总纲要求的 CLI 定向测试是 login Mock 成功、过期、取消、凭证不可用。当前证据分布如下：

- `crates/yunxi-agent-weixin/tests/login_store_tests.rs` 直接使用 `ScriptedTransport` 测试 `WeixinLoginStateMachine`，覆盖确认、过期、redirect、验证码、验证码阻断和取消前置网络调用。
- `crates/yunxi-agent-cli/src/weixin.rs` 的单元测试只调用泛型 `persist_login`，覆盖 fake store 成功写入和 secure store 不可用时不生成 metadata。
- `crates/yunxi-agent-cli/tests/cli_tests.rs` 只覆盖帮助、status/doctor/pair list 脱敏、`--json` 拒绝、未实现命令和未配置 logout；没有真正让 `yunxi weixin login` 走 Mock transport，也没有 CLI 层的 success/expired/cancelled/unavailable 验收。
- 生产 `run_login` 在 `crates/yunxi-agent-cli/src/weixin.rs:142-177` 直接创建 `IlinkHttpClient`、启动 Ctrl-C 监听、运行状态机，再固定创建 `SystemWeixinSecretStore`，当前没有只对测试开放的 transport/store 注入 seam。

整改要求：保留生产 endpoint 固定和 fake store 仅测试可用的安全边界，在 CLI 内抽取一个私有、可测试的登录执行辅助函数，生产入口继续绑定真实 `IlinkHttpClient` 和 `SystemWeixinSecretStore`；测试入口注入 `ScriptedTransport` 与 fake store。补齐 CLI 层成功、过期、取消、凭证不可用、metadata 写失败回滚和秘密不出 stdout/stderr/JSON 的黑盒或模块集成测试。

## 二、总纲要求逐项核对

| v2.1.3 总纲要求 | 核对结果 | 证据 |
| --- | --- | --- |
| QR 登录状态机覆盖获取二维码、轮询和终态 | 实现通过，验收闭环阻塞 | `crates/yunxi-agent-weixin/src/login.rs:35-267`；fake transport 测试通过，但真实扫码未完成 |
| 终端二维码/安全文本显示 | 实现通过 | `crates/yunxi-agent-weixin/src/login.rs:50-100` 与 `crates/yunxi-agent-cli/src/weixin.rs:228-242`；ANSI 清理后只打印终端文本 |
| 处理等待、扫码、确认、redirect、过期、验证码、取消、超时 | Mock 测试通过 | `login_store_tests.rs:79-159` 覆盖确认、终态失败和取消；真实链路尚未验证 |
| Windows 系统安全凭证存储 | 源码和编译通过，真实运行未闭环 | `secret_store.rs:341-410` 调用 `CredWriteW`、`CredReadW`、`CredDeleteW`；本次未完成真实账户写入读取证据 |
| fake secret store | 通过 | `secret_store.rs:171-313`；账号隔离、不可用和秘密边界测试通过 |
| 随机数据加密密钥 | 通过 | `secret_store.rs:315-324` 使用 `getrandom` 生成 32 字节随机值；只存入安全 store |
| `.yunxi/weixin/` 非机密账户 metadata | 源码通过，真实登录写入未验证 | `account_store.rs:15-153` 保存 schema、哈希账户、官方 endpoint、凭证引用、workspace 哈希和时间戳 |
| metadata 不写 token、数据密钥、二维码或原始用户 ID | 通过 | `record_is_safe`、`login_store_tests.rs:161-198`、CLI persist 测试 |
| `yunxi weixin login --account` 可执行 | 实现存在，真实闭环未验收 | `weixin.rs:142-177`；`--json` 参数校验实际通过，未执行真实扫码确认 |
| `status`/`doctor` 读取脱敏登录状态 | 未配置状态通过，已登录状态未真实验证 | `weixin.rs:258-387`；JSON 输出含 `secrets_included=false`，真实 metadata/credential state 未由真实账户产生 |
| logout 只删除指定微信凭证和 metadata | 源码通过，删除失败场景测试不足 | `weixin.rs:390-443` 要求 `--confirm` 并限定账户路径；未覆盖系统凭证部分删除失败的 CLI 集成测试 |
| 不启动长轮询、不收发消息、不绑定 Runtime、不群聊 | 通过 | `serve`、pair 和文档均明确关闭；当前版本 diff 未改 Runtime 行为 |
| 日志、诊断、账户 JSON 搜索式脱敏 | fake/静态边界通过 | `redaction_tests.rs`、`login_store_tests.rs`、CLI persist 测试；真实登录输出未完成验证 |
| 旧 CLI、Provider、陪伴、TUI、JSON/JSONL 不回归 | 通过 | 第五节统一验证结果 |
| 新 annotated tag，历史 tag 不变 | 本地通过 | `v2.1.3` annotated tag 指向 `f9f7dbb`；历史 tag 未被移动或删除 |

## 三、源码审核

### 1. 登录状态机

`WeixinLoginStateMachine` 通过 `WeixinLoginTransport` 抽象获取二维码和轮询状态，默认轮询间隔为 1500ms、总超时为 300 秒。确认状态缺少 token 或 token 为空时失败；过期、验证码、验证码阻断、redirect、取消和超时均不猜测继续。`WeixinQrDisplay::terminal_text` 只返回清理后的安全终端文本，`SecretString` 的 Debug/Display 不暴露原文。

状态机本身的 Rust 设计符合总纲，但它是可测试核心，不等于 CLI 真实闭环已经通过。当前 CLI 没有公开任意 endpoint，也没有将 fake transport 接入生产命令，这是安全边界正确，但测试 seam 不完整。

### 2. 系统凭证存储

Windows 后端使用 `windows-sys` 的 `Win32_Security_Credentials`：`CredWriteW` 写入 generic credential，`CredReadW` 读取，`CredDeleteW` 删除；非 Windows 后端明确返回 `Unavailable`，不会回退明文文件。凭证目标由安装标识哈希和微信账户哈希组成，token 与数据 key 不进入账户 JSON。

fake store 对账户使用脱敏哈希作为内部键，支持不可用、写失败、读写和删除，适合测试但没有被 CLI 生产路径选择。该边界正确。

### 3. 账户 metadata

`WeixinAccountRecord` 保存 schema version、脱敏 account id、固定官方 endpoint、credential reference、workspace hash 和创建/更新时间。`record_is_safe` 拒绝错误 schema、错误账户哈希、非官方 endpoint、空凭证引用和空 workspace 标识。文件位于工作区 `.yunxi/weixin/`，没有硬编码 C 盘路径。

当前写入采用同一路径 truncate + write + sync_all，而不是临时文件加同卷原子替换。总纲把原子状态写入明确放在 v2.1.4，因此这不是 v2.1.3 的阻塞点，但 v2.1.4 必须补齐并覆盖崩溃恢复。

### 4. CLI 与安全删除

`run_login` 的真实生产路径顺序是：固定 iLink 客户端 -> Ctrl-C 取消监听 -> 状态机 -> 生成数据 key -> 写安全 store -> 写账户 metadata；安全 store 或 metadata 失败时执行回滚，不能明文降级。`logout --confirm` 只定位当前账户对应的 credential target 和 metadata 文件，未触碰 YunXi session、persona、memory 或其他账户。

当前的主要设计缺口是测试可观测性：真实命令直接绑定具体网络客户端和系统 store，导致总纲要求的 CLI Mock 集成验收没有落地。该问题不是要求开放任意 base URL，而是要求在私有命令层保留安全的依赖注入测试入口。

## 四、参考源码核对

本版本开发报告明确并实际引用了以下参考输入：

1. `D:\源码\openclaw-weixin`，remote `https://github.com/Tencent/openclaw-weixin.git`，HEAD `cef0bfc390393f716903e16d50408118047f87e0`。
2. `D:\源码\openclaw-weixin\src\auth\login-qr.ts`：参考 QR 状态、轮询节奏、过期和确认逻辑。
3. `D:\源码\openclaw-weixin\src\auth\accounts.ts` 与 `account-store.test.ts`：参考账户索引、凭证引用和非机密 metadata 测试边界。
4. `D:\源码\reasonix\internal\bot\weixin\weixin_login.go`：参考二维码状态机、限时 HTTP、扫码确认、过期和取消逻辑。
5. `D:\源码\reasonix\internal\bot\weixin\weixin.go`：只作为后续轮询/context token 边界参考，本版本没有实现消息长轮询。

非 Rust 参考内容均以 Rust `serde`、`tokio`、trait、显式错误类型和 Windows API 绑定重写，没有把 Node/TypeScript 或 Go runtime 直接带入默认运行路径。

## 五、统一验证记录

### 1. Rust、微信和 CLI 测试

- `cargo fmt --all -- --check`：通过。
- `cargo check --workspace`：通过。
- `cargo test --workspace`：通过，所有 workspace 测试和文档测试无失败。
- `cargo test -p yunxi-agent-weixin`：通过，iLink 3/3、models 2/2、login/store 6/6、redaction 2/2。
- `cargo build --workspace --release`：通过，Windows 目标包含 Credential Manager 后端编译。
- CLI 主集成测试：48/48；JSONL：10/10。
- CLI 实测 `yunxi 2.1.3`、微信帮助、未配置 `status --json`、`doctor --json`、`pair list --json` 和确认 logout：通过且无秘密输出。
- CLI 实测 `weixin login --json`：在网络前明确拒绝交互式 JSON 模式；没有伪造登录成功。

### 2. 总纲未满足的测试项

- CLI login Mock 成功：未发现。
- CLI login Mock 过期：未发现。
- CLI login Mock 取消：未发现。
- CLI login Mock 凭证不可用：未发现。
- 真实微信扫码确认、Windows Credential Manager 写入、重启后读取：本次未完成。

`login_store_tests.rs` 的状态机和 store 测试不能替代上述 CLI 层验收，因为它们没有经过 `yunxi weixin login` 命令分发、输出、取消监听和生产依赖选择路径。

### 3. 真实 Provider、陪伴和 TUI/ConPTY

- 真实 DeepSeek Provider smoke：exit code 0，最终标记 `YUNXI_V213_REAL_PROVIDER_OK`，Provider `deepseek`，工具调用状态为 0；该结果只证明既有 Provider 路径未被破坏，不代表微信真实登录完成。
- `yunxi eval companion --json`：31/31，`golden_passed=true`，`memory_forbidden_writes=0`，`tool_approval_bypass_count=0`，`proactive_boundary_violation_count=0`。
- `npm.cmd run verify --prefix scripts/conpty/v210`：`ok=true`、`read_only=true`，历史 evidence SHA-256 `a291a66cf91ee788bba9944edbafbf4c8dfce43bcd2d6c7b2cc78bc6c2990fc6`。
- TUI 代码差异仅为版本号断言、快照首行和既有回归版本同步，没有修改布局、焦点、滚动、审批或流式渲染逻辑；当前路线图也不要求本版本 TUI 改造。

### 4. Git、文档和依赖

- `git diff --check`：通过。
- 工作树在审核前无源码改动；构建产生的 `target` 等项目内产物保留，未执行清理。
- `git fsck --full --no-reflogs`：未发现当前可达对象损坏；输出的 dangling 对象为历史遗留对象，未执行 gc/prune。
- 项目 Markdown 本地链接：125 个 Markdown 文件，失效本地链接 0 个。
- `v2.1.3` 为新的 annotated tag，目标为发布提交 `f9f7dbb`；`v2.1.2` 和其他历史 tag 未移动、删除或覆盖。
- v2.1.3 相对 v2.1.2 的 TUI/Runtime/Provider 行为差异仅为版本同步和测试断言，未发现默认 CLI 运行路径重新依赖 Codex 兼容 crate 的变更。

## 六、清理与安全记录

本次审核只在 `D:\YunXi Agent` 内执行源码读取、测试、构建和 CLI/Provider 验证，并读取了 `D:\源码\openclaw-weixin`、`D:\源码\reasonix` 参考路径；没有修改参考源码。没有删除、递归清理、移动、强制覆盖、系统安装/卸载、PATH/注册表/系统配置修改或用户数据清理。

本次构建重新产生或更新了 `D:\YunXi Agent\target` 等项目内编译产物；根据固定约束未执行清理。后续清理必须针对精确绝对路径另行取得用户确认。

## 七、v2.1.3 整改前不得进入 v2.1.4

开发者必须在当前版本完成以下整改后重新提交审核：

1. 增加 CLI 私有测试 seam：生产命令仍固定官方 endpoint 和 Windows system store，测试命令路径注入 scripted transport/fake secret store，不得开放用户可控任意 base URL，也不得让 fake store 进入生产路径。
2. 补齐 CLI login Mock 成功、过期、取消、凭证不可用、metadata 写失败回滚和秘密输出搜索测试；测试必须经过命令层而不是只调用状态机或 `persist_login`。
3. 在 Windows 上完成一次真实 QR 扫码确认，验证真实凭证写入 Credential Manager、`.yunxi/weixin/` 只出现安全 metadata、`status/doctor` 脱敏、logout 定向删除，以及受控重启后的凭证引用状态。
4. 真实扫码失败、凭证不可用、取消、过期和写入失败均不得产生半成品 metadata 或明文秘密；重新运行全部 workspace、Provider、陪伴、TUI/ConPTY 和文档验证。
5. 整改完成前不得创建新的 v2.1.3-hotfix 之外的下一阶段功能，不得开始 v2.1.4 的状态持久化、长轮询或消息接入工作。

## 八、最终意见

v2.1.3 的主要代码能力已经形成，但当前版本尚未满足总纲要求的完整可验证闭环。**本次审核不通过，当前版本必须整改，整改通过后才允许进入 v2.1.4。**

**署名：审核者**
