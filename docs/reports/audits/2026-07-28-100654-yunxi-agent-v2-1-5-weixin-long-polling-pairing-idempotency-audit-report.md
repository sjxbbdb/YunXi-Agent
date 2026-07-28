# YunXi Agent v2.1.5 微信私聊长轮询、配对与幂等审核报告

- 审核时间：2026-07-28 10:06:54 +08:00
- 审核版本：v2.1.5
- 审核目录：D:/YunXi Agent
- 审核范围：只对照总纲 v2.1.5 要求，不进行多个版本横向比较
- 总纲正本：D:/YunXi Agent/docs/superpowers/plans/2026-07-22-yunxi-agent-v2-1-1-to-v2-2-0-personal-wechat-roadmap.md
- 总纲 SHA-256：2DF30D503F46CFE7496567F5011BF5CBFB8BA73C91C2D2FA3E9F29AF0932EA0F
- 开发报告：D:/YunXi Agent/docs/reports/development/2026-07-27-233307-yunxi-agent-v2-1-5-weixin-long-polling-pairing-idempotency-development-report.md
- 发布 tag：v2.1.5
- tag object：03d9bd6d7985dddb7719da6ae0cd3d25a717dcb9
- tag target：44d89488d421748527547f52faa73caad84ef6b2
- 审核时 HEAD：a106684a32b423b252040b79fb5c0057c370937c

## 一、审核结论

**审核不通过，禁止进入总纲图中的 v2.1.6 开发。必须留在 v2.1.5 完善。**

本轮确认 v2.1.5 已实现前台 getupdates 轮询、账户锁、游标/receipt/pair 的批次提交、陌生发送者配对和重复消息幂等，但发现一个总纲级 P1 阻塞点：**已准入私聊消息没有被真正加密和持久化，pending inbound 只保存一个引用字符串，消息正文在本轮进程结束后无法恢复。**

这直接违反总纲 v2.1.5 的以下要求：

- 加密 accepted/ready 队列项必须与 get_updates_buf 游标在同一次持久化提交中接纳。
- 重启后必须能够恢复非终态 pending inbound。
- 未准入消息不能进入 Agent，但已准入消息不能因为只保存哈希而悄然丢失。
- data key 不应只作为启动检查项，必须实际参与 pending payload 的认证加密。

因此当前版本不能宣称“加密 pending inbound”已经完成，也不能进入 v2.1.6 Runtime 会话绑定开发。

## 二、P1 阻塞点：加密 pending inbound 未实现

### 1. 实际源码行为

路径：D:/YunXi Agent/crates/yunxi-agent-cli/src/weixin.rs

- 170-250 行的 run_serve 会读取 data key，并以此判断 encrypted pending queue 可用。
- 196-198 行只调用 get_data_key 做存在性检查；拿到的 data key 没有传入 serve loop、消息归一化或 state commit。
- 213-218 行只把 envelope.encrypted_payload_ref 写入 WeixinInboundCommitItem，没有把消息正文、密文、nonce、算法版本或认证标签传入提交。

路径：D:/YunXi Agent/crates/yunxi-agent-weixin/src/inbound.rs

- 37-61 行的 WeixinInboundEnvelope 只生成 account hash、peer hash、message hash、direct-message key、时间、context reference 和 kind。
- 74-82 行的 encrypted_payload_ref 只是由哈希拼接得到的脱敏引用，不是加密结果。
- 归一化后没有留下可供重启恢复的文本正文或加密 payload。

路径：D:/YunXi Agent/crates/yunxi-agent-storage/src/weixin_state.rs

- 590-595 行的 WeixinInboundCommitItem 只有 item_id、message_id_hash、peer_id_hash 和 encrypted_payload_ref。
- 322-334 行写入 WeixinPendingInbound 时只复制该引用，未写入 ciphertext、nonce、AAD 或密文版本。
- 769-781 行的 WeixinPendingInbound 结构没有任何加密 payload 字段。
- 1155-1165 行只验证引用非空且不能包含 token/context，不能证明引用对应的消息正文已被加密保存。

路径：D:/YunXi Agent/crates/yunxi-agent-weixin/src/secret_store.rs

当前 secret store 提供 data key 的写入、读取和删除，但没有 payload encrypt/decrypt API。全仓源码搜索也未发现 authenticated encryption、ciphertext、nonce 或 decrypt recovery 实现。data key 目前只是被 serve 启动检查读取。

### 2. 影响

对已准入私聊，轮询收到的原始文本只存在于当前内存中的 WeixinMessage；提交后 state 文件只留下哈希和引用。服务重启后虽然能看到 pending inbound 计数，但无法解密或还原消息内容，后续 v2.1.6 无法把这条消息交给既有 YunXi Runtime。

这不是测试数量问题，而是数据流缺失：当前实现同时避免了明文泄漏，也丢失了应当安全保存的已准入消息。不能用“有 data key”“有 encrypted_payload_ref”或“pending_count 增加”替代真正的密文持久化。

## 三、总纲逐项核对

| 总纲 v2.1.5 要求 | 结果 | 审核说明 |
| --- | --- | --- |
| 前台 yunxi weixin serve | 通过 | CLI 已调用 run_weixin_serve_loop，支持 max_polls、账户锁和取消路径。 |
| getupdates 长轮询与游标 | 通过 | Ilink client 调用 get_updates，state 保存 get_updates_buf；真实单轮 serve 已执行。 |
| timeout hint、有界退避和网络错误 | 部分通过 | 有 backoff 和空轮询等待，网络错误有脱敏 health；真实本机单轮结果为 network_error_count=1。 |
| 批次、游标、receipt、pending、pair 同一原子提交 | 通过 | commit_inbound_batch 同一快照更新这些元数据，原子失败测试通过。 |
| 加密 accepted/ready pending inbound | **不通过** | 只保存 encrypted_payload_ref，未生成或持久化密文，P1 阻塞。 |
| 陌生私聊 pairing | 通过 | 陌生文本生成短时不透明 request ID，不创建 Runtime/session。 |
| 已准入私聊接纳 | 部分通过 | 哈希、receipt 和 pending 状态写入，但正文没有可恢复密文。 |
| 重复 message ID 幂等 | 通过 | receipt/pending 查重，storage 和 serve 测试通过。 |
| 群聊、自消息、附件、未知消息边界 | 通过 | envelope 分类和脱敏跳过计数测试通过，群聊配置关闭。 |
| token 过期、凭证失效和健康诊断 | 通过 | 过期写 suspended/credential_expired，不删除账户或凭证。 |
| 未准入消息不进入 Agent/session/persona/memory/tool | 通过 | 当前版本没有 Runtime dispatch 入口，测试覆盖陌生消息不产生 pending。 |
| v2.1.6 以上能力不提前实现 | 通过 | 未发现 Session 绑定、Provider 调用、sendmessage、远程审批或群聊实现。 |
| 验证项 | 结果 |
| --- | --- |
| cargo fmt --all -- --check | 通过 |
| cargo check --workspace | 通过 |
| cargo test --workspace -- --test-threads=1 | 通过，无失败；CLI 集成 54/54，JSONL 10/10，TUI 161/161 |
| cargo test -p yunxi-agent-storage -- --test-threads=1 | 通过；新增 inbound batch 相关测试 11 项全部通过 |
| cargo test -p yunxi-agent-weixin -- --test-threads=1 | 通过；serve、inbound、backoff、iLink、login/store、redaction 测试全部通过 |
| cargo test -p yunxi-agent-cli -- --test-threads=1 | 通过；CLI 单元 23/23、兼容二进制 23/23、集成 54/54、JSONL 10/10 |
| cargo test -p yunxi-agent-provider -- --test-threads=1 | 通过；Provider 46/46 |
| cargo test -p yunxi-agent-tui -- --test-threads=1 | 通过；TUI 161/161 |
| cargo build --workspace --release | 通过；release 版本 yunxi 2.1.5 |
| eval companion --json | 通过；31/31，golden_passed=true，禁止记忆写入 0，审批绕过 0，主动边界违规 0 |
| release status --json | 通过；receive_messages=true、foreground_long_polling=true、send_messages=false、group_chat=false、secrets_included=false |
| release doctor --json | 通过；message_receive_enabled=true、message_send_enabled=false、group_chat_enabled=false、network_request_performed=false、secrets_included=false |
| release serve max_polls=1 | 通过；polls=1、stopped_reason=max_polls、runtime_dispatch_enabled=false、send_message_enabled=false、network_error_count=1、secrets_included=false |
| Provider smoke | 通过；exit_code=0、22 条 JSONL 事件、secret_leak_detected=false；只证明既有 Provider 未回归 |
| ConPTY v210 verifier | 通过；ok=true、read_only=true，evidence SHA-256=a291a66cf91ee788bba9944edbafbf4c8dfce43bcd2d6c7b2cc78bc6c2990fc6 |
| git diff --check | 通过 |
| cargo tree -p yunxi-agent-cli --depth 1 | 通过；未发现 yunxi-agent-codex、codex-* 或 vendor 依赖 |
| git fsck --full --no-dangling | 通过 |

测试全部通过只能证明当前实现的轮询、元数据提交和幂等模型可运行，不能解除 P1 的密文缺失。

## 五、整改要求：必须留在 v2.1.5

开发者不得开始 v2.1.6。必须先完成以下整改并重新审核：

1. 在 D:/YunXi Agent/crates/yunxi-agent-weixin 或明确的 storage facade 中实现 Rust 原生 authenticated encryption，不得手写密码算法。可评估维护良好的 aes-gcm 或 chacha20poly1305 crate，并把算法版本固定进 schema。
2. 从系统凭证存储取得 data key，只在进程内使用；为每条已准入消息生成随机 nonce，并使用 account hash、peer hash、message hash、item id 作为 AAD，防止密文跨账户/对端/消息替换。
3. 扩展 WeixinInboundCommitItem 和 WeixinPendingInbound，使同一次 state-store 原子提交同时写入密文、nonce、算法/schema 标识、密文引用和状态；不得把原始正文写入 JSON、日志、错误或普通临时文件。
4. 将消息正文或严格定义的最小可恢复 envelope 在进入 batch commit 前加密；若加密失败、data key 缺失或密文校验失败，游标不得推进，批次不得部分提交。
5. 增加重启恢复 API 或最小解密 API，为 v2.1.6 Runtime 提供可审计的 pending item 恢复入口；当前版本仍不得调用 Runtime。
6. 对密文做边界校验：密文不能包含原文、nonce 长度固定、AAD 不匹配必须拒绝、错误 key 必须拒绝、旧 schema 必须安全迁移或拒绝。
7. 修正 README.md、docs/README.md、docs/weixin.md、开发报告和报告索引中“已写入加密 pending inbound”的表述，只有真实密文和恢复测试通过后才可保留。
- approved private text 加密后写入 state，原文不出现在 state 文件、stdout、stderr、JSON、日志和错误中。
- 使用同一 data key 重启新进程读取并成功解密，得到相同 envelope/body；不能依赖内存缓存。
- nonce 唯一性、AAD 绑定、错误 key、篡改 ciphertext、截断 ciphertext 和未知算法版本均安全失败。
- 加密失败注入时，cursor、receipt、pending inbound、pair 和 health 均保持旧快照，不产生半提交。
- data key 缺失或 secure store 不可用时 serve 启动拒绝，且不会进行 getupdates 网络请求。
- 批次包含重复 ID、陌生 peer、已准入 peer、群消息、自消息和附件时，密文与 skip/pair 计数保持正确。
- 重启恢复后 pending item 仍保留正确状态和过期/容量清理边界。
- 继续通过现有 workspace、Provider、companion、TUI、ConPTY 和 JSON/JSONL 回归。

本版本已使用或应继续参考以下路径。所有非 Rust 项目只抽取逻辑和测试思路，必须 Rust 化，不得复制宿主框架。

- D:/源码/reasonix/internal/bot/weixin/weixin.go：轮询游标、自消息过滤、context token 和账户状态边界。
- D:/源码/reasonix/internal/bot/weixin/weixin_test.go：重复消息、超时、重启、账户隔离和状态恢复测试组织。
- D:/源码/openclaw-weixin/src/api/api.ts：getupdates、协议错误、重连和 timeout 行为。
- D:/源码/openclaw-weixin/src/api/types.ts：消息、游标、context token 和未知字段安全反序列化。
- D:/源码/openclaw-weixin/src/messaging/inbound.ts：私聊/群聊、自消息和附件分类边界。
- D:/源码/openclaw-weixin/src/auth/pairing.ts：pair request 过期、白名单和 request ID。
- D:/源码/openclaw-weixin/src/storage/sync-buf.ts：游标及同步缓冲恢复边界。
- D:/源码/openclaw-weixin/src/storage/state-dir.ts：本地状态目录和持久化边界参考。
- D:/YunXi Agent/crates/yunxi-agent-weixin/src/secret_store.rs：现有 Windows Credential Manager data key 入口；需要在其上增加真正的 payload 加密使用边界。
- D:/YunXi Agent/crates/yunxi-agent-weixin/src/inbound.rs：当前 envelope 和脱敏边界；整改不得让原文重新进入普通状态模型。
- D:/YunXi Agent/crates/yunxi-agent-storage/src/weixin_state.rs：当前原子 state store、receipt、pair、pending 状态机和 schema 校验。

## 八、版本准入与发布状态

- v2.1.5 当前审核不通过，必须继续完善当前版本。
- 禁止进入 v2.1.6 Runtime 会话绑定、WeixinConversationBinding、WeixinTurnSupervisor、Session 复用或 Provider dispatch。
- 本地 v2.1.5 annotated tag 已存在且不得移动、删除或覆盖；整改应创建新的 hotfix annotated tag，不得重写 v2.1.5。
- v2.1.5 tag object 为 03d9bd6d7985dddb7719da6ae0cd3d25a717dcb9，target 为 44d89488d421748527547f52faa73caad84ef6b2。
- 本轮 GitHub 远端 refs 因连接重置和超时未能独立复核；开发报告记录远端发布成功，但下一次复审必须重新核验远端 master、tag object 和 peeled target。
- 本轮未修改生产源码、未创建修复 commit、未推送、未创建或移动 tag。

## 九、清理与安全状态

- 本轮已在精确路径确认后清理 D:/YunXi Agent/target；删除前存在，删除后不存在。
- 未删除、移动或清空 D:/YunXi Agent/.yunxi、.tmp、源码、正式 evidence、Git 历史、历史 tag、C:/Users 用户目录、Windows Credential Manager 或系统配置。
- 未执行 git clean、gc、prune、系统安装/卸载、PATH/注册表修改或用户数据清理。
- Provider smoke 使用 C:/Users/24763/Desktop/api.txt，仅记录汇总状态和泄漏检测结果，未输出凭证内容。

署名：审核者
