# YunXi Agent v2.1.4-hotfix.1 微信旧账户状态迁移复审审核报告

- 审核时间：2026-07-27 23:07:18 +08:00
- 审核版本：v2.1.4-hotfix.1
- 审核目录：D:/YunXi Agent
- 审核对象：v2.1.4 的整改 hotfix，仍只对照总纲 v2.1.4 要求
- 总纲正本：D:/YunXi Agent/docs/superpowers/plans/2026-07-22-yunxi-agent-v2-1-1-to-v2-2-0-personal-wechat-roadmap.md
- 总纲 SHA-256：2DF30D503F46CFE7496567F5011BF5CBFB8BA73C91C2D2FA3E9F29AF0932EA0F
- 旧版审核报告：D:/YunXi Agent/docs/reports/audits/2026-07-27-201143-yunxi-agent-v2-1-4-weixin-state-lifecycle-audit-report.md
- 整改开发报告：D:/YunXi Agent/docs/reports/development/2026-07-27-203440-yunxi-agent-v2-1-4-hotfix-1-weixin-legacy-state-migration-development-report.md
- 发布 tag：v2.1.4-hotfix.1
- 发布 tag object：9285eb9c455b82c0dcdce3b4ba95e8af0ed1aed1
- 发布 tag target：ef06b87a2e4cc1328d0ed1516d98e5e046d986b5
- 审核时 HEAD：664feb3b9be2c8a179a203e297b6e72cf7a7084b

## 一、审核结论

审核通过，允许进入总纲图中的 v2.1.5 开发。

本次只比较当前 v2.1.4-hotfix.1 与总纲 v2.1.4 的要求，不对比多个版本。上一轮 v2.1.4 的唯一 P1 阻塞点是：已有登录账户的旧 account metadata 没有初始化为新的 WeixinStateStore 状态记录。本轮已确认该路径已经补齐：

- 旧 metadata 存在、state 缺失时，status 和 doctor 都能安全初始化当前 schema。
- 已有 state 时返回 already_current，不覆盖既有 pair、pending inbound、delivery、cursor 和 last error。
- 初始化仅使用脱敏账户标识、workspace hash、官方 endpoint 和凭证引用，不读取、复制或打印 token、二维码 payload、原始用户 ID、data key 或 context token。
- 未来 schema、损坏 state、损坏 metadata 均有拒绝或安全诊断路径。
- release CLI 对真实已有账户的 status、doctor、新进程重读均为当前 schema，输出不包含秘密。

当前源码没有发现阻塞 v2.1.5 的问题。v2.1.5 的长轮询、消息入站、配对准入 dispatch 和幂等 dispatch 尚未完成，这是总纲明确安排在下一版本的范围，不属于本版本缺陷。

## 二、总纲逐项核对

| 总纲 v2.1.4 要求 | 结果 | 核验说明 |
| --- | --- | --- |
| 独立、版本化 WeixinStateStore，不修改 SessionRecord | 通过 | storage 独立保存微信状态，未向既有 SessionRecord 塞入微信字段。 |
| 临时同目录文件、flush、sync、同卷原子替换 | 通过 | FileWeixinStateStore 继续使用临时文件、flush/sync 和同卷替换；原子失败保留旧状态测试通过。 |
| 识别未完成临时文件、损坏 JSON、未来 schema | 通过 | temp_file_candidates、InvalidJson、FutureSchema 和损坏/未来 schema 测试均存在且通过。 |
| 账户、游标、回执、绑定、reply context、待投递、pair、pending inbound 状态模型 | 通过 | WeixinStateSnapshot 保留独立字段、schema version、更新时间和状态转换时间。 |
| accepted -> ready -> running -> terminal 模型 | 通过 | pending inbound 状态转换和终态不恢复测试通过；本版本不宣称已经完成长轮询 dispatch。 |
| 账户粒度跨进程锁、活动锁拒绝、陈旧锁恢复 | 通过 | storage 锁竞争、活动锁和陈旧锁探测测试通过。 |
| pair request ID 脱敏、过期、单次消费 | 通过 | pair 生命周期测试和 CLI pair 集成测试通过。 |
| status --json 和 doctor 检查 state、凭证、锁、私聊策略、脱敏错误 | 通过 | 两个命令均接入旧 metadata 初始化，真实 release 输出为当前 state、凭证 present、无秘密。 |
| logout 活动锁拒绝和指定账户删除边界 | 通过 | CLI 活动锁拒绝测试通过；本轮未执行确认 logout，避免破坏真实审核账户。 |
| 旧账户目录迁移/初始化 | 通过 | CLI 真实目录形状测试覆盖旧 metadata、缺失 state、status/doctor 初始化、幂等、未来 schema 和损坏 metadata。 |
| 不提前实现 v2.1.5 以上微信闭环 | 通过 | 当前仍明确 receive/send、persistent service、group chat 为关闭状态，未发现提前实现长轮询或 Runtime 绑定。 |

## 三、整改源码核验

### 1. CLI 迁移入口

路径：D:/YunXi Agent/crates/yunxi-agent-cli/src/weixin.rs

- 356-423 行：print_status 在读取旧账户 metadata 后调用安全初始化 helper，并在机器输出中报告 state_store_configured、schema version 和 migration 状态。
- 471-516 行：WeixinStateInitialization 和 ensure_weixin_state_initialized_from_metadata 实现 already_current 与 initialized_from_legacy_metadata 两条路径。
- 502-515 行：state 已存在时只读返回；state 缺失时调用 FileWeixinStateStore::upsert_account_state，不复制秘密字段。
- 518-606 行：print_doctor 同样触发初始化，并对 FutureSchema、InvalidJson 和 metadata 错误做结构化脱敏诊断。
- 609-654 行：损坏 metadata 只返回安全错误标签，不创建错误 state。

### 2. 独立状态存储

路径：D:/YunXi Agent/crates/yunxi-agent-storage/src/weixin_state.rs

- 15 行：WEIXIN_STATE_SCHEMA_VERSION 为 1。
- 154-173 行：upsert_account_state 在缺失时构造最小快照，在已存在时保留已有运行状态字段。
- 194-257 行：write_snapshot 执行临时同目录文件、写入、flush、sync 和原子替换。
- 402-435 行：state 目录中的未完成临时文件可被识别。
- 437-501 行：WeixinStateSnapshot 独立定义账户、workspace、endpoint、凭证引用、游标、回执、投递、绑定、pair 和 pending inbound 字段。
- 836-866 行：未来 schema 被拒绝，不会静默降级或覆盖。
- 914-978 行：快照、pair 和 pending inbound 的 schema 与账户一致性校验继续生效。

### 3. 测试核验

路径：D:/YunXi Agent/crates/yunxi-agent-cli/tests/cli_tests.rs

- 205-287 行：旧 metadata 存在、state 缺失时，首次 status 初始化当前 state，重复调用保持幂等。
- 288-335 行：doctor 初始化旧 metadata state。
- 336-368 行：未来 schema 拒绝覆盖。
- 369-402 行：损坏 metadata 不创建 state，并返回安全诊断。

路径：D:/YunXi Agent/crates/yunxi-agent-storage/tests/weixin_state_tests.rs

- 35-66 行：原子写入失败时保留旧 state。
- 68-94 行：损坏 JSON 和未来 schema 拒绝重写。
- 96-116 行：旧 state schema 的兼容迁移。
- 118-148 行：pair 过期、终态和账户归属校验。
- 150-183 行：pending inbound 状态机和终态恢复边界。

## 四、真实 release 与安全核验

本轮使用 release 构建产物独立核验。第一次旧 metadata 初始化结果由整改开发报告和 CLI 集成测试提供；本轮真实账户在 state 已生成后进行新进程重读，以确认结果不依赖进程缓存。

| 验证项 | 结果 |
| --- | --- |
| release 版本 | yunxi 2.1.4-hotfix.1 |
| release status --json | state_store_configured=true，state_store_schema_version=1，state_store_migration=already_current，credential_state=present，secrets_included=false |
| release doctor --json | state_store=current，state_store_schema_current=true，schema version=1，credentials_configured=true，network_request_performed=false，secrets_included=false |
| 新进程再次 status | 与前一次一致，仍为 current schema 和 already_current |
| 真实 state 文件 | 仅检查字段形状和非敏感计数；schema=1，包含游标、回执、投递、绑定、reply context、pair 和 pending inbound 字段，未输出正文或凭证引用 |
| CLI 迁移集成测试 | status 初始化、doctor 初始化、幂等、未来 schema、损坏 metadata 全部通过 |

status 和 doctor 的真实核验没有触发网络请求、登录、消息发送或 logout。报告只记录脱敏状态，不记录真实账户原值、系统凭证内容或本地秘密。

## 五、统一验证结果

| 验证项 | 结果 |
| --- | --- |
| cargo fmt --all -- --check | 通过 |
| cargo check --workspace | 通过 |
| cargo test --workspace -- --test-threads=1 | 通过，无失败；CLI 集成 54/54，JSONL 10/10，storage state 6/6，TUI 161/161 |
| cargo test -p yunxi-agent-storage -- --test-threads=1 | 通过；storage 22/22、state 6/6 |
| cargo test -p yunxi-agent-weixin -- --test-threads=1 | 通过；iLink 3/3、models 2/2、login/store 6/6、redaction 2/2 |
| cargo test -p yunxi-agent-cli -- --test-threads=1 | 通过；CLI 单元 23/23、兼容二进制 23/23、集成 54/54、JSONL 10/10 |
| cargo test -p yunxi-agent-provider -- --test-threads=1 | 通过；Provider 46/46 |
| cargo test -p yunxi-agent-tui -- --test-threads=1 | 通过；TUI 161/161 |
| cargo build --workspace --release | 通过 |
| eval companion --json | 通过；harness_version=2.0.6，scenario_count=31，passed=31，failed=0，golden_passed=true，memory_forbidden_writes=0，tool_approval_bypass_count=0，proactive_boundary_violation_count=0 |
| Provider smoke | 通过；项目脚本使用固定最小提示，exit_code=0，22 条 JSONL 事件，secret_leak_detected=false；仅证明既有 Provider 未回归，不代表微信真实消息联调 |
| ConPTY v210 verifier | 通过；ok=true，read_only=true，evidence SHA-256=a291a66cf91ee788bba9944edbafbf4c8dfce43bcd2d6c7b2cc78bc6c2990fc6 |
| git diff --check | 通过 |
| 默认 CLI 依赖树 | 通过；cargo tree -p yunxi-agent-cli --depth 1 未发现 yunxi-agent-codex、codex-* 或 vendor 依赖 |
| Git fsck | 退出码 0；仓库存在历史 dangling 对象，未执行 gc 或 prune |

## 六、Git 发布状态

- 本地 v2.1.4-hotfix.1 是 annotated tag，tag object 为 9285eb9c455b82c0dcdce3b4ba95e8af0ed1aed1，target 为 ef06b87a2e4cc1328d0ed1516d98e5e046d986b5。
- 历史 v2.1.4 tag object cc6e8ba3fd21311330e55bdcae120c907c3e3cdc 未移动、删除或覆盖。
- origin/master 为 664feb3b9be2c8a179a203e297b6e72cf7a7084b。
- 远端 v2.1.4-hotfix.1 tag object 与本地一致，peeled target 与本地一致。
- v2.1.4-hotfix.1 tag 指向发布提交，HEAD 只比 tag 多一条 docs-only 发布记录，未发现源码差异。
- 审核前工作树干净；本次审核新增报告和日志属于 docs-only 工作树变更，未创建审核 commit，未推送，也未创建、移动或删除任何 tag。

## 七、参考源码核对

本版本总纲明确的参考源码均已核对存在。本轮没有修改参考源码，也没有把 Go/TypeScript 代码直接复制进 Rust。

### 当前 v2.1.4 参考

- D:/源码/reasonix/internal/bot/weixin/weixin.go：账户/context 持久化、账户隔离和通道状态边界。
- D:/源码/reasonix/internal/bot/weixin/weixin_test.go：账户状态、过期、隔离和重启测试组织方式。
- D:/源码/openclaw-weixin/src/auth/accounts.ts：账户索引、凭证引用、账户删除和迁移边界。
- D:/源码/openclaw-weixin/src/auth/account-store.test.ts：账户存储、秘密不落盘和迁移测试思路。
- D:/YunXi Agent/crates/yunxi-agent-storage/src/lib.rs：既有 SessionStore 的 schema 与兼容边界。

Rust 化边界保持正确：使用 serde 数据模型、明确错误类型、原子文件写入和 Rust 测试；没有引入 Node/OpenClaw 宿主、Reasonix gateway/controller、明文凭证或第二套 Runtime。

## 八、下一版本 v2.1.5 开发建议

以下是允许进入 v2.1.5 后，开发者必须依据总纲实现的内容。它不是本次多版本对比，而是审核通过后的下一版本开发准入说明。

### 1. 必须完成的能力

1. 实现前台 yunxi weixin serve，按 iLink getupdates 长轮询契约运行，尊重 timeout hint，使用带 jitter 的有界退避，并支持 Ctrl-C 及时退出。
2. 将入站消息批次、加密 accepted/ready 队列项和 get_updates_buf 游标放入同一次原子持久化提交；dispatch 前写入 receipt 和状态转换。
3. 对同一 account + peer + dm 会话施加串行锁；同一消息 ID 不得并发执行两次；崩溃处于 Provider、工具或外部副作用边界时必须标记不确定，不能宣称恰好一次。
4. 将文本消息归一化为 WeixinInboundEnvelope，日志只保留账户和对端哈希；附件暂时明确回复不支持，不得伪造文本 prompt。
5. 默认启用 pairing；陌生私聊只生成短时、不透明 request ID 的配对请求，不创建 Session，不写入 persona 或 memory；群消息在代码和配置层关闭。
6. 增加自消息过滤、空轮询、token 过期、队列饱和、网络错误、健康状态和游标恢复处理。
7. 在 serve、pair list 和后续 pair 操作进入 state store 前，确保旧账户 metadata 已完成安全初始化，避免只依赖用户先手动执行 status 或 doctor。

### 2. 必须补齐的测试

- mock getupdates 批次中断、重复消息 ID、游标推进和崩溃重启恢复。
- 加密队列恢复、终态 receipt TTL/容量清理和 Provider 边界不确定状态。
- timeout hint、jitter 退避、token 过期、空轮询、Ctrl-C。
- 群聊拒绝、自消息过滤、陌生发送者只生成配对请求。
- 未准入消息不创建 YunXi session、不写 persona、不写 memory、不触发工具。
- 既有 CLI 取消路径和 TUI/ConPTY 回归。
- pair list 和 serve 对旧账户 metadata 自动初始化的真实目录形状测试。

### 3. 参考源码与 Rust 化建议

- D:/源码/reasonix/internal/bot/weixin/weixin.go：迁移登录后的轮询游标、自消息过滤、context token 重试和账户级状态边界；Rust 使用 tokio 任务、reqwest、显式状态机和 bounded channel。
- D:/源码/reasonix/internal/bot/weixin/weixin_test.go：迁移轮询超时、状态恢复、重复消息和账户隔离测试组织；不要照搬 Go 类型或 goroutine 结构。
- D:/源码/openclaw-weixin/src/api/api.ts：iLink getupdates、sendmessage、错误和重连行为；使用 Rust reqwest + serde 重写。
- D:/源码/openclaw-weixin/src/api/types.ts：消息、游标、context token 和接口字段；使用 Rust enum、serde 反序列化和未知字段安全策略。
- D:/源码/openclaw-weixin/src/messaging/inbound.ts：入站消息归一化、私聊/群聊边界和自消息过滤。
- D:/源码/openclaw-weixin/src/messaging/process-message.ts：消息准入和处理顺序；不得引入 OpenClaw 第二套 Agent Runtime。
- D:/源码/openclaw-weixin/src/auth/pairing.ts：配对请求、过期、白名单和 request ID 逻辑。
- D:/源码/openclaw-weixin/src/storage/sync-buf.ts：游标和同步缓冲恢复边界。
- D:/源码/openclaw-weixin/src/monitor/monitor.ts：长轮询健康、重连和退避观察方式。
- D:/YunXi Agent/crates/yunxi-agent-storage/src/weixin_state.rs：继续复用当前独立 state store、原子提交、锁和 schema，不新建第二套状态文件格式。

所有 TypeScript 和 Go 代码只能提取协议、状态机和测试思路，必须以 idiomatic Rust 重写。v2.1.5 不得提前实现 v2.1.6 Runtime 会话绑定、v2.1.7 远程审批、v2.1.8 流式回信或群聊。

## 九、审核边界与风险记录

- pair list 当前直接读取 state。对于从未触发 status/doctor 的旧账户，它不会主动完成 lazy initialization；这不构成本次 v2.1.4 阻塞点，因为总纲验收要求的迁移入口已在 status/doctor 中完成且有测试。v2.1.5 必须在 serve/pair 状态操作前统一调用初始化路径。
- 本次没有执行确认 logout、微信扫码、微信真实消息收发或长轮询；这些动作不属于 v2.1.4 当前验收范围，且不会被本报告宣称为已完成。
- TUI 视觉重构不属于当前微信路线 v2.1.4 范围；本轮使用 TUI 161/161、ConPTY 只读 verifier 和既有回归作为兼容性检查，没有把新的视觉功能写入本版本结论。
- Git fsck 的 dangling 对象是历史状态，未执行 gc/prune，也没有用删除操作掩盖任何问题。

## 十、清理、文件变更与署名

- 本轮未修改任何生产 Rust 源码、测试源码、Cargo 配置、vendor、extracted、ConPTY 脚本或正式 evidence。
- 新增项目内审核报告：D:/YunXi Agent/docs/reports/audits/2026-07-27-230718-yunxi-agent-v2-1-4-hotfix-1-weixin-state-migration-reaudit-report.md
- 追加项目内开发日志：D:/YunXi Agent/docs/development-log.md
- 待同步桌面审核报告：C:/Users/24763/Desktop/YunXi Agent审核报告/2026-07-27-230718-YunXi-Agent-v2.1.4-hotfix.1-微信状态迁移复审审核报告.md
- 待同步桌面开发日志：C:/Users/24763/Desktop/YunXi Agent开发日志.md
- 本轮已在精确路径确认后清理 D:/YunXi Agent/target；删除前存在，删除后不存在，未清理 D:/YunXi Agent/.yunxi、.tmp、用户目录、Git、正式 evidence 或 ConPTY 依赖。
- 本次审核不创建 commit、不推送、不创建或移动 tag；开发者发布 tag 和远端状态已核对。

署名：审核者
