# YunXi Agent v2.1.1 项目目录治理、Git 忽略边界与文档索引基线开发报告

- 撰写时间：2026-07-22 13:01:40 +08:00
- 审核依据：`C:\Users\24763\Desktop\YunXi Agent审核报告\2026-07-22-120829-YunXi-Agent-v2.1.0-进入v2.1.1开发准入审核报告.md`
- 项目内审核报告归档：`D:\YunXi Agent\docs\reports\audits\2026-07-22-120829-yunxi-agent-v2-1-0-v2-1-1-entry-audit-report.md`
- 总纲正本：`D:\YunXi Agent\docs\superpowers\plans\2026-07-22-yunxi-agent-v2-1-1-to-v2-2-0-personal-wechat-roadmap.md`
- 总纲正本 SHA-256：`2DF30D503F46CFE7496567F5011BF5CBFB8BA73C91C2D2FA3E9F29AF0932EA0F`
- 桌面路线图副本：`C:\Users\24763\Desktop\2026-07-22-yunxi-agent-v2-1-1-to-v2-2-0-personal-wechat-roadmap.md`
- 桌面路线图副本 SHA-256：`ECE3375748A6EF4BD70DCA03FFB65941AE2F566CCC32FBCC12505DB511987372`
- 文档状态：项目内总纲正本与桌面副本当前不一致；本报告以项目内正本与准入审核报告为准。
- 审核结论转化：`v2.1.0` 审核通过，可以进入总纲图 `v2.1.1` 开发。
- 发布基线：`v2.1.0`，发布提交 `a8293905af55d659d647515786699ab313a51a07`，annotated tag object `c42ca8b4e2837dcff1e8ae0cd3860936c947875d`。
- 开发目录：`D:\YunXi Agent`
- 报告类型：下一阶段开发指令报告，主要面向开发者；本报告不是完成声明。

## 一、硬性约束

以下约束是 `v2.1.1` 及后续版本开发、验证、发布和文档同步的前置要求，不得因为上下文过长、任务拆分、实现困难或局部修复而忽略、弱化或绕过。

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

## 二、阶段目标

`v2.1.1` 的主题是项目目录治理、Git 忽略边界与文档索引基线。该版本是后续微信接入发布线的工程地基，不实现微信网络、登录、凭证存储、长轮询、会话桥接或新的通道 Runtime。

本阶段必须交付：

1. 根目录资产清单：把稳定源码、参考输入、文档、验证脚本、本地状态和可再生产物分别标注用途、是否应跟踪、是否可清理和当前引用方。
2. `.gitignore` 目录级规则补齐：统一忽略 `/.tmp/`、`/scripts/conpty/**/node_modules/`、`/scripts/conpty/**/.work/` 等本地产物，同时确认不隐藏已跟踪文件。
3. 文档入口：新增或完善 `docs/README.md` 或 `docs/index.md`，从根 README 可在两步内进入路线图、报告索引、ConPTY 说明和提取索引。
4. 报告落位规则：后续审核报告进入 `docs/reports/audits/`，开发报告进入 `docs/reports/development/`，证据继续保留在 `docs/reports/evidence/`；本版本不迁移历史报告。
5. `scripts/conpty/` 总览：列出各版本脚本目录、验证场景、依赖、输出目录和 evidence 链接，不抽取共享代码，不改变既有 `npm run verify` 路径。
6. 可再生产物清理候选清单：记录 `target`、`.tmp`、`.yunxi`、ConPTY `node_modules/.work` 等的绝对路径、性质和清理条件，但不得执行清理。
7. 项目内路线图正本与桌面副本的状态必须可追溯；若无法同步桌面副本，必须明确记录副本滞后。

## 三、版本与发布边界

当前有效结论是：`v2.1.0` 稳定，允许进入 `v2.1.1`。本版本只允许目录治理、Git 忽略规则和文档索引基线工作。

必须遵守：

- 不新增 `crates/yunxi-agent-weixin`；微信 crate、CLI 骨架和 iLink 协议客户端推迟到项目内正本规定的 `v2.1.2`。
- 不修改 `Cargo.toml`、`Cargo.lock`、Rust 源码、TUI、Runtime、Provider、Storage、Protocol 或 CLI 行为。
- 不移动、不删除、不重命名 `crates/`、`evals/`、`scripts/`、`docs/`、`vendor/`、`extracted/`、`docs/reports/evidence/` 和既有 ConPTY 版本目录。
- 不使用 `git clean`、递归删除或宽泛通配符实现“干净目录”；清理必须另行取得用户对精确绝对路径的确认。
- `v2.1.1` 完成后必须创建新的发布提交和新的 annotated `v2.1.1` tag；`v2.1.0` 及全部历史 tag 不得移动、删除或覆盖。

## 四、源码与目录接入点

### 1. `.gitignore`

当前 `.gitignore` 已忽略 `target`、`.yunxi`、`.codegraph`、`.worktrees` 以及 v205/v206 的 ConPTY 本地产物，但未统一覆盖 v207、v207-hotfix、v208、v209、v210 的 `node_modules` 和 `.work`。

整改要求：

- 增加目录级通配规则，优先使用 `/scripts/conpty/**/node_modules/` 和 `/scripts/conpty/**/.work/`，避免逐版本追加。
- 增加或确认 `/.tmp/` 规则，用于审计和 ConPTY 临时输出。
- 修改前后必须执行 `git ls-files` 检查，确认新规则没有隐藏已跟踪文件。
- 使用 `git check-ignore -v` 证明现存 ConPTY `node_modules/` 和 `.work/` 已由统一规则忽略。

### 2. `README.md`

根 README 只保留项目级入口和核心能力摘要，不再继续堆叠所有报告、路线图和证据说明。

整改要求：

- 增加到 `docs/README.md` 或 `docs/index.md` 的稳定入口。
- 保持 `v2.1.0` 当前能力描述，不借目录治理版本宣称微信已可用。
- 如提到下一阶段，应明确 `v2.1.1` 是目录治理，`v2.1.2` 才进入微信模块/CLI 骨架。

### 3. `docs/README.md` 或 `docs/index.md`

该文档应成为长期文档导航。

建议结构：

- 架构与项目状态：`README.md`、`docs/extraction-status.md`、`docs/tui-presentation.md`
- 路线图：`docs/superpowers/plans/`
- 审核报告：`docs/reports/audits/`
- 开发报告：`docs/reports/development/`
- 历史报告兼容区：`docs/reports/` 顶层旧报告
- 证据：`docs/reports/evidence/`
- 脚本与 ConPTY：`scripts/README.md`、`scripts/conpty/`
- 提取索引：`docs/extraction-index/`

### 4. `docs/reports/README.md`

该文档应定义报告命名和落位规则。

整改要求：

- 明确新报告落位：审核报告进入 `audits/`，开发报告进入 `development/`，证据进入 `evidence/`。
- 明确历史顶层报告暂不迁移，避免断开既有 Markdown 引用。
- 说明每份报告应包含来源、版本、结论、验证状态和署名。
- 说明桌面副本只是分发件，项目内文档为正本。

### 5. `scripts/README.md` 与 `scripts/conpty/`

当前 `scripts/README.md` 已有 v205 至 v210 说明。`v2.1.1` 应把这些内容整理成可持续索引，不改变脚本行为。

整改要求：

- 列出每个 ConPTY 版本目录、场景用途、依赖安装方式、capture/verify 输出目录和正式 evidence 链接。
- 明确 Node.js 依赖只属于证据采集，不进入 YunXi 默认运行路径。
- 保持 v209/v210 capture/verify 分离语义，不改默认验证路径。

### 6. `docs/superpowers/plans/`

项目内路线图正本必须是唯一可编辑来源。

整改要求：

- 在文档索引中明确 `D:\YunXi Agent\docs\superpowers\plans\2026-07-22-yunxi-agent-v2-1-1-to-v2-2-0-personal-wechat-roadmap.md` 是正本。
- 记录本次桌面副本 SHA-256 与项目正本不一致：`ECE337...` vs `2DF30...`。
- 不在桌面和项目内分别修改路线图；需要同步时，应从项目内正本生成桌面分发副本并做 SHA-256 校验。

## 五、参考源码建议

本次 `v2.1.1` 只需要参考本机已有 `D:\源码\reasonix` 的目录治理方式，不需要拉取新的源码项目。

重点参考：

1. `D:\源码\reasonix\.gitignore`：构建产物、Node 依赖、局部状态、CodeGraph 索引统一忽略。
2. `D:\源码\reasonix\docs\`：用户文档、规格、发布、指南和图片资产的分类方式。
3. `D:\源码\reasonix\scripts\`：脚本入口与说明文档的边界。
4. `D:\源码\reasonix\REASONIX.md`：短小、稳定、可长期加载的项目约束文档。

不得照搬：

- Go 项目的 `cmd/internal` 外形；
- Reasonix 的站点、桌面端、发布系统；
- Go/Python/TypeScript 源码；
- 任何会改变 YunXi Cargo workspace、crate 边界或默认运行路径的结构。

`Tencent/openclaw-weixin`、CowAgent、OpenAkita、Leon、Letta Code、Project N.E.K.O. 是后续微信功能版本的参考来源，不属于 `v2.1.1` 目录治理的拉取要求。本次报告不要求新拉取这些项目。

## 六、推荐执行顺序

开发者应按目录治理批量推进，不要在单个空目录或单个历史报告上反复纠结：

1. 生成根目录资产清单，记录顶层目录用途、文件量、是否跟踪、是否可清理和引用方。
2. 扫描已跟踪文件，确认新增 `.gitignore` 规则不会隐藏已跟踪资产。
3. 补齐 `.gitignore` 的统一规则，但不删除本地已存在的生成物。
4. 创建或更新 `docs/README.md`、`docs/reports/README.md` 和 `scripts/README.md` 的索引内容。
5. 根 README 只增加文档索引入口，不把全部目录说明堆回根文档。
6. 记录项目内路线图正本与桌面副本的 SHA-256 状态；能同步时同步，不能同步时记录原因。
7. 运行轻量验证：`git check-ignore -v`、`git ls-files` 引用检查、Markdown 链接抽查、`git diff --check`。
8. 如需执行 Rust 回归，等一批文档治理完成后统一运行；目录治理本身不得用测试绿灯替代文档准确性。
9. 阶段结束时列出清理候选，但清理前必须取得用户对精确绝对路径的确认。
10. 记录日志，完成提交和新的 annotated `v2.1.1` tag。

## 七、测试与验收清单

完成本阶段批量整改后，至少执行以下验证：

| 验证项 | 目的 |
| --- | --- |
| `git ls-files` 与新增 `.gitignore` 规则交叉检查 | 确认没有已跟踪文件被忽略规则隐藏 |
| `git check-ignore -v scripts/conpty/v207/node_modules` 等 | 确认 ConPTY 本地依赖由统一规则忽略 |
| `git status --short` | 确认 ConPTY `node_modules/` 不再显示为未跟踪 |
| Markdown 链接抽查 | 确认新增索引可达路线图、报告、证据和脚本说明 |
| `git diff --check` | 确认无空白和行尾问题 |
| `cargo fmt --all -- --check` | 如触发 Rust/配置格式相关工作，确认格式无漂移 |
| `cargo check --workspace` / `cargo test --workspace` | 若审查方要求继承功能基线，统一运行，不得频繁碎片化验证 |

验收必须证明：

- 本版本修改只限 `.gitignore`、README、索引、报告等文档治理文件。
- `Cargo.toml`、Rust 源码、`vendor/`、`extracted/`、历史 evidence 和 ConPTY 版本脚本没有被移动、删除或重写。
- 从根 README 两步内可进入当前路线图、报告索引、ConPTY 说明和提取索引。
- 项目内路线图正本路径和 SHA-256 被记录；桌面副本同步状态被如实记录。
- 未通过验证前不得宣称 `v2.1.1` 完成。

## 八、清理、日志与发布纪律

本阶段会识别清理候选，但不得直接清理。以下类型必须先列出精确绝对路径并取得用户确认：

- `D:\YunXi Agent\target`
- `D:\YunXi Agent\.tmp`
- `D:\YunXi Agent\.yunxi`
- `D:\YunXi Agent\scripts\conpty\*\node_modules`
- `D:\YunXi Agent\scripts\conpty\*\.work`
- 其他构建、采集、缓存或临时状态目录

禁止使用 `git clean`、宽泛递归删除、强制移动、清空目录、系统级安装/卸载、PATH/注册表/系统配置修改。

日志必须追加到：

- `D:\YunXi Agent\docs\development-log.md`
- `C:\Users\24763\Desktop\YunXi Agent开发日志.md`

日志必须包含时间戳、工作目标、执行流程、修改文件、文件路径、验证结果、提交和推送状态，并在结尾署名为开发报告撰写者。

## 九、当前任务状态

本报告只完成 `v2.1.1` 的开发指令撰写，没有修改 YunXi Rust 源码，没有运行构建、测试、ConPTY 采集或发布流程。本次依据的项目内总纲正本与桌面路线图副本不一致，后续开发与审核必须以项目内正本为准，并在可同步时将桌面副本作为分发件重新生成。

署名：开发报告撰写者

## 十、实施结果与发布前状态

实施时间：2026-07-22 15:04:21 +08:00

### 1. 实施结果

1. 新增 `docs/directory-governance.md`，以 6,674 个已跟踪文件为基线，分类记录根配置、`crates/`、`evals/`、`scripts/`、`docs/`、`vendor/`、`extracted/` 的用途、跟踪边界、清理纪律和引用方；另行记录 `target`、`.tmp`、两处 `.yunxi`、`.codegraph`、`.worktrees`、ConPTY `node_modules/.work` 的绝对路径、状态和清理条件。
2. 根 `.gitignore` 在本次实施开始前已经具备 `/.tmp/`、`/scripts/conpty/**/node_modules/` 和 `/scripts/conpty/**/.work/` 统一规则，因此没有为了制造差异而重复修改。交叉检查确认上述模式匹配 0 个已跟踪文件，v207、v207-hotfix、v208、v209、v210 的依赖探针和 v207/v210 `.work` 探针均由统一规则命中。
3. 更新根 `README.md`，明确 `v2.1.1` 仅为目录治理，微信模块/CLI 骨架和确定性 iLink Mock 从 `v2.1.2` 开始；保持 `v2.1.0` 现有能力描述，不宣称微信可用。
4. 更新 `docs/README.md` 和 `docs/reports/README.md`，增加当前准入审核、开发报告、治理基线、路线图正本哈希、报告落位/署名/桌面分发规则，以及到 ConPTY 总览的直接入口。从根 README 两步内可到路线图、报告、ConPTY 和提取索引。
5. `scripts/conpty/README.md` 现有 v205 至 v210 矩阵已完整覆盖场景、依赖、capture/verify、输出目录和 evidence，本次不抽取共享代码、不改脚本；仅在 `scripts/README.md` 明确 Node.js 依赖不进入默认运行路径，并移除安装器说明中的陈旧固定版本号。
6. 项目内路线图继续作为唯一可编辑正本，SHA-256 保持 `2DF30D503F46CFE7496567F5011BF5CBFB8BA73C91C2D2FA3E9F29AF0932EA0F`。桌面副本同步前为 `ECE3375748A6EF4BD70DCA03FFB65941AE2F566CCC32FBCC12505DB511987372`，本次经授权从正本精确覆盖后变为 `2DF30D503F46CFE7496567F5011BF5CBFB8BA73C91C2D2FA3E9F29AF0932EA0F`，已与正本一致。
7. 本次没有新增或迁移历史报告；v2.1.0 的两份分类归档是本报告实施前已存在的治理基线。准入审核报告和本开发报告按新规则分别落在 `audits/` 与 `development/`。

### 2. 修改与新增路径

- `D:\YunXi Agent\README.md`
- `D:\YunXi Agent\docs\README.md`
- `D:\YunXi Agent\docs\directory-governance.md`
- `D:\YunXi Agent\docs\reports\README.md`
- `D:\YunXi Agent\scripts\README.md`
- `D:\YunXi Agent\docs\superpowers\plans\2026-07-22-yunxi-agent-v2-1-1-to-v2-2-0-personal-wechat-roadmap.md`
- `D:\YunXi Agent\docs\reports\audits\2026-07-22-120829-yunxi-agent-v2-1-0-v2-1-1-entry-audit-report.md`
- `D:\YunXi Agent\docs\reports\development\2026-07-22-130141-yunxi-agent-v2-1-1-directory-governance-development-report.md`
- `D:\YunXi Agent\docs\development-log.md`
- 桌面分发副本：`C:\Users\24763\Desktop\2026-07-22-yunxi-agent-v2-1-1-to-v2-2-0-personal-wechat-roadmap.md`

### 3. 验证结果

- 46 个关键 Markdown 本地链接全部存在；根 README 两步入口 4/4 可达。
- 七组 ConPTY 只读 verifier 全部通过：v205/v206/v207 各 8 个场景，v207-hotfix/v208 各 2 个场景，v209/v210 分别保持正式 evidence SHA-256 `714b9c2d9c01b1616ffc8789e940ea778559335e20998679a904b5c335fcfdc8` 与 `a291a66cf91ee788bba9944edbafbf4c8dfce43bcd2d6c7b2cc78bc6c2990fc6`；未产生 evidence 或脚本改动。
- `cargo fmt --all -- --check`、`cargo check --workspace --offline`、`cargo test --workspace --offline` 全部通过；CLI integration 45/45、JSONL 10/10、TUI 161/161 等无失败。
- 默认 `yunxi-agent-cli` 依赖树 457 行，`yunxi-agent-codex`/`codex-*` 依赖数为 0；`crates/yunxi-agent-weixin` 不存在。
- `Cargo.toml`、`Cargo.lock`、Rust 源码、`evals/`、`vendor/`、`extracted/`、历史 evidence 和 ConPTY 版本脚本变更数为 0；`git diff --check` 通过。

### 4. 清理候选与边界

本版本按报告要求只记录、不清理。统一测试后：`D:\YunXi Agent\target` 仍为 8,114 个文件、2,507,699,882 字节；`.tmp` 为 158,810 字节且包含审计采集资产；项目 `.yunxi` 为 14,601 字节；CLI `.yunxi` 为 6,240 字节；`.codegraph` 为 192,658,181 字节；`.worktrees` 为空但仍受 Git worktree 边界保护；v210 部分 `node_modules` 为 32,558,228 字节，v205 至 v209 `node_modules` 和全部 `.work` 均不存在。未执行删除、递归清理、目录移动、`git clean`、系统配置修改或任何用户目录清理。

### 5. 提交、推送与 tag 状态

发布前本地基线为 `HEAD=origin/master=d6b6132ce9ad73b980f0208b618959671889fbf4`，本地 tag 共 50 个且 `v2.1.1` 不存在。上述实施资产待形成新的发布提交和 annotated `v2.1.1` tag；发布必须非强制执行，`v2.1.0` 及全部历史 tag 不移动、不删除、不覆盖。

署名：开发报告撰写者

## 十一、发布结果

发布时间：2026-07-22 15:12:34 +08:00

- 发布提交：`58fb10f2f9e192056dea2660f34fc5c1bd8232b5`
- 发布 tree：`7c9060746fd0354f005eb3fa283497bef9e81a70`
- 发布父提交：`d6b6132ce9ad73b980f0208b618959671889fbf4`
- annotated tag：`v2.1.1`
- tag object：`75c4169d09344a359239f820ca89f052408d1e76`
- tag target：`58fb10f2f9e192056dea2660f34fc5c1bd8232b5`
- tagger：`开发者 <developer@yunxi-agent.local>`
- GitHub 远程 master：`58fb10f2f9e192056dea2660f34fc5c1bd8232b5`
- GitHub tag 总数：51
- 原 50 个历史 tag SHA 变化数：0
- force push：未使用

发布前远程 master 严格等于预期基线，远程不存在 `v2.1.1`；分支和新 tag 通过 `git push --atomic` 原子、非强制发布。GitHub API key 仅在单个进程的临时环境变量中使用，未输出、未写入仓库或 Git 配置，退出时已清除。发布后的报告与日志收口只更新 master，不移动、删除或覆盖 `v2.1.1` 及任何历史 tag。

清理状态保持不变：本版本没有执行删除、递归清理、目录移动、`git clean` 或用户目录清理；测试产物和本地状态继续按治理基线列为需单独授权的候选。

署名：开发报告撰写者
