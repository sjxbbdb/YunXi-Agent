# YunXi Agent v2.3.2 陪伴层开发日志

## 时间戳

- 开始时间：2026-08-02 12:52（Asia/Shanghai）
- 日志记录时间：2026-08-02 13:15（Asia/Shanghai）

## 本次目标

在不破坏既有 CLI、TUI、微信、工具调用、记忆和回复逻辑的前提下，补齐陪伴层的上下文、策略、降级、成功回复记忆门禁以及可观测性，并完成集成回归。

## 修改内容与路径

1. `crates/yunxi-agent-companion/src/lib.rs`
   - 新增 `CompanionContext`，汇总人格、关系状态、记忆摘要和情绪线索。
   - 新增 `CompanionMemorySummary`、`CompanionTone`、`CompanionFollowUp`。
   - 新增 `CompanionPolicy` trait 和确定性 `DeterministicCompanionPolicy`。
   - 策略只使用规则，不增加模型调用；补充上下文缺失和情绪线索测试。

2. `crates/yunxi-agent-runtime/src/lib.rs`
   - 将 `PersonaTurnContext` 映射为统一 `CompanionContext`。
   - CLI、TUI、微信继续通过 Runtime 使用同一套陪伴决策。
   - 策略 panic、上下文缺失和策略超时均回退到既有 `SafeCompanionPlanner`。
   - 在 `AgentEvent::TurnMetadata` 写入陪伴上下文、策略、总耗时、回退原因、计划数和记忆延迟门禁指标。
   - 仅在非空、成功生成的最终回复之后运行记忆抽取；空回复写入延迟警告且不写入长期记忆。

3. `crates/yunxi-agent-runtime/tests/runtime_tests.rs`
   - 锁定陪伴计划与最终回复分离行为。
   - 增加陪伴指标事件回归。
   - 增加空最终回复不触发记忆抽取/写入回归。
   - 保留并运行工具调用、记忆写入、流式回复、审批和取消路径回归。

4. `crates/yunxi-agent-cli/tests/cli_tests.rs`
   - 增加 JSONL 回归：确认主回复保持原内容，同时暴露陪伴策略耗时和计划指标。

5. `Cargo.toml` / `Cargo.lock`
   - 版本从 `2.3.1` 升级到 `2.3.2`。

## 验证结果

- `cargo fmt --all`：通过。
- `cargo check --workspace`：通过。
- `cargo test -p yunxi-agent-cli -p yunxi-agent-weixin -p yunxi-agent-tools -p yunxi-agent-runtime`：通过。
- `cargo test --workspace`：通过。

## 数据安全与清理边界

- 未删除、移动或覆盖用户目录。
- 未删除或移动历史 Git tag。
- 未使用 force 推送。
- 本次只修改项目源码、测试、版本和本日志文件。

## 版本与发布

- 目标版本：`v2.3.2`
- 发布方式：使用 GitHub CLI/API 进行非强制提交和 tag 推送。
- 历史 tag 保留。

开发者
