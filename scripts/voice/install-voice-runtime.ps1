[CmdletBinding()]
param(
    [string]$RuntimeRoot = $env:YUNXI_VOICE_RUNTIME_ROOT,
    [string]$Uv,
    [switch]$SkipSource,
    [switch]$SkipModels,
    [switch]$IncludeQuality,
    [switch]$SkipQualityModels
)

$ErrorActionPreference = "Stop"

if ([string]::IsNullOrWhiteSpace($RuntimeRoot)) {
    $RuntimeRoot = "D:\YunXi Voice Runtime"
}
$RuntimeRoot = [System.IO.Path]::GetFullPath($RuntimeRoot)

function Resolve-UvExecutable {
    if (-not [string]::IsNullOrWhiteSpace($script:Uv)) {
        return $script:Uv
    }
    $command = Get-Command uv -ErrorAction SilentlyContinue
    if ($null -ne $command) {
        return $command.Source
    }
    $wingetUv = Get-ChildItem -Path "$env:LOCALAPPDATA\Microsoft\WinGet\Packages" -Filter "uv.exe" -Recurse -ErrorAction SilentlyContinue | Select-Object -First 1
    if ($null -ne $wingetUv) {
        return $wingetUv.FullName
    }
    throw "uv is required. Install it with: winget install -e --id astral-sh.uv"
}

function Invoke-UvPip {
    param([Parameter(ValueFromRemainingArguments = $true)][string[]]$Arguments)
    & $script:Uv pip install --python $script:Python @Arguments
    if ($LASTEXITCODE -ne 0) {
        throw "uv pip install failed"
    }
}

function Invoke-QualityUvPip {
    param([Parameter(ValueFromRemainingArguments = $true)][string[]]$Arguments)
    & $script:Uv pip install --python $script:QualityPython @Arguments
    if ($LASTEXITCODE -ne 0) {
        throw "quality uv pip install failed"
    }
}

function Expand-GitHubArchive {
    param(
        [string]$Url,
        [string]$Archive,
        [string]$ExpandedRoot,
        [string]$Destination,
        [string]$RequiredFile
    )
    $requiredPath = Join-Path $Destination $RequiredFile
    if (Test-Path -LiteralPath $requiredPath -PathType Leaf) {
        return
    }
    for ($attempt = 1; $attempt -le 5; $attempt++) {
        try {
            Invoke-WebRequest -UseBasicParsing -Uri $Url -OutFile $Archive
            break
        } catch {
            if ($attempt -eq 5) { throw }
            Start-Sleep -Seconds 2
        }
    }
    $temporary = "$Archive.expanded"
    if (Test-Path -LiteralPath $temporary) {
        Remove-Item -LiteralPath $temporary -Recurse -Force
    }
    Expand-Archive -LiteralPath $Archive -DestinationPath $temporary -Force
    $source = Join-Path $temporary $ExpandedRoot
    if (-not (Test-Path -LiteralPath $source -PathType Container)) {
        throw "downloaded source archive has an unexpected layout"
    }
    $parent = Split-Path -Parent $Destination
    New-Item -ItemType Directory -Path $parent -Force | Out-Null
    if (Test-Path -LiteralPath $Destination) {
        $destinationPath = [System.IO.Path]::GetFullPath($Destination)
        $runtimePrefix = $script:RuntimeRoot.TrimEnd('\') + '\'
        if (-not $destinationPath.StartsWith($runtimePrefix, [StringComparison]::OrdinalIgnoreCase)) {
            throw "refusing to replace an incomplete source directory outside the voice runtime root"
        }
        Remove-Item -LiteralPath $Destination -Recurse -Force
    }
    Move-Item -LiteralPath $source -Destination $Destination
    Remove-Item -LiteralPath $temporary -Recurse -Force
}

$Uv = Resolve-UvExecutable
$env:UV_CACHE_DIR = Join-Path $RuntimeRoot "cache\uv"
$env:UV_PYTHON_INSTALL_DIR = Join-Path $RuntimeRoot "python"
$env:UV_HTTP_TIMEOUT = "600"
$env:MODELSCOPE_CACHE = Join-Path $RuntimeRoot "cache\modelscope"
$env:HF_HOME = Join-Path $RuntimeRoot "cache\huggingface"

@("cache", "logs", "models", "python", "sources") | ForEach-Object {
    New-Item -ItemType Directory -Path (Join-Path $RuntimeRoot $_) -Force | Out-Null
}

$Python = Join-Path $RuntimeRoot "venv\Scripts\python.exe"
if (-not (Test-Path -LiteralPath $Python -PathType Leaf)) {
    & $Uv python install 3.10
    if ($LASTEXITCODE -ne 0) { throw "failed to install Python 3.10 with uv" }
    & $Uv venv --python 3.10 (Join-Path $RuntimeRoot "venv")
    if ($LASTEXITCODE -ne 0) { throw "failed to create the voice virtual environment" }
}

Invoke-UvPip "setuptools==69.5.1" "wheel==0.47.0" "filelock" "typing-extensions" "sympy" "networkx" "jinja2" "fsspec"
Invoke-UvPip "--index-url" "https://download.pytorch.org/whl/cu129" "--no-deps" "torch==2.8.0+cu129" "torchaudio==2.8.0+cu129"
Invoke-UvPip "--no-build-isolation" "openai-whisper==20231117"
$RuntimePackages = @(
    "numpy==1.26.4"
    "funasr==1.4.1"
    "conformer==0.3.2"
    "diffusers==0.29.0"
    "gdown==5.1.0"
    "hydra-core==1.3.2"
    "HyperPyYAML==1.2.3"
    "inflect==7.3.1"
    "librosa==0.10.2"
    "lightning==2.2.4"
    "matplotlib==3.7.5"
    "modelscope==1.20.0"
    "omegaconf==2.3.0"
    "onnx==1.16.0"
    "onnxruntime==1.18.0"
    "protobuf==4.25.8"
    "pyarrow==18.1.0"
    "pydantic==2.7.0"
    "pyworld==0.3.4"
    "rich==13.7.1"
    "soundfile==0.12.1"
    "transformers==4.51.3"
    "x-transformers==2.11.24"
    "wget==3.2"
    "tqdm==4.66.5"
)
Invoke-UvPip @RuntimePackages

if (-not $SkipSource) {
    $cosyVoiceRoot = Join-Path $RuntimeRoot "sources\CosyVoice"
    $CosyVoiceArchive = @{
        Url = "https://codeload.github.com/FunAudioLLM/CosyVoice/zip/refs/heads/main"
        Archive = Join-Path $RuntimeRoot "cache\CosyVoice-main.zip"
        ExpandedRoot = "CosyVoice-main"
        Destination = $cosyVoiceRoot
        RequiredFile = "cosyvoice\cli\cosyvoice.py"
    }
    Expand-GitHubArchive @CosyVoiceArchive
    $MatchaArchive = @{
        Url = "https://codeload.github.com/shivammehta25/Matcha-TTS/zip/refs/heads/main"
        Archive = Join-Path $RuntimeRoot "cache\Matcha-TTS-main.zip"
        ExpandedRoot = "Matcha-TTS-main"
        Destination = Join-Path $cosyVoiceRoot "third_party\Matcha-TTS"
        RequiredFile = "matcha\models\components\flow_matching.py"
    }
    Expand-GitHubArchive @MatchaArchive
}

if (-not $SkipModels) {
    $env:YUNXI_INSTALL_STT_DIR = Join-Path $RuntimeRoot "models\SenseVoiceSmall"
    $env:YUNXI_INSTALL_TTS_DIR = Join-Path $RuntimeRoot "models\CosyVoice-300M-SFT"
    & $Python -c "import os; from modelscope import snapshot_download; snapshot_download('iic/SenseVoiceSmall', local_dir=os.environ['YUNXI_INSTALL_STT_DIR']); snapshot_download('iic/CosyVoice-300M-SFT', local_dir=os.environ['YUNXI_INSTALL_TTS_DIR'])"
    if ($LASTEXITCODE -ne 0) { throw "voice model download failed" }
}

if ($IncludeQuality) {
    $IndexTtsCommit = "90ca4d608209584bad3a5bd5becc0b80c146e60f"
    $indexTtsRoot = Join-Path $RuntimeRoot "sources\index-tts"
    if (-not $SkipSource) {
        $IndexTtsArchive = @{
            Url = "https://codeload.github.com/index-tts/index-tts/zip/$IndexTtsCommit"
            Archive = Join-Path $RuntimeRoot "cache\index-tts-$IndexTtsCommit.zip"
            ExpandedRoot = "index-tts-$IndexTtsCommit"
            Destination = $indexTtsRoot
            RequiredFile = "indextts\infer_v2.py"
        }
        Expand-GitHubArchive @IndexTtsArchive
    }
    if (-not (Test-Path -LiteralPath (Join-Path $indexTtsRoot "pyproject.toml") -PathType Leaf)) {
        throw "pinned IndexTTS2 source is required for the quality runtime"
    }

    $qualityVenv = Join-Path $RuntimeRoot "quality-venv"
    $QualityPython = Join-Path $qualityVenv "Scripts\python.exe"
    $previousProjectEnvironment = $env:UV_PROJECT_ENVIRONMENT
    try {
        $env:UV_PROJECT_ENVIRONMENT = $qualityVenv
        & $Uv sync --project $indexTtsRoot --no-dev
        if ($LASTEXITCODE -ne 0) { throw "failed to install the pinned IndexTTS2 environment" }
    } finally {
        $env:UV_PROJECT_ENVIRONMENT = $previousProjectEnvironment
    }
    Invoke-QualityUvPip "faster-whisper==1.2.1" "huggingface-hub[hf-xet]"

    if (-not $SkipModels -and -not $SkipQualityModels) {
        $env:YUNXI_INSTALL_WHISPER_DIR = Join-Path $RuntimeRoot "models\faster-whisper-large-v3"
        $env:YUNXI_INSTALL_INDEXTTS_DIR = Join-Path $RuntimeRoot "models\IndexTTS-2"
        & $QualityPython -c "import os; from huggingface_hub import snapshot_download; snapshot_download('Systran/faster-whisper-large-v3', local_dir=os.environ['YUNXI_INSTALL_WHISPER_DIR']); snapshot_download('IndexTeam/IndexTTS-2', revision='740dcaff396282ffb241903d150ac011cd4b1ede', local_dir=os.environ['YUNXI_INSTALL_INDEXTTS_DIR'])"
        if ($LASTEXITCODE -ne 0) { throw "quality voice model download failed" }
    }

    & $QualityPython -c "import torch, torchaudio, faster_whisper; assert torch.cuda.is_available(); print(torch.__version__, torchaudio.__version__, torch.cuda.get_device_name(0), faster_whisper.__version__)"
    if ($LASTEXITCODE -ne 0) { throw "quality voice runtime verification failed" }
}

$env:YUNXI_COSYVOICE_REPO = Join-Path $RuntimeRoot "sources\CosyVoice"
$env:PYTHONPATH = "$env:YUNXI_COSYVOICE_REPO;$env:YUNXI_COSYVOICE_REPO\third_party\Matcha-TTS"
& $Python -c "import torch, torchaudio; from funasr import AutoModel; from cosyvoice.cli.cosyvoice import CosyVoice; assert torch.cuda.is_available(); value=(torch.tensor([2.0], device='cuda')*3).item(); assert value == 6.0; print(torch.__version__, torchaudio.__version__, torch.cuda.get_device_name(0))"
if ($LASTEXITCODE -ne 0) { throw "voice runtime verification failed" }

Write-Output "Voice runtime installed at $RuntimeRoot"
Write-Output "Start it with: .\start-voice-runtime.ps1 -RuntimeRoot `"$RuntimeRoot`""
if ($IncludeQuality) {
    Write-Output "Quality runtime installed. Keep YUNXI_VOICE_MODE=stable until a local VoiceProfile is configured."
}
