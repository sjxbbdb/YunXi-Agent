# YunXi Agent v2.1.1 历史报告路径整改开发报告

- 撰写时间：2026-07-22 15:53:06 +08:00
- 审核依据：`C:\Users\24763\Desktop\YunXi Agent审核报告\2026-07-22-154559-YunXi-Agent-v2.1.1-审核报告.md`
- 项目内审核报告归档：`D:\YunXi Agent\docs\reports\audits\2026-07-22-154559-yunxi-agent-v2-1-1-audit-report.md`
- 审核报告 SHA-256：`C31EF3846A50294FFC5C10BD6CEF3668CA1BC29A3035586232ABF9DDAA0291C6`
- 总纲正本：`D:\YunXi Agent\docs\superpowers\plans\2026-07-22-yunxi-agent-v2-1-1-to-v2-2-0-personal-wechat-roadmap.md`
- 参考源码依据：`D:\源码\reasonix`
- 当前工作树基线：`HEAD=3f9f1ca81906aecb5660c4cacf69215b6cc983e9`，`git describe=v2.1.1-1-g3f9f1ca-dirty`
- 审核结论转化：`v2.1.1` 审核不通过，暂不可进入 `v2.1.2`。
- 整改版本建议：由于 `v2.1.1` annotated tag 已存在且不得移动、覆盖或删除，本次整改应使用新的 `v2.1.1-hotfix.1` annotated tag 承接，完成复审后再进入 `v2.1.2`。
- 开发目录：`D:\YunXi Agent`
- 报告类型：面向开发者的整改开发指令；本报告不是完成声明。

## 一、硬性约束

以下约束是本次 `v2.1.1` 整改、复审、发布和后续 `v2.1.2` 准入的前置条件，不得因为上下文过长、任务拆分、实现困难或局部修复而忽略、弱化或绕过。

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

## 二、审核结论与阶段目标

本次审核只发现一个阻塞点：`v2.1.1` 实际差异迁移了一份历史 `v2.1.0` 开发报告，违反项目内总纲对 `v2.1.1` 的明确要求：本版本不迁移历史报告，以避免断开既有 Markdown 引用。

已通过但不得替代阻塞整改的内容包括：

- 源码边界、Rust 回归、CLI/TUI/Provider 基线通过。
- `.gitignore` 统一规则和已跟踪文件隐藏风险通过。
- 根 README 两步文档导航、本地 Markdown 链接、ConPTY v210 只读 verifier 通过。
- 项目内路线图正本与桌面副本 SHA-256 一致。
- `v2.1.1` 是 annotated tag，历史 tag 未移动。

整改目标不是继续做微信功能，也不是进入 `v2.1.2`。开发者必须先恢复历史报告的保留语义，解决旧路径与新分类路径之间的单一正本问题，并重新提交复审。复审未通过前，不得宣称 `v2.1.1` 完成，也不得启动 `crates/yunxi-agent-weixin` 或 iLink 协议开发。

## 三、阻塞点技术拆解

审核报告给出的精确状态如下：

| 项目 | 当前状态 |
| --- | --- |
| 历史旧路径 | `D:\YunXi Agent\docs\reports\2026-07-22-074211-yunxi-agent-v2-1-0-integrated-release-regression-development-report.md` 不存在 |
| 新分类路径 | `D:\YunXi Agent\docs\reports\development\2026-07-22-074211-yunxi-agent-v2-1-0-integrated-release-regression-development-report.md` 存在 |
| Git 识别 | `git diff --find-renames=90% v2.1.0..v2.1.1 -- docs/reports` 显示 `R092` rename |
| 关联提交 | `ddd4cd4 docs: migrate v2.1.0 reports into typed archives` |
| 冲突点 | 总纲要求 `v2.1.1` 不迁移历史报告，当前实际状态已经迁移一份历史开发报告 |

活动 Markdown 链接当前没有断裂，迁移映射也记录了路径和哈希，但这两点不能替代总纲中的“不迁移历史报告”硬约束。开发者必须让版本差异重新满足“历史报告继续保留在原路径”的语义。

## 四、整改边界

### 必须做

1. 将历史 `v2.1.0` 开发报告恢复到原始顶层路径：
   - `D:\YunXi Agent\docs\reports\2026-07-22-074211-yunxi-agent-v2-1-0-integrated-release-regression-development-report.md`
2. 解决新分类路径与旧路径之间的单一正本问题：
   - 不允许让旧路径和 `development/` 路径各自成为会继续演进的正本。
   - 推荐做法是恢复旧路径为唯一正本，并从版本化树中移除本次误迁移产生的新分类路径。
   - 若开发者选择保留辅助说明，必须只保留指向旧路径的索引记录或迁移回滚说明，不保留第二份报告正文。
3. 更新所有当前索引和状态文档，让它们与恢复后的实际路径一致：
   - `D:\YunXi Agent\docs\reports\README.md`
   - `D:\YunXi Agent\docs\README.md`
   - `D:\YunXi Agent\docs\directory-governance.md`
   - `D:\YunXi Agent\docs\reports\2026-07-22-111558-yunxi-agent-v2-1-0-report-path-migration-map.md`
   - 其他通过引用扫描发现的 Markdown 文件。
4. 使用引用扫描确认仓库内没有活动链接继续指向被移除的新分类路径，除非该链接明确是“历史迁移回滚说明”。
5. 重新生成开发/审核日志，记录本次路径整改、验证、提交、推送和 tag 状态。

### 禁止做

- 不得移动、删除或重命名 Rust 源码、Cargo 文件、`vendor/`、`extracted/`、`evals/`、ConPTY 版本脚本或历史 evidence。
- 不得移动、覆盖或删除 `v2.1.1` tag。
- 不得用 `git clean`、宽泛递归删除、清空目录或强制移动来处理报告路径。
- 不得借本次修复新增微信 crate、CLI 命令骨架、iLink 登录、长轮询、凭证存储或 Runtime 桥接。
- 不得把测试通过、链接未断或迁移映射完整写成“审核通过”；复审通过前只能写“整改候选已完成”。

## 五、推荐执行流程

开发者应一次性完成本批路径治理，再统一验证。

1. 固定基线：
   - 记录当前 `HEAD`、`git status --short`、`git describe --tags --always --dirty`。
   - 确认 `v2.1.1` tag object 与 target，不移动、不覆盖。
2. 扫描引用：
   - 搜索旧路径文件名和新路径文件名在 `README.md`、`docs/`、`scripts/` 中的全部引用。
   - 特别检查 `docs/reports/README.md`、`docs/README.md`、`docs/directory-governance.md` 和迁移映射报告。
3. 恢复历史路径：
   - 将误迁移的历史报告恢复到 `docs/reports/` 顶层旧路径。
   - 如果操作涉及移动或移除文件，必须仅限 `D:\YunXi Agent` 内精确绝对路径；如使用强制移动、删除或清空命令，必须先取得用户确认。
4. 消除双正本：
   - 推荐让旧路径保留为唯一正本，`docs/reports/development/` 只接收 `v2.1.1` 之后新生成的开发报告。
   - 更新索引，不让“当前入口”继续指向误迁移路径。
5. 更新迁移映射：
   - 记录该历史报告迁移已被回滚或撤销。
   - 明确 `v2.1.1-hotfix.1` 的目标是恢复总纲语义，而不是继续扩大历史报告迁移。
6. 统一验证：
   - 文档路径、Markdown 链接、Git rename 差异和忽略规则验证通过后，再运行 Rust/ConPTY/TUI/Provider 继承基线复核。
7. 发布收口：
   - 提交整改 commit。
   - 创建新的 annotated `v2.1.1-hotfix.1` tag。
   - 非强制推送 commit 与新 tag；不得移动 `v2.1.1` 或任何历史 tag。
8. 复审：
   - 复审报告必须明确 `v2.1.1-hotfix.1` 是否解决历史报告迁移阻塞。
   - 复审通过后才允许进入总纲 `v2.1.2`。

## 六、源码与文档接入点

本次整改是文档治理和 Git 历史边界修复，不涉及 Rust 代码改动。开发者应聚焦以下路径：

| 路径 | 整改职责 |
| --- | --- |
| `D:\YunXi Agent\docs\reports\2026-07-22-074211-yunxi-agent-v2-1-0-integrated-release-regression-development-report.md` | 恢复历史报告旧路径，作为该历史报告唯一正本。 |
| `D:\YunXi Agent\docs\reports\development\2026-07-22-074211-yunxi-agent-v2-1-0-integrated-release-regression-development-report.md` | 误迁移产生的新分类路径；整改后不应继续作为第二正本。 |
| `D:\YunXi Agent\docs\reports\README.md` | 更新报告索引和历史兼容说明，确保“新报告分类、历史报告不迁移”的规则自洽。 |
| `D:\YunXi Agent\docs\README.md` | 确保文档总索引指向实际存在路径。 |
| `D:\YunXi Agent\docs\directory-governance.md` | 更新目录治理资产清单，记录历史报告恢复后的分类边界。 |
| `D:\YunXi Agent\docs\reports\2026-07-22-111558-yunxi-agent-v2-1-0-report-path-migration-map.md` | 增加整改说明，避免迁移映射继续被误读为当前有效迁移计划。 |
| `D:\YunXi Agent\docs\development-log.md` | 追加整改实施日志，记录时间戳、流程、验证、提交、推送和 tag。 |

除上述路径和由引用扫描明确命中的 Markdown 文件外，不应扩大修改范围。

## 七、参考源码建议

审核报告只要求参考本机已有 `D:\源码\reasonix`，当前本机路径存在，不需要拉取新源码。

建议参考点：

1. `D:\源码\reasonix\.gitignore`：参考本地产物、Node 依赖、临时状态和索引目录的统一忽略方式。
2. `D:\源码\reasonix\docs\`：参考文档分类、长期导航和报告入口稳定性。
3. `D:\源码\reasonix\scripts\`：参考脚本说明与执行入口的边界。
4. `D:\源码\reasonix\REASONIX.md`：参考短小稳定的项目级约束入口。

不得照搬 Reasonix 的 Go `cmd/internal` 外形，也不得复制 Go 代码。YunXi 继续保持 Cargo workspace 和 `crates/` 边界。

本次整改不需要拉取 Tencent/openclaw-weixin、CowAgent、OpenAkita、Leon、Letta Code 或 Project N.E.K.O.。这些项目属于后续微信功能版本的参考输入，不属于 `v2.1.1` 路径整改范围。

## 八、验证与复审清单

整改完成后至少运行以下验证：

| 验证项 | 必须结论 |
| --- | --- |
| `Test-Path docs/reports/2026-07-22-074211-yunxi-agent-v2-1-0-integrated-release-regression-development-report.md` | 返回 `True`。 |
| `Test-Path docs/reports/development/2026-07-22-074211-yunxi-agent-v2-1-0-integrated-release-regression-development-report.md` | 推荐返回 `False`，除非开发者用只读说明而非正文副本证明不会形成双正本。 |
| `git diff --find-renames=90% v2.1.0..HEAD -- docs/reports` | 不再显示该历史开发报告从旧路径迁移到 `development/` 的 `R092` 记录。 |
| Markdown 链接检查 | 活动 Markdown 链接失效数为 0，且关键入口指向实际正本。 |
| `git diff --check` | 通过。 |
| `git ls-files -ci --exclude-standard -- scripts/conpty` | 返回 0 个已跟踪且被忽略文件。 |
| `git check-ignore -v` | 现存 ConPTY `node_modules/` 仍由统一规则命中。 |
| `cargo fmt --all -- --check` | 通过。 |
| `cargo check --workspace --offline` | 通过。 |
| `cargo test --workspace --offline` | 通过。 |
| ConPTY v210 只读 verifier | `ok=true`、`read_only=true`，不得生成新的正式 evidence 覆盖旧证据。 |

如果整改只涉及 Markdown 文件，Rust、TUI、Provider、ConPTY 可以在本批文档修复完成后统一执行；不要在单个索引文件上反复运行完整测试。

## 九、清理、日志与发布纪律

本次整改不得执行实际清理。阶段末尾只允许记录清理候选；任何清理前必须列出精确绝对路径并再次取得用户确认。

日志必须追加到：

- `D:\YunXi Agent\docs\development-log.md`
- `C:\Users\24763\Desktop\YunXi Agent开发日志.md`

日志必须包含时间戳、工作目标、执行流程、修改文件、文件路径、验证结果、提交和推送状态，并在结尾署名为开发报告撰写者。

发布纪律：

- 不能移动、删除、覆盖 `v2.1.1` tag。
- 整改完成后使用新的 annotated `v2.1.1-hotfix.1` tag。
- 推送必须非强制；历史 tag SHA 必须保持不变。
- 复审报告未写明通过前，不得进入 `v2.1.2`。

## 十、当前任务状态

本报告只完成整改开发指令撰写和文档同步要求，没有修改 YunXi Rust 源码，没有执行历史报告恢复、删除、移动、构建、测试、ConPTY 采集、提交、推送或 tag 创建。本次依据的参考源码 `D:\源码\reasonix` 已在本机存在，因此没有拉取新源码。

署名：开发报告撰写者

## 十一、整改实施结果与发布前状态

实施时间：2026-07-22 16:06:09 +08:00

### 1. 路径与文档整改

1. 使用单一、精确且不带强制参数的 `git mv`，将历史 v2.1.0 开发报告从误迁移后的 `docs/reports/development/` 回迁到原始 `docs/reports/` 路径；移动前后 SHA-256 均为 `B70BF0D3F3BBDEEB7DE1DB515D8FA60B09BB161F63C47160AF28997EB61D0413`。
2. 整改后旧路径存在，新分类路径不存在，没有复制正文或形成双正本；相对 `v2.1.0` 的候选差异不再显示该报告的 `R092` rename，而是旧路径上的普通内容差异。
3. 更新 `docs/README.md`、`docs/reports/README.md`、`docs/directory-governance.md` 和 `docs/reports/2026-07-22-111558-yunxi-agent-v2-1-0-report-path-migration-map.md`，将活动入口恢复到旧路径，并把 2026-07-22 11:15 的历史迁移事实与热修复后的当前唯一正本状态明确区分。
4. 审核报告中的当时状态、历史开发日志和基线清单继续作为历史事实保留，没有改写审核结论；新增报告仍按 `audits/`、`development/`、`evidence/` 分类落位。

### 2. 统一验证

- 6 个治理入口文件共检查 50 个本地 Markdown 链接，失效链接 0 个；活动链接对误迁移新路径的引用为 0。
- `git diff --check` 通过；`git ls-files -ci --exclude-standard -- scripts/conpty` 返回 0 个文件。
- `scripts/conpty/v210/node_modules` 继续命中根 `.gitignore` 的 `/scripts/conpty/**/node_modules/` 统一规则。
- `cargo fmt --all -- --check`、`cargo check --workspace --offline`、`cargo test --workspace --offline` 全部通过；CLI integration 45/45、JSONL 10/10、TUI 161/161 等 workspace 测试无失败。
- v210 ConPTY 只读 verifier 返回 `ok=true`、`read_only=true`，正式 evidence SHA-256 保持 `a291a66cf91ee788bba9944edbafbf4c8dfce43bcd2d6c7b2cc78bc6c2990fc6`。
- `Cargo.toml`、`Cargo.lock`、Rust 源码、`evals/`、`vendor/`、`extracted/`、ConPTY 版本脚本和历史 evidence 的变更数为 0。

### 3. 安全、版本与复审状态

本次没有执行删除、递归清理、目录移动、`git clean`、gc、prune、系统安装、PATH/注册表/系统配置修改或用户目录清理。首次 `git mv` 仅因沙箱不能创建 `.git/index.lock` 而未执行；确认没有文件变化后，使用完全相同的精确命令取得授权并成功完成，未使用 `--force`。

当前结论仅为：**`v2.1.1-hotfix.1` 整改候选已完成，待独立复审。** 在复审报告明确通过前，不宣称 v2.1.1 审核通过，不进入 v2.1.2。发布将创建新的 annotated `v2.1.1-hotfix.1`，不移动、删除或覆盖 `v2.1.1` 及任何历史 tag。

署名：开发报告撰写者

## 十二、发布结果

发布时间：2026-07-22 16:10:50 +08:00

- 发布 commit：`d6312aebc8600697524e13a2ef96499ff60620f7`
- tree：`a8af5ce215d5adade1c1f13029d6771817d9fb08`
- parent：`3f9f1ca81906aecb5660c4cacf69215b6cc983e9`
- commit author/committer：`开发者 <developer@yunxi-agent.local>`
- annotated tag：`v2.1.1-hotfix.1`
- tag object：`12262fa6a19cd444403414606810077d6dfc81f3`
- tag target：`d6312aebc8600697524e13a2ef96499ff60620f7`
- tagger：`开发者 <developer@yunxi-agent.local>`
- GitHub master：`d6312aebc8600697524e13a2ef96499ff60620f7`
- GitHub tag 总数：52
- 发布前 51 个历史 tag SHA 变化数：0
- `v2.1.1` tag object：仍为 `75c4169d09344a359239f820ca89f052408d1e76`
- force：未使用；master 与新 tag 通过一次 atomic push 发布。

本节和最终开发日志作为 tag 后 docs-only 收口仅推进 `master`，不会移动 `v2.1.1-hotfix.1`、`v2.1.1` 或任何历史 tag。发布成功不替代独立复审；当前状态仍是“整改候选已完成，待独立复审”，不得提前进入 v2.1.2。

署名：开发报告撰写者
