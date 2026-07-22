# Reports Index

返回[文档总索引](../README.md)。

## 当前入口

- [v2.1.0 集成发布审核报告](audits/2026-07-22-100333-yunxi-agent-v2-1-0-integrated-release-audit-report.md)
- [v2.1.0 集成发布开发报告](development/2026-07-22-074211-yunxi-agent-v2-1-0-integrated-release-regression-development-report.md)
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

## 历史兼容

历史 development、remediation、audit 和 evidence 报告继续保留在当前路径。本阶段不批量移动、不重命名，也不修改历史报告正文中的事实记录。任何后续迁移必须先建立旧路径到新路径的完整映射，在单独提交中更新全部仓库引用，并核验迁移前后文件 SHA-256 一致。
