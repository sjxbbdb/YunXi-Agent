# YunXi Agent v2.0.4 窄屏 Header 信息层级整改开发报告

- 撰写时间：2026-07-19 20:51:31 +08:00
- 审核报告：`C:\Users\24763\Desktop\YunXi Agent审核报告\2026-07-19-204523-YunXi-Agent-v2.0.4-源码与TUI视觉审核报告.md`
- 原开发报告：`D:\YunXi Agent\docs\reports\2026-07-19-173413-yunxi-agent-v2-0-4-i18n-responsive-layout-development-report.md`
- 开发目录：`D:\YunXi Agent`
- 项目内报告：`D:\YunXi Agent\docs\reports\2026-07-19-205131-yunxi-agent-v2-0-4-responsive-header-remediation-development-report.md`
- 桌面报告：`C:\Users\24763\Desktop\YunXi Agent开发报告\2026-07-19-205131-yunxi-agent-v2-0-4-responsive-header-remediation-development-report.md`
- 报告类型：当前版本整改开发报告
- 审核结论转化：`v2.0.4` 审核不通过，不可进入下一版本开发。

## 一、硬性约束

以下约束是本项目后续开发、验证、发布和文档同步的前置要求，不得因为上下文过长、任务拆分、实现难度或局部修复而忽略、弱化或绕过。

1. 开发目录固定在 `D:\YunXi Agent` 及其工作树内。
2. 核心目标是开发一个通用型陪伴 agent。
3. 涉及到源码参考需要先抽取、迁移和构建整体能力，不要在单个点上反复纠结；如果参考的源码不为 Rust 语言，则需参考其逻辑，进行 Rust 复刻。
4. 中间无需频繁验证，完成一批构建后再统一测试和验证。
5. 验证通过前不能宣称完成。
6. 每次阶段结束后要清理编译中间产物，避免占用大量硬盘空间。
7. 每次任务结束后，都要在日志中追加本次工作的详细记录。
8. 记录必须包含时间戳、工作目标、执行流程、修改文件、文件路径、验证结果、提交和推送状态，以及日志结尾处要进行署名，署名为开发者。
9. 后续开发报告、设计文档、索引和状态文档要与实际代码状态保持一致。
10. 回复用户使用中文。
11. 之后每个版本更迭都必须提交为新的 Git tag；以前的版本 tag 不得删除，方便出错后回滚。
12. 涉及到 shell 命令时要万分小心，例如递归删除、强制移动、清空目录、系统级安装/卸载、修改 PATH/注册表/系统配置等操作必须得到用户确认。
13. 不可因为过长的上下文忽略硬性要求；如果察觉到硬性要求的效力已经减弱，则必须汇报当前情况。
14. 工作尽量固定在项目文件夹，涉及到其他文件夹的操作，要明确告知用户，尤其是 C 盘的用户目录。

## 二、整改目标

`v2.0.4` 已通过大部分国际化排版、响应式布局、显示宽度、cursor、wrap、80/100/120/200 snapshot、真实 Provider 和伴侣评测验收；但审核报告明确指出窄屏 header 信息层级不符合总纲图硬要求。因此本阶段不是下一版本开发，而是补齐 `v2.0.4` 当前版本验收。

核心整改目标：

1. `80x24` 窄屏 header 只能显示产品/版本与 provider/live 连接状态。
2. 窄屏不得出现 `model=`、cwd 前缀、路径尾段或任何 cwd 截断形式。
3. 中屏允许显示 model，但不得显示 cwd。
4. 宽屏才允许显示经路径边界压缩的 cwd。
5. 用固定宽度档位测试锁定窄/中/宽三档语义，不能只断言字符串总宽度不越界。
6. 更新 `full_frame_80x24.txt` golden，并新增针对 model/cwd 不出现的负向断言。

## 三、版本与发布边界

当前审核对象为：

- 审核提交：`bfd90b659a172f636e83545ceb477c23954cf1a7`
- 版本号：`2.0.4`
- annotated tag：`v2.0.4`

开发者必须遵守：

- 已发布 `v2.0.4` tag 不得移动、删除或覆盖。
- 旧 `v2.0.2`、`v2.0.2-hotfix.1`、`v2.0.3`、`v2.0.3-hotfix.1` tag 均不得移动、删除或覆盖。
- 由于 `v2.0.4` 已发布且审核失败，整改发布编号与 tag 策略必须先取得项目负责人明确决定。
- 不得自行创建 hotfix tag，也不得把整改伪装为下一版本正常功能开发。
- 重新审核通过前，不得宣称 `v2.0.4` 完成，也不得进入下一版本开发。

## 四、已通过能力保持要求

本次整改范围很窄，不能牵动已经通过的能力：

1. `TextLayout` 的 Grapheme 与 terminal cell width 测量不回退。
2. CJK、日文、Emoji ZWJ、URL、Windows 长路径、代码块和长 token 的 wrap/cursor 测试继续通过。
3. 80/100/120/200 四宽度完整 frame 不出现重叠、异常空白、scrollbar 越界或 composer 丢失。
4. 100/120/200 列的 model、路径和正文信息密度应保持符合预期。
5. 1,000 高频 delta、30 FPS 合帧、viewport anchor、resize、active-stream `Ctrl+C` 取消不回退。
6. 主动陪伴默认关闭和工具审批边界不变。

## 五、必须整改的问题

### 1. 窄屏 Header 仍显示低优先级信息

审核发现 `crates\yunxi-agent-tui\src\app.rs` 中 `YunxiTuiApp::header_for_width` 将 product、provider、model、cwd 全部交给 `TextLayout::priority_line`，导致 80 列只要能放下，就仍会保留 Optional/DebugOnly 段。当前 `full_frame_80x24.txt` 第 0 行实际显示 `model=deepseek-chat-u...` 与 `C:/.../yunxi-agent-cli`。

整改要求：

- 在 `header_for_width` 中建立显式宽度档位，而不是只依赖 `priority_line` 的可容纳裁剪。
- 窄屏档位必须只构造 product/version 与 provider/live 状态两个段。
- 窄屏档位的测试必须负向断言不包含 `model=`，不包含 cwd，不包含路径尾段，不包含 `.../` 形式。
- 80 列完整 frame golden 必须同步更新。

### 2. 中屏与宽屏语义需要锁定

审核要求中屏保留 model，宽屏才展示 cwd。当前实现没有明确三档信息集合，只是通过 `ClipPriority` 在不同宽度下自然取舍。

整改要求：

- 建议明确三档：
  - 窄屏：`width < 90`，只显示 `YunXi <version>` 与 `<provider> <mode>`。
  - 中屏：`90 <= width < 120`，显示产品、连接状态与 model。
  - 宽屏：`width >= 120`，显示产品、连接状态、model 与经路径边界压缩的 cwd。
- 如开发者选择其他阈值，必须在代码注释、测试名和开发日志中说明理由。
- 80、100、120、200 四宽度矩阵必须分别断言对应信息集合。

### 3. Snapshot 与测试证据需要同步

整改不只是改一行 header 文案，还必须补齐证据：

- 更新 `crates\yunxi-agent-tui\src\snapshots\full_frame_80x24.txt`。
- 增加或更新 `narrow_header_keeps_complete_status_tokens`，让它明确断言 model/cwd 不出现。
- 更新 `responsive_status_priority_is_stable_across_width_matrix`，分别断言 80 无 model/cwd、100 有 model 无 cwd、120/200 有 cwd。
- 保留现有 80/100/120/200 完整 frame 逐行宽度、scrollbar containment、composer 与 footer 断言。

## 六、源码接入点

### `crates\yunxi-agent-tui\src\app.rs`

重点修改：

- `YunxiTuiApp::header_for_width`
- `model_label`
- `compact_path`
- header 相关测试：
  - `narrow_header_keeps_complete_status_tokens`
  - `wide_header_preserves_provider_and_model_details`
  - `medium_header_keeps_model_before_cwd`
  - `responsive_status_priority_is_stable_across_width_matrix`

建议实现方式：

- 先根据 width 选择 header tier，再构造对应 segment list。
- 窄屏不要把 model/cwd 放入 segment list，避免 `priority_line` 在空间足够时重新带出低优先级字段。
- 中屏可以继续用 `TextLayout::priority_line` 裁剪 product/provider/model，但 cwd 不应参与。
- 宽屏再加入 cwd，cwd 仍通过 `compact_path` 做路径边界压缩。

### `crates\yunxi-agent-tui\src\text_layout.rs`

`TextLayout::priority_line` 本身已服务多处优先级裁剪，不建议为了 header 例外改变其全局行为。除非测试证明优先级算法自身有问题，否则本次应在 `app.rs` 的 header tier 构造层解决。

### `crates\yunxi-agent-tui\src\render.rs`

需要确保 80x24 snapshot 使用真实 header 路径，而不是测试专用字符串。同步更新 snapshot 后，应继续断言：

- header 没有越界。
- transcript、scrollbar、footer、composer 不重叠。
- active streaming cell、国际化文本、pinned history、new output below 场景仍在 frame 中可读。

### `crates\yunxi-agent-tui\src\snapshots\full_frame_80x24.txt`

必须更新首行 golden，确保 80 列只保留产品/版本与连接状态。不得在 golden 中保留 `model=`、`C:/`、`D:/`、`.../yunxi-agent-cli` 等宽屏才应出现的信息。

## 七、参考源码建议

继续参考总纲图指定的显示宽度和折行边界思想：

- `D:\源码\codex\codex-rs\tui\src\width.rs`
- `D:\源码\codex\codex-rs\tui\src\wrapping.rs`
- `D:\源码\codex\codex-rs\tui\src\chatwidget\rendering.rs`
- `D:\源码\codex\codex-rs\tui\src\render\renderable.rs`

同时可参考 Lazygit 的窄屏信息优先级逻辑：窄屏不是“能塞多少塞多少”，而是按语义裁掉低优先级上下文。

参考边界：

- 只抽取信息优先级、显示宽度和响应式裁剪思路。
- 不复制上游 UI 代码，不引入上游 TUI runtime。
- 不依赖 `vendor/codex-rs`、`codex-*` crate 或 `yunxi-agent-codex`。
- 非 Rust 参考源码只参考逻辑，必须以 YunXi 自身 Rust 模块复刻。

## 八、推荐执行顺序

1. 等待用户明确整改发布编号与 tag 策略。
2. 在 `app.rs` 中为 header 建立显式窄/中/宽档位。
3. 更新 header 单元测试，补齐 model/cwd 负向断言。
4. 更新 80x24 完整 frame golden。
5. 跑 TUI 相关测试确认 snapshot 与 header 行为。
6. 再统一执行 workspace 验证、release 验证、Evaluation Harness、真实 Provider 和真实在线 TUI 复核。
7. 经用户确认后清理 `target`、`.yunxi` 或其他临时产物。
8. 按用户确认的发布编号创建新发布提交和新的 annotated tag，再 non-force 推送并重新审核。

## 九、最低测试要求

必须新增或调整以下测试：

1. 80 列 header 包含产品/版本。
2. 80 列 header 包含 provider/live 或 provider/offline 状态。
3. 80 列 header 不包含 `model=`。
4. 80 列 header 不包含 cwd、路径尾段或 `.../` 路径截断形式。
5. 100 列 header 包含 model，但不包含 cwd。
6. 120 列与 200 列 header 允许显示经路径边界压缩或完整 cwd。
7. `full_frame_80x24.txt` 与真实 render 输出一致。
8. 现有 `TextLayout`、URL、Windows 路径、代码块、CJK、Emoji、cursor、composer、approval、viewport、resize、cancel、30 FPS、四宽度 snapshot 回归继续通过。

## 十、真实 TUI 复核要求

重新审核前需要实际复核：

1. 80x24：首行只显示产品/版本与连接状态。
2. 100 宽度：显示 model，不显示 cwd。
3. 120/200 宽度：显示 cwd 时不得遮挡状态或造成重叠。
4. resize 从 120/200 缩到 80 后，model/cwd 应立即消失。
5. resize 从 80 放大到 100/120 后，model/cwd 按档位恢复。
6. 国际化输入、滚动、End follow-tail、active-stream `Ctrl+C` 取消不回退。
7. 普通视图不暴露 thinking、memory/context、协议 JSON、工具参数或内部错误栈。

## 十一、统一验证要求

完成整改后统一验证：

1. `cargo fmt --all`
2. `cargo fmt --all -- --check`
3. `cargo check --workspace`
4. `cargo test --workspace`
5. `cargo test -p yunxi-agent-tui`
6. `cargo build --workspace`
7. `cargo build -p yunxi-agent-cli --release --bins`
8. release `yunxi --version`
9. Evaluation Harness、golden、JSON、JSONL 检查
10. offline TUI snapshot 与真实 TUI 复核
11. online Provider 和 online TUI 视觉/交互复核
12. `git diff --check`
13. `git status --short --branch`
14. 发布后远程 master 与 annotated tag refs 核验

涉及 `cargo clean`、递归删除 `.yunxi` 状态目录、删除 smoke 临时目录、强制移动、清空目录、系统级安装/卸载、修改 PATH/注册表/系统配置等操作前，必须先取得用户确认。

## 十二、文档同步要求

以下文档必须与实际代码、验证证据、版本号和发布状态一致：

- `D:\YunXi Agent\README.md`
- `D:\YunXi Agent\docs\extraction-status.md`
- `D:\YunXi Agent\docs\tui-presentation.md`
- `D:\YunXi Agent\docs\persona-memory.md`
- `D:\YunXi Agent\docs\development-log.md`
- `D:\YunXi Agent\docs\reports\2026-07-19-173413-yunxi-agent-v2-0-4-i18n-responsive-layout-development-report.md`
- `D:\YunXi Agent\docs\reports\2026-07-19-205131-yunxi-agent-v2-0-4-responsive-header-remediation-development-report.md`
- 如新增 evidence、snapshot 索引或状态文档，也必须同步当前整改版本的实际状态。

## 十三、本报告生成状态

本次任务只根据 `v2.0.4` 审核报告生成整改开发报告，并同步日志。未修改 Rust 源码，未运行构建、测试、清理、提交、推送或创建 Git tag。

当前 Git 状态显示 `crates\yunxi-agent-cli\.yunxi` 为未跟踪目录，来源与审核报告所述真实 Provider 短请求产物一致。本次未读取、删除或清理该目录；如后续需要处理，必须先确认具体路径并取得用户许可。

署名：开发报告撰写者

## 十四、整改发布编号确认

项目负责人已明确确认本次整改发布编号与新 tag 为 `v2.0.4-hotfix.1`。已发布
`v2.0.4` 以及 `v2.0.2`、`v2.0.2-hotfix.1`、`v2.0.3`、
`v2.0.3-hotfix.1` 均保持不可移动、不可删除、不可覆盖。本次未进入下一版本功能开发。

## 十五、实际整改实现

2026-07-19 21:36:58 +08:00 前已完成以下整改：

- `D:\YunXi Agent\crates\yunxi-agent-tui\src\app.rs` 在调用共享优先级裁剪前先选择
  header 语义档位：小于 90 列只构造产品/版本与连接状态，90 至 119 列增加 model 但
  不构造 cwd，120 列及以上增加经路径边界压缩或完整保留的 cwd。
- 80 列测试新增 `model=`、盘符 cwd、路径尾段和 `.../` 负向断言；100 列锁定有 model
  无 cwd；120 与 200 列锁定 model 和 cwd 均可见。
- 四份完整 frame snapshot 通过真实 `render_tui_frame` 重新生成；80 列首行只剩
  `YunXi v2.0.4-hotfix.1 | deepseek live`，100 列无 cwd，120/200 列有 cwd。
- workspace、CLI、persona、runtime、Evaluation Harness、README 和状态文档统一为
  `2.0.4-hotfix.1`，并新增
  `D:\YunXi Agent\docs\reports\evidence\2026-07-19-v2-0-4-hotfix-1-responsive-header-evidence.md`。
- `TextLayout` 全局优先级算法、Unicode grapheme、wrap/cursor、approval、viewport、
  1,000 delta、30 FPS、resize、cancel、主动陪伴默认关闭和工具审批边界均未改动。

## 十六、统一验证结果

- `cargo fmt --all`、fmt check、`cargo check --workspace`、`cargo test --workspace`、
  `cargo test -p yunxi-agent-tui`、`cargo build --workspace` 与 release 双 binary 构建通过。
- TUI 101/101、CLI integration 44/44、provider 44/44，其余 workspace 单元、集成和
  doc tests 全部通过。
- 两个 release binary 均返回 `yunxi 2.0.4-hotfix.1`；`yunxi.exe` SHA-256 为
  `27C7332C5D46188EB00F96694575B8E5CBA51ED9168B976ACEA1C709E2436B29`，
  `yunxi-agent-cli.exe` 为
  `9354BBDABBEB568521CDABBFAEDF59B1CAAABFF57E71F2C35CC8B90EC1DC62B6`。
- Evaluation Harness 为 `2.0.4-hotfix.1`，31/31、失败 0、golden true、JSON 可解析、
  JSONL 恰好一行，质量率 1.0，主动边界违规和 approval bypass 均为 0。
- offline one-shot 通过；DeepSeek live one-shot 退出码 0，精确输出
  `V204H1_LIVE_OK`。

## 十七、真实 TUI 复核

Windows ConPTY offline 会话退出码 0、66,212 bytes、7 个检查点；DeepSeek live /
`deepseek-v4-flash` 会话退出码 0、112,368 bytes、11 个检查点。两者覆盖 80/100/120/200
及双向 resize，证实窄屏 model/cwd 立即消失、中屏只恢复 model、宽屏恢复 cwd；同时覆盖
国际化 bracketed paste、Emoji ZWJ、组合字符退格、PageUp、End、active `Ctrl+C`、
partial 保留、取消通知和下一轮 `[assistant] V204H1_NEXT_OK`。

驱动只在内存中维护 terminal buffer 与泄漏扫描，没有保存 raw 流。普通终端输出未发现
API key、Authorization/Bearer、`arguments_json`、provider wire、memory/context 或内部
错误栈标记。

## 十八、清理与发布状态

用户明确授权后执行 `cargo clean`，移除 17,265 个文件、4.8 GiB；随后在不读取内容的
前提下递归删除精确 CLI `.yunxi` 路径。最终 `D:\YunXi Agent\target` 与
`D:\YunXi Agent\crates\yunxi-agent-cli\.yunxi` 均不存在，仓库原有 `.tmp` 保留，未触碰
其他目录。

截至本节更新时，尚未创建发布提交、annotated `v2.0.4-hotfix.1` tag 或远程推送。
发布时只能新增该 tag 并对 `master` 与新 tag 执行 non-force 推送；所有旧 tag 必须再次
核验不变。

署名：开发者
