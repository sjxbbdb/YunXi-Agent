[CmdletBinding()]
param(
    [string]$Python,
    [string]$RuntimeRoot = $env:YUNXI_VOICE_RUNTIME_ROOT,
    [ValidateRange(1, 65535)]
    [int]$Port = 17862,
    [ValidateSet("stable", "quality", "auto")]
    [string]$Mode = $(if ($env:YUNXI_VOICE_MODE) { $env:YUNXI_VOICE_MODE } else { "stable" }),
    [ValidateRange(1, 65535)]
    [int]$QualityPort = 17864,
    [string]$VoiceProfile = $env:YUNXI_VOICE_PROFILE,
    [switch]$Mock
)

$ErrorActionPreference = "Stop"
$ScriptRoot = Split-Path -Parent $MyInvocation.MyCommand.Path
$ServerScript = Join-Path $ScriptRoot "runtime_server.py"
$QualityServerScript = Join-Path $ScriptRoot "quality_runtime_server.py"

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
if ([string]::IsNullOrWhiteSpace($env:YUNXI_VOICE_DEFAULT_LANGUAGE)) {
    $env:YUNXI_VOICE_DEFAULT_LANGUAGE = "zh"
}
if ([string]::IsNullOrWhiteSpace($env:YUNXI_VOICE_QUALITY_WARMUP)) {
    $env:YUNXI_VOICE_QUALITY_WARMUP = "1"
}
$qualityWarmupEnabled = $env:YUNXI_VOICE_QUALITY_WARMUP -in @("1", "true", "yes", "on")
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
$env:YUNXI_VOICE_MODE = $Mode
$env:YUNXI_VOICE_QUALITY_URL = "http://127.0.0.1:$QualityPort"
if (-not [string]::IsNullOrWhiteSpace($VoiceProfile)) {
    $env:YUNXI_VOICE_PROFILE = [System.IO.Path]::GetFullPath($VoiceProfile)
}
$env:YUNXI_VOICE_QUALITY_STT_MODEL_DIR = Join-Path $RuntimeRoot "models\faster-whisper-large-v3"
$env:YUNXI_INDEXTTS_SOURCE = Join-Path $RuntimeRoot "sources\index-tts"
$env:YUNXI_INDEXTTS_MODEL_DIR = Join-Path $RuntimeRoot "models\IndexTTS-2"
$env:YUNXI_INDEXTTS_CONFIG = Join-Path $env:YUNXI_INDEXTTS_MODEL_DIR "config.yaml"

$qualityProcess = $null
$qualityWasStarted = $false
if (-not $Mock -and $Mode -in @("quality", "auto")) {
    $qualityPython = Join-Path $RuntimeRoot "quality-venv\Scripts\python.exe"
    if ((Test-Path -LiteralPath $qualityPython -PathType Leaf) -and (Test-Path -LiteralPath $QualityServerScript -PathType Leaf)) {
        $qualityReady = $false
        try {
            $qualityProbe = Invoke-RestMethod -Uri "$env:YUNXI_VOICE_QUALITY_URL/health" -TimeoutSec 2
            $qualityReady = -not $qualityWarmupEnabled -or $null -eq $qualityProbe.warmup -or $qualityProbe.warmup.complete
        } catch {
            $qualityReady = $false
        }
        if (-not $qualityReady) {
            $logRoot = Join-Path $RuntimeRoot "logs"
            New-Item -ItemType Directory -Path $logRoot -Force | Out-Null
            $qualityArguments = @("`"$QualityServerScript`"", "--bind", "127.0.0.1", "--port", $QualityPort)
            $qualityProcess = Start-Process -FilePath $qualityPython -ArgumentList $qualityArguments -PassThru -WindowStyle Hidden `
                -RedirectStandardOutput (Join-Path $logRoot "quality-runtime.stdout.log") `
                -RedirectStandardError (Join-Path $logRoot "quality-runtime.stderr.log")
            $qualityWasStarted = $true
            for ($attempt = 0; $attempt -lt 600; $attempt++) {
                try {
                    $qualityHealth = Invoke-RestMethod -Uri "$env:YUNXI_VOICE_QUALITY_URL/health" -TimeoutSec 2
                    $qualityReady = $true
                    if (-not $qualityWarmupEnabled -or $null -eq $qualityHealth.warmup -or $qualityHealth.warmup.complete) {
                        break
                    }
                } catch {
                    $qualityReady = $false
                }
                Start-Sleep -Milliseconds 200
            }
            if (-not $qualityReady) {
                Write-Warning "quality voice worker did not become ready; stable fallback remains available"
            }
        }
    } else {
        Write-Warning "quality runtime is not installed; stable fallback remains available"
    }
}

$Arguments = @($ServerScript, "--bind", "127.0.0.1", "--port", $Port)
if ($Mock) {
    $Arguments += "--mock"
}

try {
    & $Python @Arguments
    $serverExitCode = $LASTEXITCODE
} finally {
    if ($qualityWasStarted -and $null -ne $qualityProcess -and -not $qualityProcess.HasExited) {
        Stop-Process -Id $qualityProcess.Id -Force -ErrorAction SilentlyContinue
    }
}
exit $serverExitCode
