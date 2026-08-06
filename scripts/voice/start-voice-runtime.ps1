[CmdletBinding()]
param(
    [string]$Python,
    [string]$RuntimeRoot = $env:YUNXI_VOICE_RUNTIME_ROOT,
    [ValidateRange(1, 65535)]
    [int]$Port = 17862,
    [ValidateSet("single", "stable", "quality", "auto")]
    [string]$Mode = $(if ($env:YUNXI_VOICE_MODE) { $env:YUNXI_VOICE_MODE } else { "single" }),
    [ValidateRange(1, 65535)]
    [int]$QualityPort = 17864,
    [string]$VoiceProfile = $env:YUNXI_VOICE_PROFILE,
    [switch]$Mock
)

$ErrorActionPreference = "Stop"
$ScriptRoot = Split-Path -Parent $MyInvocation.MyCommand.Path
$ServerScript = Join-Path $ScriptRoot "runtime_server.py"

if ([string]::IsNullOrWhiteSpace($RuntimeRoot)) {
    $RuntimeRoot = "D:\YunXi Voice Runtime"
}
$RuntimeRoot = [System.IO.Path]::GetFullPath($RuntimeRoot)

if ([string]::IsNullOrWhiteSpace($Python)) {
    $Python = Join-Path $RuntimeRoot "venv\Scripts\python.exe"
}
if (-not (Test-Path -LiteralPath $Python -PathType Leaf)) {
    throw "voice Python executable does not exist"
}
if (-not (Test-Path -LiteralPath $ServerScript -PathType Leaf)) {
    throw "voice runtime server script does not exist"
}

if ($Mode -ne "single") {
    Write-Warning "Mode '$Mode' is deprecated; YunXi now loads one SenseVoiceSmall + CosyVoice3 chain."
}
$null = $QualityPort
$env:YUNXI_VOICE_MODE = "single"

if ([string]::IsNullOrWhiteSpace($env:YUNXI_VOICE_STT_DEVICE)) {
    $env:YUNXI_VOICE_STT_DEVICE = "cpu"
}
if ([string]::IsNullOrWhiteSpace($env:YUNXI_VOICE_TTS_DEVICE)) {
    $env:YUNXI_VOICE_TTS_DEVICE = "cuda:0"
}
if ([string]::IsNullOrWhiteSpace($env:YUNXI_VOICE_DEFAULT_LANGUAGE)) {
    $env:YUNXI_VOICE_DEFAULT_LANGUAGE = "zh"
}
if ([string]::IsNullOrWhiteSpace($env:YUNXI_VOICE_STT_MODEL_DIR)) {
    $env:YUNXI_VOICE_STT_MODEL_DIR = Join-Path $RuntimeRoot "models\SenseVoiceSmall"
}
if ([string]::IsNullOrWhiteSpace($env:YUNXI_VOICE_TTS_MODEL_DIR)) {
    $env:YUNXI_VOICE_TTS_MODEL_DIR = Join-Path $RuntimeRoot "models\Fun-CosyVoice3-0.5B-2512"
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

if (-not $Mock) {
    $cosyVoiceConfig = Join-Path $env:YUNXI_VOICE_TTS_MODEL_DIR "cosyvoice3.yaml"
    if (-not (Test-Path -LiteralPath $cosyVoiceConfig -PathType Leaf)) {
        throw "YUNXI_VOICE_TTS_MODEL_DIR must point to a Fun-CosyVoice3 model"
    }
    if ([string]::IsNullOrWhiteSpace($VoiceProfile)) {
        $defaultProfile = Join-Path $RuntimeRoot "profiles\yunxi-primary\profile.json"
        if (Test-Path -LiteralPath $defaultProfile -PathType Leaf) {
            $VoiceProfile = $defaultProfile
        }
    }
    if ([string]::IsNullOrWhiteSpace($VoiceProfile) -or -not (Test-Path -LiteralPath $VoiceProfile -PathType Leaf)) {
        throw "CosyVoice3 requires a local VoiceProfile with a reference audio file"
    }
    $env:YUNXI_VOICE_PROFILE = [System.IO.Path]::GetFullPath($VoiceProfile)
}

$Arguments = @($ServerScript, "--bind", "127.0.0.1", "--port", $Port)
if ($Mock) {
    $Arguments += "--mock"
}
& $Python @Arguments
exit $LASTEXITCODE
