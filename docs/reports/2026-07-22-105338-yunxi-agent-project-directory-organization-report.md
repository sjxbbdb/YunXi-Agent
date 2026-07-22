# YunXi Agent 项目目录整理报告

制定时间：2026-07-22 10:53:38 +08:00

## 一、结论与底线

YunXi Agent 当前的 Rust workspace 与 crate 职责划分是可用的；目录观感杂乱的主因是
根目录同时可见长期源码、参考输入、运行状态、编译产物和版本化 ConPTY 采集依赖。整理
应以**隔离可再生产物、建立索引、逐步迁移文档**为主，而不是重命名 crate 或大规模移动
目录。

本报告的底线如下：

1. 不移动、重命名或删除 `crates/`、`vendor/`、`extracted/`、`evals/`、`scripts/conpty/`
   下的已有内容，除非先完成调用与文档引用清单、通过对应验证，并取得明确授权。
2. 不执行 `git clean`、`Remove-Item -Recurse`、强制移动、批量覆盖、`git reset --hard` 或
   删除 Git tag。
3. `target/`、`.codegraph/`、`.tmp/`、`.yunxi/` 与 ConPTY 的 `node_modules/` 都是本地
   产物或状态；可以在明确授权后清理，但当前不清理。
4. 先做只影响 Git 跟踪边界的整理，再做文档索引；任何目录迁移必须单独提交、单独验证，
   不能与功能开发混在同一个提交中。
5. 每个整理阶段完成后保留 Git 状态、验证记录和回滚点。整理不是版本功能，不得修改、
   移动或覆盖历史 release tag。

## 二、参考项目：Reasonix 的分类方式

参考路径：`D:\源码\reasonix`。

Reasonix 使用的是“稳定资产在根目录，产品表面与运行产物相互隔离”的结构：

| Reasonix 分类 | 代表路径 | 方法 | 对 YunXi 的借鉴 |
| --- | --- | --- | --- |
| 可执行入口 | `cmd/` | 每个可执行程序只有薄入口 | YunXi 已由 `crates/yunxi-agent-cli/` 承担，不另建 `cmd/`。 |
| 核心领域能力 | `internal/` | 一个 package 负责一个领域，前端共用控制层 | YunXi 的 `crates/` 已是 Rust 等价物，继续按 crate 职责维护。 |
| 产品表面 | `desktop/`、`site/`、`workers/` | GUI、站点、后台 worker 不混入核心 | YunXi 当前以 CLI 为核心，未来微信适配器应新增独立 crate，而非堆入 CLI/TUI。 |
| 工程支撑 | `scripts/`、`tools/`、`benchmarks/` | 构建、维护、性能验证分开 | 保留 YunXi 的 `scripts/` 与 `evals/`，为其补齐说明和忽略规则。 |
| 长期记录 | `docs/`、`release-notes/` | 产品文档、规范、版本记录可导航 | 为 YunXi `docs/` 增加总索引，按新旧文档分阶段归档。 |
| 再生产物 | `.gitignore` | 构建、Node 依赖、工作状态一律忽略 | YunXi 需要补齐 ConPTY 各版本 `node_modules/` 与 `.work/` 的通用规则。 |

Reasonix 的形式不能原样照搬：它是 Go 单体项目，而 YunXi 是 Cargo workspace。对 YunXi
而言，`crates/` 不应被拆散或改名；清晰的根目录边界比模仿 `cmd/internal` 更重要。

## 三、当前目录基线

当前项目根目录为 `D:\YunXi Agent`，已观察到以下分类：

| 分类 | 当前路径 | 状态与处理原则 |
| --- | --- | --- |
| YunXi 自有源码 | `crates/` | 稳定核心，保持不动。 |
| 可重复评测 | `evals/` | 稳定验证资产，保持不动。 |
| 文档与证据 | `docs/` | 应以索引和分层治理，禁止先批量移动。 |
| 验证/安装脚本 | `scripts/` | 保持版本化 ConPTY 场景，先补忽略规则和索引。 |
| Codex 原始参考 | `vendor/codex-rs/` | 仍被 Cargo 配置、迁移脚本和提取索引引用，禁止移动。 |
| 已提取迁移输入 | `extracted/codex-core-agent-sources/` | 仍是迁移依据，禁止移动。 |
| Rust 编译产物 | `target/` | 约 3.15 GB；已被忽略，当前不清理。 |
| CodeGraph 索引 | `.codegraph/` | 约 184 MB；已被忽略，可重建，当前不清理。 |
| 临时采集 | `.tmp/` | 约 0.15 MB；需要显式目录级忽略规则，当前不清理。 |
| 本地 Agent 状态 | `.yunxi/`、`crates/yunxi-agent-cli/.yunxi/` | 已被忽略，保持本地状态边界。 |
| worktree | `.worktrees/` | 已被忽略，非源码结构的一部分。 |
| ConPTY Node 依赖 | `scripts/conpty/v207*/` 至 `v210/` 的 `node_modules/` | 当前存在未跟踪目录，是第一批只改忽略规则的对象。 |

本报告形成时，工作树中已有两份未跟踪的正式文档和多个 ConPTY `node_modules/` 目录。前者
是待归档资产，不得清理；后者是可再生依赖，先忽略、后续在授权下按精确路径清理。

## 四、目标结构与边界

不改变现有兼容路径的前提下，YunXi 根目录应呈现以下语义：

```text
D:\YunXi Agent\
  crates\                  Rust 核心能力与 CLI/TUI 产品入口
  evals\                   可重复功能、协议和回归验证
  scripts\                 安装、迁移、Provider、ConPTY 验证工具
  docs\                    架构、路线图、操作手册、报告和证据
  vendor\                  固定快照的外部参考源码
  extracted\               从参考源码提取的迁移输入与映射
  Cargo.toml / Cargo.lock   workspace 定义
  target\                  本地 Rust 编译产物，Git 忽略
  .tmp\                    本地临时采集目录，Git 忽略
  .yunxi\                  本地运行状态，Git 忽略
  .codegraph\              本地索引，Git 忽略
  .worktrees\              本地工作树，Git 忽略
```

`docs/` 的目标分类不要求一次完成。新文档优先进入以下位置，旧文档等完成链接盘点后再逐批迁移：

```text
docs/
  README.md 或 index.md              文档导航与归档规则
  architecture/                      当前架构、crate 边界、数据流
  operations/                        构建、诊断、恢复、ConPTY 操作手册
  superpowers/specs/                 已有设计规格，保持路径兼容
  superpowers/plans/                 已有版本路线图，保持路径兼容
  reports/                           带时间戳的正式报告
    audits/                          后续新增审核报告
    development/                     后续新增开发报告
    evidence/                        可复核的采集证据
  extraction-index/                  Codex 参考映射与迁移状态
```

在迁移完成前，历史报告可以继续保留在 `docs/reports/` 根部；禁止为了“看起来整齐”而一次性
移动全部历史报告。

## 五、分阶段整理方案

### 阶段 0：冻结基线，不改目录

目的：把当前状态变成可验证的起点。

1. 记录 `git status --short`、根目录清单、`Cargo.toml`、`.gitignore` 与当前版本/tag。
2. 列出所有涉及 `vendor/`、`extracted/`、`docs/superpowers/`、`scripts/conpty/` 的源码、
   文档和脚本引用；在清单完成前，不迁移这些路径。
3. 将本报告和当前正式审核/路线图文档作为待提交资产保留，不执行清理命令。
4. 验收：仅产生报告或清单；`git diff --check` 通过；没有文件被移动或删除。

### 阶段 1：只收紧 Git 忽略边界

目的：让可再生的依赖和临时状态不再污染工作树，不改变任何运行时路径。

建议在 `.gitignore` 中新增目录级规则：

```gitignore
/.tmp/
/scripts/conpty/**/node_modules/
/scripts/conpty/**/.work/
```

实施前必须先用 `git ls-files` 确认上述模式不匹配已跟踪文件；实施后用
`git check-ignore -v` 分别验证 v207、v207-hotfix、v208、v209、v210 的依赖目录。

本阶段禁止删除任何 `node_modules/`、`.work/`、`target/` 或 `.tmp/` 内容。忽略规则只改变
Git 的候选文件视图，不会移动、更改或清空磁盘上的数据。

验收：`git status --short` 不再列出这些 ConPTY 依赖目录；已有正式报告仍可见；脚本路径和
`npm` 锁文件均未改动；`git diff --check` 通过。

### 阶段 2：建立文档索引，不迁移历史文档

目的：先解决“找不到文档”，再处理“文档放在哪里”。

1. 新增 `docs/README.md` 或 `docs/index.md`，列出架构、当前路线图、审核报告、开发报告、
   ConPTY 证据、提取索引和恢复/操作文档的稳定入口。
2. 为 `docs/reports/` 写明命名格式：`时间戳-yunxi-agent-版本-主题-报告类型.md`。
3. 规定后续审核报告写入 `docs/reports/audits/`，开发报告写入
   `docs/reports/development/`，证据继续写入 `docs/reports/evidence/`；历史文件不在本阶段
   移动。
4. 在根 `README.md` 添加一次指向文档索引的链接，避免继续把所有说明堆入根 README。

验收：从根 README 可在两步内进入任意报告类别；现有 Markdown 链接不失效；无需运行
Cargo 构建。

### 阶段 3：受控迁移历史文档

目的：把历史报告按类型归档，同时保持所有引用可用。

1. 每次只迁移一个版本区间或一个报告类型，先用 `rg` 制作“旧路径 -> 新路径”映射表。
2. 同一提交内更新所有仓库内 Markdown、脚本和索引中的引用；不修改报告正文中的事实记录。
3. 迁移后检查不存在悬挂的旧路径引用，并人工打开根 README、文档索引、最近三份审核报告和
   最近三份开发报告。
4. 每一个迁移批次独立提交；确认无误后才开始下一批次。

验收：`rg` 不再发现被迁移文件的陈旧链接；`git diff --check` 通过；历史报告内容与文件
SHA-256 在迁移前后一致。

### 阶段 4：ConPTY 验证资产索引化

目的：保留真实 Windows 验证证据的可追溯性，同时避免版本目录看起来像散落的临时工程。

1. 保持 `scripts/conpty/v205` 至 `v210`、`v207-hotfix` 的既有路径不变，因为 README、报告和
   evidence 已依赖这些路径。
2. 为 `scripts/conpty/` 增加总览说明，列出每个版本目录的场景、入口、依赖、生成目录和
   对应 evidence 路径。
3. 新验证版本沿用版本目录，但把共享说明、共用 PowerShell/Node 辅助函数放入明确的
   `scripts/conpty/shared/`；只有在两套旧脚本均被验证复用后才抽取共享代码。
4. `node_modules/` 永远不提交；`package.json`、锁文件、脚本和脱敏证据才是版本化资产。

验收：每个 ConPTY 目录都能由总览定位；现有 `npm run verify` 路径不变；真实采集证据仍可
被离线验证器读取。

### 阶段 5：仅在单独授权后清理再生产物

目的：释放磁盘空间，不影响源码、证据和可回滚性。

可候选清理对象包括 `D:\YunXi Agent\target\`、`.codegraph\`、已验证可再生的特定
`scripts\conpty\<版本>\node_modules\` 以及已归档的指定 `.tmp\` 采集目录。每一项均需：

1. 用户明确确认**精确绝对路径**。
2. 先验证该路径位于 `D:\YunXi Agent` 内，且不包含已跟踪文件、审计证据或用户数据。
3. 清理后只重建必要内容，并记录释放空间、命令、验证和 Git 状态。

禁止使用泛化模式、盘符根目录、父目录或“清空所有临时文件”作为清理目标。

## 六、不可触碰清单

以下内容在首次整理中必须保持原路径：

- `Cargo.toml`、`Cargo.lock`、`crates/`、`evals/`；
- `vendor/codex-rs/`、`extracted/codex-core-agent-sources/`；
- `scripts/conpty/` 内所有现有版本目录和 `package.json`/锁文件；
- `docs/reports/evidence/`、历史审核报告、开发报告、提取索引；
- `.git/`、已有 annotated tag、现有 worktree；
- 用户目录、桌面、C 盘配置和系统级路径。

## 七、每阶段验证与回滚

| 阶段 | 最低验证 | 回滚方式 |
| --- | --- | --- |
| 阶段 1 | `git check-ignore -v`、`git status --short`、`git diff --check` | 仅还原本阶段 `.gitignore` 的单一提交。 |
| 阶段 2 | Markdown 链接人工抽查、`git diff --check` | 仅还原索引文件提交。 |
| 阶段 3 | 引用扫描、抽查文档、迁移前后 SHA-256 | 还原单一迁移提交，不改历史 tag。 |
| 阶段 4 | 各版本 `npm run verify`、证据路径检查 | 还原 ConPTY 索引/共享脚本提交。 |
| 阶段 5 | 精确路径复核、必要的重建/验证、Git 状态 | 从保留的提交和可再生构建流程恢复；不以删除 tag 回滚。 |

涉及 Rust 代码或 Cargo 路径的整理，除上述检查外还必须通过：

```text
cargo fmt --all -- --check
cargo check --workspace
cargo test --workspace
git diff --check
```

## 八、建议的执行顺序

建议只先执行阶段 0、阶段 1 和阶段 2。它们不会移动源码或删除文件，却能立刻减少 Git
状态噪音并改善查找体验。阶段 3 至阶段 5 应在独立任务中逐项确认；尤其是清理
`target/`、`.codegraph/` 或任何 `node_modules/` 前，必须由用户重新给出精确路径授权。

本报告不实施任何整理动作，不改变项目版本，不创建或修改 Git tag。

署名：整理报告撰写者

## 九、阶段 0 至阶段 2 实施结果

实施时间：2026-07-22 11:09:06 +08:00

按本报告建议，本轮仅实施阶段 0、阶段 1 和阶段 2，没有进入历史文档迁移、ConPTY 共享代码抽取或再生产物清理。

1. 阶段 0 建立 `docs/reports/2026-07-22-110139-yunxi-agent-project-directory-baseline-report.md`，记录 HEAD、版本、50 个 tag、根目录边界、未跟踪正式资产、再生产物和四类受保护路径的完整引用文件清单；正式审核报告、本整理报告和个人微信路线图原样纳入 Git。
2. 阶段 1 将 v205/v206 的 ConPTY 忽略特例收敛为 `/scripts/conpty/**/node_modules/` 与 `/scripts/conpty/**/.work/`，并增加显式 `/.tmp/`。`git ls-files` 证明这些模式不包含已跟踪文件；v205、v206、v207、v207-hotfix、v208、v209、v210 探针均由预期规则命中。
3. 阶段 2 新增 `docs/README.md`，更新 `docs/reports/README.md` 的命名与归档规则，并在根 `README.md` 增加稳定入口。历史报告、spec、plan、evidence 和脚本均未移动或改名。
4. 三阶段分别建立独立回滚提交：`a8d539810dd51829c21fdc2877609e8329de3bf7`、`6116369d8aa035a4a0bafa4957459806317572b5`、`13ea19bdf0a57fe9500418bd82ef1ba57e2b5406`。

验证结果：23 个本地 Markdown 链接全部存在；`cargo fmt --all -- --check`、`cargo check --workspace`、`cargo test --workspace` 全部通过；v210 与 v209 verifier 均返回 `read_only=true`，evidence SHA-256 分别保持 `a291a66cf91ee788bba9944edbafbf4c8dfce43bcd2d6c7b2cc78bc6c2990fc6` 与 `714b9c2d9c01b1616ffc8789e940ea778559335e20998679a904b5c335fcfdc8`。

边界结果：未执行删除、递归清理、目录移动、强制覆盖、`git clean`、`git reset`、tag 修改或用户目录写入。五个现存 ConPTY `node_modules` 目录仍在磁盘上，只是不再污染 Git 状态；`target/`、`.tmp/`、`.codegraph/`、`.yunxi/`、`.worktrees/`、源码、安装目录和全部 release tag 均保持原位。

署名：开发者

## 九、阶段 0 至阶段 2 实施结果

实施时间：2026-07-22 11:09:06 +08:00

按本报告建议，本轮仅实施阶段 0、阶段 1 和阶段 2，没有进入历史文档迁移、ConPTY 共享代码抽取或再生产物清理。

1. 阶段 0 建立 `docs/reports/2026-07-22-110139-yunxi-agent-project-directory-baseline-report.md`，记录 HEAD、版本、50 个 tag、根目录边界、未跟踪正式资产、再生产物和四类受保护路径的完整引用文件清单；正式审核报告、本整理报告和个人微信路线图原样纳入 Git。
2. 阶段 1 将 v205/v206 的 ConPTY 忽略特例收敛为 `/scripts/conpty/**/node_modules/` 与 `/scripts/conpty/**/.work/`，并增加显式 `/.tmp/`。`git ls-files` 证明这些模式不包含已跟踪文件；v205、v206、v207、v207-hotfix、v208、v209、v210 探针均由预期规则命中。
3. 阶段 2 新增 `docs/README.md`，更新 `docs/reports/README.md` 的命名与归档规则，并在根 `README.md` 增加稳定入口。历史报告、spec、plan、evidence 和脚本均未移动或改名。
4. 三阶段分别建立独立回滚提交：`a8d539810dd51829c21fdc2877609e8329de3bf7`、`6116369d8aa035a4a0bafa4957459806317572b5`、`13ea19bdf0a57fe9500418bd82ef1ba57e2b5406`。

验证结果：23 个本地 Markdown 链接全部存在；`cargo fmt --all -- --check`、`cargo check --workspace`、`cargo test --workspace` 全部通过；v210 与 v209 verifier 均返回 `read_only=true`，evidence SHA-256 分别保持 `a291a66cf91ee788bba9944edbafbf4c8dfce43bcd2d6c7b2cc78bc6c2990fc6` 与 `714b9c2d9c01b1616ffc8789e940ea778559335e20998679a904b5c335fcfdc8`。

边界结果：未执行删除、递归清理、目录移动、强制覆盖、`git clean`、`git reset`、tag 修改或用户目录写入。五个现存 ConPTY `node_modules` 目录仍在磁盘上，只是不再污染 Git 状态；`target/`、`.tmp/`、`.codegraph/`、`.yunxi/`、`.worktrees/`、源码、安装目录和全部 release tag 均保持原位。

署名：开发者
