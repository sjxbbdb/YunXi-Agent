# YunXi Agent v2.1.5-hotfix.2 微信 Windows stale lock 恢复整改开发报告

- 撰写时间：2026-07-28 16:26:30 +08:00
- 审核依据：`D:\YunXi Agent\docs\reports\audits\2026-07-28-161052-yunxi-agent-v2-1-5-hotfix-1-weixin-encrypted-pending-inbound-reaudit-report.md`
- 审核报告 SHA-256：`3563ECC466E655AF80EC96583150077C92FCD18A3DB56D4FA36F72AF799E501A`
- 总纲正本：`D:\YunXi Agent\docs\superpowers\plans\2026-07-22-yunxi-agent-v2-1-1-to-v2-2-0-personal-wechat-roadmap.md`
- 总纲 SHA-256：`2DF30D503F46CFE7496567F5011BF5CBFB8BA73C91C2D2FA3E9F29AF0932EA0F`
- 当前工作树基线：`HEAD=ddb7e5b4720c1277be4938a2a467160c48ea5581`，`git describe=v2.1.5-hotfix.1-1-gddb7e5b-dirty`
- 审核结论转化：`v2.1.5-hotfix.1` 审核不通过，禁止进入总纲图中的 `v2.1.6`。
- 整改范围：仍属于总纲 `v2.1.5` 的长轮询、崩溃恢复和状态可恢复验收。
- 整改目标版本：`v2.1.5-hotfix.2`
- 已存在且不得移动的 tag：annotated `v2.1.5-hotfix.1`，tag object `983e4ea426777a7bba9f74ba66b54506163c4049`，target `f990ba1f539d25052211fca3baf3ebc39280a720`
- 必须创建的 tag：新的 annotated `v2.1.5-hotfix.2`
- 开发目录：`D:\YunXi Agent`
- 报告类型：面向开发者的整改开发指令；本报告不是完成声明。

## 一、硬性约束

以下约束是 `v2.1.5-hotfix.2` 整改、验证、发布和文档同步的前置要求，不得因为上下文过长、任务拆分、实现困难或局部修复而忽略、弱化或绕过。

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

`v2.1.5-hotfix.1` 已完成上一轮加密 pending inbound 的 P1：当前代码已经使用 ChaCha20-Poly1305 生成密文，保存 nonce、算法版本、AAD 版本和 ciphertext，并在同一 state 快照中提交 cursor、receipt、pending inbound、pair 和 health。

本轮复审的新 P1 是 Windows 默认账户锁探测无法恢复 stale lock：真实 release CLI 中，持有锁的 `weixin serve` 进程已结束，PID 已不存在，但下一次 `weixin serve` 仍把锁判定为 active，导致服务无法重启。

必须修复：

1. Windows `OpenProcess` 失败后要读取 `GetLastError()`，区分“PID 不存在”和“权限不足/无法证明退出”。
2. 对明确 PID 不存在的情况返回 `false`，允许 `try_acquire_account_lock` 按现有逻辑回收 stale lock。
3. 对 `ERROR_ACCESS_DENIED` 或无法确定的错误继续保守返回 active，不得为了恢复方便把所有失败都当作 stale。
4. 增加真实 Windows 子进程集成测试，覆盖活动锁拒绝和已退出 PID 的 stale lock 回收。
5. 补齐独立新进程 pending 解密测试；同一测试进程重新加载 state 不能单独证明“重启恢复”。

本阶段仍禁止进入 `v2.1.6`。不得实现 `WeixinConversationBinding`、`WeixinTurnSupervisor`、Runtime dispatch、Session 复用、Provider 调用、sendmessage、远程审批、流式回信或群聊。

## 三、Windows stale lock 修复方案

### 1. 代码接入点

主要路径：`D:\YunXi Agent\crates\yunxi-agent-storage\src\weixin_state.rs`

当前问题点：

- `try_acquire_account_lock` 依赖 `process_probe(existing.pid)` 判断锁是否 active。
- Windows `default_process_is_running` 调用 `OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid)`。
- `OpenProcess` 返回空句柄时当前直接返回 `true`，没有读取错误码。

建议实现：

```text
handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid)
if handle == NULL:
  error = GetLastError()
  if error == ERROR_INVALID_PARAMETER:
    return false
  if error == ERROR_ACCESS_DENIED:
    return true
  return true
GetExitCodeProcess(handle, &exit_code)
CloseHandle(handle)
return ok && exit_code == STILL_ACTIVE
```

说明：

- `ERROR_INVALID_PARAMETER` 应视为 PID 不存在或无效，返回 `false`。
- `ERROR_ACCESS_DENIED`、未知错误、`GetExitCodeProcess` 失败应保守返回 `true`。
- `pid == std::process::id()` 仍返回 `true`。
- 非 Windows 实现可以继续保守，但不能影响 Windows 生产路径。

### 2. 不允许的绕过

- 不得手工删除 `C:\Users\24763\AppData\Local\YunXi Agent\weixin-account-locks\account-933b5bde.lock.json` 伪造通过。
- 不得把所有 `OpenProcess` 失败都视作 stale。
- 不得放宽账户锁为 workspace 锁或进程内锁。
- 不得在 `status`、`doctor`、`serve` 中静默覆盖未知锁。
- 不得要求用户手动清理锁文件作为正常恢复路径。

## 四、必须补齐的测试

### 1. Windows 真实进程锁测试

建议在 storage 或 CLI 集成测试中使用真实子进程，而不是只注入 fake probe：

- 父进程创建临时 lock root，避免触碰用户 `LOCALAPPDATA` 正式锁目录。
- 子进程使用默认生产 `process_probe` 持有账户锁并保持运行。
- 父进程尝试同账户锁，必须得到 `LockActive`。
- 子进程正常退出或被受控结束后，父进程不删除锁文件，直接再次获取同账户锁，必须回收 stale lock 并创建新锁。
- 对不同账户仍可独立持锁。

测试命令可以使用当前测试二进制或专用 test helper，通过环境变量进入 child mode。必须记录没有使用 fake process probe 覆盖 Windows 默认逻辑。

### 2. Release CLI 异常退出恢复

必须用 release `yunxi.exe` 复现并关闭 P1：

1. 启动 `weixin serve`，获取账户锁。
2. 受控结束该服务进程，模拟异常退出，不删除锁文件。
3. 只读确认旧 PID 不存在。
4. 再次启动 `YUNXI_WEIXIN_SERVE_MAX_POLLS=1 yunxi.exe weixin serve ...`。
5. 第二次启动必须回收 stale lock、完成限定轮次 `getupdates` 或给出非锁阻塞的网络诊断，并在退出后释放新锁。
6. `status` 和 `doctor` 必须能区分 active、stale/free，且输出无秘密。

### 3. 独立新进程 pending 解密

上一轮加密已经通过同进程重载，但还缺独立新进程证据。必须补齐：

- 父进程创建已准入私聊 pending inbound，写入真实密文 state。
- 子进程重新从系统凭证或测试 secret store 读取 data key。
- 子进程从 state 文件加载 pending item，调用 `decrypt_pending_inbound`。
- 子进程只输出脱敏校验结果，例如 `payload_recovered=true`、hash、item id，不输出正文。
- 错误 key、AAD 不匹配、篡改/截断 ciphertext、未知算法版本仍安全失败。

## 五、保留已通过能力

整改时必须保留 `v2.1.5-hotfix.1` 已通过的加密与轮询能力：

- ChaCha20-Poly1305 认证加密。
- 12 字节随机 nonce。
- 算法版本、AAD 版本和 ciphertext 持久化。
- account、peer、message、item AAD 绑定。
- 错误 key、AAD 变化、篡改/截断密文、未知算法版本失败。
- `getupdates` 批次、cursor、receipt、pending inbound、pair 和 health 同一 state 快照原子提交。
- data key 缺失时不发起网络请求。
- 陌生私聊只生成 pair request，不创建 pending/Runtime/session/persona/memory/tool。
- 群聊、自消息、附件和未知消息的脱敏跳过边界。

不得为了修锁回归上述能力。

## 六、参考源码状态与使用边界

审核报告提到的参考源码均已在本机存在，本轮无需新增拉取：

| 参考输入 | 使用方式 |
| --- | --- |
| `D:\源码\reasonix\internal\bot\weixin\weixin.go` | 参考轮询、重启、账户状态和异常恢复思路。 |
| `D:\源码\reasonix\internal\bot\weixin\weixin_test.go` | 参考重复消息、超时、重启、账户隔离和状态恢复测试组织。 |
| `D:\源码\openclaw-weixin\src\api\api.ts` | 参考 getupdates、超时、协议错误和重连行为。 |
| `D:\源码\openclaw-weixin\src\storage\sync-buf.ts` | 参考游标与同步缓冲恢复边界。 |
| `D:\源码\openclaw-weixin\src\storage\state-dir.ts` | 参考本地状态目录、原子状态和恢复边界。 |
| `D:\YunXi Agent\crates\yunxi-agent-storage\src\weixin_state.rs` | 当前锁、原子 state、schema 和 pending 状态机；本轮 P1 位于 Windows process probe。 |
| `D:\YunXi Agent\crates\yunxi-agent-weixin\src\payload_cipher.rs` | 认证加密、AAD、nonce、metadata 校验和解密恢复入口。 |
| `D:\YunXi Agent\crates\yunxi-agent-weixin\src\inbound.rs` | 脱敏 envelope 和最小可恢复 payload。 |
| `D:\YunXi Agent\crates\yunxi-agent-weixin\src\serve.rs` | 长轮询、加密前置、批次提交和 stale lock 复验入口。 |
| `D:\YunXi Agent\crates\yunxi-agent-storage\tests\weixin_state_tests.rs` | 原子写入、schema、密文和锁测试扩展点。 |

所有 Go 和 TypeScript 源码只能提取协议、状态机和测试思路，必须使用 Rust 2024 重新实现。不得引入 OpenClaw/Reasonix 宿主、第二套 Runtime、明文凭证落盘、群聊或后续版本功能。

## 七、统一验证与发布门禁

整改完成后至少执行：

| 类别 | 验收要求 |
| --- | --- |
| 格式与编译 | `cargo fmt --all -- --check`、`cargo check --workspace`、`cargo build --workspace --release`。 |
| 全量测试 | `cargo test --workspace -- --test-threads=1`。 |
| storage 定向 | Windows stale lock、active lock、different account、atomic state、pending decrypt、schema 和密文测试全部通过。 |
| weixin 定向 | payload cipher、serve、inbound、iLink、redaction 和 login/store 测试全部通过。 |
| CLI 定向 | release/集成 `serve` 异常退出恢复、status/doctor 锁状态、pair、JSON/JSONL 输出无秘密。 |
| 独立进程 | 子进程 stale lock 恢复和新进程 pending 解密均有证据。 |
| 回归 | Provider smoke、companion eval、TUI、ConPTY、sessions、persona、memory 均不得回归。 |
| Git | `git diff --check`、依赖树边界、`git fsck --full --no-dangling`、远端 master、tag object 和 peeled target 全部核验。 |

验证通过前不能宣称 `v2.1.5-hotfix.2` 完成，也不能进入 `v2.1.6`。

发布要求：

1. 创建新的发布 commit。
2. 创建新的 annotated `v2.1.5-hotfix.2` tag。
3. 推送 commit 和 tag。
4. 不移动、覆盖或删除 `v2.1.5-hotfix.1`、`v2.1.5` 或任何历史 tag。
5. 更新 `README.md`、`docs/README.md`、`docs/weixin.md`、报告索引、状态文档和开发日志。
6. 阶段结束后清理编译中间产物；涉及递归删除、清空目录、`git clean` 或 stale lock 精确路径删除前，必须得到用户明确确认。

`v2.1.5-hotfix.2` 复审通过前，任何文档、日志、发布说明或回复都不得宣称 Runtime 会话绑定、Provider dispatch、sendmessage、远程审批、流式回信、群聊或完整微信聊天闭环已经完成。

署名：开发报告撰写者
