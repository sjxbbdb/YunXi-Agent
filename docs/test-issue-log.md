# YunXi Agent 测试问题记录

本文件记录用户测试期间发现的问题。测试阶段只记录问题，不在未获授权时修改实现。

## ISSUE-001：TUI 中 shell 工具调用失败并返回无诊断错误

- **发现时间**：2026-08-01 16:44:37 +08:00
- **测试环境**：Windows PowerShell，YunXi Agent v2.2.0，`deepseek_live`，`model=deepseek-v4-flash`，`backend=yunxi`，`source=auto_live`，TUI 交互模式
- **测试功能**：工具调用 / shell 命令执行
- **用户操作**：在交互会话中要求 YunXi 执行一个简单 shell 命令，以确认工具链是否正常。
- **预期结果**：工具调用进入审批或执行流程，命令完成后返回 stdout/stderr、退出状态和可定位的错误信息。
- **实际结果**：
  - TUI 显示工具状态：`shell · running; path=running · approval_required -> running; output captured: 1 line(s), 1 char(s)`。
  - 随后返回：`YX-UNKNOWN-001 operation failed: operation failed; retryable=yes; next: Open details for diagnostics, then retry if appropriate.`
  - 未显示具体 shell 命令失败原因、退出码、stderr 或可直接定位的诊断细节。
  - TUI 仍停留在 Composer，可继续输入。
- **初步分类**：工具执行链 / 错误呈现
- **严重程度**：P1（核心工具调用失败；当前未确认是否为稳定复现）
- **状态**：待复现
- **证据截图**：`C:\Users\24763\AppData\Local\Temp\codex-clipboard-04984b0a-d012-4f98-b530-0ae32a4713bc.png`
- **补充证据**：
  - `C:\Users\24763\AppData\Local\Temp\codex-clipboard-4ae2b85e-d8e7-418d-ba2f-245c8c643fb4.png`
  - 该截图里同一条工具调用在 transcript 中出现了重复的 `[tool] shell: running; output captured: 1 line(s), 1 char(s)` 行。
  - 错误仍然是泛化的 `YX-UNKNOWN-001`，没有展开具体失败源。
- **相关日志**：待用户提供 TUI Details 或 `.yunxi` 运行日志后补充
- **待确认事项**：
  1. 失败的是命令启动、审批状态切换、输出捕获还是结果归一化阶段。
  2. 是否仅发生在 Windows PowerShell，还是所有 shell 工具调用均受影响。
  3. 打开 Details 后是否存在被默认 transcript 隐藏的底层错误。
  4. 重试同一命令是否稳定复现。
- **处理原则**：本条记录创建时未修改源码、未重试命令、未清理任何目录。

## ISSUE-002：历史视图滚动定位异常，底部输出不可达

- **发现时间**：2026-08-01 17:01:11 +08:00
- **测试环境**：Windows PowerShell，YunXi Agent v2.2.0，`deepseek_live`，`model=deepseek-v4-flash`，`backend=yunxi`，`source=auto_live`，TUI 交互模式
- **测试功能**：TUI 历史记录滚动 / 终端输出定位
- **用户操作**：在历史输出较长时，尝试通过滚动条查看最底部输出内容。
- **预期结果**：滚动条可到达尾部，底部最新输出可见，历史视图应能平滑滚到底。
- **实际结果**：
  - 截图显示 `history | cells=74`，但 transcript 视图停在 `Transcript 86-106 / 117 | history`。
  - 滚动条位置明显卡在中间附近，无法继续看到更底部的新输出。
  - 用户明确反馈“看不到最底下的输出，滑块卡到中间就不动了”。
- **初步分类**：TUI 历史滚动 / 视图同步
- **严重程度**：P2（影响可读性和诊断效率，但不阻断功能执行）
- **状态**：待复现
- **证据截图**：
  - `C:\Users\24763\AppData\Local\Temp\codex-clipboard-b6eaa730-c420-4c03-a309-86da085014b2.png`
  - `C:\Users\24763\AppData\Local\Temp\codex-clipboard-fb38fcb4-9f50-4575-9207-6a2b3fcd68cf.png`
- **补充证据**：
  - `C:\Users\24763\AppData\Local\Temp\codex-clipboard-dfc65907-3e76-43c1-9b93-266d76b292c8.png`
  - `C:\Users\24763\AppData\Local\Temp\codex-clipboard-6255e2b2-02d7-436c-b17b-645e90f48621.png`
  - `C:\Users\24763\AppData\Local\Temp\codex-clipboard-1bc76f61-752e-45ce-8085-49cd2605e5c7.png`
  - `C:\Users\24763\AppData\Local\Temp\codex-clipboard-98bf6adb-e32a-4d3f-9673-39e86fef5576.png`
  - `C:\Users\24763\AppData\Local\Temp\codex-clipboard-825270fc-a7d2-4465-887d-4ce4b9b58062.png`
  - 补充截图显示 transcript 已处于 `tail` 且行号到达尾部，例如 `Transcript 25-45 / 45 | tail`，但右侧滚动条滑块视觉位置仍未贴近底部；需确认这是滚动条比例计算问题还是视口状态同步问题。
- **待确认事项**：
  1. 滚动条是否只是在 history 模式下失效，还是在 transcript tail / details / debug 也会失效。
  2. 问题是否与窗口高度、内容总行数、debug 开关或重复 debug 行有关。
  3. 使用 PgUp/PgDn、滚轮、拖动滚条时是否有不同表现。
  4. 是否存在“底部最新输出已到达，但视口没有跟随”的状态同步问题。
- **处理原则**：本条记录创建时未修改源码、未清理任何目录、未重置视图。

## 2026-08-01 17:04:07 +08:00 状态类命令测试记录

- **测试环境**：Windows PowerShell，YunXi Agent v2.2.0，`deepseek_live`，`model=deepseek-v4-flash`，`backend=yunxi`，`source=auto_live`，TUI 交互模式
- **测试命令与结果**：
  - `/status`：通过，返回 session、cwd、provider、model、tools、mcp、usage 等状态。
  - `/tools`：通过，列出 8 个固定工具：shell、patch、mcp、skill、multi_agent、tool_search、request_user_input、view_image。
  - `/cost`：通过，返回 `last_turn_usage: none` 与 `session_usage: unavailable`。
  - `/session`：通过，返回当前 session 状态、turns、cwd、provider 和 model。
  - `/provider`：通过，返回 `provider: deepseek`。
  - `/model`：通过，返回 `model: deepseek-v4-flash`。
- **本轮新增问题**：无新的独立问题。
- **关联既有问题**：截图继续支持 ISSUE-002，滚动条视觉位置与 tail/尾行状态不完全一致。
- **证据截图**：
  - `C:\Users\24763\AppData\Local\Temp\codex-clipboard-b15caa69-2268-4f4a-919c-157964a556b0.png`
  - `C:\Users\24763\AppData\Local\Temp\codex-clipboard-dfc65907-3e76-43c1-9b93-266d76b292c8.png`
  - `C:\Users\24763\AppData\Local\Temp\codex-clipboard-6255e2b2-02d7-436c-b17b-645e90f48621.png`
  - `C:\Users\24763\AppData\Local\Temp\codex-clipboard-1bc76f61-752e-45ce-8085-49cd2605e5c7.png`
  - `C:\Users\24763\AppData\Local\Temp\codex-clipboard-98bf6adb-e32a-4d3f-9673-39e86fef5576.png`
  - `C:\Users\24763\AppData\Local\Temp\codex-clipboard-825270fc-a7d2-4465-887d-4ce4b9b58062.png`
- **处理原则**：本轮只记录测试结果，未修改源码、未重试失败工具、未清理任何目录。

## 2026-08-01 17:09:30 +08:00 Companion/Controls 与长文本换行测试记录

- **测试环境**：Windows PowerShell，YunXi Agent v2.2.0，`deepseek_live`，`model=deepseek-v4-flash`，`backend=yunxi`，`source=auto_live`，TUI 交互模式
- **测试命令与结果**：
  - `/controls`：通过，打开 `Companion UX & Controls` 面板；显示 Local companion OFF、Cloud control OFF、memory off、persona on、relationship read-only。
  - `/companion`：通过，打开同一控制面板并刷新 Recent change。
  - `/debug events on`：通过，状态栏变为 `debug on`，返回 `event debug enabled`。
  - 长中文三段输出测试：通过，assistant 输出三段中文长文本；自动换行可读。
- **观察事项**：
  - 用户输入 `PgUp/PgDn` 时被作为普通自然语言 prompt 发送给模型；这不是按键测试结果，后续滚动测试需实际按 PageUp/PageDown 键或用鼠标滚轮。
  - 开启 debug 后 transcript 细胞数量快速增长到 800，属于 debug on 的可见副作用；是否需要节流或折叠，待后续体验评估。
- **本轮新增问题**：无新的独立问题。
- **关联既有问题**：长输出后仍进入 history 视图并显示右侧滚动条，继续关注 ISSUE-002 的滚动条定位表现。
- **证据截图**：
  - `C:\Users\24763\AppData\Local\Temp\codex-clipboard-0d43b0c4-35b7-4aa2-9b99-ce102545a785.png`
  - `C:\Users\24763\AppData\Local\Temp\codex-clipboard-6343ea08-3223-4a5e-be7a-bfebdb4088b0.png`
  - `C:\Users\24763\AppData\Local\Temp\codex-clipboard-064959fd-1f25-4301-b66c-ae04890a58d5.png`
  - `C:\Users\24763\AppData\Local\Temp\codex-clipboard-b259cb30-3544-4209-8418-76aa2e2be3aa.png`
- **处理原则**：本轮只记录测试结果，未修改源码、未清理目录、未执行 memory clear 或 controls clear。

## ISSUE-003：TUI/默认路径下长期记忆写入失败，明确“请记住”后没有生成 pending/active 记忆

- **发现时间**：2026-08-01 17:16:45 +08:00
- **测试环境**：Windows PowerShell，YunXi Agent v2.2.0，`deepseek_live`，`model=deepseek-v4-flash`，`backend=yunxi`，`source=auto_live`，TUI 交互模式；另一个 PowerShell 标签页执行 `yunxi memory ...`
- **测试功能**：长期记忆开关、记忆候选生成、pending/search 查询闭环
- **用户操作**：
  1. 执行 `yunxi memory status`，确认初始 `memory_enabled: false`、records 为 0。
  2. 执行 `yunxi memory on`，随后 `yunxi memory status` 显示 `memory_enabled: true`。
  3. 在 TUI 中输入明确记忆指令：记住测试偏好 `YUNXI_MEMORY_TEST_20260801`，以后测试报告使用“三段式：现象、影响、复现”。
  4. 执行 `yunxi memory pending` 和 `yunxi memory search YUNXI_MEMORY_TEST_20260801`。
- **预期结果**：
  - memory 开启后，明确“请记住”的低风险测试偏好应生成 pending 或 active 记忆。
  - `memory pending` 或 `memory search YUNXI_MEMORY_TEST_20260801` 至少应能找到候选记录。
- **实际结果**：
  - `yunxi memory on` 成功，状态变为 `memory_enabled: true`。
  - 明确记忆指令后，`yunxi memory pending` 返回 `records: 0`。
  - `yunxi memory search YUNXI_MEMORY_TEST_20260801` 返回 `records: 0`。
  - TUI 中没有形成可审批或可搜索的长期记忆记录。
  - 后续顺序复核显示，显式 `--memory-extraction provider` 的非交互 one-shot 记忆测试可以生成 active 记忆；因此该问题范围缩小为 TUI/默认 auto/rule-only 路径，不是 `memory` 存储底层完全不可用。
- **初步分类**：Persona/Memory 写入管线 / 明确记忆指令识别
- **严重程度**：P1（陪伴 Agent 的核心长期记忆闭环不可用）
- **状态**：待复现
- **证据截图**：
  - `C:\Users\24763\AppData\Local\Temp\codex-clipboard-423f8915-abef-49e4-b0d4-f283144f4646.png`
  - `C:\Users\24763\AppData\Local\Temp\codex-clipboard-657f38d6-66fd-42d3-a575-2a9cff2c7316.png`
  - `C:\Users\24763\AppData\Local\Temp\codex-clipboard-1013d2e2-f565-4812-b35d-f14e76434234.png`
- **补充证据**：
  - `C:\Users\24763\AppData\Local\Temp\codex-clipboard-0a848ad0-cecd-40cd-877d-a3bea2eae2ce.png`
  - 该截图显示显式指定项目 cwd 后，`yunxi --cwd 'D:\YunXi Agent' memory status` 返回 `memory_enabled: true` 且 records 为 0；`yunxi --cwd 'D:\YunXi Agent' memory pending` 仍返回 `records: 0`。
  - 这说明 ISSUE-003 不是单纯由查询 cwd 选择错误造成；至少项目 cwd 下也没有生成 pending 记忆。
  - `C:\Users\24763\AppData\Local\Temp\codex-clipboard-41dcdcee-85a4-4bbe-b65f-fc317131f6fc.png`
  - `C:\Users\24763\AppData\Local\Temp\codex-clipboard-fae6ac35-9d2d-4809-a76b-7c2a979cc21f.png`
  - 这组截图显示重启到 `D:\YunXi Agent` cwd 后，再次输入 `YUNXI_MEMORY_TEST_20260801_RESTART` 明确记忆指令，随后 `yunxi --cwd 'D:\YunXi Agent' memory pending` 和 `memory search YUNXI_MEMORY_TEST_20260801_RESTART` 仍均为 `records: 0`。
  - 非交互复核修正：此前并行执行 one-shot 与 `memory pending/search` 得到的 `records: 0` 是查询早于抽取落盘的假阴性；顺序复核后，`YUNXI_MEMORY_TEST_SHELL_20260801` 已生成 active 记忆 `mem-1785576472370-provider-0`。
- **待确认事项**：
  1. 当前 TUI turn 是否因为工具错误中断，导致 turn-end memory extraction 未运行。
  2. 记忆写入是否只在特定 `--memory-extraction` 模式下运行，当前默认 `auto` 是否没有触发。
  3. 明确“请记住”是否需要直接进入 memory candidate，而不是依赖模型自行调用其他工具。
  4. `memory on` 是否写入的是当前 cwd 的配置，而 TUI 会话是否已经加载旧配置，需要重启 TUI 才生效。
- **处理原则**：本条记录创建时未修改源码、未清理目录、未执行 memory clear。

## ISSUE-004：明确记忆指令误路由到 tool_search，并在用户目录 Temp\WinSAT 上报拒绝访问

- **发现时间**：2026-08-01 17:16:45 +08:00
- **测试环境**：Windows PowerShell，YunXi Agent v2.2.0，TUI 交互模式，当前 TUI 顶部显示 cwd 为 `C:\Users\24763`
- **测试功能**：工具路由、记忆意图处理、工具作用域安全
- **用户操作**：在 TUI 中要求“请记住一个测试偏好：YUNXI_MEMORY_TEST_20260801...”
- **预期结果**：
  - 明确记忆意图应进入长期记忆候选生成路径，或给出无法记忆的明确说明。
  - 不应把“保存记忆”误当作 workspace 搜索任务。
  - 工具不应无提示遍历用户目录下受保护临时目录。
- **实际结果**：
  - Debug transcript 显示工具路由到 `tool_search`，query 为 `memory record save preference`。
  - 工具请求需要 approval，随后执行失败。
  - 错误为：`YX-TOOL-001 tool execution failed... failed to read tool_search directory C:\Users\24763\AppData\Local\Temp\WinSAT: 拒绝访问。 (os error 5)`。
- **初步分类**：工具路由 / Workspace 作用域 / 错误诊断
- **严重程度**：P1（错误工具路由导致记忆任务失败，并触达用户目录受保护路径）
- **状态**：待复现
- **证据截图**：
  - `C:\Users\24763\AppData\Local\Temp\codex-clipboard-a6fc3fa9-c966-42be-a4e7-0751480f511a.png`
- **待确认事项**：
  1. 当 cwd 为普通用户目录时，是否应拒绝或警告使用该目录作为 YunXi workspace。
  2. `tool_search` 是否应该跳过不可访问目录，而不是将单个拒绝访问升级为整个 turn 失败。
  3. 记忆意图是否应从工具选择策略中排除 `tool_search`。
  4. TUI 是否应在 Details 中提供更清晰的修复建议，例如改用 `yunxi memory ...` 或重启到项目 cwd。
- **处理原则**：本条记录创建时未修改源码、未清理目录、未重试工具。

## 2026-08-01 17:16:45 +08:00 Companion 与 Memory 测试记录

- **测试环境**：Windows PowerShell，YunXi Agent v2.2.0，`deepseek_live`，`model=deepseek-v4-flash`，TUI 交互模式
- **测试结果**：
  - companion 自然语言陪伴输出：通过；开启陪伴后，assistant 按 15 分钟节奏给出三步安排，未主动调用 shell。
  - `/companion off`：通过；控制面板显示 `Local companion: OFF`，Recent change 为 companion disable completed。
  - `yunxi memory status`：通过；初始状态显示 `memory_enabled: false`、records 为 0。
  - `yunxi memory on`：通过；状态变为 `memory_enabled: true`，并输出安全说明。
  - `yunxi memory pending/search`：未通过；详见 ISSUE-003。
- **环境风险观察**：
  - 当前 TUI 顶部 cwd 显示为 `C:\Users\24763`，另一个 PowerShell 也在 `PS C:\Users\24763>` 下执行 memory 命令。
  - 因此本轮 memory storage 输出为 `C:\Users\24763\.yunxi\memory`，并且 tool_search 触达 `C:\Users\24763\AppData\Local\Temp\WinSAT`。
  - 后续涉及 memory、tool_search、shell 或项目文件的测试建议改用 `D:\YunXi Agent` 作为 cwd，避免把用户目录当作 workspace。
- **证据截图**：
  - `C:\Users\24763\AppData\Local\Temp\codex-clipboard-f5ba0eee-9efc-453d-89e2-fbd3433cdaa7.png`
  - `C:\Users\24763\AppData\Local\Temp\codex-clipboard-3ac2e4c6-98ad-481b-a9c7-008e673d8da5.png`
  - `C:\Users\24763\AppData\Local\Temp\codex-clipboard-423f8915-abef-49e4-b0d4-f283144f4646.png`
  - `C:\Users\24763\AppData\Local\Temp\codex-clipboard-657f38d6-66fd-42d3-a575-2a9cff2c7316.png`
  - `C:\Users\24763\AppData\Local\Temp\codex-clipboard-a6fc3fa9-c966-42be-a4e7-0751480f511a.png`
  - `C:\Users\24763\AppData\Local\Temp\codex-clipboard-1013d2e2-f565-4812-b35d-f14e76434234.png`
- **处理原则**：本轮只记录测试结果，未修改源码、未清理目录、未执行 memory clear。

## ISSUE-006：默认 auto 与 rule-only 记忆抽取不落盘，但显式 provider 模式可写入

- **发现时间**：2026-08-01 17:32:19 +08:00
- **测试环境**：Windows PowerShell，YunXi Agent v2.2.0，显式 `--cwd 'D:\YunXi Agent'`，`deepseek_live` 可用
- **测试功能**：Memory extraction mode 行为一致性
- **用户操作/自动测试**：
  1. `yunxi --cwd 'D:\YunXi Agent' --memory-extraction provider "请记住...YUNXI_MEMORY_TEST_SHELL_20260801..."`。
  2. 顺序执行 `memory list/search`。
  3. `yunxi --cwd 'D:\YunXi Agent' "请记住...YUNXI_MEMORY_TEST_AUTOSEQ_20260801..."`。
  4. 顺序执行 `memory search YUNXI_MEMORY_TEST_AUTOSEQ_20260801`。
  5. `yunxi --cwd 'D:\YunXi Agent' --memory-extraction rule-only "请记住...YUNXI_MEMORY_TEST_RULESEQ_20260801..."`。
  6. 顺序执行 `memory search YUNXI_MEMORY_TEST_RULESEQ_20260801`。
- **预期结果**：
  - 默认 auto 在 live provider 已配置时，应能对明确“请记住”的低风险偏好生成记忆，或至少给出为何未保存的可读说明。
  - rule-only 对“请记住一个测试偏好”这类明确模式也应能生成候选，或文档/提示应明确说明只能用 provider。
- **实际结果**：
  - 显式 `--memory-extraction provider` 成功生成 active 记忆：`mem-1785576472370-provider-0`，内容包含 `YUNXI_MEMORY_TEST_SHELL_20260801` 和“三段式：现象、影响、复现”。
  - 默认 auto 顺序测试返回 `已记录测试偏好。`，但 `memory search YUNXI_MEMORY_TEST_AUTOSEQ_20260801` 仍为 `records: 0`。
  - `--memory-extraction rule-only` 顺序测试返回 `已记录测试偏好。`，但 `memory search YUNXI_MEMORY_TEST_RULESEQ_20260801` 仍为 `records: 0`。
- **初步分类**：Persona/Memory extraction mode 路由 / 用户承诺与实际落盘不一致
- **严重程度**：P1（默认体验下用户以为已记住，但实际未保存）
- **状态**：已初步复现
- **证据**：
  - 项目截图：`D:\YunXi Agent\docs\reports\evidence\manual-testing\2026-08-01\20260801-1727-memory-noninteractive-shell-evidence.png`
  - 项目截图：`D:\YunXi Agent\docs\reports\evidence\manual-testing\2026-08-01\20260801-173219-desktop-state.png`
  - `memory list --json` 显示 `mem-1785576472370-provider-0` 为 active，scope 为 `global_user`。
- **待确认事项**：
  1. 默认 auto 是否应在 live provider 下选择 provider extractor；若不应，CLI 输出不应让用户误以为“已记录”。
  2. rule-only 是否缺少中文“请记住/偏好”规则，或规则候选只在 provider 模式内被消费。
  3. TUI turn 结束后的 memory extraction 是否与 one-shot 使用了不同路径。
- **处理原则**：本条记录创建时未修改源码、未清理目录、未执行 memory clear。

## ISSUE-007：`companion check` 触发 Rust panic，候选记忆为空时越界访问

- **发现时间**：2026-08-01 17:36:55 +08:00
- **测试环境**：Windows PowerShell，YunXi Agent v2.2.0，显式 `--cwd 'D:\YunXi Agent'`
- **测试功能**：`yunxi companion check` / Companion planner / Memory merge candidate
- **复现命令**：
  - `yunxi --cwd 'D:\YunXi Agent' companion check "用户正在连续测试 YunXi，需要一个不打扰的节奏提醒，不要调用工具"`
  - `RUST_BACKTRACE=full` 下同命令稳定复现。
- **预期结果**：
  - `companion check` 应返回保守的陪伴建议、空建议或可读错误。
  - 即使没有可合并的记忆候选，也不应 panic。
- **实际结果**：
  - 进程退出码为 1。
  - panic 输出：`index out of bounds: the len is 0 but the index is 0`。
  - panic 位置：`crates\yunxi-agent-storage\src\lib.rs:1347:65`。
  - `RUST_BACKTRACE=full` 可复现，但 release 符号帧大多显示为 `<unknown>`。
- **源码定位**：
  - `D:\YunXi Agent\crates\yunxi-agent-storage\src\lib.rs:1343-1347`
  - 当前代码片段：`(candidates.len() == 1).then_some(candidates[0])`
  - 根因判断：`Option::then_some` 会立即求值参数；当 `candidates.len() == 0` 时仍会先访问 `candidates[0]`，导致越界 panic。应改为惰性求值或显式分支。
- **初步分类**：Storage / Memory merge / Panic safety
- **严重程度**：P0/P1（普通 CLI 子命令可触发进程 panic）
- **状态**：已复现并定位根因
- **证据**：
  - 自动测试终端输出：普通和 `RUST_BACKTRACE=full` 两次复现。
  - 源码定位：`D:\YunXi Agent\crates\yunxi-agent-storage\src\lib.rs:1347`
- **待确认事项**：
  1. 该越界是否也会影响普通 turn 的 memory extraction。
  2. 是否已有历史 memory 为空或实体为空的记录会扩大触发面。
  3. 修复后需要补 `candidates` 为空、1 条、多条且无共享实体的单元测试。
- **处理原则**：本条记录创建时未修改源码、未清理目录、未执行 memory clear。

## ISSUE-005：TUI runtime warning 存在乱码，错误诊断文本不可读

- **发现时间**：2026-08-01 17:22:36 +08:00
- **测试环境**：Windows PowerShell，YunXi Agent v2.2.0，`deepseek_live`，`model=deepseek-v4-flash`，`backend=yunxi`，`source=auto_live`，TUI 交互模式，cwd 为 `D:\YunXi Agent`
- **测试功能**：TUI 错误呈现 / runtime warning 编码
- **用户操作**：在重启后的项目 cwd TUI 中输入明确长期记忆测试指令。
- **预期结果**：runtime warning 应显示可读中文或英文诊断，至少应保留可理解的错误原因。
- **实际结果**：
  - transcript 中多次出现 `[notice] runtime warning:`，后面的内容是 `�`、菱形问号和破损字符。
  - 乱码出现在工具失败附近，无法通过默认 transcript 判断具体原因。
- **初步分类**：TUI 错误呈现 / 文本编码 / 诊断可读性
- **严重程度**：P2（不一定阻断执行，但严重影响问题定位）
- **状态**：待复现
- **证据截图**：
  - `C:\Users\24763\AppData\Local\Temp\codex-clipboard-41dcdcee-85a4-4bbe-b65f-fc317131f6fc.png`
  - `C:\Users\24763\AppData\Local\Temp\codex-clipboard-ab26bf4c-f5a3-4bac-9b2d-d41bd8901220.png`
- **待确认事项**：
  1. 乱码来自 shell stdout/stderr 解码、PowerShell 编码、provider 文本，还是 TUI 渲染层。
  2. `/details` 是否能显示未损坏的原始诊断。
  3. 乱码是否只在工具失败时出现，还是普通中文 warning 也会出现。
- **处理原则**：本条记录创建时未修改源码、未清理目录、未重试工具。

## 2026-08-01 17:22:36 +08:00 重启到项目 cwd 后的 Memory 复测记录

- **测试环境**：Windows PowerShell，YunXi Agent v2.2.0，TUI 顶部显示 cwd 为 `D:\YunXi Agent`
- **测试命令与结果**：
  - TUI 输入 `YUNXI_MEMORY_TEST_20260801_RESTART` 明确记忆指令：未通过，触发多次工具调用和错误，未形成长期记忆。
  - `yunxi --cwd 'D:\YunXi Agent' memory pending`：返回 `records: 0`。
  - `yunxi --cwd 'D:\YunXi Agent' memory search YUNXI_MEMORY_TEST_20260801_RESTART`：返回 `records: 0`。
- **关联既有问题**：
  - ISSUE-003：重启到项目 cwd 后仍无 pending/active 记忆，长期记忆闭环失败进一步确认。
  - ISSUE-001：shell 工具调用仍多次失败，错误为 `YX-TOOL-001` 或 `YX-UNKNOWN-001`，仍需诊断。
  - ISSUE-005：runtime warning 乱码。
- **证据截图**：
  - `C:\Users\24763\AppData\Local\Temp\codex-clipboard-41dcdcee-85a4-4bbe-b65f-fc317131f6fc.png`
  - `C:\Users\24763\AppData\Local\Temp\codex-clipboard-ab26bf4c-f5a3-4bac-9b2d-d41bd8901220.png`
  - `C:\Users\24763\AppData\Local\Temp\codex-clipboard-fae6ac35-9d2d-4809-a76b-7c2a979cc21f.png`
- **处理原则**：本轮只记录测试结果，未修改源码、未清理目录、未执行 memory clear。

## 2026-08-01 17:19:32 +08:00 项目 cwd Memory 状态补充测试

- **测试环境**：Windows PowerShell，YunXi Agent v2.2.0，显式 `--cwd 'D:\YunXi Agent'`
- **测试命令与结果**：
  - `yunxi --cwd 'D:\YunXi Agent' memory status`：通过，显示 `memory_enabled: true`，records/active/pending/rejected/archived 均为 0。
  - `yunxi --cwd 'D:\YunXi Agent' memory pending`：返回 `records: 0`。
- **结论**：项目 cwd 下长期记忆已开启，但仍无 pending 记录；继续支持 ISSUE-003。
- **证据截图**：
  - `C:\Users\24763\AppData\Local\Temp\codex-clipboard-0a848ad0-cecd-40cd-877d-a3bea2eae2ce.png`
- **处理原则**：本轮只记录测试结果，未修改源码、未清理目录、未执行 memory clear。

## 2026-08-01 19:31:21 +08:00 整改结果

- ISSUE-001：已修复。shell / tool 执行失败的呈现链路已稳定到可诊断错误与具体上下文。
- ISSUE-002：已修复。TUI transcript 滚动条在 tail 状态下可到达视觉底部。
- ISSUE-003：已缓解并验证。非交互 `rule-only` / 默认 `auto` 记忆写入可落盘；TUI 侧已加记忆意图防误路由提示。
- ISSUE-004：已修复。`tool_search` 不再对明确记忆意图进行 workspace 文件扫描，并跳过用户主目录扫描。
- ISSUE-005：已修复。runtime warning 展示会归一化无效编码标记，不再直接暴露乱码。
- ISSUE-006：已修复并验证。默认 `auto` 与 `rule-only` 路径的记忆抽取闭环恢复。
- ISSUE-007：已修复。`companion check` 的空候选越界 panic 已在先前修复后保持稳定。
- 验证命令：`cargo test --workspace`
- 验证结果：通过
- 署名：开发者
