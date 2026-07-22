# YunXi Agent 项目目录治理基线

- 基线版本：`v2.1.1`
- 盘点时间：2026-07-22 14:58:04 +08:00
- 项目根目录：`D:\YunXi Agent`
- 盘点时 HEAD：`d6b6132ce9ad73b980f0208b618959671889fbf4`
- 适用范围：目录用途、Git 跟踪边界、文档入口、本地状态与可再生产物

本基线只治理目录与文档，不改变 Cargo workspace、Rust 行为、CLI/TUI/Runtime、Provider、Storage、Protocol 或 ConPTY 脚本。YunXi 默认运行路径继续由自有 crate 提供，不依赖 `vendor/codex-rs`、`codex-*` crate 或 `yunxi-agent-codex`。

## 稳定资产清单

下表中的数量是盘点时 `git ls-files` 的基线，共 6,674 个已跟踪文件；后续新增治理文档会使 `docs/` 数量正常增加。

| 类别 | 路径 | 已跟踪文件 | 用途 | Git 与清理边界 | 当前引用方 |
| --- | --- | ---: | --- | --- | --- |
| 根配置 | 根目录文件 | 6 | workspace、Git、项目约束与项目入口 | 应跟踪；不得作为清理对象 | Cargo、Git、开发工具、根文档入口 |
| 产品源码 | `crates/` | 146 | YunXi 自有 core、runtime、provider、tools、storage、persona、companion、TUI 与 CLI | 应跟踪；不得因目录治理移动或清理 | `Cargo.toml`、测试、README、发布流程 |
| 评测 | `evals/` | 9 | 陪伴能力离线评测与 golden | 应跟踪；不得清理正式 fixture | workspace、评测脚本、发布门禁 |
| 工程脚本 | `scripts/` | 47 | 安装、迁移、Provider 验证与版本化 ConPTY 采集/复核 | 脚本和锁文件应跟踪；生成目录不跟踪 | README、开发/审核报告、evidence manifest |
| 正式文档 | `docs/` | 151 | 架构、路线图、报告、证据、提取索引和开发日志 | 应跟踪；历史路径迁移必须单独提交并更新引用 | 根 README、脚本、发布与审核流程 |
| 参考输入 | `vendor/` | 4,953 | Codex Rust 参考源码与兼容边界 | 应跟踪；默认 YunXi 路径不得依赖；禁止目录治理迁移 | Cargo 兼容层、提取索引、迁移脚本 |
| 提取输入 | `extracted/` | 1,362 | 已抽取的 Codex 核心能力参考与映射 | 应跟踪；只作参考输入，不进入默认运行路径 | 提取状态、提取索引、迁移脚本 |

Reasonix 只提供分类逻辑参考：稳定产品资产、文档、维护脚本、本地状态和再生产物相互隔离。本仓库不复制其 Go `cmd/internal`、站点、桌面端或发布系统结构。

## 本地状态与可再生产物

| 绝对路径 | 盘点状态 | 性质 | 是否可直接清理 | 条件与引用方 |
| --- | --- | --- | --- | --- |
| `D:\YunXi Agent\target` | 存在；8,114 文件，2,507,699,882 字节（约 2.39 GiB） | Cargo 构建产物 | 否 | 可重建；必须先确认无构建进程、无重解析点、无跟踪文件，并取得精确路径授权 |
| `D:\YunXi Agent\.tmp` | 存在；11 文件，158,810 字节 | 审计/ConPTY 临时采集 | 否 | 当前包含 v2.1.0 审计采集资产；只能在证据归档核验后按子目录精确授权 |
| `D:\YunXi Agent\.yunxi` | 存在；2 文件，14,601 字节 | 项目级 YunXi session/状态 | 否 | 属于用户状态；不得作为普通缓存清理 |
| `D:\YunXi Agent\crates\yunxi-agent-cli\.yunxi` | 存在；1 文件，5,824 字节 | CLI 测试/本地状态 | 否 | 必须先判断是否为测试 fixture 或用户状态，再单独授权 |
| `D:\YunXi Agent\.codegraph` | 存在；6 文件，192,658,181 字节（约 183.73 MiB） | 可重建代码索引 | 否 | 开发检索正在使用；只有确认可重建且取得精确授权后才可删除 |
| `D:\YunXi Agent\.worktrees` | 存在且为空 | Git worktree 容器 | 否 | 只能结合 `git worktree list` 判断；禁止直接递归清理 |
| `D:\YunXi Agent\scripts\conpty\v210\node_modules` | 部分存在；53 文件，32,558,228 字节（约 31.05 MiB） | ConPTY 证据采集依赖 | 否 | 不进入 YunXi 默认运行路径；删除前须确认无 Node 占用，恢复使用 `npm.cmd ci --prefix scripts\conpty\v210` |
| `D:\YunXi Agent\scripts\conpty\v205` 至 `v209` 的 `node_modules` | 均不存在 | ConPTY 证据采集依赖 | 不适用 | 需要采集时按对应锁文件安装，永不提交 |
| `D:\YunXi Agent\scripts\conpty\v205` 至 `v210` 的 `.work` | 均不存在 | ConPTY 中间输出 | 不适用 | 由根 `.gitignore` 统一忽略，需要时可再生 |

`.agents/`、`.codex/` 和 `.superpowers/` 是项目开发工具/约束输入；本版本不移动、不重命名、不清理。用户目录、安装目录、系统 PATH、注册表和系统配置不属于项目目录治理范围。

## Git 忽略边界

根 `.gitignore` 统一维护以下目录级规则：

```gitignore
/target/
/.yunxi/
/crates/yunxi-agent-cli/.yunxi/
/scripts/conpty/**/node_modules/
/scripts/conpty/**/.work/
/.tmp/
/.codegraph/
/.worktrees/
```

盘点时 `git ls-files -- '.tmp/**' 'scripts/conpty/**/node_modules/**' 'scripts/conpty/**/.work/**'` 返回 0 个文件。v207、v207-hotfix、v208、v209、v210 的 `node_modules` 探针及 v207/v210 的 `.work` 探针均由统一规则命中；这些规则不会隐藏已跟踪资产。

## 路线图正本与桌面分发副本

- 唯一可编辑正本：`D:\YunXi Agent\docs\superpowers\plans\2026-07-22-yunxi-agent-v2-1-1-to-v2-2-0-personal-wechat-roadmap.md`
- 正本 SHA-256：`2DF30D503F46CFE7496567F5011BF5CBFB8BA73C91C2D2FA3E9F29AF0932EA0F`
- 桌面分发副本：`C:\Users\24763\Desktop\2026-07-22-yunxi-agent-v2-1-1-to-v2-2-0-personal-wechat-roadmap.md`
- 本次同步前桌面 SHA-256：`ECE3375748A6EF4BD70DCA03FFB65941AE2F566CCC32FBCC12505DB511987372`
- 本次从正本重新生成后桌面 SHA-256：`2DF30D503F46CFE7496567F5011BF5CBFB8BA73C91C2D2FA3E9F29AF0932EA0F`

桌面文件只是分发件，不得与项目正本并行编辑。后续同步必须始终执行“项目正本 -> 桌面副本”的单向复制并校验 SHA-256。

## 历史报告路径整改状态

- `docs/reports/2026-07-22-074211-yunxi-agent-v2-1-0-integrated-release-regression-development-report.md` 是该历史开发报告的唯一正本，SHA-256 为 `B70BF0D3F3BBDEEB7DE1DB515D8FA60B09BB161F63C47160AF28997EB61D0413`。
- `docs/reports/development/2026-07-22-074211-yunxi-agent-v2-1-0-integrated-release-regression-development-report.md` 不保留正文副本，避免形成两个可独立演进的正本。
- 该回迁只修复 v2.1.1 审核指出的历史路径问题；新报告仍按 `audits/`、`development/`、`evidence/` 分类落位。
- 当前状态仅为 `v2.1.1-hotfix.1` 整改候选，必须经独立复审后才能宣称 v2.1.1 审核通过或进入 v2.1.2。

## 版本与清理纪律

- `v2.1.1` 只交付目录治理、Git 忽略边界和文档索引，不实现微信网络、登录、凭证、轮询、会话桥接或新 Runtime。
- 微信 crate、CLI 骨架和 iLink Mock 协议客户端从 `v2.1.2` 开始。
- 本版本只记录清理候选，不执行清理。任何删除、递归清理或目录移动都必须先核对精确绝对路径、Git 跟踪文件、重解析点和进程占用，并再次取得用户授权。
- 禁止 `git clean`、宽泛通配符删除、force push、移动或覆盖历史 tag。

署名：开发者
