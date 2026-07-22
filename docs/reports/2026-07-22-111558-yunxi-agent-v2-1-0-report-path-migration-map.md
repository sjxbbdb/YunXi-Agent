# YunXi Agent v2.1.0 报告路径迁移映射

执行时间：2026-07-22 11:15:58 +08:00

## 一、批次边界

本批次只迁移 v2.1.0 的一份审核报告和一份开发报告，不迁移其他版本，不修改报告正文事实，不移动 evidence、源码、脚本或 tag。

本节及下表记录 2026-07-22 11:15:58 +08:00 当时发生的迁移事实，不代表 `v2.1.1-hotfix.1` 整改后的当前正本路径。

## 二、精确映射与哈希

| 类型 | 旧路径 | 新路径 | 迁移前后 SHA-256 |
| --- | --- | --- | --- |
| 审核报告 | `docs/reports/2026-07-22-100333-yunxi-agent-v2-1-0-integrated-release-audit-report.md` | `docs/reports/audits/2026-07-22-100333-yunxi-agent-v2-1-0-integrated-release-audit-report.md` | `A56EB0C6D0E7B79EF6C95FD337398B3C48F89D1D7100EACFDE2F6E44C46E4A90` |
| 开发报告 | `docs/reports/2026-07-22-074211-yunxi-agent-v2-1-0-integrated-release-regression-development-report.md` | `docs/reports/development/2026-07-22-074211-yunxi-agent-v2-1-0-integrated-release-regression-development-report.md` | `B70BF0D3F3BBDEEB7DE1DB515D8FA60B09BB161F63C47160AF28997EB61D0413` |

两份源文件均为已跟踪普通文件，目标目录在本批次创建；移动使用两个精确 `git mv`，没有使用通配符、递归移动、强制覆盖或删除命令。

## 三、引用处理

活动链接已更新：

- `docs/README.md`
- `docs/reports/README.md`

`v2.1.1-hotfix.1` 整改后，上述两个活动入口均重新指向历史旧路径上的唯一正本。

`docs/development-log.md` 和 `docs/reports/2026-07-22-110139-yunxi-agent-project-directory-baseline-report.md` 中的旧路径属于迁移前状态的历史事实与基线清单，按整理纪律保留原文；它们不是活动 Markdown 链接。后续验证应检查活动链接，不应改写这些历史记录。

## 四、回滚边界

本批次作为独立提交，仅包含两份报告的 Git rename、两个活动索引更新和本映射文件。回滚时只还原该提交，不修改任何 release tag。

## 五、v2.1.1-hotfix.1 整改后的当前状态

| 类型 | 当前唯一正本 | 已撤销的误迁移路径 | 当前 SHA-256 |
| --- | --- | --- | --- |
| 开发报告 | `docs/reports/2026-07-22-074211-yunxi-agent-v2-1-0-integrated-release-regression-development-report.md` | `docs/reports/development/2026-07-22-074211-yunxi-agent-v2-1-0-integrated-release-regression-development-report.md` | `B70BF0D3F3BBDEEB7DE1DB515D8FA60B09BB161F63C47160AF28997EB61D0413` |

本次整改只按审核报告恢复上述历史开发报告；审核报告的分类归档和其他报告路径不在本次整改范围内。旧路径存在、新路径正文不存在，因此没有双正本。该状态仍须独立复审，不能据此提前宣称 v2.1.1 审核通过。

署名：开发者
