# YunXi Agent v2.3.3-hotfix.8 权威灵魂人格重建开发日志

时间：2026-08-04 09:20:55 +08:00

## 工作目标

清除 YunXi 旧内置人格层对回复的叠加影响，使用户提供的 `soul.txt` 成为人格表达的唯一权威来源。灵魂文件正文必须保持原始字节，不修改、不总结、不转码、不归一化换行，也不得进入 Git 仓库或远端。

## 实现内容

1. 为 `PersonaProfile` 增加运行时 `authoritative_soul` 标记；该标记只允许受控的本地 soul 加载器设置，自定义 profile JSON 不能伪造权威来源。
2. `PersonaProfileStore` 读取独立 `soul.txt` 后进入权威模式，清空旧 identity、values、voice、companion style、work style、boundaries、addressing、旧 companion rules 与旧 constraints。
3. 权威模式只把 display name 与完整 soul 原文放入 persona block，不再注入旧人格层和旧陪伴规则。
4. 工具授权、安全、隐私、文件系统、网络、现实伤害和记忆真实性边界改为独立的必需策略块，不由 soul 内容改变。
5. companion runtime 在权威模式下不再读取旧 warmth/directness/initiative/humor 等 profile 风格参数；一致性 key 加入 soul 内容哈希，文件更新后会产生新的稳定人格版本标识。
6. Web 人格页识别权威模式，按 `soul.txt` 原有 6 个顶层章节生成 6 张卡片，卡片详情展示原始章节，不创建二次概括的人格层。
7. 最小结构安全预算由 1,000 提升到 1,400 字符，以容纳新增的必需安全隔离声明。
8. 工作区版本从 `2.3.3-hotfix.7` 更新为 `2.3.3-hotfix.8`。

## 灵魂文件与清理记录

- 用户源文件：`C:\Users\24763\Desktop\soul.txt`
- 当前活动文件：`C:\Users\24763\.yunxi\persona\soul.txt`
- 精确回滚备份：`C:\Users\24763\.yunxi\persona\backups\soul-before-authoritative-20260804-091315.txt`
- 文件大小：`21,957` 字节
- SHA-256：`8DC9F48053B2633BDD99E40326F1650288F6384C626F79FFB59870EF6247F074`
- 源文件、活动文件、回滚备份与运行时加载结果哈希一致
- 未删除 persona 目录、配置文件、记忆数据或其他用户目录内容
- soul 正文未写入本日志、仓库、commit、tag 或 GitHub

## 修改路径

- `D:\YunXi Agent\Cargo.toml`
- `D:\YunXi Agent\Cargo.lock`
- `D:\YunXi Agent\crates\yunxi-agent-persona\src\profile.rs`
- `D:\YunXi Agent\crates\yunxi-agent-persona\src\registry.rs`
- `D:\YunXi Agent\crates\yunxi-agent-persona\src\compiler.rs`
- `D:\YunXi Agent\crates\yunxi-agent-persona\tests\persona_tests.rs`
- `D:\YunXi Agent\crates\yunxi-agent-runtime\src\lib.rs`
- `D:\YunXi Agent\crates\yunxi-agent-cli\src\web\app.js`
- `D:\YunXi Agent\docs\reports\development\2026-08-04-092055-yunxi-agent-v2-3-3-hotfix-8-authoritative-soul-persona-rebuild-development-log.md`

## 测试与验证

- `node --check crates/yunxi-agent-cli/src/web/app.js`：通过
- `cargo fmt --all -- --check`：通过
- `cargo test -p yunxi-agent-persona`：通过，新增权威来源、防伪造、旧层清空和完整 soul 注入覆盖
- `cargo test -p yunxi-agent-runtime -p yunxi-agent-cli`：通过
- `cargo test`：全仓通过；保留原有 1 个 Windows 条件性 ignored 测试
- `cargo build -p yunxi-agent-cli --bins --release`：通过
- 初次测试发现 1,000 字符最小安全预算不足；提高到 1,400 后相关回归与全仓测试均通过
- 实机 profile：`authoritative_soul=true`
- 实机旧层：7 个旧 layer 全空，旧 soul signature 为空，旧 rule 数量 0，旧 constraint 数量 0
- 实机原文：加载 `21,957` 字节，哈希与活动文件一致，识别 6 个原始顶层章节
- 真实模型冒烟：请求完成、返回非空、识别身份信号、无 persona/companion 内部标签泄露

## 安装与运行

- 正式安装目录：`D:\Apps\YunXi Agent\bin`
- 最终安装前备份：`D:\Apps\YunXi Agent\bin\backup-20260804-092002-authoritative-soul-source-guard`
- 最终 `yunxi.exe` SHA-256：`CEBF9B2E9FBB78E0E348B0CAB139B8CC3D7F4E3A837CC44E413B6485D2AC1771`
- 最终 `yunxi-agent-cli.exe` SHA-256：`2023A3E95F70CCF91F13029B73BABC58F40C60787586928338EBD9649ACE1D38`
- Web：PID `33512`，`status=ok`，版本 `2.3.3-hotfix.8`
- 微信：PID `28032`，由 Web 自动拉起，`state=ready`、账户锁 `active`、三类待处理队列均为 0、版本 `2.3.3-hotfix.8`
- 未修改 C 盘旧兼容安装、PATH、注册表或其他系统配置

## 安全与发布约束

未执行递归清理、目录级移动、用户目录删除、`git clean`、force push、历史 tag 删除、移动或覆盖。人格文件只影响回复表达，不改变工具审批、sandbox、隐私、安全或用户授权边界。本版本准备发布为新 tag `v2.3.3-hotfix.8`。

署名：开发者
