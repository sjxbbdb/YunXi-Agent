# YunXi Agent v1.2 DeepSeek Default Provider Design

生成时间：2026-07-11 14:02:39 +08:00

## 目标

YunXi Agent v1.2 要把已经存在并通过独立 smoke 验证的 DeepSeek
OpenAI-compatible transport 接入正式 CLI 路径。完成后，在本机已配置 DeepSeek
凭据时，直接运行 `yunxi` 或 `yunxi "prompt"` 应调用真实模型；没有凭据时仍可明确回退到
offline provider。

本阶段不新增 TUI、桌面端、云任务、更新器、doctor、completion、marketplace 或复杂安装器。

## 已确认方案

本阶段采用“自动选择 + 显式覆盖”方案：

- 默认模式为 `auto`。检测到可用 provider 凭据时启用真实 provider，否则使用 offline
  provider。
- 保留 `--provider-live`，用于强制启用真实 provider；缺少凭据时启动失败并给出脱敏错误。
- 新增 `--offline`，用于强制使用静态 provider，并与 `--provider-live` 互斥。
- 在未显式指定 provider 且检测到 `DEEPSEEK_API_KEY` 时，自动采用 `deepseek` profile。
- 本机的 `api.txt` 只作为凭据导入和 live 验收输入，不成为 YunXi 的运行时依赖。

未采用的替代方案：

1. 继续要求每次传入 `--provider-live`。兼容性最保守，但直接运行 `yunxi` 仍容易误入离线模式。
2. 无条件绑定 DeepSeek，缺少密钥就拒绝启动。体验直接，但破坏 offline 回退和 provider
   可替换边界。

自动模式既能让当前用户直接获得真实回答，也保留脚本、离线开发和后续替换模型层的空间。

## 架构边界

### Provider 层

`yunxi-agent-provider` 继续拥有 provider profile、认证来源和 transport 构造，不把 DeepSeek
逻辑放进 REPL：

- `ProviderConfig::from_agent_config` 解析显式 CLI/config provider、
  `YUNXI_PROVIDER_PROFILE` 和自动检测出的 DeepSeek profile。
- `ProviderBootstrap` 负责选择认证来源，并提供“不暴露密钥值”的凭据可用性判断。
- DeepSeek 继续使用现有 `ProviderConfig::deepseek()`、Chat Completions wire API、retry、
  stream parser 和错误分类。
- provider 接口保持 `AgentProvider`，runtime 不依赖 DeepSeek 专有类型。

认证解析优先级：

1. `YUNXI_PROVIDER_API_KEY`。
2. `YUNXI_PROVIDER_API_KEY_ENV` 指向的环境变量。
3. DeepSeek profile 下的 `DEEPSEEK_API_KEY`。
4. 其他 OpenAI-compatible profile 下的 `OPENAI_API_KEY`。

空字符串视为未配置。任何错误、banner、JSONL 或日志只能包含 provider/profile、环境变量名、
HTTP 状态和脱敏 classification，不能包含凭据值或 Authorization header。

### CLI 层

`yunxi-agent-cli` 增加一个小型 provider selection 模块，将 CLI 标志与凭据状态解析为：

- `AutoLive`
- `ForcedLive`
- `ForcedOffline`
- `AutoOffline`

one-shot 与 interactive 共用同一解析函数，避免两条路径行为漂移。CLI 仍把最终的 live/offline
决定传给 `YunXiRuntimeBackend::for_workspace_with_live_provider` 或
`YunXiRuntimeBackend::for_workspace`。

交互式 session 保存“请求模式”而不是启动时的裸布尔值。`/provider` 或 `/model` 修改配置后，
下一轮重新解析 provider；banner 与 `/session` 显示最终模式、profile 和 model，不显示认证值。

### 本机凭据导入

正式二进制不读取桌面文件。仓库内增加一个可审计的 PowerShell 辅助脚本，仅在用户明确运行时：

- 从传入的 API 文件中读取 DeepSeek 凭据到内存；多候选文件要求显式指定候选序号。
- 不向 stdout、stderr 或 transcript 输出凭据。
- 将 `DEEPSEEK_API_KEY`、`YUNXI_PROVIDER_PROFILE=deepseek` 和已确认的默认 model 写入当前用户环境。
- 只输出是否成功、变量名和是否需要重新打开 PowerShell。
- 遇到零个候选时直接失败；遇到多个候选且未显式指定序号时直接失败，不猜测、不覆盖原配置。

当前任务的 live 验收可直接通过临时进程环境使用 `api.txt`，避免为了测试把凭据复制进仓库。

## 数据流

```text
CLI arguments + AgentConfig + environment
                  |
                  v
        Provider selection (auto/live/offline)
            |                    |
            | live               | offline
            v                    v
    ProviderBootstrap       StaticProvider
            |
            v
 OpenAiTransportProvider (DeepSeek profile)
            |
            v
   YunXi runtime -> stream/events -> terminal renderer
```

自动模式只有在凭据可用性检查通过后才进入 live 路径。真实请求中的密钥只在 provider transport
构造 Authorization header 时使用，不进入 Agent event、session storage 或 renderer。

## CLI 行为

| 场景 | 结果 |
| --- | --- |
| `yunxi`，存在 DeepSeek 凭据 | 进入交互模式并显示 `provider: deepseek`、`mode: live` |
| `yunxi "你好"`，存在 DeepSeek 凭据 | 执行真实 one-shot 请求 |
| 默认启动，无任何凭据 | 进入 offline 模式并明确显示回退状态 |
| `yunxi --provider-live`，无凭据 | provider error，退出码保持 provider 分类 |
| `yunxi --offline` | 无论是否存在凭据都使用 offline provider |
| 同时传入 `--provider-live --offline` | Clap 参数冲突错误 |
| `--json` / `--jsonl` | 不混入 banner 或未结构化诊断 |

## 错误处理

- 401/403 保持 authentication classification，不输出响应中的疑似 token。
- 429、5xx、transport timeout 沿用现有 retry 与 provider error 分类。
- 自动模式检测到“变量存在但值为空”时按无凭据处理。
- 强制 live 模式缺少凭据时在发起 HTTP 请求前失败。
- DeepSeek profile 的 base URL、model 或 stream 可继续由现有通用环境变量覆盖。
- provider 请求失败不自动切换到 offline，以免把真实故障伪装成成功回答。

## 测试与统一验收

遵守项目硬约束：先一次性写完测试和实现，不在构建过程中运行测试；全部接线结束后统一验证。

测试覆盖：

- provider profile 与认证来源优先级。
- auto、forced-live、forced-offline 四种解析结果。
- one-shot 与 interactive 使用同一选择结果。
- `--provider-live` / `--offline` 冲突。
- 缺少凭据时的脱敏错误。
- JSON/JSONL 输出纯净性。
- DeepSeek stream 与 non-stream 真实请求。
- assistant 内容能够在交互终端显示，而不是 offline 固定确认语。
- owned source、文档和 Git diff 的密钥模式扫描。
- 默认依赖树不包含 `vendor/codex-rs`、`codex-*` 或 `yunxi-agent-codex`。

最终门包括 `cargo fmt`、workspace tests/check、release build、CLI one-shot/interactive smoke、
DeepSeek live smoke、依赖扫描、秘密扫描、CodeGraph 同步和安装后的 PATH smoke。验证完成后清理
`target`、仓库根目录 `.yunxi` 和临时 fixture。

## 版本与发布

- workspace 与 CLI 版本提升到 `1.2.0`。
- 新建 `v1.2.0` tag，保留且不移动 `v1.0.0`、`v1.1.0`。
- 更新 README、`docs/extraction-status.md`、v1.2 开发报告和桌面开发日志。
- GitHub 的远端读取、blob/tree/commit/ref/tag 写入和最终核验全部通过 GitHub API 完成。
- API token 仅从本机私密输入加载，不出现在命令输出、日志、提交或远端树中。

## 完成定义

v1.2 只有同时满足以下条件才算完成：

- 新 PowerShell 中直接运行 `yunxi` 能在检测到本机 DeepSeek 凭据后进入真实模型会话。
- 输入“你好”能获得 DeepSeek 返回的 assistant 内容，而不是 offline 固定确认语。
- 无凭据和 `--offline` 场景仍可稳定运行。
- one-shot、interactive、JSON、JSONL 行为均有回归覆盖。
- 默认运行路径保持 YunXi-owned，并继续独立于上游 Codex runtime。
- 所有统一验证通过，安装产物更新，临时产物清理。
- 本地 commit、`v1.2.0` tag、GitHub API 远端 refs 与树内容一致。
- 开发日志完整记录变更文件、验证结果、commit/tree/tag 哈希，但不记录任何密钥值。
