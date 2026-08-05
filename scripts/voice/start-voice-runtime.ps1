[CmdletBinding()]
param(
    [string]$Python,
    [string]$RuntimeRoot = $env:YUNXI_VOICE_RUNTIME_ROOT,
    [ValidateRange(1, 65535)]
    [int]$Port = 17862,
    [switch]$Mock
)

$ErrorActionPreference = "Stop"
$ScriptRoot = Split-Path -Parent $MyInvocation.MyCommand.Path
$ServerScript = Join-Path $ScriptRoot "runtime_server.py"

if ([string]::IsNullOrWhiteSpace($RuntimeRoot)) {
    $RuntimeRoot = "D:\YunXi Voice Runtime"
}

if ([string]::IsNullOrWhiteSpace($Python)) {
    $Python = Join-Path $RuntimeRoot "venv\Scripts\python.exe"
}

if (-not (Test-Path -LiteralPath $Python -PathType Leaf)) {
    throw "voice Python executable does not exist"
}
if (-not (Test-Path -LiteralPath $ServerScript -PathType Leaf)) {
    throw "voice runtime server script does not exist"
}

if ([string]::IsNullOrWhiteSpace($env:YUNXI_VOICE_DEVICE)) {
    $env:YUNXI_VOICE_DEVICE = "cuda:0"
}
if ([string]::IsNullOrWhiteSpace($env:YUNXI_VOICE_STT_MODEL_DIR)) {
    $env:YUNXI_VOICE_STT_MODEL_DIR = Join-Path $RuntimeRoot "models\SenseVoiceSmall"
}
if ([string]::IsNullOrWhiteSpace($env:YUNXI_VOICE_TTS_MODEL_DIR)) {
    $env:YUNXI_VOICE_TTS_MODEL_DIR = Join-Path $RuntimeRoot "models\CosyVoice-300M-SFT"
}
if ([string]::IsNullOrWhiteSpace($env:YUNXI_COSYVOICE_REPO)) {
    $env:YUNXI_COSYVOICE_REPO = Join-Path $RuntimeRoot "sources\CosyVoice"
}
if ([string]::IsNullOrWhiteSpace($env:MODELSCOPE_CACHE)) {
    $env:MODELSCOPE_CACHE = Join-Path $RuntimeRoot "cache\modelscope"
}
if ([string]::IsNullOrWhiteSpace($env:HF_HOME)) {
    $env:HF_HOME = Join-Path $RuntimeRoot "cache\huggingface"
}

$Arguments = @($ServerScript, "--bind", "127.0.0.1", "--port", $Port)
if ($Mock) {
    $Arguments += "--mock"
}

& $Python @Arguments
exit $LASTEXITCODE
