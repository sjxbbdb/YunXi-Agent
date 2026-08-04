# YunXi Agent v2.3.3-hotfix.9 陪伴邮箱与情书生成开发日志

时间：2026-08-04 18:13:33 +08:00

## 工作目标

在不改变现有聊天、工具审批、人格、长期记忆、TUI、Web 与微信运行边界的前提下，为陪伴层增加后端邮箱，并根据权威人格和有效长期记忆不定期生成情书。当前版本只实现后端生成、加密存储、恢复和读取契约，不增加前端邮箱页面或投递界面。

## 实现内容

1. 新增独立 `LoveLetterSettings`，与普通主动陪伴限频分离；默认关闭，支持持久化 `love_letters_enabled` 和 `YUNXI_LOVE_LETTERS_ENABLED` 进程级覆盖。
2. 新增 `LoveLetterMemorySelector`，只选择可召回的 active 记忆，排除 pending、rejected、archived、过期、失效、高敏感、秘密特征、工具轨迹和不适合情书的工程上下文。
3. 新增确定性资格策略，要求人格、记忆和陪伴均启用，关系阶段至少为 familiar，默认至少 3 条有效记忆、1 条新记忆，并使用可复现的 3～10 天冷却和每日独立上限。
4. 新增情书任务状态与邮箱状态双状态机；生成使用 `pending/generating/ready/failed`，邮箱使用 `unread/read/archived`，避免把生成完成与用户已读混为同一状态。
5. 新增 `FileCompanionMailboxStore`，提供幂等入队、任务租约、过期恢复、失败退避、崩溃对账、稳定游标分页、读取、已读、未读、归档和未读计数。
6. 情书主题与正文采用 ChaCha20-Poly1305 加密落盘；Windows 数据密钥保存在系统凭据管理器。任务日志和邮箱元数据只记录哈希、引用、时间、耗时、次数和脱敏错误标签。
7. 正常回复成功、记忆管线完成且会话落盘后才进行资格检查和持久化入队；模型生成在后台运行，不阻塞当前聊天。CLI 提前退出时任务保留，并在租约到期后的后续运行中恢复。
8. 情书生成器重新加载当前活动人格，校验 profile ID 和一致性 key，将权威 `soul.txt` 原文作为人格指令，将筛选记忆作为 `context_not_instruction` 事实数据；人格或记忆版本变化时拒绝旧任务。
9. 情书模型调用不注册任何工具；模型返回工具调用时直接拒绝。输出必须是结构化 JSON，并受标题、正文长度和生成超时约束。
10. 生成的情书不进入记忆提取器，不写回长期记忆，避免形成“记忆 → 情书 → 情书再次成为记忆”的循环。
11. 修正 `yunxi_no_tui_keeps_plain_interactive_mode` 测试，使其显式禁用微信自动启动；该修正只隔离测试环境，不改变产品默认同步拉起微信服务的行为。
12. 工作区版本从 `2.3.3-hotfix.8` 更新为 `2.3.3-hotfix.9`。

## 数据目录与后续接入

- 任务元数据：`<workspace>\.yunxi\companion-mailbox\tasks.jsonl`
- 邮箱元数据：`<workspace>\.yunxi\companion-mailbox\items.jsonl`
- 加密正文：`<workspace>\.yunxi\companion-mailbox\content\*.json`
- 后端读取门面：`FileCompanionMailboxStore::list/get/mark_state/unread_count`
- 协议文档：`D:\YunXi Agent\docs\protocol\companion-mailbox.md`

后续 Web、TUI 与微信应读取同一邮箱门面，不直接依赖情书生成器。

## 实机验证

- 使用真实 `deepseek/deepseek-v4-flash` Provider。
- 主聊天：`completed`，耗时 `2,022ms`，回复非空。
- 情书任务：`Ready`；邮箱状态：`Unread`。
- 生成尝试：1 次；后台生成耗时：`12,882ms`。
- 情书主题 11 字符，正文 283 字符；通过正式邮箱 API 成功解密，未在日志中输出正文。
- 加密算法：`chacha20poly1305`，算法版本 1。
- 邮箱目录中 `[live-test]` 明文匹配数：0。
- 生成前后隔离记忆记录数均为 3，确认情书没有回写长期记忆。
- 实机灵魂文件使用只读隔离副本，SHA-256 与活动 `soul.txt` 一致：`8DC9F48053B2633BDD99E40326F1650288F6384C626F79FFB59870EF6247F074`。
- 第一轮探针验证默认冷却门禁生效；两条误路由到全局作用域的测试记录已通过正式记忆 API 精确归档，最终状态均为 `archived/not_recallable`。未物理重写用户记忆 JSONL。
- 临时服务器、端口、隔离目录和探针源码均已清理。

## 修改路径

- `D:\YunXi Agent\Cargo.toml`
- `D:\YunXi Agent\Cargo.lock`
- `D:\YunXi Agent\crates\yunxi-agent-core\src\config.rs`
- `D:\YunXi Agent\crates\yunxi-agent-core\src\lib.rs`
- `D:\YunXi Agent\crates\yunxi-agent-core\tests\config_tests.rs`
- `D:\YunXi Agent\crates\yunxi-agent-persona\src\settings.rs`
- `D:\YunXi Agent\crates\yunxi-agent-companion\Cargo.toml`
- `D:\YunXi Agent\crates\yunxi-agent-companion\src\lib.rs`
- `D:\YunXi Agent\crates\yunxi-agent-companion\src\love_letter.rs`
- `D:\YunXi Agent\crates\yunxi-agent-companion\src\mailbox.rs`
- `D:\YunXi Agent\crates\yunxi-agent-storage\Cargo.toml`
- `D:\YunXi Agent\crates\yunxi-agent-storage\src\lib.rs`
- `D:\YunXi Agent\crates\yunxi-agent-storage\src\companion_mailbox.rs`
- `D:\YunXi Agent\crates\yunxi-agent-runtime\src\lib.rs`
- `D:\YunXi Agent\crates\yunxi-agent-runtime\src\love_letter.rs`
- `D:\YunXi Agent\crates\yunxi-agent-runtime\tests\general_companion_tests.rs`
- `D:\YunXi Agent\crates\yunxi-agent-cli\src\main.rs`
- `D:\YunXi Agent\crates\yunxi-agent-cli\tests\cli_tests.rs`
- `D:\YunXi Agent\crates\yunxi-agent-weixin\tests\turn_supervisor_tests.rs`
- `D:\YunXi Agent\docs\protocol\companion-mailbox.md`
- `D:\YunXi Agent\docs\reports\development\2026-08-04-181333-yunxi-agent-v2-3-3-hotfix-9-companion-mailbox-love-letter-development-log.md`

## 测试与审核

- `cargo fmt --all -- --check`：通过。
- `cargo test -p yunxi-agent-companion`：通过。
- `cargo test -p yunxi-agent-storage`：通过，覆盖加密、幂等、租约、退避、崩溃对账和邮箱状态。
- `cargo test -p yunxi-agent-runtime --test general_companion_tests`：通过，覆盖异步入箱、解密读取和禁止记忆回流。
- `cargo test`：全仓通过；保留原有 Windows 条件性 ignored 测试。
- 受影响 companion、storage、runtime 新代码 Clippy 检查通过；严格全仓 Clippy 仍受未修改历史文件中的既有告警阻挡。
- `cargo build -p yunxi-agent-cli --bins --release`：通过。

## 安装

- 安装目录：`D:\Apps\YunXi Agent\bin`
- 回滚备份：`D:\Apps\YunXi Agent\bin\backup-20260804-181240-companion-mailbox-love-letter`
- `yunxi.exe` 版本：`2.3.3-hotfix.9`
- `yunxi.exe` SHA-256：`96717249098497F7D28F423D44161B44820B30794B7EBDDD116C7278ECD0F86A`
- `yunxi-agent-cli.exe` 版本：`2.3.3-hotfix.9`
- `yunxi-agent-cli.exe` SHA-256：`FF4A8346088E7D5AD02361CB1ACCBF510D1E326D3991CDC6DDDCD77A7F004C03`
- 安装后离线运行时冒烟：`status=completed`。
- 未修改 PATH、注册表、C 盘旧兼容安装或其他系统配置。

## 安全与发布约束

未执行 force push、历史 tag 删除、移动或覆盖；未执行 `git clean`、工作区重置或用户目录递归删除。情书功能默认关闭，只有显式启用后才会生成私人内容。本版本发布为新 tag `v2.3.3-hotfix.9`，保留全部历史 tag 以供回滚。

署名：开发者
