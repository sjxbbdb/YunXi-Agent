# YunXi Agent v2.1.5-hotfix.2 微信 stale lock 恢复整改复审审核报告

- 审核时间：2026-07-28 18:40:05 +08:00
- 审核版本：v2.1.5-hotfix.2
- 审核目录：D:/YunXi Agent
- 审核范围：只对照总纲 v2.1.5 要求及本版本整改验收条件，不进行多个版本横向比较
- 总纲正本：D:/YunXi Agent/docs/superpowers/plans/2026-07-22-yunxi-agent-v2-1-1-to-v2-2-0-personal-wechat-roadmap.md
- 总纲 SHA-256：2DF30D503F46CFE7496567F5011BF5CBFB8BA73C91C2D2FA3E9F29AF0932EA0F
- 上一轮审核报告：D:/YunXi Agent/docs/reports/audits/2026-07-28-161052-yunxi-agent-v2-1-5-hotfix-1-weixin-encrypted-pending-inbound-reaudit-report.md
- 当前 HEAD：e4f00e1a1f4581b390957934e63d1ca877090fa4
- 审核 tag：annotated v2.1.5-hotfix.2
- tag object：da416e2afab1f34d429f460d5f3a448731d44ee7
- tag target：e4f00e1a1f4581b390957934e63d1ca877090fa4

## 一、审核结论

**审核通过，允许进入总纲图中的 v2.1.6 开发。**

本版本完成了上一轮两个整改要求：

1. Windows 默认账户锁进程探测已区分 `ERROR_INVALID_PARAMETER`、`ERROR_ACCESS_DENIED`
   和未知错误；不存在的 PID 会被视为 stale，权限不足和未知状态仍保守视为 active。
2. 新增 Windows 真实子进程锁测试和独立进程 pending inbound 解密测试，证明活动锁会拒绝、
   已退出 PID 会回收，密文可由新进程重新从系统凭证读取 data key 后恢复。

真实 release CLI 已验证：旧账户锁从 `stale` 开始，`weixin serve` 成功回收锁并完成一轮
`getupdates`，随后账户锁回到 `free`。当前源码、测试和发布边界没有发现阻塞点。

本报告只放行进入 v2.1.6 开发，不宣称 v2.1.6 的 Runtime 会话绑定已经完成。

## 二、整改源码核对

### 1. Windows stale lock

路径：`D:/YunXi Agent/crates/yunxi-agent-storage/src/weixin_state.rs`

- `try_acquire_account_lock` 在已有锁时仍先调用 `process_probe`；活动进程继续返回
  `LockActive`，没有静默覆盖锁。
- Windows `default_process_is_running` 现在读取 `GetLastError()`：
  - `ERROR_INVALID_PARAMETER` -> `false`，允许 stale lock 回收。
  - `ERROR_ACCESS_DENIED` -> `true`，保持保守 active。
  - 其他未知错误 -> `true`，不把不确定状态当成 stale。
- `GetExitCodeProcess` 失败 -> `true`；成功后只把 `STILL_ACTIVE` 判为活动。
- 非 Windows 路径未影响 Windows 生产行为。

### 2. 真实子进程锁测试

路径：`D:/YunXi Agent/crates/yunxi-agent-storage/tests/weixin_state_tests.rs`

`account_lock_real_windows_process_probe_rejects_active_and_recovers_exited_pid` 使用
`current_exe()` 启动独立测试子进程：

- 子进程持有临时 lock root 的账户锁并写入 PID。
- 父进程确认活动锁状态并验证第二次获取返回 `LockActive`。
- 受控结束子进程，不删除锁文件。
- 父进程观察锁变为 stale，并用默认生产 process probe 成功回收和重新获取锁。
- 测试使用临时目录，不触碰用户正式锁目录。

测试中的 `windows_account_lock_child_holds_lock` 被标为 ignored 是为了作为子进程入口，
由父测试显式以 `--ignored` 启动；父测试本身已执行并通过，不属于未执行的验收项。

### 3. 独立进程 pending 解密

路径：`D:/YunXi Agent/crates/yunxi-agent-weixin/tests/pending_inbound_recovery_tests.rs`

`pending_inbound_decrypts_in_independent_process_from_system_secret_store` 已完成：

- 父进程创建带真实 ciphertext、nonce、算法版本和 AAD 版本的 pending state。
- data key 写入 Windows Credential Manager 测试账户。
- 子进程重新从 Credential Manager 读取 data key，从 state 文件加载 pending，并执行
  `decrypt_pending_inbound`。
- 子进程只写入 `payload_recovered=true`、item/hash 等脱敏结果，不写正文或 data key。
- 父进程验证 state、结果文件没有原始消息、context token 或 data key。
- 测试结束删除测试账户 data key，不触碰正式微信账户凭证。

### 4. 上一轮加密能力保留

以下能力在本轮复核通过：

- `chacha20poly1305` authenticated encryption。
- 每条消息随机 12 字节 nonce。
- 算法版本、AAD 版本、nonce、ciphertext 持久化。
- account、peer、message、item AAD 绑定。
- 错误 key、AAD 修改、篡改/截断 ciphertext、未知算法和未知版本安全失败。
- cursor、receipt、pending inbound、pair 和 health 同一 state snapshot 原子提交。
- data key 缺失时在网络请求前拒绝启动。
- 陌生私聊只生成 pair request，不创建 Runtime、session、persona、memory 或 tool 输入。

## 三、总纲 v2.1.5 逐项核对

| 总纲要求 | 结果 | 审核说明 |
| --- | --- | --- |
| 前台 `yunxi weixin serve` | 通过 | release CLI 完成限定一轮，`stopped_reason=max_polls`。 |
| getupdates 长轮询、timeout 和退避 | 通过 | mock 与 release 一轮真实网络诊断均通过；网络错误有脱敏计数。 |
| 账户级跨进程锁和 stale lock 恢复 | 通过 | 真实子进程测试和 release CLI stale -> serve -> free 均通过。 |
| cursor、receipt、pending、pair、health 同一原子提交 | 通过 | storage 原子提交和失败回滚测试通过。 |
| accepted/ready pending inbound 认证加密 | 通过 | ciphertext、nonce、AAD/算法版本真实持久化并可解密。 |
| data key 系统凭证保护 | 通过 | Windows Credential Manager 读取和独立进程恢复通过。 |
| AAD 绑定与密文完整性 | 通过 | 错误 key、篡改、截断、未知版本测试通过。 |
| 重启恢复非终态 pending | 通过 | 独立子进程从系统凭证和 state 成功恢复。 |
| 陌生私聊 pairing | 通过 | pair request 脱敏、无 Runtime/session 进入。 |
| 重复 message ID 幂等 | 通过 | 重复 receipt/pending 测试通过。 |
| 群聊、自消息、附件、未知消息边界 | 通过 | 混合批次分类和群聊关闭策略通过。 |
| data key 缺失时不发起网络请求 | 通过 | 无效 key 测试确认 transport 未消费。 |
| 不提前实现 v2.1.6 及以后能力 | 通过 | 未发现 Runtime dispatch、sendmessage、远程审批、流式回信或群聊实现。 |
| CLI、Provider、companion、TUI 回归 | 通过 | workspace、真实 Provider、companion 和 ConPTY 均通过。 |

## 四、统一验证结果

### 源码与测试

- `cargo fmt --all -- --check`：通过。
- `cargo check --workspace`：通过。
- `cargo test --workspace -- --test-threads=1`：通过，无失败。
- storage Windows state 定向测试：13 passed，1 ignored（子进程入口 helper）。
- pending inbound 独立进程测试：1 passed，1 ignored（子进程入口 helper）。
- weixin crate：14 项单元测试、3 项 iLink、2 项模型、6 项登录存储、2 项脱敏和独立
  pending recovery 全部通过。
- CLI：23/23 单元、兼容二进制 23/23、集成 54/54、JSONL 10/10 全部通过。
- TUI：161/161 通过。
- `cargo build --workspace --release`：通过。

### Release、真实 Provider 和 CLI/TUI

- release `yunxi --version`：`yunxi 2.1.5-hotfix.2`。
- release `weixin status --json`：通过，最终 `account_lock_state=free`、
  `state_store_schema_version=2`、`secrets_included=false`。
- release `weixin doctor --json`：通过，接收开启、发送关闭、网络字段和秘密字段符合边界。
- release `weixin serve`：通过，`polls=1`、`stopped_reason=max_polls`、
  `network_error_count=1`、`runtime_dispatch_enabled=false`、`send_message_enabled=false`；
  退出后锁回到 free。`network_error_count=1` 是官方接口本轮网络诊断结果，不是锁或状态
  提交失败，服务没有虚报收到消息。
- `eval companion --json`：31/31，`golden_passed=true`、记忆禁止写入 0、审批绕过 0。
- Provider smoke：`exit_code=0`、61 条 JSONL、`secret_leak_detected=false`。
- ConPTY v210 只读 verifier：`ok=true`、`read_only=true`，evidence SHA-256：
  `a291a66cf91ee788bba9944edbafbf4c8dfce43bcd2d6c7b2cc78bc6c2990fc6`。

### Git 与发布完整性

- annotated `v2.1.5-hotfix.2` tag object：`da416e2afab1f34d429f460d5f3a448731d44ee7`。
- tag target：`e4f00e1a1f4581b390957934e63d1ca877090fa4`。
- 当前 HEAD 与 tag target 一致。
- 远端 `master`、远端 tag object 和 peeled target 与本地一致。
- `cargo tree -p yunxi-agent-cli --depth 1`：未发现 `yunxi-agent-codex`、`codex-*` 或
  vendor 生产依赖。
- `git diff --check`：通过。
- `git fsck --full --no-dangling`：通过。
- 历史 tag 未移动、覆盖或删除。

## 五、允许进入 v2.1.6 的开发要求

本节是给下一阶段开发者的准入边界，不代表 v2.1.6 已完成。v2.1.6 必须对照总纲实现
“微信会话绑定既有 YunXi Runtime”，范围如下：

1. 在 `D:/YunXi Agent/crates/yunxi-agent-storage/src/weixin_state.rs` 增加或完善
   `WeixinConversationBinding`，固定以 `account + peer + dm` 映射既有 `SessionId`，记录
   最后活动和来源标签；不得创建第二套 session、persona 或 memory store。
2. 在 `D:/YunXi Agent/crates/yunxi-agent-weixin` 增加 `WeixinTurnSupervisor`，只接收已
   配对且已恢复的 pending inbound，构造现有 `AgentInput::text`，调用既有
   `Agent::run_with_backend_stream`；v2.1.6 先使用测试投递 sink，不逐 token 发微信。
3. 从 `D:/YunXi Agent/crates/yunxi-agent-cli/src/main.rs` 和现有 run 路径抽取最小共享构造
   辅助，确保 Provider、模型、cwd、sandbox、审批和 companion 配置与本地 `yunxi run`
   使用同一路径；不得复制第二套 Runtime 初始化。
4. 按 `WeixinConversationKey` 串行执行并设置有界队列；同一会话不能交叉 turn 或交叉
   memory 写入，不同已准入对端可以独立运行。
5. v2.1.6 继续禁止 `/approve`、`/deny`、`/answer`、`/stop` 远程文字控制（总纲 v2.1.7），
   禁止 sendmessage 和流式回信（总纲 v2.1.8），禁止新的 companion/persona/memory 体系。

### v2.1.6 参考源码与 Rust 化建议

- `D:/YunXi Agent/crates/yunxi-agent-runtime/src/lib.rs`：复用 YunXi Runtime backend 和
  现有流式运行入口；只抽取共享构造，不复制 Runtime。
- `D:/YunXi Agent/crates/yunxi-agent-storage/src/lib.rs`：复用现有 SessionStore、SessionId
  和 session history 兼容边界。
- `D:/源码/reasonix/internal/bot/types.go`、`gateway.go`、`internal/botruntime/runtime.go`：
  参考通道消息归一化、会话串行、Session 映射和通道到 Runtime 路由；必须 Rust 化为
  明确 struct、trait、tokio task 和显式错误。
- `D:/源码/CowAgent/channel/channel_factory.py`、`channel/weixin/`：参考个人陪伴通道
  所有权、二维码恢复和通道到核心的交接；不得引入 Python Runtime 或泛化 channel factory。
- `D:/源码/openclaw-weixin/src/storage/sync-buf.ts`、`state-dir.ts`：继续参考 pending
  恢复和游标边界；不得复制 Node/OpenClaw 宿主。

v2.1.6 验收至少需要 fake backend 证明同一对端复用同一 SessionId、不同账户/对端隔离、
Provider/sandbox/approval/cwd 与本地 CLI 一致，并保持当前 release、TUI、Provider、
companion 和安全状态回归通过。

## 六、清理与安全状态

- 本轮未修改生产源码、用户正式微信 state、Credential Manager 凭证、系统配置或历史 tag。
- release build 和测试生成了精确路径 `D:/YunXi Agent/target`；按用户硬性要求，递归清理
  该路径需要对精确绝对路径的明确确认，本轮审核报告写入前未执行删除。
- 真实账户 stale lock 已由程序正常回收，release serve 退出后确认正式锁文件不存在，未手工
  删除锁文件绕过保护。
- Provider smoke 仅读取 `C:/Users/24763/Desktop/api.txt` 作为受保护凭证输入，未输出或
  写回凭证内容。

署名：审核者
