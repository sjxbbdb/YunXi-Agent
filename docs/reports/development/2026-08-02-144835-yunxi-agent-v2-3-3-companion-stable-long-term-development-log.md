# YunXi Agent v2.3.3 陪伴层长期稳定体验开发日志

时间戳：2026-08-02 14:48:35 +08:00

## 工作目标

在不破坏既有 CLI、TUI、微信、工具调用、记忆写入和回复行为的前提下，一次性完成以下陪伴层增强：

1. 更细的人格/灵魂文件规则。
2. 更长期的关系状态演化。
3. 更自然的情绪识别。
4. 微信端与 TUI 的长期陪伴体验回归验证。
5. 将陪伴策略从“规则可用”升级为“稳定、有个性、长期一致”。

## 硬性边界执行情况

- 未删除、移动、递归清理或覆盖用户目录。
- 未使用 force。
- 未删除、移动或覆盖历史 tag。
- 未引入额外模型调用；陪伴策略仍为确定性规则，避免增加回复延迟。
- 记忆仍在成功回复后写入，失败/空回复不会污染长期记忆。
- 人格、记忆和关系上下文仍不能覆盖 AGENTS.md、用户当轮指令、安全策略、隐私策略、sandbox policy 或工具边界。

## 主要修改路径与内容

- `D:\YunXi Agent\crates\yunxi-agent-persona\src\profile.rs`
  - 新增 `PersonaCompanionRules` 与 `PersonaRuleLevel`。
  - 人格文件新增可选 `companion_rules`，包含 `soul_signature`、温度、直接度、主动性、幽默、情绪贴合度、回复规则、记忆使用规则、关系规则和禁止风格。
  - 所有新增字段均使用 `serde(default)`，旧人格 JSON 不写新字段也能继续加载。
  - 内置 `yunxi_companion_strong` 升级到 `2.3.3`，补入稳定、有边界、长期一致的默认陪伴规则。

- `D:\YunXi Agent\crates\yunxi-agent-persona\src\compiler.rs`
  - persona context 版本升级到 `2.3.3`。
  - 新增 `<companion_rules role="reply_style_guidance">` block，将人格/灵魂规则注入 CLI、TUI、微信共用的 provider system context。
  - 调整上下文预算裁剪优先级：companion_rules 可选内容优先裁剪，记忆上下文优先保留；极限预算下仍保留安全边界和 memory context 结构。

- `D:\YunXi Agent\crates\yunxi-agent-companion\src\lib.rs`
  - 新增 `CompanionRelationshipStage`、`CompanionEmotionKind`、`CompanionEmotion`、`CompanionPersonaStyle`。
  - 扩展 `CompanionContext` 和 `CompanionPolicyDecision`，加入关系阶段、情绪类型/强度/置信度、人格风格、稳定一致性 key。
  - 新增确定性情绪识别，覆盖焦虑、难过、疲惫、烦躁/卡住、开心/进展、不确定/困惑、孤独/需要陪伴等中英混合表达。
  - 策略根据工具请求、情绪、人格直接度和上下文可用性选择 direct / supportive / warm / neutral 语气。

- `D:\YunXi Agent\crates\yunxi-agent-runtime\src\lib.rs`
  - 从 active 长期记忆中派生 `RelationshipState`：熟悉度、trust notes、recent emotional context、last meaningful check-in。
  - 将派生关系状态传入 `PersonaPromptCompiler`，不再每轮使用默认空关系。
  - 每轮都会计算 companion policy metadata；没有主动信号时不额外发送 companion 消息。
  - 收窄关系 milestone 的主动消息触发条件：普通聊天召回关系记忆只影响上下文和语气，不再额外弹 `[关心] 最近上下文有变化`。
  - 新增 metadata：`companion_policy_emotion_kind`、`companion_policy_emotion_intensity`、`companion_policy_emotion_confidence`、`companion_policy_relationship_stage`、`companion_policy_consistency_key`。

- `D:\YunXi Agent\crates\yunxi-agent-weixin\src\turn_supervisor.rs`
  - 加固微信公开回复过滤：如果内部 `companion_policy_*`、`context_phase`、`yunxi_persona_context`、`<companion_rules>`、memory context 等内容误入最终文本，会被过滤，不对微信用户展示。

- `D:\YunXi Agent\crates\yunxi-agent-tui\src\render.rs`
  - 控制页回归覆盖 relationship stage 展示。

- `D:\YunXi Agent\crates\yunxi-agent-cli\src\main.rs`
  - `persona profile` 文本输出补充 soul、values、addressing 和 companion_rules，便于检查人格/灵魂文件是否实际生效。

- `D:\YunXi Agent\crates\yunxi-agent-eval\src\lib.rs`
  - companion harness 版本升级到 `2.3.3`。
  - 增加结构化人格规则与情绪策略 deterministic check。

- `D:\YunXi Agent\evals\companion\scenarios\persona_consistency.jsonl`
  - 新增 `persona-007`：验证内置人格具备结构化 companion rules。

- `D:\YunXi Agent\evals\companion\scenarios\proactive_boundaries.jsonl`
  - 新增 `proactive-007`：验证情绪策略 deterministic 且带关系阶段/一致性 key。

- `D:\YunXi Agent\Cargo.toml`
  - 工作区版本升级为 `2.3.3`。

- `D:\YunXi Agent\Cargo.lock`
  - workspace crate 版本同步为 `2.3.3`。

- `D:\YunXi Agent\crates\yunxi-agent-tui\src\snapshots\*.txt`
  - TUI snapshot 版本号同步为 `2.3.3`。

## 新增与更新测试

- companion 单元测试：
  - 情绪识别覆盖高强度焦虑/压力信号。
  - policy 保持人格风格、关系阶段和一致性 key。

- persona 单元测试：
  - 旧人格 JSON 不含 `companion_rules` 仍可加载。
  - 自定义 soul/rules 可编译进共享 persona context。
  - 极限预算下安全结构和记忆上下文不回归。

- runtime 单元与集成测试：
  - active 长期记忆推动关系状态演化。
  - 普通长期陪伴聊天形成情绪/关系/persona policy metadata，但不额外弹公开 companion 消息。
  - 显式 companion check 才触发关系 milestone 主动消息。

- CLI/TUI/微信测试：
  - CLI JSONL 暴露新增 companion metadata。
  - TUI 控制页展示 relationship stage。
  - 微信公开回复过滤内部 companion policy/persona context 内容。

## 验证结果

- `cargo fmt --all`：通过。
- `cargo test -p yunxi-agent-companion -p yunxi-agent-persona -p yunxi-agent-runtime`：通过。
- `cargo test -p yunxi-agent-cli -p yunxi-agent-tui -p yunxi-agent-weixin -p yunxi-agent-tools`：通过。
- `cargo run -q -p yunxi-agent-cli --bin yunxi -- --json eval companion`：通过，`33/33`，`golden_passed=true`。
- `cargo run -q -p yunxi-agent-cli --bin yunxi -- --json eval weixin`：通过，离线门禁通过；真实扫码/重启恢复为人工 not_run 门禁。
- `cargo test --workspace`：通过。
- `cargo build -p yunxi-agent-cli --release --bins`：通过。
- `D:\YunXi Agent\target\release\yunxi.exe --version`：`yunxi 2.3.3`。
- `D:\YunXi Agent\target\release\yunxi-agent-cli.exe --version`：`yunxi 2.3.3`。
- `D:\YunXi Agent\target\release\yunxi.exe --json eval companion`：通过，`33/33`，`golden_passed=true`。

## 真实体验说明

本轮已完成 CLI、TUI、微信真实代码路径的离线自动化回归。真实微信 iLink 扫码、真实私聊收发与重启恢复属于人工验收门禁，需要用户参与扫码和发消息；本日志不将该人工门禁伪造为已执行。

## 提交与发布状态

发布结果：

- 本地 release commit：`3bcc89b5cd353f10d2ab0eda8a9d9845a94874fa`。
- 本地 annotated tag：`v2.3.3`，tag object `4000c3f3ec9d3131f06acd3ebe4f0b8f8c050ae5`，target `3bcc89b5cd353f10d2ab0eda8a9d9845a94874fa`。
- GitHub API 发布基线：远端 `master` 从 `d3e0b58496241a49f72c7937b5e77b0195b85f23` 快进。
- 远端 release commit：`db5a955b5f478452b94cc48c34a087ab26c8afa4`。
- 远端 annotated tag：`v2.3.3`，tag object `8abc284e65062774be2b33a6ae3e7b756e529624`，target `db5a955b5f478452b94cc48c34a087ab26c8afa4`。
- 远端 release tree：`ac54bbc87656c2becc5041eeff8469cd013c9416`。
- 远端核验：`master_matches=true`、`tag_object_matches=true`、`tag_target_matches=true`，历史 `v2.3.2` tag 仍存在。

说明：本机 Git smart HTTP 连续无法连接 GitHub，发布采用 GitHub CLI + Git Data API，以远端 master 为父提交创建 release commit 和 annotated tag；未使用 force，未删除、移动或覆盖历史 tag。API key 仅放入当前 PowerShell 进程环境变量，未打印、未写入仓库、未写入 Git 配置或 remote URL。

本条发布结果日志作为 tag 后 docs-only 收口提交推进 `master`；不会移动 `v2.3.3` tag。

署名：开发者
