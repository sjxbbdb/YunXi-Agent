# YunXi Agent v2.1.5-hotfix.1 微信加密 pending inbound 整改开发报告

- 撰写时间：2026-07-28 11:47:10 +08:00
- 审核依据：`D:\YunXi Agent\docs\reports\audits\2026-07-28-100654-yunxi-agent-v2-1-5-weixin-long-polling-pairing-idempotency-audit-report.md`
- 审核报告 SHA-256：`948C9F265ED26E7C91936F3F044CC686768987A034F3F7253C7AD03C56AD22CD`
- 总纲正本：`D:\YunXi Agent\docs\superpowers\plans\2026-07-22-yunxi-agent-v2-1-1-to-v2-2-0-personal-wechat-roadmap.md`
- 总纲 SHA-256：`2DF30D503F46CFE7496567F5011BF5CBFB8BA73C91C2D2FA3E9F29AF0932EA0F`
- 当前工作树基线：`HEAD=a106684a32b423b252040b79fb5c0057c370937c`，`git describe=v2.1.5-1-ga106684-dirty`
- 审核结论转化：`v2.1.5` 审核不通过，禁止进入总纲图中的 `v2.1.6`。
- 整改目标版本：`v2.1.5-hotfix.1`
- 已存在且不得移动的 tag：annotated `v2.1.5`，tag object `03d9bd6d7985dddb7719da6ae0cd3d25a717dcb9`，target `44d89488d421748527547f52faa73caad84ef6b2`
- 必须创建的 tag：新的 annotated `v2.1.5-hotfix.1`
- 开发目录：`D:\YunXi Agent`
- 报告类型：面向开发者的整改开发指令；本报告不是完成声明。

## 一、硬性约束

以下约束是 `v2.1.5-hotfix.1` 整改、验证、发布和文档同步的前置要求，不得因为上下文过长、任务拆分、实现困难或局部修复而忽略、弱化或绕过。

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

`v2.1.5` 已实现前台 `getupdates` 轮询、账户锁、游标/receipt/pair 批次提交、陌生发送者配对和重复消息幂等，但审核发现唯一 P1：

> 已准入私聊消息没有被真正加密和持久化。`pending inbound` 只保存一个引用字符串，当前进程结束后无法恢复消息正文。

当前实现的具体问题：

- `run_serve` 只读取 data key 判断“加密队列可用”，没有把 data key 传入消息加密流程。
- `WeixinInboundEnvelope::encrypted_payload_ref` 是由哈希拼接的脱敏引用，不是 ciphertext。
- `WeixinInboundCommitItem` 和 `WeixinPendingInbound` 没有 ciphertext、nonce、AAD、算法版本或密文版本字段。
- `WeixinStateStore` 只保存引用和哈希，重启后无法解密、还原已准入消息。

因此必须继续留在 `v2.1.5`，完成 `v2.1.5-hotfix.1` 后重新审核。不得以“有 data key”“有 encrypted_payload_ref”或“pending count 增加”替代真正的认证加密和恢复测试。

本阶段只修复加密 pending inbound 的数据流，不进入 `v2.1.6` Runtime 会话绑定，不调用 Provider，不创建 YunXi session，不执行工具。

## 三、必须实现的技术方案

### 1. 认证加密边界

在 `crates/yunxi-agent-weixin` 或清晰的 storage facade 中增加 Rust 原生 payload 加密抽象：

```text
WeixinPayloadCipher
  encrypt(data_key, plaintext, aad) -> EncryptedPendingPayload
  decrypt(data_key, payload, aad) -> plaintext
```

要求：

- 不手写密码算法。
- 从维护良好的 Rust authenticated encryption crate 中选择一种算法，例如 `aes-gcm` 或 `chacha20poly1305`，完成依赖审查后锁定版本。
- 算法名称、算法版本、nonce 长度、AAD 版本必须进入状态 schema。
- 生产环境使用随机 nonce；测试可以使用固定向量，但不得把固定 nonce 带入生产路径。
- data key 只能从现有系统凭证存储读取，使用范围限于当前进程内，不能写入 state、日志、JSON、错误或临时明文文件。
- 不得把原始正文交给 `Debug`、`Display`、serde 普通字段、错误文本或 CLI 输出。

推荐密文记录字段：

```text
EncryptedPendingPayload {
  algorithm: "chacha20-poly1305" | "aes-256-gcm",
  algorithm_version: u32,
  aad_version: u32,
  nonce: base64,
  ciphertext: base64,
  ciphertext_ref: redacted reference,
}
```

实际字段名称可以按现有 Rust 风格调整，但必须存在真正的 ciphertext，而不是只有引用。

### 2. AAD 绑定

每条已准入消息的 AAD 至少绑定以下稳定、脱敏的标识：

- account hash。
- peer hash。
- message hash。
- item id。
- AAD/schema version。

解密时必须重新构造完全相同的 AAD。以下情况必须失败：

- 换账户。
- 换对端。
- 换消息 ID。
- 换 item ID。
- AAD 版本未知。
- 算法版本未知。
- AAD 被篡改。

AAD 中不得放入 token、context token、原始 user ID、原始 peer ID 或原始消息正文。

### 3. pending inbound 数据结构

扩展以下结构：

- `D:\YunXi Agent\crates\yunxi-agent-storage\src\weixin_state.rs` 中的 `WeixinPendingInbound`。
- `D:\YunXi Agent\crates\yunxi-agent-storage\src\weixin_state.rs` 中的 `WeixinInboundCommitItem`。
- `D:\YunXi Agent\crates\yunxi-agent-weixin\src\inbound.rs` 中的入站 envelope 或其安全转换层。

必须保存：

- `item_id`。
- `message_id_hash`。
- `peer_id_hash`。
- `encrypted_payload_ref`。
- 加密算法与版本。
- AAD 版本。
- nonce。
- ciphertext。
- 状态和状态转换时间。
- 可选的 `payload_kind`，例如 text envelope、unsupported attachment marker。

必须禁止：

- state 文件中出现原文。
- state 文件中出现 token、context token、data key、原始联系人标识或完整消息 API payload。
- 仅保存哈希而不保存密文。

### 4. 批次原子提交

加密必须发生在 `getupdates` 批次进入 state store 之前：

1. 读取当前 data key。
2. 对每个已准入私聊构造最小可恢复 envelope 或正文。
3. 为每条消息生成随机 nonce。
4. 使用 account/peer/message/item AAD 执行认证加密。
5. 校验 ciphertext、nonce、算法和 AAD 元数据。
6. 调用现有 `commit_inbound_batch`，同一次原子提交写入：
   - `get_updates_buf`。
   - inbound receipt。
   - encrypted pending inbound。
   - pair/skip/health 计数。
   - 状态转换时间。

任一消息加密失败、data key 缺失、密文校验失败或 state 写入失败时：

- 游标不得推进。
- receipt 不得部分提交。
- pending inbound 不得部分提交。
- pair 和 health 快照保持旧状态，或只写入明确允许的脱敏失败状态。
- 不得留下半成品密文或临时明文。

### 5. 重启恢复 API

为后续 `v2.1.6` Runtime 提供最小解密入口，但本阶段不得调用 Runtime：

```text
load_pending_inbound(account_id, item_id)
decrypt_pending_inbound(account_id, item_id, data_key)
```

恢复流程必须：

- 新进程重新从系统凭证存储读取同一 data key。
- 从 state 文件加载 ciphertext、nonce、算法和 AAD 版本。
- 根据持久化哈希和 item id 重建 AAD。
- 成功解密并得到与提交前一致的 envelope/body。
- 错误 key、篡改 ciphertext、截断 ciphertext、未知算法和 AAD 不匹配安全失败。
- 不把恢复出的正文写入日志、状态快照或普通错误。

## 四、代码接入点

| 路径 | 当前职责 | 整改要求 |
| --- | --- | --- |
| `D:\YunXi Agent\crates\yunxi-agent-cli\src\weixin.rs` | `run_serve` 读取 data key、启动轮询和提交批次。 | 把 data key 交给加密 facade；禁止只做启动检查；加密失败时阻止网络批次提交。 |
| `D:\YunXi Agent\crates\yunxi-agent-weixin\src\inbound.rs` | 入站 envelope、哈希、peer/message 分类和脱敏。 | 保留脱敏外壳，增加最小可恢复 payload 构造，不让原文进入普通状态结构。 |
| `D:\YunXi Agent\crates\yunxi-agent-weixin\src\secret_store.rs` | 系统凭证、data key 读取和删除。 | 增加或配合 payload cipher 的安全读取边界；不把密钥暴露到 Debug/Display。 |
| `D:\YunXi Agent\crates\yunxi-agent-storage\src\weixin_state.rs` | state snapshot、atomic commit、pending inbound、receipt、pair。 | 保存真实密文及加密元数据；增加加载校验和批次原子提交约束。 |
| `D:\YunXi Agent\crates\yunxi-agent-weixin\src\serve.rs` 或轮询模块 | `getupdates` 长轮询与退避。 | 在网络请求前确保 secure store 和 cipher 可用；将加密后的 commit item 交给 storage。 |
| `D:\YunXi Agent\crates\yunxi-agent-storage\tests\weixin_state_tests.rs` | state store 测试。 | 覆盖密文字段、schema、原子失败、篡改、截断和恢复。 |
| `D:\YunXi Agent\crates\yunxi-agent-weixin\src\inbound.rs` 测试 | 入站分类和脱敏测试。 | 覆盖 text、attachment、group、self、unknown 与加密 payload 对应关系。 |
| `D:\YunXi Agent\crates\yunxi-agent-cli\tests\cli_tests.rs` | CLI serve/pair/status 集成测试。 | 覆盖 data key 缺失时无网络启动、混合批次、重启新进程读取。 |
| `D:\YunXi Agent\docs\weixin.md` | 微信能力边界说明。 | 修正“加密 pending inbound 已完成”的不准确表述，整改通过后再写入真实密文恢复结果。 |
| `D:\YunXi Agent\README.md`、`D:\YunXi Agent\docs\README.md` | 项目入口文档。 | 与实际状态保持一致，不能把未通过的加密能力写成完成。 |

## 五、测试与验收要求

开发完成后至少通过：

| 类别 | 验收要求 |
| --- | --- |
| 加密正确性 | approved private text 加密后写入 state，state、stdout、stderr、JSON、日志和错误中均不存在原文。 |
| 重启恢复 | 新进程从系统凭证读取同一 data key，成功解密相同 envelope/body，不依赖内存缓存。 |
| nonce | 每条消息使用随机 nonce；重复加密相同正文不能复用 nonce。 |
| AAD | account、peer、message、item 任一变化都导致解密失败。 |
| 密文完整性 | 错误 key、篡改 ciphertext、截断 ciphertext、未知算法版本、未知 AAD 版本均安全失败。 |
| 原子失败 | 注入加密失败或 state 写入失败时，cursor、receipt、pending、pair 和 health 不产生半提交。 |
| 启动安全 | data key 缺失、secure store 不可用或 cipher 初始化失败时，`serve` 在发起 `getupdates` 前拒绝启动。 |
| 混合批次 | 重复 ID、陌生 peer、已准入 peer、群消息、自消息和附件同时出现时，密文、skip、pair 和 receipt 计数正确。 |
| 状态恢复 | pending item 保留正确状态，非终态可恢复，终态不重复执行，TTL/容量清理不泄露原文。 |
| 回归 | `cargo fmt --all -- --check`、`cargo check --workspace`、`cargo test --workspace -- --test-threads=1`、storage/weixin/CLI/provider/TUI 定向测试、companion eval、ConPTY、Provider smoke 和 `git diff --check` 全部通过。 |

验证通过前不能宣称 `v2.1.5-hotfix.1` 完成，也不能进入 `v2.1.6`。

## 六、参考源码状态与使用边界

审核报告提到的参考源码均已在本机存在，本轮无需新增拉取：

| 参考输入 | 使用方式 |
| --- | --- |
| `D:\源码\reasonix\internal\bot\weixin\weixin.go` | 参考轮询游标、自消息过滤、context token 和账户状态边界。 |
| `D:\源码\reasonix\internal\bot\weixin\weixin_test.go` | 参考重复消息、超时、重启、账户隔离和状态恢复测试组织。 |
| `D:\源码\openclaw-weixin\src\api\api.ts` | 参考 getupdates、协议错误、重连和 timeout 行为。 |
| `D:\源码\openclaw-weixin\src\api\types.ts` | 参考消息、游标、context token 和未知字段安全反序列化。 |
| `D:\源码\openclaw-weixin\src\messaging\inbound.ts` | 参考私聊、群聊、自消息和附件分类边界。 |
| `D:\源码\openclaw-weixin\src\auth\pairing.ts` | 参考 pair request 过期、白名单和 request ID。 |
| `D:\源码\openclaw-weixin\src\storage\sync-buf.ts` | 参考游标及同步缓冲恢复边界。 |
| `D:\源码\openclaw-weixin\src\storage\state-dir.ts` | 参考本地状态目录和持久化边界。 |
| `D:\YunXi Agent\crates\yunxi-agent-weixin\src\secret_store.rs` | 复用现有 Windows Credential Manager data key 入口，不能明文降级。 |
| `D:\YunXi Agent\crates\yunxi-agent-weixin\src\inbound.rs` | 复用当前 envelope 和脱敏边界，补齐可恢复密文。 |
| `D:\YunXi Agent\crates\yunxi-agent-storage\src\weixin_state.rs` | 复用原子 state store、receipt、pair、pending 状态机和 schema 校验。 |

所有 Go 和 TypeScript 源码只能提取协议、状态机和测试思路，必须使用 Rust 2024 重新实现。不得引入 OpenClaw/Reasonix 宿主、手写密码算法、明文凭证落盘、第二套 Runtime、群聊或后续版本功能。

## 七、发布、清理与日志要求

整改完成后必须：

1. 先完成一批实现，再统一执行格式、编译、测试和安全验证。
2. 验证通过后更新 `README.md`、`docs/README.md`、`docs/weixin.md`、报告索引、状态文档和开发日志。
3. 每个阶段结束后清理编译中间产物；涉及递归删除、清空目录或 `git clean` 前必须得到用户对精确绝对路径的确认。
4. 创建新的发布 commit。
5. 创建新的 annotated `v2.1.5-hotfix.1` tag。
6. 重新核验并推送 commit、tag、远端 master、tag object 和 peeled target；上一轮 `v2.1.5` 审核明确记录远端 refs 未能独立复核，本次必须补齐。
7. 不移动、覆盖或删除 `v2.1.5` 或任何历史 tag。
8. 日志必须记录时间戳、工作目标、执行流程、修改文件、文件路径、验证结果、提交和推送状态，并以“开发报告撰写者”署名。

`v2.1.5-hotfix.1` 复审通过前，任何文档、日志、发布说明或回复都不得宣称 Runtime 会话绑定、Provider dispatch、sendmessage、远程审批、流式回信、群聊或完整微信聊天闭环已经完成。

署名：开发报告撰写者

## 八、实际完成与发布记录

时间戳：2026-07-28 14:43:16 +08:00

本次已完成的实际改动：

- `D:\YunXi Agent\crates\yunxi-agent-weixin\src\payload_cipher.rs`：实现 `WeixinPayloadCipher`，用 `chacha20poly1305` 做 authenticated encryption，加入随机 nonce、AAD、算法版本和解密校验。
- `D:\YunXi Agent\crates\yunxi-agent-storage\src\weixin_state.rs`：将 pending inbound 升级为 schema v2，持久化真实密文、nonce、算法版本和 AAD 版本，并增加恢复加载与校验。
- `D:\YunXi Agent\crates\yunxi-agent-weixin\src\serve.rs` 与 `D:\YunXi Agent\crates\yunxi-agent-cli\src\weixin.rs`：把 data key 接入加密提交流程，确保加密失败时不推进游标、不半提交。
- `D:\YunXi Agent\crates\yunxi-agent-weixin\src\inbound.rs`、`D:\YunXi Agent\crates\yunxi-agent-storage\tests\weixin_state_tests.rs`、`D:\YunXi Agent\crates\yunxi-agent-weixin\tests\ilink_client_tests.rs`、`D:\YunXi Agent\crates\yunxi-agent-cli\tests\cli_tests.rs`：补齐加密、AAD 绑定、篡改/截断失败、重启恢复和混合批次测试。
- `D:\YunXi Agent\README.md`、`D:\YunXi Agent\docs\README.md`、`D:\YunXi Agent\docs\weixin.md`、`D:\YunXi Agent\docs\reports\README.md`：同步能力边界与报告入口。
- `D:\YunXi Agent\docs\development-log.md` 与本文件：记录本轮整改、验证和发布收口。

验证结果：

- `cargo fmt --all -- --check`、`cargo check --workspace`、`cargo test --workspace -- --test-threads=1`、`cargo build --workspace --release` 均通过。
- `yunxi --version` 输出 `yunxi 2.1.5-hotfix.1`。
- `weixin status --json`、`weixin doctor --json`、单轮 `weixin serve`、`eval companion --json`、ConPTY verifier、Provider smoke 均通过。
- `git diff --check`、`git fsck --full --no-dangling` 通过。

提交和推送状态：

- release commit：`f990ba1f539d25052211fca3baf3ebc39280a720`
- annotated tag：`v2.1.5-hotfix.1`
- tag object：`983e4ea426777a7bba9f74ba66b54506163c4049`
- 远端 `master`、新 tag object 与 peeled target 已通过 GitHub CLI 核验；旧 `v2.1.5` tag 未变。

清理状态：

- 仅清理了 `D:\YunXi Agent\target`，未触碰用户目录、`.git`、`.yunxi`、凭证存储或历史 tag。

署名：开发者
