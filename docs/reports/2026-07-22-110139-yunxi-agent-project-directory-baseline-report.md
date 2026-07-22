# YunXi Agent 项目目录整理阶段 0 基线报告

记录时间：2026-07-22 11:01:39 +08:00

## 一、仓库基线

- 项目根目录：`D:\YunXi Agent`
- 分支：`master`
- HEAD：`8dde407c4d680f68433f172efbe02d3061888698`
- workspace 版本：`2.1.0`
- 最近 release tag：`v2.1.0`
- tag 总数：50
- 根目录稳定资产：`crates/`、`evals/`、`scripts/`、`docs/`、`vendor/`、`extracted/`、`Cargo.toml`、`Cargo.lock`
- 根目录本地状态/再生产物：`target/`、`.tmp/`、`.yunxi/`、`.codegraph/`、`.worktrees/`
- 根目录还存在 `.agents/`、`.codex/`、`.superpowers/`；本阶段不移动、不删除、不改写。

## 二、整理前 Git 状态

正式待提交资产：

- `docs/reports/2026-07-22-100333-yunxi-agent-v2-1-0-integrated-release-audit-report.md`
- `docs/reports/2026-07-22-105338-yunxi-agent-project-directory-organization-report.md`
- `docs/superpowers/plans/2026-07-22-yunxi-agent-v2-1-1-to-v2-2-0-personal-wechat-roadmap.md`

可再生但本阶段不清理的未跟踪依赖：

- `scripts/conpty/v207/node_modules/`
- `scripts/conpty/v207-hotfix/node_modules/`
- `scripts/conpty/v208/node_modules/`
- `scripts/conpty/v209/node_modules/`
- `scripts/conpty/v210/node_modules/`

`git ls-files -- '.tmp/**' 'scripts/conpty/**/node_modules/**' 'scripts/conpty/**/.work/**'` 无输出，证明阶段 1 拟增加的忽略模式不会吞掉已跟踪文件。

## 三、受保护路径引用清单

扫描命令统一排除 `target/`、`.git/`、`.codegraph/` 和所有 `node_modules/`，同时包含未跟踪正式文档。以下文件清单是后续任何迁移前必须重新核对的基线。

### `vendor/codex-rs` 引用文件（75）

```text
.\.gitattributes
.\Cargo.toml
.\crates\yunxi-agent-cli\tests\cli_tests.rs
.\crates\yunxi-agent-codex\Cargo.toml
.\docs\extraction-index\codex-core-agent-parity-map.md
.\docs\extraction-status.md
.\docs\reports\2026-07-10-yunxi-autonomous-agent-runtime-development-report.md
.\docs\reports\2026-07-10-yunxi-stage-4a-runtime-boundary-report.md
.\docs\reports\2026-07-10-yunxi-stage-4b-full-autonomy-development-report.md
.\docs\reports\2026-07-10-yunxi-stage-4c-provider-policy-runtime-report.md
.\docs\reports\2026-07-10-yunxi-stage-4d-codex-core-agent-parity-report.md
.\docs\reports\2026-07-10-yunxi-stage-4e-core-parity-gap-development-report.md
.\docs\reports\2026-07-10-yunxi-stage-4g-full-core-agent-parity-one-pass-development-report.md
.\docs\reports\2026-07-10-yunxi-stage-4h-upstream-core-gap-closure-development-report.md
.\docs\reports\2026-07-10-yunxi-stage-4i-behavior-level-core-parity-closure-development-report.md
.\docs\reports\2026-07-10-yunxi-stage-4j-child-runtime-e2e-parity-development-report.md
.\docs\reports\2026-07-10-yunxi-stage-4k-deepseek-live-provider-parity-development-report.md
.\docs\reports\2026-07-11-yunxi-agent-v1-1-interactive-cli-development-report.md
.\docs\reports\2026-07-11-yunxi-agent-v1-2-deepseek-default-provider-development-report.md
.\docs\reports\2026-07-11-yunxi-stage-4l-codex-agent-deep-parity-closure-development-report.md
.\docs\reports\2026-07-11-yunxi-stage-4m-real-runtime-parity-deepening-development-report.md
.\docs\reports\2026-07-12-yunxi-agent-v1-2-1-interactive-provider-recovery-development-report.md
.\docs\reports\2026-07-12-yunxi-agent-v1-3-terminal-streaming-approval-cancellation-development-report.md
.\docs\reports\2026-07-12-yunxi-agent-v1-4-terminal-command-surface-development-report.md
.\docs\reports\2026-07-12-yunxi-agent-v1-5-honesty-streaming-safety-development-report.md
.\docs\reports\2026-07-12-yunxi-agent-v1-6-policy-guard-hardening-development-report.md
.\docs\reports\2026-07-12-yunxi-agent-v1-7-1-codex-tui-parity-development-report.md
.\docs\reports\2026-07-12-yunxi-agent-v1-7-2-tui-scroll-streaming-development-report.md
.\docs\reports\2026-07-12-yunxi-agent-v1-7-3-tui-event-filtering-development-report.md
.\docs\reports\2026-07-12-yunxi-agent-v1-7-terminal-tui-foundation-development-report.md
.\docs\reports\2026-07-13-yunxi-agent-v1-7-4-tui-scrollbar-viewport-development-report.md
.\docs\reports\2026-07-13-yunxi-agent-v1-7-5-cli-protocol-safety-development-report.md
.\docs\reports\2026-07-13-yunxi-agent-v1-7-6-cli-tui-sandbox-hardening-development-report.md
.\docs\reports\2026-07-13-yunxi-agent-v1-7-7-sandbox-tui-hard-gate-development-report.md
.\docs\reports\2026-07-13-yunxi-agent-v1-7-8-sandbox-schema-execution-policy-hardening-development-report.md
.\docs\reports\2026-07-13-yunxi-agent-v1-8-0-persona-memory-foundation-development-report.md
.\docs\reports\2026-07-13-yunxi-agent-v1-8-1-persona-memory-correctness-transparency-development-report.md
.\docs\reports\2026-07-14-yunxi-agent-v1-8-2-memory-dedup-migration-diagnostics-development-report.md
.\docs\reports\2026-07-17-144543-yunxi-agent-v1-8-7-persona-context-blocks-development-report.md
.\docs\reports\2026-07-17-153715-yunxi-agent-v1-8-8-memory-schema-v3-development-report.md
.\docs\reports\2026-07-17-165118-yunxi-agent-v1-8-9-l0-l3-memory-pipeline-development-report.md
.\docs\reports\2026-07-17-180911-yunxi-agent-v1-9-0-boot-context-recall-router-development-report.md
.\docs\reports\2026-07-17-185913-yunxi-agent-v1-9-1-relationship-graph-lite-development-report.md
.\docs\reports\2026-07-17-220239-yunxi-agent-v1-9-2-proactive-companion-loop-development-report.md
.\docs\reports\2026-07-18-091620-yunxi-agent-v1-9-3-companion-ux-controls-development-report.md
.\docs\reports\2026-07-18-114905-yunxi-agent-v1-9-4-evaluation-harness-development-report.md
.\docs\reports\2026-07-18-144921-yunxi-agent-v2-0-0-general-companion-agent-development-report.md
.\docs\reports\2026-07-19-100611-yunxi-agent-v2-0-2-streaming-remediation-development-report.md
.\docs\reports\2026-07-19-111151-yunxi-agent-v2-0-2-remediation-release-gate-development-report.md
.\docs\reports\2026-07-19-131210-yunxi-agent-v2-0-3-redraw-scroll-resize-development-report.md
.\docs\reports\2026-07-19-151023-yunxi-agent-v2-0-3-redraw-scroll-resize-remediation-development-report.md
.\docs\reports\2026-07-19-173413-yunxi-agent-v2-0-4-i18n-responsive-layout-development-report.md
.\docs\reports\2026-07-19-205131-yunxi-agent-v2-0-4-responsive-header-remediation-development-report.md
.\docs\reports\2026-07-19-220953-yunxi-agent-v2-0-5-tool-approval-error-activity-development-report.md
.\docs\reports\2026-07-20-085931-yunxi-agent-v2-0-5-error-presentation-conpty-remediation-development-report.md
.\docs\reports\2026-07-20-143150-yunxi-agent-v2-0-6-composer-input-recovery-dialog-consistency-development-report.md
.\docs\reports\2026-07-20-213832-yunxi-agent-v2-0-7-interaction-focus-details-shortcut-consistency-development-report.md
.\docs\reports\2026-07-21-083557-yunxi-agent-v2-0-7-hotfix-interaction-focus-details-remediation-development-report.md
.\docs\reports\2026-07-21-162210-yunxi-agent-v2-0-8-visual-semantics-information-density-development-report.md
.\docs\reports\2026-07-21-193509-yunxi-agent-v2-0-9-cross-path-terminal-recovery-streaming-resilience-development-report.md
.\docs\reports\2026-07-22-074211-yunxi-agent-v2-1-0-integrated-release-regression-development-report.md
.\docs\reports\2026-07-22-105338-yunxi-agent-project-directory-organization-report.md
.\docs\superpowers\plans\2026-07-11-yunxi-agent-v1-2-1-interactive-provider-recovery.md
.\docs\superpowers\plans\2026-07-11-yunxi-agent-v1-2-deepseek-default-provider.md
.\docs\superpowers\plans\2026-07-11-yunxi-agent-v1-cli-packaging.md
.\docs\superpowers\plans\2026-07-12-yunxi-agent-v1-3-terminal-streaming-approval-cancellation.md
.\docs\superpowers\specs\2026-07-09-yunxi-codex-source-vendoring-design.md
.\docs\superpowers\specs\2026-07-11-yunxi-agent-v1-2-deepseek-default-provider-design.md
.\docs\superpowers\specs\2026-07-11-yunxi-agent-v1-cli-packaging-design.md
.\extracted\codex-core-agent-sources\manifest.json
.\extracted\codex-core-agent-sources\README.md
.\README.md
.\scripts\README.md
.\scripts\stage4e_migrate_codex_core_sources.ps1
.\vendor\README.md
```

### `extracted/codex-core-agent-sources` 引用文件（13）

```text
.\docs\extraction-index\codex-core-agent-parity-map.md
.\docs\extraction-status.md
.\docs\reports\2026-07-10-yunxi-stage-4g-full-core-agent-parity-one-pass-development-report.md
.\docs\reports\2026-07-10-yunxi-stage-4h-upstream-core-gap-closure-development-report.md
.\docs\reports\2026-07-10-yunxi-stage-4i-behavior-level-core-parity-closure-development-report.md
.\docs\reports\2026-07-10-yunxi-stage-4j-child-runtime-e2e-parity-development-report.md
.\docs\reports\2026-07-10-yunxi-stage-4k-deepseek-live-provider-parity-development-report.md
.\docs\reports\2026-07-11-yunxi-agent-v1-1-interactive-cli-development-report.md
.\docs\reports\2026-07-11-yunxi-stage-4l-codex-agent-deep-parity-closure-development-report.md
.\docs\reports\2026-07-11-yunxi-stage-4m-real-runtime-parity-deepening-development-report.md
.\docs\reports\2026-07-22-105338-yunxi-agent-project-directory-organization-report.md
.\extracted\codex-core-agent-sources\manifest.json
.\scripts\stage4e_migrate_codex_core_sources.ps1
```

### `docs/superpowers` 引用文件（11）

```text
.\docs\reports\2026-07-10-yunxi-autonomous-agent-runtime-development-report.md
.\docs\reports\2026-07-11-yunxi-agent-v1-2-deepseek-default-provider-development-report.md
.\docs\reports\2026-07-12-yunxi-agent-v1-4-terminal-command-surface-development-report.md
.\docs\reports\2026-07-12-yunxi-agent-v1-5-honesty-streaming-safety-development-report.md
.\docs\reports\2026-07-12-yunxi-agent-v1-6-policy-guard-hardening-development-report.md
.\docs\reports\2026-07-12-yunxi-agent-v1-7-1-codex-tui-parity-development-report.md
.\docs\reports\2026-07-12-yunxi-agent-v1-7-terminal-tui-foundation-development-report.md
.\docs\reports\2026-07-22-105338-yunxi-agent-project-directory-organization-report.md
.\docs\superpowers\plans\2026-07-08-yunxi-agent-core-extraction.md
.\docs\superpowers\plans\2026-07-09-yunxi-headless-agent-stack-extraction.md
.\README.md
```

### `scripts/conpty` 引用文件（56）

```text
.\.gitignore
.\docs\development-log.md
.\docs\extraction-status.md
.\docs\reports\2026-07-20-085931-yunxi-agent-v2-0-5-error-presentation-conpty-remediation-development-report.md
.\docs\reports\2026-07-20-124813-yunxi-agent-v2-0-5-hotfix-1-conpty-independent-reaudit-report.md
.\docs\reports\2026-07-20-131709-yunxi-agent-v2-0-5-conpty-native-install-reproducibility-remediation-development-report.md
.\docs\reports\2026-07-20-140910-yunxi-agent-v2-0-5-hotfix-2-release-atomicity-audit-report.md
.\docs\reports\2026-07-20-142235-yunxi-agent-v2-0-5-hotfix-2-reaudit-report.md
.\docs\reports\2026-07-20-143150-yunxi-agent-v2-0-6-composer-input-recovery-dialog-consistency-development-report.md
.\docs\reports\2026-07-20-213832-yunxi-agent-v2-0-7-interaction-focus-details-shortcut-consistency-development-report.md
.\docs\reports\2026-07-21-082757-yunxi-agent-v2-0-7-interaction-focus-details-audit-report.md
.\docs\reports\2026-07-21-083557-yunxi-agent-v2-0-7-hotfix-interaction-focus-details-remediation-development-report.md
.\docs\reports\2026-07-21-115531-yunxi-agent-v2-0-7-hotfix-focus-routing-reaudit-report.md
.\docs\reports\2026-07-21-162210-yunxi-agent-v2-0-8-visual-semantics-information-density-development-report.md
.\docs\reports\2026-07-21-190909-yunxi-agent-v2-0-8-visual-semantics-audit-report.md
.\docs\reports\2026-07-21-193509-yunxi-agent-v2-0-9-cross-path-terminal-recovery-streaming-resilience-development-report.md
.\docs\reports\2026-07-22-072005-yunxi-agent-v2-0-9-terminal-recovery-streaming-resilience-audit-report.md
.\docs\reports\2026-07-22-074211-yunxi-agent-v2-1-0-integrated-release-regression-development-report.md
.\docs\reports\2026-07-22-100333-yunxi-agent-v2-1-0-integrated-release-audit-report.md
.\docs\reports\2026-07-22-105338-yunxi-agent-project-directory-organization-report.md
.\docs\reports\evidence\2026-07-20-v2-0-5-error-presentation-conpty-evidence.md
.\docs\reports\evidence\2026-07-20-v2-0-6-composer-input-conpty-evidence.md
.\docs\reports\evidence\2026-07-21-v2-0-7-hotfix-focus-routing-conpty-evidence.md
.\docs\reports\evidence\2026-07-21-v2-0-7-interaction-focus-conpty-evidence.md
.\docs\reports\evidence\2026-07-21-v2-0-8-visual-semantics-conpty-evidence.md
.\docs\reports\evidence\2026-07-21-v2-0-9-terminal-streaming-resilience-conpty-evidence.md
.\docs\reports\evidence\frames\v205-conpty\manifest.json
.\docs\reports\evidence\frames\v206-conpty\manifest.json
.\docs\reports\evidence\frames\v207-conpty\manifest.json
.\docs\reports\evidence\frames\v207-hotfix-conpty\manifest.json
.\docs\reports\evidence\frames\v208-conpty\manifest.json
.\docs\reports\evidence\frames\v209-conpty\manifest.json
.\docs\reports\evidence\frames\v210-conpty\manifest.json
.\docs\tui-presentation.md
.\README.md
.\scripts\conpty\v205\capture.js
.\scripts\conpty\v205\README.md
.\scripts\conpty\v206\capture.js
.\scripts\conpty\v206\README.md
.\scripts\conpty\v206\verify.js
.\scripts\conpty\v207-hotfix\capture.js
.\scripts\conpty\v207-hotfix\README.md
.\scripts\conpty\v207-hotfix\verify.js
.\scripts\conpty\v207\capture.js
.\scripts\conpty\v207\README.md
.\scripts\conpty\v207\verify.js
.\scripts\conpty\v208\capture.js
.\scripts\conpty\v208\README.md
.\scripts\conpty\v208\verify.js
.\scripts\conpty\v209\capture.js
.\scripts\conpty\v209\README.md
.\scripts\conpty\v209\verify.js
.\scripts\conpty\v210\capture.js
.\scripts\conpty\v210\README.md
.\scripts\conpty\v210\verify.js
.\scripts\README.md
```

## 四、阶段 0 结论

- 未移动、重命名或删除任何文件或目录。
- 未修改 Cargo workspace、Rust 源码、安装目录、PATH、用户目录或 Git tag。
- `vendor/`、`extracted/`、`docs/superpowers/` 与 `scripts/conpty/` 仍有广泛引用，因此本轮禁止迁移。
- 三份正式文档作为阶段 0 资产保留；ConPTY `node_modules/` 保留在磁盘上，等待阶段 1 仅通过忽略规则收紧 Git 边界。
- 后续阶段只允许在独立提交中修改 `.gitignore`、文档索引和根 README 链接。

署名：开发者
