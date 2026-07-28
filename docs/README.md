# YunXi Agent 文档索引

本页是 `docs/` 的稳定导航入口。`v2.1.6` 保留 v2.1.4/hotfix 独立版本化微信状态 store、原子写入、账户锁诊断、配对请求本地生命周期、pending 计数、安全 logout、旧登录 metadata 到 `WeixinStateStore` 的安全幂等初始化、前台 `yunxi weixin serve` 私聊长轮询接纳层、认证加密 pending inbound、重启恢复入口和 Windows stale lock 恢复；本版本新增 `WeixinConversationBinding`，把已准入私聊的 `account + peer + dm` 绑定到既有 YunXi `SessionId`，并通过 `WeixinTurnSupervisor` 复用 `Agent::run_with_backend_stream`、共享 Provider/model/cwd/sandbox/approval/context-window/companion 配置、同会话有界串行队列和测试 sink 最终文本。本版本仍不发送或流式回复微信消息，不实现远程审批、群聊、附件解析或完整微信聊天闭环。

## 架构与运行边界

- [项目总览](../README.md)：版本、workspace 布局、能力和使用方式。
- [提取状态](extraction-status.md)：YunXi 自有实现、Codex 参考边界和已知差距。
- [Codex 核心能力映射](extraction-index/codex-core-agent-parity-map.md)：参考源码与迁移状态索引。
- [人格与记忆](persona-memory.md)：人格、Memory Schema、召回、隐私和审核边界。
- [TUI 表现与终端生命周期](tui-presentation.md)：布局、流式输出、焦点和恢复约束。
- [Sandbox 协议事件](protocol/sandbox-events.md)：执行策略与协议事件说明。
- [微信接入边界](weixin.md)：v2.1.6 QR 登录、安全凭证引用、状态持久化、旧 metadata 初始化、账户锁、pair 生命周期、前台私聊长轮询接纳、认证加密 pending inbound、Runtime session binding、同会话串行、测试 sink、logout 安全边界和明确未实现能力。

## 规格与路线图

- [设计规格目录](superpowers/specs/)：历史设计规格，保持现有路径。
- [实施计划目录](superpowers/plans/)：版本实施计划，保持现有路径。
- [v2.1.1 至 v2.2.0 个人微信接入与目录治理路线图](superpowers/plans/2026-07-22-yunxi-agent-v2-1-1-to-v2-2-0-personal-wechat-roadmap.md)：唯一可编辑正本，SHA-256 为 `2DF30D503F46CFE7496567F5011BF5CBFB8BA73C91C2D2FA3E9F29AF0932EA0F`；桌面分发副本已由正本重新生成并校验一致。

## 报告与证据

- [报告索引与归档规则](reports/README.md)：报告命名、分类和历史兼容规则。
- [v2.1.6 微信会话绑定既有 Runtime 开发报告](reports/development/2026-07-28-200713-yunxi-agent-v2-1-6-weixin-runtime-session-binding-development-report.md)：实现 `WeixinConversationBinding`、`WeixinTurnSupervisor`、共享 Runtime 配置、会话串行队列和 fake backend 集成的开发依据。
- [v2.1.6 微信会话绑定既有 Runtime 审核报告](reports/audits/2026-07-28-195752-yunxi-agent-v2-1-6-runtime-session-binding-audit-report.md)：开发前审核不通过，确认当时源码仍停留在 v2.1.5-hotfix.2，并转化为本轮 v2.1.6 开发任务。
- [v2.1.5-hotfix.2 微信 stale lock 恢复整改复审审核报告](reports/audits/2026-07-28-184005-yunxi-agent-v2-1-5-hotfix-2-weixin-stale-lock-recovery-reaudit-report.md)：审核通过，允许进入 v2.1.6。
- [v2.1.5-hotfix.2 微信 Windows stale lock 恢复整改开发报告](reports/development/2026-07-28-162630-yunxi-agent-v2-1-5-hotfix-2-weixin-windows-stale-lock-recovery-development-report.md)：修复 Windows 默认 process probe 将不存在 PID 判为 active、异常退出后账户锁无法回收的问题，并补独立新进程 pending 解密证据。
- [v2.1.5-hotfix.1 微信加密 pending inbound 整改开发报告](reports/development/2026-07-28-114710-yunxi-agent-v2-1-5-hotfix-1-weixin-encrypted-pending-inbound-development-report.md)：修复已准入私聊缺少真实 ciphertext、nonce、AAD 和重启恢复入口的 P1。
- [v2.1.5 微信私聊长轮询、配对准入与幂等接纳开发报告](reports/development/2026-07-27-233307-yunxi-agent-v2-1-5-weixin-long-polling-pairing-idempotency-development-report.md)：前台 `serve`、iLink `getupdates`、配对准入、游标原子提交和重复消息幂等开发记录。
- [v2.1.4-hotfix.1 微信旧账户状态迁移整改开发报告](reports/development/2026-07-27-203440-yunxi-agent-v2-1-4-hotfix-1-weixin-legacy-state-migration-development-report.md)：旧登录 metadata 到 `WeixinStateStore` 的幂等初始化整改、验证和发布门禁。
- [v2.1.4 微信状态生命周期审核报告](reports/audits/2026-07-27-201143-yunxi-agent-v2-1-4-weixin-state-lifecycle-audit-report.md)：确认 v2.1.4 审核不通过，要求先完成 hotfix。
- [v2.1.4 微信状态持久化、诊断与账户生命周期开发报告](reports/development/2026-07-27-183050-yunxi-agent-v2-1-4-weixin-state-store-diagnostics-lifecycle-development-report.md)：状态 store、锁、pair、logout、安全清理和发布门禁记录。
- [v2.1.3-hotfix.1 微信登录复审核报告](reports/audits/2026-07-27-181754-yunxi-agent-v2-1-3-hotfix-1-weixin-login-audit-report.md)：确认登录整改通过，允许进入 v2.1.4。
- [v2.1.3-hotfix.1 微信登录闭环整改开发报告](reports/development/2026-07-27-165954-yunxi-agent-v2-1-3-hotfix-1-weixin-login-verification-remediation-development-report.md)：本阶段整改依据、CLI Mock 验收、真实扫码验证结果和发布门禁。
- [v2.1.3 微信二维码登录审核报告](reports/audits/2026-07-23-125451-yunxi-agent-v2-1-3-weixin-qr-login-audit-report.md)：确认 v2.1.3 审核不通过，要求先完成 hotfix。
- [v2.1.2 微信模块、CLI 骨架与 iLink Mock 开发报告](reports/development/2026-07-22-164655-yunxi-agent-v2-1-2-weixin-cli-ilink-mock-development-report.md)：本版本实现范围、参考快照与发布门禁。
- [v2.1.3 微信二维码登录与系统安全凭证存储开发报告](reports/development/2026-07-22-215224-yunxi-agent-v2-1-3-weixin-qr-login-secret-store-development-report.md)：本阶段 QR 状态机、安全凭证引用、账户元数据和 CLI 登录边界。
- [v2.1.2 微信骨架审核报告](reports/audits/2026-07-22-211826-yunxi-agent-v2-1-2-weixin-skeleton-audit-report.md)：允许进入 v2.1.3 的独立审核结论。
- [v2.1.1-hotfix.1 独立复审审核报告](reports/audits/2026-07-22-164039-yunxi-agent-v2-1-1-hotfix-1-independent-reaudit-report.md)：v2.1.2 的正式准入依据。
- [v2.1.1-hotfix.1 路径整改独立复审报告](reports/audits/2026-07-22-163044-yunxi-agent-v2-1-1-hotfix-1-report-path-remediation-reaudit-report.md)：确认原唯一阻塞点关闭，允许进入 v2.1.2 开发阶段。
- [v2.1.1 审核报告](reports/audits/2026-07-22-154559-yunxi-agent-v2-1-1-audit-report.md)：记录历史开发报告路径迁移这一唯一阻塞点；结论为审核不通过。
- [v2.1.1 历史报告路径整改开发报告](reports/development/2026-07-22-155306-yunxi-agent-v2-1-1-hotfix-report-path-remediation-development-report.md)：`v2.1.1-hotfix.1` 整改依据与复审门禁。
- [v2.1.0 集成发布审核报告](reports/audits/2026-07-22-100333-yunxi-agent-v2-1-0-integrated-release-audit-report.md)：当前发布审核基线。
- [v2.1.0 集成发布开发报告](reports/2026-07-22-074211-yunxi-agent-v2-1-0-integrated-release-regression-development-report.md)：历史旧路径上的唯一正本。
- [v2.1.0 进入 v2.1.1 准入审核](reports/audits/2026-07-22-120829-yunxi-agent-v2-1-0-v2-1-1-entry-audit-report.md)：目录治理版本的开发准入依据。
- [v2.1.1 目录治理开发报告](reports/development/2026-07-22-130141-yunxi-agent-v2-1-1-directory-governance-development-report.md)：本版本的硬性边界、实施顺序和验收门禁。
- [v2.1.1 项目目录治理基线](directory-governance.md)：根目录资产、跟踪规则、本地状态、清理条件和路线图副本状态。
- [项目目录整理报告](reports/2026-07-22-105338-yunxi-agent-project-directory-organization-report.md)：目录治理原则与阶段边界。
- [项目目录整理阶段 0 基线](reports/2026-07-22-110139-yunxi-agent-project-directory-baseline-report.md)：受保护路径引用和整理前状态。
- [可复核证据目录](reports/evidence/)：脱敏说明、ConPTY frames 和 SHA-256 manifest。

## 工程与操作入口

- [脚本索引](../scripts/README.md)：安装、迁移、Provider 与版本化 ConPTY 验证入口。
- [Windows ConPTY 验证总览](../scripts/conpty/README.md)：v205 至 v210 场景、依赖、输出和正式 evidence 对照表。
- [开发日志](development-log.md)：按时间记录开发、验证、发布和安装操作。

## 目录治理规则

1. 新架构文档进入 `docs/architecture/`，新操作手册进入 `docs/operations/`；目录在首次新增正式文档时创建。
2. 新审核报告进入 `docs/reports/audits/`，新开发报告进入 `docs/reports/development/`；历史报告暂留 `docs/reports/` 根部。
3. 证据继续进入 `docs/reports/evidence/`，正式 evidence 不与 `.tmp/` 采集输出混用。
4. 移动历史文档前必须建立旧路径到新路径的完整引用映射，并在同一独立提交中更新所有链接。
5. 不因整理文档而移动源码、脚本、证据、release tag 或本地用户状态。

署名：开发者
