# Reports Index

返回[文档总索引](../README.md)。

## 当前入口

- [v2.1.5 微信私聊长轮询、配对准入与幂等接纳开发报告](development/2026-07-27-233307-yunxi-agent-v2-1-5-weixin-long-polling-pairing-idempotency-development-report.md)：前台 `serve`、iLink `getupdates`、配对、游标原子提交和幂等接纳开发记录与发布门禁。
- [v2.1.4-hotfix.1 微信旧账户状态迁移复审审核报告](audits/2026-07-27-230718-yunxi-agent-v2-1-4-hotfix-1-weixin-state-migration-reaudit-report.md)：整改复审通过，允许进入 v2.1.5。
- [v2.1.4-hotfix.1 微信旧账户状态迁移整改开发报告](development/2026-07-27-203440-yunxi-agent-v2-1-4-hotfix-1-weixin-legacy-state-migration-development-report.md)：旧登录 metadata 到 `WeixinStateStore` 的幂等初始化整改记录。
- [v2.1.4 微信状态生命周期审核报告](audits/2026-07-27-201143-yunxi-agent-v2-1-4-weixin-state-lifecycle-audit-report.md)
- [v2.1.4 微信状态持久化、诊断与安全账户生命周期开发报告](development/2026-07-27-183050-yunxi-agent-v2-1-4-weixin-state-store-diagnostics-lifecycle-development-report.md)：状态 store、原子写入、诊断、账户锁、pair 生命周期、安全 logout 和发布门禁记录。
- [v2.1.3-hotfix.1 微信登录复审核报告](audits/2026-07-27-181754-yunxi-agent-v2-1-3-hotfix-1-weixin-login-audit-report.md)
- [v2.1.3 微信二维码登录审核报告](audits/2026-07-23-125451-yunxi-agent-v2-1-3-weixin-qr-login-audit-report.md)
- [v2.1.3-hotfix.1 微信登录闭环整改开发报告](development/2026-07-27-165954-yunxi-agent-v2-1-3-hotfix-1-weixin-login-verification-remediation-development-report.md)：CLI Mock 与 Windows 真实扫码整改证据。
- [v2.1.2 微信骨架审核报告](audits/2026-07-22-211826-yunxi-agent-v2-1-2-weixin-skeleton-audit-report.md)
- [v2.1.3 微信二维码登录与系统安全凭证存储开发报告](development/2026-07-22-215224-yunxi-agent-v2-1-3-weixin-qr-login-secret-store-development-report.md)
- [v2.1.1-hotfix.1 独立复审审核报告](audits/2026-07-22-164039-yunxi-agent-v2-1-1-hotfix-1-independent-reaudit-report.md)
- [v2.1.2 微信模块、CLI 骨架与 iLink Mock 开发报告](development/2026-07-22-164655-yunxi-agent-v2-1-2-weixin-cli-ilink-mock-development-report.md)
- [v2.1.1-hotfix.1 路径整改独立复审报告](audits/2026-07-22-163044-yunxi-agent-v2-1-1-hotfix-1-report-path-remediation-reaudit-report.md)
- [v2.1.1 审核报告](audits/2026-07-22-154559-yunxi-agent-v2-1-1-audit-report.md)
- [v2.1.1 历史报告路径整改开发报告](development/2026-07-22-155306-yunxi-agent-v2-1-1-hotfix-report-path-remediation-development-report.md)
- [v2.1.0 进入 v2.1.1 准入审核报告](audits/2026-07-22-120829-yunxi-agent-v2-1-0-v2-1-1-entry-audit-report.md)
- [v2.1.1 目录治理开发报告](development/2026-07-22-130141-yunxi-agent-v2-1-1-directory-governance-development-report.md)
- [v2.1.0 集成发布审核报告](audits/2026-07-22-100333-yunxi-agent-v2-1-0-integrated-release-audit-report.md)
- [v2.1.0 集成发布开发报告](2026-07-22-074211-yunxi-agent-v2-1-0-integrated-release-regression-development-report.md)
- [v2.1.0 报告路径迁移映射](2026-07-22-111558-yunxi-agent-v2-1-0-report-path-migration-map.md)
- [项目目录整理报告](2026-07-22-105338-yunxi-agent-project-directory-organization-report.md)
- [项目目录整理阶段 0 基线](2026-07-22-110139-yunxi-agent-project-directory-baseline-report.md)
- [证据目录](evidence/)

## 新报告命名与归档

新报告使用以下格式：

```text
YYYY-MM-DD-HHMMSS-yunxi-agent-vX-Y-Z-topic-audit-report.md
YYYY-MM-DD-HHMMSS-yunxi-agent-vX-Y-Z-topic-development-report.md
```

- 新审核报告写入 `docs/reports/audits/`。
- 新开发报告写入 `docs/reports/development/`。
- 脱敏说明、机器采集和 manifest 写入 `docs/reports/evidence/`。
- 新分类目录在第一份正式文档进入时创建，不提交空目录占位文件。
- 每份报告必须记录来源、适用版本、结论、验证状态和署名。
- 项目内报告是正本；桌面同名文件只是分发件，必须由项目正本单向生成并校验 SHA-256。

## 历史兼容

历史 development、remediation、audit 和 evidence 报告继续保留在当前路径。本阶段不批量移动、不重命名，也不修改历史报告正文中的事实记录。`2026-07-22-074211-yunxi-agent-v2-1-0-integrated-release-regression-development-report.md` 已恢复到本目录根部并作为唯一正本，`development/` 下不保留第二份正文。任何后续迁移必须先建立旧路径到新路径的完整映射，在单独提交中更新全部仓库引用，并核验迁移前后文件 SHA-256 一致。

署名：开发者
