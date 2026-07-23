# YunXi Agent v2.1.2 微信模块、CLI 骨架与 iLink Mock 审核报告

- 审核时间：2026-07-22 21:18:26 +08:00
- 审核版本：`v2.1.2`
- 审核目录：`D:\YunXi Agent`
- 审核依据：`D:\YunXi Agent\docs\superpowers\plans\2026-07-22-yunxi-agent-v2-1-1-to-v2-2-0-personal-wechat-roadmap.md`
- 总纲 SHA-256：`2DF30D503F46CFE7496567F5011BF5CBFB8BA73C91C2D2FA3E9F29AF0932EA0F`
- 发布提交：`b09f442adeaebc854f0ec00fb4c497bc6ec90e41`
- 发布 tag：`v2.1.2`，annotated tag object `7aa184e4b58fddad050d9affb64a5ce27121489b`
- 当前 HEAD：`0524dab`，`git describe=v2.1.2-2-g0524dab`
- 报告类型：基于总纲图的当前版本独立源码审核

## 一、审核结论

**审核通过，允许进入总纲图中的 v2.1.3 开发。**

本次只对照总纲图中 v2.1.2 的要求进行审核，不把 v2.1.3 及之后尚未要求的真实扫码、凭证持久化、长轮询、会话绑定、远程审批、流式回信或真实微信联调倒灌为本版本阻塞点。v2.1.2 已完成总纲要求的微信 Rust crate、CLI 命令骨架、固定生产端点 iLink 客户端、结构化协议模型、Mock 测试、秘密脱敏、文档边界和发布 tag。

未发现当前版本源码层面的阻塞点。当前 tag 之后的 `35d70bc` 与 `0524dab` 均为发布结果和编译产物清理记录文档，不改变功能源码，因此不构成版本行为漂移或发布阻塞。

## 二、总纲要求逐项核对

| v2.1.2 总纲要求 | 核对结果 | 证据 |
| --- | --- | --- |
| 新增唯一生产 crate `crates/yunxi-agent-weixin` | 通过 | `Cargo.toml` 已加入 workspace/default-members；crate 内含 domain、error、redaction、iLink client/models 与测试 |
| 领域类型和非机密账户元数据 | 通过 | `crates/yunxi-agent-weixin/src/domain.rs:1-165`，含 `WeixinAccountId`、`WeixinPeerId`、`WeixinMessageId`、`WeixinConversationKey`、`WeixinConnectionState`、`WeixinAccountMetadata` |
| CLI 挂载到既有 `CliCommand`，不重写分发 | 通过 | `crates/yunxi-agent-cli/src/main.rs:1684-1686` 增加 `CliCommand::Weixin` 分支，调用内部 `weixin` 模块 |
| `login/status/doctor/serve/pair/logout` 帮助与参数校验骨架 | 通过 | `crates/yunxi-agent-cli/src/weixin.rs:1-68`；release 二进制逐项帮助命令均返回 0 |
| 从当前 run 路径抽取共享配置辅助 | 通过 | `main.rs:508` 和 `main.rs:536-568` 使用私有 `build_agent_config`；微信命令从共同 `run_cli -> run_command` 入口接收同一套 cwd、model、provider、context、memory、persona、companion 配置 |
| `weixin serve` 不调起 Runtime | 通过 | `weixin.rs:88-109` 只校验并规范化 workspace、复用 `ProviderMode::resolve`，明确报告未启动长轮询和 agent runtime；未调用 `run_agent_backend` 或 `run_agent_backend_stream` |
| Provider 选择、sandbox、approval 和根 cwd 不被微信消息改写 | 通过 | 本阶段无远程消息处理；CLI 只在共同配置入口构造配置，`serve` 仅验证 workspace，未实现网络循环或 Runtime 桥接 |
| 私有 `IlinkHttpClient`、reqwest+rustls、超时、响应上限、请求 ID、错误归一化 | 通过 | `crates/yunxi-agent-weixin/src/ilink/client.rs:21-30`、`:61-114`、`:206-307`；生产端点固定，测试端点仅允许 loopback |
| QR、状态、getupdates、sendmessage、sendtyping、上传 URL 模型 | 通过 | `crates/yunxi-agent-weixin/src/ilink/models.rs` 及 `qr.rs`、`poll.rs`、`send.rs` facade |
| message ID 接受数字或字符串，关键字段缺失安全失败 | 通过 | `domain.rs:76-119` 自定义反序列化；`ilink_models_tests.rs` 覆盖数字/字符串、未知字段和关键字段缺失 |
| 维护的 Rust HTTP Mock 库，不手写 HTTP 解析器 | 通过 | `wiremock` workspace dev-dependency；`ilink_client_tests.rs` 使用 Mock server |
| token 不进入 Debug、Display、错误和诊断 snapshot | 通过 | `redaction.rs:9-76`、`error.rs`；`redaction_tests.rs` 与 CLI secret-free 测试通过 |
| docs 索引明确本版本能力边界 | 通过 | `docs/README.md:13-24`、`docs/weixin.md:1-53` |
| 格式、check、workspace test、diff check | 通过 | 统一验证结果见第五节 |
| 新 annotated tag，历史 tag 不删除、不移动 | 通过 | `v2.1.2` 指向发布提交 `b09f442`；历史 `v2.1.0`、`v2.1.1`、`v2.1.1-hotfix.1` 仍存在 |

## 三、源码与边界审核

### 1. CLI 接入

`weixin` 是 `crates/yunxi-agent-cli/src/weixin.rs` 内部模块，现有 CLI 分发仍由 `main.rs` 负责。`build_agent_config` 将原先位于 `run_cli` 中的 cwd 规范化、AgentConfig 构造和全局选项应用集中起来；随后所有命令，包括微信命令，都经过共同的 `run_command` 路由。

`status`、`doctor`、`pair list` 是离线只读命令，输出版本、固定官方端点、能力开关和脱敏账户标识，不携带秘密。`login`、`serve`、`pair approve`、`pair deny` 和确认后的 `logout` 都明确返回尚未实现，而不是伪造成功。该行为与 v2.1.2 的骨架范围一致。

`weixin serve` 的当前实现只接受未来服务所需的 workspace 校验和 Provider 选择准备，未调用 Runtime、未启动长轮询、未连接腾讯网络，也未创建第二套 Agent 执行路径。`--jsonl` 对微信元数据命令会被拒绝，并提示使用 `--json`，保持既有 JSON/JSONL 约定。

### 2. iLink 协议客户端

`IlinkHttpClient::new` 只能使用 `https://ilinkai.weixin.qq.com/`；`new_for_test` 仅面向测试，并且只接受 loopback Mock 地址。请求统一携带 `AuthorizationType`、`X-WECHAT-UIN`、`iLink-App-Id`、`iLink-App-ClientVersion` 和 `X-YunXi-Request-Id`。客户端禁用自动重定向，设置请求超时，并按流读取响应以执行 1 MiB 默认上限。

HTTP 状态错误、腾讯 API 错误码、无效 JSON、网络失败、超时和协议关键字段错误均归一到 `WeixinApiError`，错误文本没有原始 body、token 或账户明文。未知非关键字段保持可扩展，关键字段缺失则安全失败，符合本阶段“不要猜测协议继续运行”的要求。

### 3. 通用型陪伴 Agent 保护

本版本只新增微信协议和 CLI 骨架，没有新建第二套 Runtime、Session、persona 或 memory。离线陪伴评测仍保持 31 个场景全部通过，memory forbidden writes 为 0，tool approval bypass 为 0，主动边界违规为 0。微信文档也明确禁止本版本宣称真实收发、后台服务、群聊或 Runtime 绑定已经完成。

## 四、参考源码核对

本版本开发报告和代码文档已明确列出并实际使用以下参考输入，本审核予以确认：

1. `D:\源码\openclaw-weixin`，remote `https://github.com/Tencent/openclaw-weixin.git`，HEAD `cef0bfc390393f716903e16d50408118047f87e0`。参考 iLink 官方协议行为、请求字段和 QR/更新/消息接口；Rust 侧以 `serde`、`reqwest`、`tokio` 和显式错误类型重写，没有复制 TypeScript 宿主层。
2. `D:\源码\reasonix\internal\bot\weixin`，参考二维码/轮询状态、字段容错、数字或字符串 ID、限时 HTTP 与 Mock 边界；Rust 侧未引入 Go runtime 或 Gateway 产品层。
3. `D:\源码\reasonix` 的 adapter/gateway 分层和目录治理材料，作为边界参考；本版本保持 Cargo workspace 与 `crates/yunxi-agent-weixin` 单 crate 方案，没有提前建立泛化通道框架。
4. YunXi 现有 `crates/yunxi-agent-cli/src/main.rs`、`provider_mode.rs`、`crates/yunxi-agent-core` 和 `crates/yunxi-agent-runtime`，用于复用既有 CLI 配置、Provider 选择和未来 Runtime 桥接边界。

本版本未把 CowAgent、OpenAkita、Leon、Letta Code 或 Project N.E.K.O 的后续能力混入 v2.1.2；这些仍属于总纲后续阶段的参考来源。

## 五、验证记录

### 1. Rust 与测试

- `cargo fmt --all -- --check`：通过。
- `cargo check --workspace`：通过。
- `cargo test --workspace`：通过，所有 workspace 测试和文档测试无失败。
- `cargo test -p yunxi-agent-weixin`：通过，iLink client 3/3、models 2/2、redaction 2/2。
- `cargo build -p yunxi-agent-cli --release --bins`：通过。
- `git diff --check`：通过。

### 2. Release CLI 实测

- `target\release\yunxi.exe --version`：`yunxi 2.1.2`。
- `weixin`、`login`、`status`、`doctor`、`serve`、`pair`、`pair list`、`pair approve`、`pair deny`、`logout` 帮助：全部返回 0。
- `status --json`、`doctor --json`、`pair list --json`：全部成功，`secrets_included=false`，账户为哈希标识，未发起网络请求。
- `login`、`pair approve`、`pair deny`、确认后的 `logout`：均以明确“未实现”错误退出，未伪造完成。
- `--offline weixin serve --workspace .`：只完成配置校验后明确返回未实现，输出含 `without starting long polling or the agent runtime`。
- `--jsonl` 用于微信元数据：按约定拒绝，并返回结构化错误提示。
- 使用 `audit-secret-account` 和 `secret-pair-id` 进行脱敏实测：输出未包含原始账户或 pair ID。

### 3. 真实 Provider

执行了无工具、短提示的真实单轮调用：

- Provider：`deepseek`。
- Model：`deepseek-v4-flash`。
- approval：`OnRequest`。
- sandbox：`WorkspaceWrite`。
- 工具调用：0。
- 最终标记：`YUNXI_V212_REAL_PROVIDER_OK`。
- exit code：0。

该结果证明新增微信 crate 和 CLI 路由没有破坏既有真实 Provider 执行路径；没有把本次真实调用结果误记为微信真实联调，微信网络联调仍属于后续版本。

### 4. 陪伴评测与 TUI/ConPTY

- `target\release\yunxi.exe eval companion --json`：31/31，`golden_passed=true`，`memory_forbidden_writes=0`，`tool_approval_bypass_count=0`，`proactive_boundary_violation_count=0`。
- `npm.cmd run verify --prefix scripts/conpty/v210`：通过，`ok=true`、`read_only=true`，既有 evidence SHA-256 为 `a291a66cf91ee788bba9944edbafbf4c8dfce43bcd2d6c7b2cc78bc6c2990fc6`。
- TUI 源码差异审查：本版本仅同步版本号、测试夹具版本设置和快照首行版本文本，没有改变布局、焦点、滚动、审批或流式渲染逻辑。
- 既有 v2.1.0 ConPTY integrated evidence：`wide_characters_visible=true`、`narrow_footer_visible=true`、`alternate_screen_restored=true`、`cursor_restored=true`；本版本 TUI 定向回归包含在 workspace test 中并通过。
- 本版本总纲不要求 TUI 重构，也没有新增微信 TUI 界面；因此没有把“尚未实现微信交互界面”误判为 v2.1.2 缺陷。

### 5. Git、文档与路径

- 工作树：干净。
- `v2.1.2`：annotated tag，指向发布提交；历史版本 tag 保留且未移动。
- tag 后两个提交：仅为发布结果和编译产物清理记录文档。
- 项目 Markdown 本地链接抽查：123 个 Markdown 文件，失效本地链接 0 个。
- `git fsck --full --no-reflogs`：未发现当前可达对象损坏；输出的 dangling 对象为历史遗留对象，未执行 gc、prune 或任何清理。

## 六、产物与清理记录

本次审核只在 `D:\YunXi Agent` 内执行读取、测试和构建。测试与 release build 会更新项目内 `target` 等编译产物，真实 Provider 会更新项目内受忽略运行状态；本次没有删除、递归清理、移动或清空任何目录，也没有触碰系统配置、PATH、注册表或用户数据。依据固定约束，未获得本次精确路径清理授权，因此保留这些产物，并在日志中记录。

## 七、允许进入 v2.1.3 的开发建议

v2.1.3 的目标应严格限定为“二维码登录与系统安全凭证存储”，不要提前实现长轮询、消息进入 Runtime、群聊或远程审批。开发报告应至少要求：

1. 在现有 `yunxi-agent-weixin` crate 内实现 QR 状态机，区分等待、已扫码、确认、过期、取消、超时和不支持平台；CLI 仍以 `weixin login` 为核心入口。
2. 凭证只进入系统安全存储或等价安全抽象，不能落入 Debug、Display、错误、JSON、Markdown 证据或普通配置文件；补充凭证不可用、存储失败、覆盖和取消恢复测试。
3. 继续固定官方 iLink endpoint，测试使用 `wiremock` 或等价维护库；参考 `D:\源码\reasonix\internal\bot\weixin\weixin_login.go` 的状态转换和超时逻辑，参考 `D:\源码\openclaw-weixin\src\auth` 的官方协议行为，但必须以 Rust 重新建模。
4. 扫码和凭证成功后仅保存账户元数据与安全引用，不在 v2.1.3 静默启动长轮询；状态迁移、账户持久化和 `doctor/status/logout` 的完整生命周期按总纲分别验收。
5. 继续运行既有 `run`、sessions、persona、memory、companion、TUI、JSON/JSONL、真实 Provider 和 ConPTY 回归，未通过统一验证前不得宣称 v2.1.3 完成。

## 八、最终意见

v2.1.2 已完成总纲图中本版本要求。可以进入 v2.1.3 的开发；后续开发者仍必须以本总纲正本为唯一功能依据，并在 v2.1.3 完成后重新提交源码、真实 Provider、CLI/TUI 回归、版本 tag 和文档状态供审核。

**署名：审核者**
