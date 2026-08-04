# YunXi Agent v2.3.3-hotfix.7 独立灵魂文件接入开发日志

时间：2026-08-04 08:36:13 +08:00

## 工作目标

将用户提供的灵魂文件原模原样写入 YunXi 的本地人格存储，并让 CLI、TUI、Web 与微信运行时通过统一人格加载链路使用同一份内容。灵魂正文不得修改、总结、重排、转码或换行归一化，也不得提交到 GitHub。

## 实现内容

1. 在 `crates/yunxi-agent-persona/src/registry.rs` 增加独立灵魂文件路径 `persona/soul.txt`，由 `PersonaProfileStore` 统一加载并覆盖当前 profile 的 soul 层。
2. 独立灵魂文件仅进行只读 UTF-8 与 128 KiB 容量校验；加载过程不写回文件，不改变原始字节。
3. 在 `crates/yunxi-agent-persona/src/compiler.rs` 增加按大型 soul 正文长度计算上下文预算的入口，避免完整灵魂正文被原有 3,200 字符预算直接裁掉。
4. 在 `crates/yunxi-agent-runtime/src/lib.rs` 让统一运行时使用 profile 感知的编译预算；CLI、TUI、Web 和微信仍共享同一条 runtime/persona 链路。
5. 增加回归测试，覆盖 CRLF/LF 原样读取、特殊字符保持、源文件不被写回、大型 soul 完整进入共享人格上下文。
6. 工作区版本由 `2.3.3-hotfix.6` 更新为 `2.3.3-hotfix.7`。

## 本地灵魂文件

- 用户源文件：`C:\Users\24763\Desktop\soul.txt`
- YunXi 存储文件：`C:\Users\24763\.yunxi\persona\soul.txt`
- 文件大小：`21,957` 字节
- SHA-256：`8DC9F48053B2633BDD99E40326F1650288F6384C626F79FFB59870EF6247F074`
- 写入方式：使用明确源/目标路径执行禁止覆盖的字节级文件复制
- 校验结果：源文件、目标文件与运行中 `/api/persona` 加载结果三者大小及 SHA-256 完全一致
- Git 状态：灵魂正文文件位于用户本地 YunXi 数据目录，不进入仓库、commit、tag 或远端

## 修改路径

- `D:\YunXi Agent\Cargo.toml`
- `D:\YunXi Agent\Cargo.lock`
- `D:\YunXi Agent\crates\yunxi-agent-persona\src\compiler.rs`
- `D:\YunXi Agent\crates\yunxi-agent-persona\src\registry.rs`
- `D:\YunXi Agent\crates\yunxi-agent-persona\tests\persona_tests.rs`
- `D:\YunXi Agent\crates\yunxi-agent-runtime\src\lib.rs`
- `D:\YunXi Agent\docs\reports\development\2026-08-04-083613-yunxi-agent-v2-3-3-hotfix-7-standalone-soul-file-integration-development-log.md`

## 验证结果

- 源文件为无 BOM 的有效 UTF-8：通过
- `cargo fmt --all -- --check`：通过
- `cargo test -p yunxi-agent-persona`：通过
- `cargo check --workspace`：通过
- `cargo test`：全仓通过；保留原有 1 个 Windows 条件性 ignored 测试
- `cargo build -p yunxi-agent-cli --bins --release`：通过
- release 双二进制版本：`yunxi 2.3.3-hotfix.7`
- release `yunxi.exe` SHA-256：`334BE9B850E6A1BA6E1FA15718D9CA468DF144A6F641B7788C5DE4E3847798DE`
- release `yunxi-agent-cli.exe` SHA-256：`BACD548001624F9A2C400D9B0225A6B0901C0B59CC9DE7592E4CFE236C53E4E3`
- Web 健康检查：`status=ok`，`version=2.3.3-hotfix.7`
- Web 实机加载：`persona_enabled=true`，active profile 为 `yunxi_companion_strong`，加载 soul 为 `21,957` 字节且哈希完全匹配
- 微信服务：由 Web 自动拉起，`state=ready`、`account_lock_state=active`、待处理入站/发送/远程控制队列均为 0、版本为 `2.3.3-hotfix.7`

## 安装记录

- 安装目录：`D:\Apps\YunXi Agent\bin`
- 安装备份：`D:\Apps\YunXi Agent\bin\backup-20260804-083452-standalone-soul-integration`
- 精确停止旧进程：PID `21684`（Web）、PID `26124`（微信）
- 新 Web 进程：PID `25184`
- 新微信进程：PID `24992`
- 安装源与目标双二进制 SHA-256 均完全一致
- 未修改 C 盘旧兼容安装，未修改 PATH、注册表或其他系统配置

## 安全与发布约束

未执行递归清理、目录移动、用户目录删除、`git clean`、force push、历史 tag 删除、移动或覆盖。历史 tag 全部保留。

## GitHub 发布结果

- 发布 commit：`33cdf900b76716d038cfe8c9bef42a1b4ea619c4`
- annotated tag：`v2.3.3-hotfix.7`
- tag target：`33cdf900b76716d038cfe8c9bef42a1b4ea619c4`
- 推送方式：GitHub CLI 当前认证账号配置 Git 凭据后执行普通 push
- 远端仓库：`https://github.com/sjxbbdb/YunXi-Agent`
- 远端 `master` 与新 tag 均推送成功；未使用 force，未修改任何历史 tag
- 本节作为 tag 后 docs-only 收口记录，不移动 `v2.3.3-hotfix.7`

署名：开发者
