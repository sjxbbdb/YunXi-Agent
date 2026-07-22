# YunXi Agent 文档索引

本页是 `docs/` 的稳定导航入口。`v2.1.1` 只建立目录治理、Git 忽略边界与文档索引基线，不实现微信功能，也不移动历史文档；已有路径继续保持兼容。

## 架构与运行边界

- [项目总览](../README.md)：版本、workspace 布局、能力和使用方式。
- [提取状态](extraction-status.md)：YunXi 自有实现、Codex 参考边界和已知差距。
- [Codex 核心能力映射](extraction-index/codex-core-agent-parity-map.md)：参考源码与迁移状态索引。
- [人格与记忆](persona-memory.md)：人格、Memory Schema、召回、隐私和审核边界。
- [TUI 表现与终端生命周期](tui-presentation.md)：布局、流式输出、焦点和恢复约束。
- [Sandbox 协议事件](protocol/sandbox-events.md)：执行策略与协议事件说明。

## 规格与路线图

- [设计规格目录](superpowers/specs/)：历史设计规格，保持现有路径。
- [实施计划目录](superpowers/plans/)：版本实施计划，保持现有路径。
- [v2.1.1 至 v2.2.0 个人微信接入与目录治理路线图](superpowers/plans/2026-07-22-yunxi-agent-v2-1-1-to-v2-2-0-personal-wechat-roadmap.md)：唯一可编辑正本，SHA-256 为 `2DF30D503F46CFE7496567F5011BF5CBFB8BA73C91C2D2FA3E9F29AF0932EA0F`；桌面分发副本已由正本重新生成并校验一致。

## 报告与证据

- [报告索引与归档规则](reports/README.md)：报告命名、分类和历史兼容规则。
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
