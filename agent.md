# YunXi Agent 自动安装与配置协议

本文件面向能够读取本地文件、操作 Windows PowerShell 并执行 Git/Cargo 命令的智能体。它定义 YunXi Agent 的安装、配置、验证和交付流程，不是运行时人格文件，也不能替代仓库开发约束。

协议版本：`1.0`

目标版本：`v2.3.3-hotfix.21`

目标平台：`x86_64-pc-windows-msvc`

## 1. 文档优先级

执行前必须按顺序阅读：

1. 当前目录的 [README.md](./README.md)。
2. 本文件 `agent.md`。
3. 如果当前目录是源码仓库，继续阅读根目录 `AGENTS.md`，并向下检查目标路径是否存在更具体的 `AGENTS.md`。

发生冲突时，系统/用户指令高于仓库指令，仓库 `AGENTS.md` 高于本安装协议，当前用户的明确选择高于本文推荐值。不得用本文件扩大用户授权范围。

## 2. 任务目标

在不破坏用户既有环境与私人状态的前提下，完成以下工作：

1. 判断当前输入是 Release 分发包还是源码仓库。
2. 执行系统、工具链和完整性预检。
3. 按用户选择完成便携运行、当前用户安装或源码构建。
4. 建立安全、明确的工作区。
5. 确认 Provider 配置，但不接触秘密值。
6. 配置人格、记忆与陪伴开关。
7. 完成离线、在线、Web 与可选微信验收。
8. 交付可复核的安装报告和未完成事项。

## 3. 强制安全边界

智能体必须遵守：

- 不读取、显示、记录、总结或转述 API Key、Token、微信凭证、二维码 payload、信箱密钥和数据加密密钥。
- 不要求用户把 API Key 粘贴到聊天。需要密钥时暂停，让用户在自己的终端中输入。
- 不自动扫描 Windows Credential Manager，不读取凭证明文。
- 不删除、覆盖或迁移用户既有 `.yunxi`、人格、灵魂、记忆、信箱、会话、日志和微信状态。
- 不使用 `git reset --hard`、`git clean`、强制 checkout、force push 或递归清理仓库/用户目录。
- 不覆盖源码仓库中未提交的用户修改；发现 dirty worktree 时必须保留并绕开。
- 不把磁盘根目录、整个用户主目录或含大量隐私文件的目录作为工作区。
- 不默认使用 `danger-full-access`、`approval=never` 或公网绑定。
- 不通过模糊进程名批量停止进程。只能处理已确认路径属于本次 YunXi 安装的精确 PID。
- 不在未经用户允许时修改系统级执行策略、系统 PATH、防火墙或开机启动项。
- 不从第三方镜像、聊天附件或无法验证的 URL 下载 YunXi 可执行文件。
- 任一步失败时停止扩大操作范围，保留已有备份，报告失败阶段、退出码和脱敏错误摘要。

## 4. 识别当前输入

把当前目录解析为绝对路径并记作 `Root`。不要硬编码分发者机器上的路径。

### Release 分发包

同时存在以下文件时，选择 `release-package` 路线：

```text
README.md
agent.md
install.ps1
SHA256SUMS.txt
manifest.json
bin\yunxi.exe
bin\yunxi-agent-cli.exe
```

### 源码仓库

同时存在以下文件时，选择 `source-checkout` 路线：

```text
README.md
agent.md
AGENTS.md
Cargo.toml
Cargo.lock
crates\yunxi-agent-cli\Cargo.toml
```

### 无法识别

若两种结构都不满足，停止并报告缺失路径。不要猜测安装脚本、二进制或仓库位置。

若两种结构同时出现，以用户明确选择为准；未指定时优先使用已校验的 Release 包完成普通安装，源码仓库仅用于开发或构建。

## 5. 只读预检

### 5.1 系统

在 PowerShell 中检查：

```powershell
[Environment]::Is64BitOperatingSystem
$PSVersionTable.PSVersion
[Environment]::OSVersion.VersionString
```

通过条件：

- 64 位 Windows 10/11。
- PowerShell 5.1 或更高版本。
- 当前目录可读。
- 目标安装目录和工作区可写。

### 5.2 端口

```powershell
Get-NetTCPConnection -LocalPort 17861 -State Listen -ErrorAction SilentlyContinue
```

端口被占用不等于安装失败。先识别监听 PID 与可执行文件路径：

- 已是用户需要保留的 YunXi Web：复用或由用户决定是否重启。
- 是其他程序：使用备用端口。
- 路径不明确：不得停止该进程。

### 5.3 现有安装

只读检查命令解析结果：

```powershell
Get-Command yunxi -ErrorAction SilentlyContinue | Select-Object Name, Source, Version
```

如果存在旧版本，记录精确路径。升级必须走 Release 安装器或经用户确认的 Cargo 安装；不要直接覆盖正在运行的 exe。

## 6. 路线 A：Release 分发包

### 6.1 来源检查

可信来源仅限：

```text
https://github.com/sjxbbdb/YunXi-Agent
https://github.com/sjxbbdb/YunXi-Agent/releases
```

若当前尚未下载包：

1. 读取 GitHub Release 元数据。
2. 选择与目标 tag 和 `windows-x64` 匹配的资产。
3. 同时下载外层 `.sha256`（若发布提供）。
4. 若目标 tag 没有预编译资产，不得拼接猜测 URL；报告情况并征得用户同意后转入源码路线。

### 6.2 外层 ZIP 校验

如果存在同名 `.zip.sha256`：

```powershell
$ZipCandidates = @(Get-ChildItem -LiteralPath . -File -Filter "YunXi-Agent-*-windows-x64.zip")
if ($ZipCandidates.Count -ne 1) {
    throw "expected exactly one YunXi Windows x64 ZIP, found $($ZipCandidates.Count)"
}
$ZipPath = $ZipCandidates[0].FullName
$ChecksumPath = "$ZipPath.sha256"
$ActualZipHash = (Get-FileHash -Algorithm SHA256 -LiteralPath $ZipPath).Hash
$ExpectedZipHash = ((Get-Content -LiteralPath $ChecksumPath -Raw) -split '\s+')[0]
[string]::Equals($ActualZipHash, $ExpectedZipHash, [StringComparison]::OrdinalIgnoreCase)
```

结果必须为 `True`。不一致时停止，不解压、不运行。

### 6.3 包内校验

解压到用户选择的普通可写目录。不得先递归删除同名目录；若目标已存在，使用新目录或让用户决定。

至少检查：

```powershell
Get-FileHash -Algorithm SHA256 .\bin\yunxi.exe
Get-FileHash -Algorithm SHA256 .\bin\yunxi-agent-cli.exe
```

必须与 `SHA256SUMS.txt` 对应条目一致。随后解析 `manifest.json`，确认：

- `containsPrivateState` 为 `false`。
- `version` 与用户选择的 tag 一致。
- 两个二进制、README、agent 手册和许可文件存在。
- 所有 manifest 文件大小和 SHA-256 一致。

任何不一致都必须停止。

### 6.4 选择运行模式

用户未指定时只询问一次：

```text
你希望使用便携模式，还是安装到当前用户并加入 PATH？
```

#### 便携模式

不运行安装器。使用包内状态目录：

```powershell
$PackageRoot = (Get-Location).Path
$PortableHome = Join-Path $PackageRoot "data\home"
$PortableWorkspace = Join-Path $PackageRoot "workspace"
New-Item -ItemType Directory -Force -Path $PortableHome, $PortableWorkspace | Out-Null
$env:YUNXI_HOME = $PortableHome
$YunXiExe = Join-Path $PackageRoot "bin\yunxi.exe"
& $YunXiExe --version
```

不得把使用后的 `data` 与 `workspace` 重新压缩分发。

#### 当前用户安装

默认目录：

```powershell
$InstallDir = Join-Path $env:LOCALAPPDATA "YunXi Agent\bin"
```

仅在用户同意加入用户 PATH 后执行：

```powershell
Set-ExecutionPolicy -Scope Process Bypass -Force
& (Join-Path $PackageRoot "install.ps1") -InstallDir $InstallDir -AddToPath
```

不同意 PATH 时省略 `-AddToPath`。`Process` 作用域只影响当前 PowerShell，不得改为 `LocalMachine`。

安装器应完成源哈希校验、文件锁检查、旧版备份、临时文件校验和替换。安装后用绝对路径验证：

```powershell
$YunXiExe = Join-Path $InstallDir "yunxi.exe"
& $YunXiExe --version
```

## 7. 路线 B：源码仓库

### 7.1 仓库保护

首先执行只读检查：

```powershell
git status --short
git branch --show-current
git remote get-url origin
git describe --tags --always --dirty
```

要求：

- 远程仓库应指向 `sjxbbdb/YunXi-Agent`，或由用户明确认可的 fork。
- Dirty worktree 不是自动失败，但不得覆盖、还原或清理任何现有修改。
- 不因安装而切换已有工作区的 branch/tag。
- 若需要固定版本，应在新的独立 clone 中检出目标 tag。

全新 clone 示例：

```powershell
git clone --branch v2.3.3-hotfix.21 --depth 1 https://github.com/sjxbbdb/YunXi-Agent.git YunXi-Agent
Set-Location .\YunXi-Agent
```

### 7.2 工具链

```powershell
git --version
rustc --version
cargo --version
```

Rust 必须为 `1.85` 或更高版本。缺少 Rust 时，只能引导用户从 [rustup.rs](https://rustup.rs/) 安装官方 stable 工具链；不得从任意镜像下载未知安装器。

Windows 需要 MSVC C++ Build Tools 与 Windows SDK。缺少链接器时，报告缺失组件，不要通过降低目标或修改 workspace 来绕过。

### 7.3 依赖与构建

在仓库根目录运行：

```powershell
cargo fetch --locked
cargo build -p yunxi-agent-cli --release --bins --locked
```

产物：

```text
target\release\yunxi.exe
target\release\yunxi-agent-cli.exe
```

验证：

```powershell
$YunXiExe = (Resolve-Path .\target\release\yunxi.exe).Path
& $YunXiExe --version
& $YunXiExe --help
```

版本必须与 checkout 中 `Cargo.toml` 的 `workspace.package.version` 一致。

### 7.4 源码门禁

未修改源码时执行：

```powershell
cargo fmt --all -- --check
cargo check --workspace --locked
cargo test --workspace --locked
```

如果用户只要求普通安装，测试可以耗时，但不能用“构建通过”冒充“全量测试通过”。测试未运行或被环境阻塞时，最终报告必须明确标为未验证。

### 7.5 可选 Cargo 安装

`cargo install` 会写入 Cargo 用户目录，可能替换已有同名二进制。执行前确认用户同意：

```powershell
cargo install --path .\crates\yunxi-agent-cli --locked
```

若用户不同意，保留 `$YunXiExe` 指向 `target\release\yunxi.exe`，不修改 PATH。

## 8. 建立工作区

若用户没有指定，建议：

```powershell
$YunXiWorkspace = Join-Path ([Environment]::GetFolderPath("MyDocuments")) "YunXi Workspace"
New-Item -ItemType Directory -Force -Path $YunXiWorkspace | Out-Null
```

所有后续状态命令显式传入：

```powershell
--cwd $YunXiWorkspace
```

如果采用便携模式，则使用 `$PortableWorkspace`，并保持 `$env:YUNXI_HOME = $PortableHome`。

## 9. Provider 配置

### 9.1 只检查存在性

不得输出密钥值：

```powershell
$ProviderProfile = $env:YUNXI_PROVIDER_PROFILE
$ModelName = $env:YUNXI_AGENT_MODEL
$DeepSeekKeyPresent = -not [string]::IsNullOrWhiteSpace($env:DEEPSEEK_API_KEY)
[pscustomobject]@{
    ProviderProfile = $ProviderProfile
    Model = $ModelName
    DeepSeekKeyPresent = $DeepSeekKeyPresent
}
```

### 9.2 用户亲自输入

密钥缺失时暂停，让用户在将要启动 YunXi 的同一终端会话中输入：

```powershell
$env:YUNXI_PROVIDER_PROFILE = "deepseek"
$env:DEEPSEEK_API_KEY = "<由用户本人输入>"
$env:YUNXI_AGENT_MODEL = "deepseek-v4-flash"
```

不得运行会回显 `$env:DEEPSEEK_API_KEY` 的命令，不得把密钥写入脚本、配置模板、任务日志或最终报告。

用户暂不配置时继续离线验收，并把在线链路标记为 `not_verified`。

## 10. 基础功能配置

推荐长期使用：

```powershell
& $YunXiExe persona on --cwd $YunXiWorkspace
& $YunXiExe memory on --cwd $YunXiWorkspace
& $YunXiExe companion on --cwd $YunXiWorkspace
```

随后检查：

```powershell
& $YunXiExe persona status --cwd $YunXiWorkspace
& $YunXiExe memory status --cwd $YunXiWorkspace
& $YunXiExe companion status --cwd $YunXiWorkspace
```

限制：

- 不自动导入外部人格、灵魂或历史记忆。
- 不自动开启情书生成；它涉及私人关系和记忆内容。
- 用户明确要求情书功能时，只在用户认可的运行环境中设置 `YUNXI_LOVE_LETTERS_ENABLED=1`。
- 不为了“全功能”而降低审批模式或 Sandbox Policy。
- 状态输出必须来自当前工作区，不能用另一个目录的结果代替。

## 11. 验收流程

### 11.1 版本

```powershell
& $YunXiExe --version
```

### 11.2 离线链路

```powershell
& $YunXiExe --offline --no-tui --json --cwd $YunXiWorkspace "YunXi 安装自检"
```

通过条件：

- 退出码为 `0`。
- JSON 状态完成。
- 输出明确属于离线 Runtime。
- 不包含 ANSI/TUI 控制字符。

离线成功不代表在线 Provider 已验证。

### 11.3 在线链路

仅当用户已亲自配置密钥后：

```powershell
& $YunXiExe --live --no-tui --cwd $YunXiWorkspace "请只回复：在线链路正常"
```

只记录是否成功、Provider/模型名称和脱敏错误分类。不得记录请求头、密钥、完整 Provider 错误 body 或私人上下文。

### 11.4 Web

前台启动：

```powershell
& $YunXiExe web --cwd $YunXiWorkspace --bind 127.0.0.1 --port 17861
```

需要后台启动时：

1. 先确认端口可用。
2. 使用 `$YunXiExe` 的精确绝对路径。
3. 保留返回 PID。
4. 核对 PID 的可执行文件路径与端口监听者一致。
5. 请求 `http://127.0.0.1:17861/api/health`。
6. 确认响应版本与安装版本一致。
7. 向用户报告实际 URL 和 PID。

不得默认绑定 `0.0.0.0`。

### 11.5 微信（可选）

必须先确认用户需要微信接入：

```powershell
& $YunXiExe weixin login --cwd $YunXiWorkspace --account default
```

出现二维码后暂停，让用户本人扫码与确认。不得截图、OCR、复制或记录二维码内容。

登录后：

```powershell
& $YunXiExe weixin status --cwd $YunXiWorkspace --account default
& $YunXiExe weixin doctor --cwd $YunXiWorkspace --account default
```

只汇报脱敏状态。用户不需要自动拉起时，在交互式 YunXi 或 Web 启动参数中加入 `--no-weixin-autostart`。

### 11.6 本地语音（可选）

只有用户明确要求本地语音且接受额外磁盘、Python 与模型依赖时才能安装。不得把语音模型放进源码仓库或 Release 安装目录。稳定链至少确认 30 GB 可用空间；同时安装 faster-whisper large-v3 与 IndexTTS2 时至少确认 50 GB。再从私有仓库 `https://github.com/sjxbbdb/YunXi-Voice-Runtime` 建立独立源码 checkout：

```powershell
gh repo clone sjxbbdb/YunXi-Voice-Runtime "D:\YunXi Voice Runtime Source"
Set-Location "D:\YunXi Voice Runtime Source"
.\install-voice-runtime.ps1 -RuntimeRoot "D:\YunXi Voice Runtime"
.\start-voice-runtime.ps1 -RuntimeRoot "D:\YunXi Voice Runtime"
& $YunXiExe voice doctor --json
```

质量链必须作为稳定链的增量安装，禁止删除 SenseVoiceSmall 或 CosyVoice：

```powershell
.\install-voice-runtime.ps1 -RuntimeRoot "D:\YunXi Voice Runtime" -IncludeQuality
.\start-voice-runtime.ps1 -RuntimeRoot "D:\YunXi Voice Runtime" -Mode auto `
  -VoiceProfile "D:\YunXi Voice Runtime\profiles\yunxi-primary\profile.json"
```

VoiceProfile 必须从 `voice-profile.example.json` 建立在本机被忽略目录中，参考音频必须由用户拥有或授权。不得把 profile、参考音频、转写、embedding、生成 WAV 或其绝对路径写进 Git、普通日志或公开健康信息。IndexTTS2 源码和模型必须保持安装器固定的 commit/revision，不得静默跟随上游 `main`。

验收至少覆盖 `voice speak`、`voice transcribe`、一次 `voice chat`、`voice devices`，以及在真实交互终端内完成两轮 `voice talk`。质量链还必须分别实测 faster-whisper 与 IndexTTS2、热态延迟、有效 WAV、VoiceProfile 缺失、质量 worker 强制退出后的双端稳定降级、`voice doctor --json` 扩展字段，以及稳定/质量模型同时驻留时的显存峰值。任何质量失败都只能重试对应 STT/TTS，不得重跑 Agent turn。还必须在已经运行的 CLI/TUI 中验证 `/voice status`、`/voice devices` 和两轮 `/voice`：语音文本必须进入当前 `InteractiveSession`，与此前文字轮次共享会话，并继续显示原有流式事件与工具审批。第二轮必须能沿用第一轮会话上下文，默认输出设备必须能播放合成结果。实时模式还要验证 `/voice realtime on|off|status`、TUI `voice=live` 状态、VAD 自动收音、自然短句分段、播放时预合成下一段、键盘停止当前播放、语音口令“关闭实时语音”以及关闭后释放麦克风。不得用 mock health 代替真实模型验收；不得把语音内容视为自动批准工具的指令；测试输入不得污染用户长期记忆。当前实时模式是免按键半双工，只能描述为支持键盘停止播放，不得描述成全双工、服务端流式或支持语音插话打断；未来流式能力必须新增协议 v2，不能破坏 HTTP v1。

## 12. 失败处理与回滚

### 二进制被占用

仅定位安装目录中的进程：

```powershell
Get-Process | Where-Object {
    $_.Path -and $_.Path.StartsWith($InstallDir, [StringComparison]::OrdinalIgnoreCase)
} | Select-Object Id, ProcessName, Path
```

向用户说明后，只处理明确属于该目录的 YunXi PID。禁止 `Stop-Process -Name yunxi -Force` 这类模糊批量操作。

### Release 安装失败

1. 保留安装器错误输出。
2. 记录安装器给出的 `backup_dir`。
3. 确认目标 YunXi 进程已关闭。
4. 必要时从备份恢复两个 exe。
5. 不删除人格、记忆、信箱、会话和微信状态。

### 源码构建失败

1. 保留首个有意义的编译错误。
2. 核对 Rust 版本、MSVC Build Tools、Windows SDK 和磁盘空间。
3. 不修改业务代码来掩盖环境缺失。
4. 不清理用户 Cargo cache 或仓库 `target`，除非用户明确授权且已确认目标路径。

### Web 健康检查失败

检查端口监听者、精确进程路径、启动参数和脱敏 stderr。先定位端口冲突、进程退出或工作区配置问题，不要无条件反复重启。

## 13. 验收矩阵

| 项目 | 通过条件 |
| --- | --- |
| 输入识别 | 明确为 Release 包或源码仓库 |
| 来源与完整性 | 官方来源；Release 哈希通过或源码 tag/commit 明确 |
| 构建/安装 | 命令退出码为 0，未覆盖未授权数据 |
| 版本 | 与包体或 checkout 版本一致 |
| 人格 | 能读取活动 profile |
| 记忆 | 状态可读取且开关符合用户选择 |
| 陪伴 | 状态可读取且开关符合用户选择 |
| 离线 | 退出码 0，明确标识 offline |
| 在线 | 有凭证时真实 Provider 成功；无凭证时标记未验证 |
| Web | `/api/health` 返回成功和正确版本 |
| 微信 | 仅在用户选择后验证，结果必须脱敏 |
| 本地语音 | 仅在用户选择后安装；真实 STT、TTS 与 voice chat 均成功 |
| 私密数据 | 日志和最终报告中无密钥、二维码、私人记忆或微信标识 |

## 14. 最终报告格式

最终向用户报告：

```text
YunXi Agent 安装报告
- 输入类型：release-package / source-checkout
- 安装方式：便携 / 当前用户 / Cargo 构建目录
- 版本：
- 可执行文件路径：
- 工作区路径：
- PATH 是否修改：
- 源码门禁：fmt / check / test / build（分别报告）
- 人格状态：
- 记忆状态：
- 陪伴状态：
- 在线 Provider：verified / not_verified / failed
- Web：URL、健康状态、PID（若启动）
- 微信：not_requested / configured / failed（仅脱敏状态）
- 备份目录：若升级产生
- 尚需用户完成：
```

最终报告不得包含 API Key、Token、二维码内容、原始微信标识、私人记忆、信箱正文或完整秘密环境变量。
