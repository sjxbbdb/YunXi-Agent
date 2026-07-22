# YunXi Agent v2.1.1 版本审核报告

- 审核时间：2026-07-22 15:45:59 +08:00
- 审核范围：仅对照项目内总纲 `v2.1.1`“项目目录治理、Git 忽略边界与文档索引基线”要求审核当前版本，不对比其他版本的完成度。
- 开发目录：`D:\YunXi Agent`
- 审核依据：`D:\YunXi Agent\docs\superpowers\plans\2026-07-22-yunxi-agent-v2-1-1-to-v2-2-0-personal-wechat-roadmap.md`
- 参考源码依据：`D:\源码\reasonix`

## 一、最终结论

**v2.1.1 审核不通过，暂不可进入总纲图中的 v2.1.2 开发。**

源码边界、Rust 回归、TUI 视觉基线、真实 Provider 基线、Git 忽略规则、文档导航、路线图副本和 annotated tag 均已核验通过；但本版本实际差异中迁移了一份历史 v2.1.0 开发报告，违反总纲对本版本的明确硬性要求：**“本版本不迁移历史报告，以避免断开既有 Markdown 引用。”**

该问题不是发布顺序问题，也不是 tag 后新增的 docs-only 收口提交问题，而是当前版本治理结果与总纲明确要求不一致，必须完成当前版本整改并重新审核。

## 二、总纲逐项核验

| v2.1.1 要求 | 结果 | 核验结论 |
| --- | --- | --- |
| 建立根目录资产清单，记录用途、跟踪状态、清理条件和引用方 | 通过 | `docs/directory-governance.md` 已记录稳定资产、本地状态和可再生产物，包含精确绝对路径。 |
| 保持既有 `crates/`、`evals/`、`scripts/`、`docs/`、`vendor/`、`extracted/` 与历史 evidence 边界 | 通过 | `v2.1.0..v2.1.1` 没有 Rust、Cargo、vendor、extracted、evals、ConPTY 版本脚本或历史 evidence 的功能性变更。 |
| 统一补齐 `/.tmp/`、`/scripts/conpty/**/node_modules/`、`/scripts/conpty/**/.work/` 忽略规则 | 通过 | 现存 `scripts/conpty/v210/node_modules` 被统一规则命中；当前没有未被忽略的目标目录。 |
| 不隐藏已跟踪文件 | 通过 | `git ls-files -ci --exclude-standard -- scripts/conpty` 返回 0 个已跟踪且被忽略文件。 |
| 从根 README 两步内进入路线图、报告索引、ConPTY 说明和提取索引 | 通过 | 6 个治理入口文件中检查到 46 个本地 Markdown 链接，失效链接 0 个；四类入口均可达。 |
| 新报告进入 `audits/`、`development/`，证据保留在 `evidence/` | **不通过** | 见第三节。新增报告落位正确，但同时迁移了一份历史 v2.1.0 开发报告。 |
| `scripts/conpty/` 总览列出目录、场景、依赖、输出和 evidence，不改变脚本路径 | 通过 | `scripts/conpty/README.md` 已形成 v205-v210 总览，v210 只读 verifier 通过。 |
| 只记录清理候选，不执行删除或递归清理 | 通过 | 资产清单记录 `target`、`.tmp`、`.yunxi`、`.codegraph` 和 ConPTY 依赖；本次未执行清理。 |
| 项目路线图正本与桌面分发副本状态可追溯 | 通过 | 项目内与桌面路线图 SHA-256 均为 `2DF30D503F46CFE7496567F5011BF5CBFB8BA73C91C2D2FA3E9F29AF0932EA0F`。 |
| 创建新的 annotated `v2.1.1` tag，保留历史 tag | 通过 | `v2.1.1` 为 annotated tag，tag object 为 `75c4169d09344a359239f820ca89f052408d1e76`，目标提交为 `58fb10f2f9e192056dea2660f34fc5c1bd8232b5`。 |

## 三、唯一阻塞点：历史报告发生迁移

总纲明确要求 v2.1.1 不迁移历史报告。实际 `v2.1.0..v2.1.1` 差异显示：

| 项目 | 实际状态 |
| --- | --- |
| 旧路径 | `D:\YunXi Agent\docs\reports\2026-07-22-074211-yunxi-agent-v2-1-0-integrated-release-regression-development-report.md` 不存在 |
| 新路径 | `D:\YunXi Agent\docs\reports\development\2026-07-22-074211-yunxi-agent-v2-1-0-integrated-release-regression-development-report.md` 存在 |
| Git 识别 | `git diff --find-renames=90% v2.1.0..v2.1.1` 识别为 92% rename |
| 关联提交 | `ddd4cd4 docs: migrate v2.1.0 reports into typed archives` |
| 当前影响 | 活动 Markdown 链接检查未发现断链，但“没有迁移历史报告”的总纲约束已被违反。 |

`docs/reports/2026-07-22-111558-yunxi-agent-v2-1-0-report-path-migration-map.md` 记录了迁移映射和哈希，这能证明迁移过程是精确、可追溯的，但不能将“迁移”转化为“未迁移”。链接未断也不能替代总纲要求。

整改要求：开发者必须依据总纲重新处理这份历史报告的存放边界，恢复历史路径的保留语义，并解决旧路径与新分类路径之间的重复/单一正本问题。整改涉及的精确文件路径、是否保留副本以及是否移除新路径，必须由开发者按 Git 历史和用户授权执行；本次审核没有执行任何恢复、删除或移动操作。整改完成后必须重新提交当前版本审核，未复审通过前不得进入 v2.1.2。

## 四、源码与版本边界审核

- 已先使用 CodeGraph 核验 CLI、TUI、Runtime、Storage 和 Codex compatibility crate 的边界；当前版本没有新增 `crates/yunxi-agent-weixin`，没有新增微信网络、登录、凭证、轮询或通道 Runtime。
- `git diff --name-only v2.1.0..v2.1.1` 的功能性路径过滤结果为 0：没有 `Cargo.toml`、`Cargo.lock`、`crates/`、`evals/`、`vendor/`、`extracted/`、ConPTY 版本脚本或 `docs/reports/evidence/` 改动。
- 当前工作树干净；当前 `HEAD=3f9f1ca` 比 `v2.1.1` tag 的发布提交多一个 `docs: record v2.1.1 release results` docs-only 提交。根据既定审核规则，该发布后文档收口属于观察项，不作为阻塞点。
- `Cargo.toml` 和调试二进制仍报告 `2.1.0`。这是因为 v2.1.1 总纲明确禁止修改 Cargo/Rust 运行边界，故记录为版本标签与产品二进制显示的非阻塞观察项，不将其误判为本版源码缺陷。

## 五、验证结果

### 1. Rust 与 CLI 回归

- `cargo fmt --all -- --check`：通过。
- `cargo check --workspace --offline`：通过。
- `cargo test --workspace --offline`：通过。
- CLI integration：45/45 通过。
- JSONL：10/10 通过。
- Provider：46/46 通过。
- TUI：161/161 通过。
- `target\debug\yunxi.exe --version`：`yunxi 2.1.0`。
- `yunxi eval companion --json`：31/31，`golden_passed=true`，`tool_approval_bypass_count=0`，`proactive_boundary_violation_count=0`。

### 2. Git、忽略规则与文档

- `git check-ignore -v`：现存 `scripts/conpty/v210/node_modules` 命中 `.gitignore:4` 的 `/scripts/conpty/**/node_modules/` 规则。
- 现存 ConPTY 生成目录探针：1 个，未发现未忽略目标；`.work` 当前不存在。
- 已跟踪且被忽略文件：0 个。
- 本地 Markdown 链接：46 个，失效 0 个。
- `git diff --check v2.1.0..v2.1.1`：通过。
- `git fsck --full`：退出码 0；输出包含历史 dangling 对象，但没有报告对象损坏或 tag 引用错误。本次未执行 gc、prune 或清理。

### 3. ConPTY、真实 Provider 与 TUI

- `npm.cmd run verify --prefix scripts/conpty/v210`：通过，返回 `ok=true`、`read_only=true`，正式 evidence SHA-256 为 `a291a66cf91ee788bba9944edbafbf4c8dfce43bcd2d6c7b2cc78bc6c2990fc6`。
- 真实 Provider：v2.1.0 发布审核已记录真实 DeepSeek 单轮结果 `YUNXI_V210_REAL_PROVIDER_OK`；本版没有 Provider、Runtime、CLI 或 TUI 功能代码变化，因此将该在线结果作为当前版本的继承基线。此次没有重复执行会发起网络请求的 live smoke，也没有把 offline 结果冒充在线结果。
- TUI 视觉帧复核：
  - `D:\YunXi Agent\.tmp\audit-v210-live-provider-20260722\visual\conversation-100x30.png`
  - `D:\YunXi Agent\.tmp\audit-v210-live-provider-20260722\visual\details-100x30.png`
  - `D:\YunXi Agent\.tmp\audit-v210-live-provider-20260722\visual\conversation-58x18.png`
- 视觉结论：100x30 与 58x18 均无重叠、越界、Details 泄漏或窄屏不可读；Details 关闭后可恢复主会话。上述帧的 header 明确为 `offline`，因此只作为布局与交互基线，不作为实时 Provider 视觉证据。
- 非阻塞观感观察：forced-offline header 存在 `offline offline` 重复文案；不影响本版本目录治理验收，也不构成本次阻塞点。

## 六、参考源码与借鉴边界

本版本总纲要求参考已有 `D:\源码\reasonix` 的目录治理方式，当前审核确认对应参考路径存在并已在开发报告中明确：

- `D:\源码\reasonix\.gitignore`：参考构建产物、Node 依赖、局部状态和索引的统一忽略方式。
- `D:\源码\reasonix\docs\`：参考文档分类与长期导航方式。
- `D:\源码\reasonix\scripts\`：参考脚本入口与说明边界。
- `D:\源码\reasonix\REASONIX.md`：参考稳定的项目约束入口。

本版本不要求拉取新的微信项目源码，也不应把 Reasonix 的 Go、站点、桌面端或发布系统结构复制进 YunXi；后续微信接入版本的参考项目不属于本次 v2.1.1 审核条件。

## 七、整改后复审条件

当前版本必须先完成第三节的历史报告路径治理整改，并再次核验：

1. 历史 v2.0/v2.1.0 报告未被本版移动、删除或重命名，或已按总纲恢复为唯一明确的历史存放语义。
2. 新增报告分类目录仍保持 `audits/`、`development/`、`evidence/` 边界，且不通过复制制造两个互相演进的正本。
3. `git diff --find-renames=90%` 不再显示本次 v2.1.1 对历史报告的迁移，相关活动 Markdown 链接和哈希记录仍然可追溯。
4. 重新执行忽略规则、Markdown 链接、Rust 回归、ConPTY 只读 verifier、真实 Provider 基线和 TUI 视觉/交互复核。

整改完成并经审核通过后，才可以进入总纲图中的 v2.1.2 开发；在此之前不得以当前 v2.1.1 状态宣称本版本审核通过。

## 八、清理、提交与署名

- 清理结果：未执行删除、移动、重命名、递归清理、`git clean`、gc、prune、系统安装、PATH/注册表/系统配置修改或用户目录清理。`target`、`.tmp`、`.codegraph`、ConPTY 依赖和审计采集资产继续保留。
- 报告写入：本项目报告落在 `D:\YunXi Agent\docs\reports\audits\`；桌面副本将写入 `C:\Users\24763\Desktop\YunXi Agent审核报告\`。
- 提交、推送与 tag：本次审核未创建 commit、未推送、未创建或移动 tag；`v2.1.1` 及历史 tag 均保持不变。

署名：审核者
