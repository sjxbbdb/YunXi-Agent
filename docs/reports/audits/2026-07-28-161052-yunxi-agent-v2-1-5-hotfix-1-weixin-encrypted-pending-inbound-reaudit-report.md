# YunXi Agent v2.1.5-hotfix.1 微信加密 pending inbound 整改复审审核报告

- 审核时间：2026-07-28 16:10:52 +08:00
- 审核版本：v2.1.5-hotfix.1
- 审核目录：D:/YunXi Agent
- 审核范围：只对照总纲 v2.1.5 要求及上一轮 P1 整改验收条件，不进行多个版本横向比较
- 总纲正本：D:/YunXi Agent/docs/superpowers/plans/2026-07-22-yunxi-agent-v2-1-1-to-v2-2-0-personal-wechat-roadmap.md
- 总纲 SHA-256：2DF30D503F46CFE7496567F5011BF5CBFB8BA73C91C2D2FA3E9F29AF0932EA0F
- 整改开发报告：D:/YunXi Agent/docs/reports/development/2026-07-28-114710-yunxi-agent-v2-1-5-hotfix-1-weixin-encrypted-pending-inbound-development-report.md
- 当前 HEAD：ddb7e5b4720c1277be4938a2a467160c48ea5581
- 审核 tag：annotated v2.1.5-hotfix.1
- tag object：983e4ea426777a7bba9f74ba66b54506163c4049
- tag target：f990ba1f539d25052211fca3baf3ebc39280a720

## 一、审核结论

**审核不通过，禁止进入总纲图中的 v2.1.6。必须继续完善 v2.1.5-hotfix.1。**

上一轮“已准入私聊正文没有真正加密持久化”的 P1 已完成整改：当前代码确实生成
ChaCha20-Poly1305 密文，保存 nonce、算法版本、AAD 版本和 ciphertext，并在同一 state
快照中提交 cursor、receipt、pending inbound、pair 和 health。可是本轮真实 release CLI
验证发现账户锁的 stale-lock 恢复在 Windows 默认路径下不可用：持有进程已经不存在，
`weixin serve` 仍报告锁 active，无法重启服务。

该问题违反总纲 v2.1.5 的账户锁、崩溃恢复和可重启运行要求，也会使上一轮已经通过的
“状态可重启恢复”在真实异常退出场景下失效。因此当前版本不能放行到 v2.1.6 Runtime
会话绑定开发。

## 二、P1 阻塞点：Windows stale lock 无法恢复

### 1. 可复现证据

本轮使用 release 二进制执行受控真实验证：

1. 启动 `D:/YunXi Agent/target/release/yunxi.exe` 的 `weixin serve`，账户为
   `account#933b5bde`。
2. 结束本轮启动的服务进程 PID 35848，模拟服务在轮询过程中异常退出；未删除锁文件。
3. 只读确认 PID 35848 已不存在。
4. 再次以正确的 `YUNXI_WEIXIN_SERVE_MAX_POLLS=1` 启动 `weixin serve`，结果为：
   `weixin serve refused to acquire account lock ... weixin account lock is active`。
5. 锁文件仍位于：
   `C:/Users/24763/AppData/Local/YunXi Agent/weixin-account-locks/account-933b5bde.lock.json`。
   文件记录 PID 为 35848、workspace 为 `workspace#73521066`；本轮没有删除或改写该文件。

### 2. 源码原因

路径：`D:/YunXi Agent/crates/yunxi-agent-storage/src/weixin_state.rs`

- `120-155` 行的 `try_acquire_account_lock` 只有在 `process_probe(existing.pid)` 返回
  false 时才会删除 stale lock 并重建锁；返回 true 就直接报告 `LockActive`。
- `1511-1528` 行的 Windows `default_process_is_running` 调用 `OpenProcess`。
- `1519-1521` 行在 `OpenProcess` 返回空句柄时直接返回 `true`，没有读取
  `GetLastError()` 区分“PID 已不存在”与“当前进程没有权限查询”。

当持有进程已经退出时，`OpenProcess` 对不存在 PID 返回空句柄；当前实现把这个明确的
“进程不存在”路径当成 active，导致 stale lock 永久阻塞下一次 `serve`。这不是仅仅因为
本轮清理不完整，而是默认生产探测逻辑无法完成总纲要求的异常退出恢复。

### 3. 影响

- `weixin serve` 正常退出时可以通过 `AccountLock::release` 清理锁，但进程崩溃、强制结束、
  系统终止或机器异常重启后，账户服务无法自动恢复。
- `status`/`doctor` 会持续把不存在持有者的锁报告为 active，用户无法通过正常 CLI 路径
  恢复服务。
- 这会放大 iLink 入站恢复风险：即使 pending inbound 密文已经安全保存在 state，负责
  继续轮询和恢复的服务也不能重新启动。

### 4. 整改要求

必须留在 v2.1.5-hotfix.1，完成以下整改后重新审核：

1. 在 Windows 原生进程探测中读取 `OpenProcess` 失败后的错误码。对明确表示 PID 不存在
   的错误返回 false；对 access denied 等无法证明进程退出的错误保持保守的 active 结果，
   不得为了恢复方便把所有失败都当成 stale。
2. 增加真实 Windows 集成测试：子进程持有账户锁并退出后，父进程使用默认生产
   `process_probe` 成功回收 stale lock；锁仍由活动进程持有时，第二进程继续拒绝。
3. 重新执行 release CLI：异常结束 `serve` 后，不删除锁文件，下一次 `serve` 必须能够
   回收 stale lock、发起限定轮次的 getupdates，并在结束后释放新锁。
4. `status` 和 `doctor` 必须分别正确报告 active、stale/free 变化；不能通过手工删除锁
   文件或修改锁 JSON 伪造通过。
5. 保留并重新通过本轮已经确认的加密验收：错误 key、AAD 变更、篡改/截断密文、未知
   算法版本、旧 pending schema、原子失败和重启恢复入口。
6. 补充真正的新进程 pending 解密测试。当前测试的“restart decrypt”在同一测试进程中
   重新从文件加载并解密，尚未形成独立进程证据；该测试缺口必须在整改复审中补齐。

整改期间不得开始 v2.1.6 的 `WeixinConversationBinding`、`WeixinTurnSupervisor`、
Runtime dispatch、Session 复用或 Provider 接入。

## 三、总纲 v2.1.5 逐项核对

| 总纲要求 | 结果 | 审核说明 |
| --- | --- | --- |
| 前台 `yunxi weixin serve` | 不通过 | 正常路径存在，但异常退出后 stale lock 无法回收，真实第二次启动被拒绝。 |
| getupdates 长轮询、timeout hint 和退避 | 通过 | 代码保留轮询、空批次 timeout、退避和取消逻辑；正确 max-polls 入口已核对。 |
| 账户级跨进程锁和陈旧锁恢复 | **不通过，P1** | `OpenProcess` 空句柄被当成 active，PID 不存在时无法恢复。 |
| cursor、receipt、pending、pair、health 同一原子提交 | 通过 | `commit_inbound_batch_with_options` 在单一 snapshot 中更新，写入失败测试保持旧快照。 |
| accepted/ready pending inbound 真实认证加密 | 通过 | `chacha20poly1305`、随机 12 字节 nonce、算法/AAD 版本和 ciphertext 均已接入。 |
| data key 由系统凭证存储保护 | 通过 | CLI 在启动前从 Windows Credential Manager 读取 data key，不写入 state 或输出。 |
| AAD 绑定 account、peer、message、item | 通过 | `WeixinPayloadAad` 重建并由 Poly1305 认证，相关变化均有失败测试。 |
| 重启后恢复非终态 pending | 部分通过 | `load_pending_inbound` 与 `decrypt_pending_inbound` 可从 state 恢复；现有自动化测试仍未 spawn 新进程。 |
| 陌生私聊 pairing | 通过 | 未准入消息只生成脱敏 pair request，不创建 pending 或 Runtime 输入。 |
| 重复 message ID 幂等 | 通过 | receipt/pending 查重，重复消息测试通过。 |
| 群聊、自消息、附件、未知消息边界 | 通过 | 混合批次和脱敏分类测试通过，群聊仍关闭。 |
| data key 缺失时不发起网络请求 | 通过 | 无效 key 在 transport 调用前失败，responses 保持未消费。 |
| 不提前进入 v2.1.6 | 通过 | 未发现 Runtime dispatch、Provider、sendmessage、远程审批或群聊实现。 |
| 现有 CLI、Provider、companion、TUI 不回归 | 通过 | workspace、Provider smoke、companion 和 ConPTY 验证均通过。 |

## 四、验证结果

### 源码与测试

- `cargo fmt --all -- --check`：通过。
- `cargo check --workspace`：通过。
- `cargo test --workspace -- --test-threads=1`：通过，无失败。
- `cargo test -p yunxi-agent-weixin -- --test-threads=1`：通过；14 项 crate 单测、3 项
  iLink client、2 项模型、6 项登录存储、2 项脱敏测试全部通过。
- `cargo test -p yunxi-agent-storage -- --test-threads=1`：通过；微信 state 12 项全部通过。
- `cargo test -p yunxi-agent-cli -- --test-threads=1`：通过；单元 23/23、兼容二进制
  23/23、集成 54/54、JSONL 10/10。
- `cargo build --workspace --release`：通过；构建版本为 `2.1.5-hotfix.1`。

### Release、真实 Provider 和 CLI/TUI

- release `yunxi --version`：代码与 Cargo 元数据为 `2.1.5-hotfix.1`；本次独立命令因
  提权审批超时未取得 stdout，版本已由 release build、status/doctor 和 tag 元数据交叉核对。
- release `weixin status --json`：通过，`state_store_schema_version=2`、
  `encrypted_pending_queue=true`、`secrets_included=false`。
- release `weixin doctor --json`：通过，`message_receive_enabled=true`、
  `message_send_enabled=false`、`network_request_performed=false`、
  `secrets_included=false`。
- release `weixin serve` 正确 max-polls 重试：**不通过**，被 stale account lock 拒绝，
  这是本报告的 P1 证据。
- `eval companion --json`：通过，31/31、golden_passed=true、memory_forbidden_writes=0、
  tool_approval_bypass_count=0、proactive_boundary_violation_count=0。
- Provider smoke：通过，`exit_code=0`、69 条 JSONL 事件、`secret_leak_detected=false`；
  凭证来自 `C:/Users/24763/Desktop/api.txt`，脚本只输出汇总。
- ConPTY v210 只读 verifier：通过，`ok=true`、`read_only=true`，evidence SHA-256 为
  `a291a66cf91ee788bba9944edbafbf4c8dfce43bcd2d6c7b2cc78bc6c2990fc6`。

### Git 与发布边界

- annotated `v2.1.5-hotfix.1` tag object：`983e4ea426777a7bba9f74ba66b54506163c4049`。
- tag target：`f990ba1f539d25052211fca3baf3ebc39280a720`。
- 远端 `master`：`ddb7e5b4720c1277be4938a2a467160c48ea5581`；远端 tag object 和 peeled
  target 与本地核对一致。
- `cargo tree -p yunxi-agent-cli --depth 1`：未发现 `yunxi-agent-codex`、`codex-*` 或
  vendor 生产依赖。
- `git diff --check`：通过。
- `git fsck --full --no-dangling`：通过。
- 当前 HEAD 相对 tag target 只包含 docs-only 发布记录；按既定审核约定，发布顺序本身不
  作为阻塞点，但源码审核包含当前 HEAD 的文档状态。

## 五、参考源码与 Rust 化边界

本版本使用或应继续参考以下路径。非 Rust 项目只抽取协议、状态机和测试思路，必须保持
Rust 2024 实现，不得复制宿主框架或密码实现：

- `D:/源码/reasonix/internal/bot/weixin/weixin.go`：轮询游标、自消息过滤、context token
  和账户状态边界。
- `D:/源码/reasonix/internal/bot/weixin/weixin_test.go`：重复消息、超时、重启、账户隔离
  和状态恢复测试组织。
- `D:/源码/openclaw-weixin/src/api/api.ts`：getupdates、超时、协议错误与重连行为。
- `D:/源码/openclaw-weixin/src/storage/sync-buf.ts`：游标与同步缓冲恢复边界。
- `D:/源码/openclaw-weixin/src/storage/state-dir.ts`：本地状态目录、原子状态和恢复边界。
- `D:/YunXi Agent/crates/yunxi-agent-storage/src/weixin_state.rs`：当前账户锁、原子 state
  store、schema migration 和 pending 状态机；本轮 P1 直接定位于其 Windows process probe。
- `D:/YunXi Agent/crates/yunxi-agent-weixin/src/payload_cipher.rs`：当前认证加密、AAD、
  nonce、metadata 校验和解密恢复入口。
- `D:/YunXi Agent/crates/yunxi-agent-weixin/src/inbound.rs`：入站哈希、脱敏 envelope 和
  最小可恢复 payload。
- `D:/YunXi Agent/crates/yunxi-agent-weixin/src/serve.rs`：长轮询、加密前置、批次提交和
  stale lock 后续复验入口。
- `D:/YunXi Agent/crates/yunxi-agent-storage/tests/weixin_state_tests.rs`：原子写入、旧
  schema 拒绝、密文字段校验和锁测试；整改时必须补默认 Windows probe 的真实进程测试。

## 六、版本准入与后续开发要求

- `v2.1.5-hotfix.1` 当前审核不通过，禁止进入 `v2.1.6`。
- 必须先修复默认 Windows process probe 的 stale lock 判断，并完成真实异常退出后恢复。
- 必须补齐独立新进程读取系统 data key、加载 state、解密 pending payload 的证据；同进程
  重载不能单独作为“重启恢复”完成证明。
- 复审通过后，才允许依据总纲进入 v2.1.6 的 Runtime 会话绑定；本报告不提前评价或放行
  v2.1.6 功能。
- 本轮没有修改生产源码、测试源码、Cargo 配置、历史 tag 或远端 refs。

## 七、清理与安全状态

- 本轮未删除、移动或清空项目文件、用户目录、Git 历史、凭证、锁文件或系统配置。
- 本轮为执行测试和 release build 生成了 `D:/YunXi Agent/target`；清理该精确目录需要
  用户对递归删除的明确确认，本报告生成前未执行清理。
- stale lock 精确路径为 `C:/Users/24763/AppData/Local/YunXi Agent/weixin-account-locks/account-933b5bde.lock.json`；该文件记录的是本轮已结束的 PID 35848，
  但本轮未擅自删除，以免绕过账户锁保护。
- Provider smoke 读取 `C:/Users/24763/Desktop/api.txt` 仅用于进程内环境变量，未输出或写回
  凭证内容；其临时输出由脚本处理。

署名：审核者
