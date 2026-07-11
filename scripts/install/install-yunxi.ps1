param(
    [string]$InstallDir = (Join-Path $env:LOCALAPPDATA "YunXi Agent\bin"),
    [ValidateSet("release", "debug")]
    [string]$Configuration = "release",
    [switch]$AddToPath,
    [switch]$SkipBuild
)

$ErrorActionPreference = "Stop"

$scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$repoRoot = Resolve-Path (Join-Path $scriptDir "..\..")
$targetDir = Join-Path $repoRoot "target"
$profileDir = if ($Configuration -eq "release") { "release" } else { "debug" }
$cargoArgs = @("build", "-p", "yunxi-agent-cli", "--bins")
if ($Configuration -eq "release") {
    $cargoArgs += "--release"
}

if (-not $SkipBuild) {
    Push-Location $repoRoot
    try {
        & cargo @cargoArgs
        if ($LASTEXITCODE -ne 0) {
            throw "cargo build failed with exit code $LASTEXITCODE"
        }
    } finally {
        Pop-Location
    }
}

$resolvedInstallDir = [System.IO.Path]::GetFullPath($InstallDir)
New-Item -ItemType Directory -Force -Path $resolvedInstallDir | Out-Null

$sourceDir = Join-Path $targetDir $profileDir
$binaries = @("yunxi.exe", "yunxi-agent-cli.exe")
foreach ($binary in $binaries) {
    $source = Join-Path $sourceDir $binary
    if (-not (Test-Path -LiteralPath $source)) {
        throw "Expected binary not found: $source"
    }
    Copy-Item -LiteralPath $source -Destination (Join-Path $resolvedInstallDir $binary) -Force
}

$pathUpdated = $false
if ($AddToPath) {
    $currentPath = [Environment]::GetEnvironmentVariable("Path", "User")
    $parts = @()
    if (-not [string]::IsNullOrWhiteSpace($currentPath)) {
        $parts = $currentPath -split ';' | Where-Object { -not [string]::IsNullOrWhiteSpace($_) }
    }
    $alreadyPresent = $parts | Where-Object {
        [string]::Equals(
            [System.IO.Path]::GetFullPath($_).TrimEnd('\'),
            $resolvedInstallDir.TrimEnd('\'),
            [System.StringComparison]::OrdinalIgnoreCase
        )
    }
    if (-not $alreadyPresent) {
        $nextPath = (($parts + $resolvedInstallDir) -join ';')
        [Environment]::SetEnvironmentVariable("Path", $nextPath, "User")
        $env:Path = (($env:Path -split ';') + $resolvedInstallDir | Select-Object -Unique) -join ';'
        $pathUpdated = $true
    }
}

Write-Output "yunxi_install_status=installed"
Write-Output "install_dir=$resolvedInstallDir"
Write-Output "configuration=$Configuration"
Write-Output "yunxi_path=$(Join-Path $resolvedInstallDir 'yunxi.exe')"
Write-Output "compat_path=$(Join-Path $resolvedInstallDir 'yunxi-agent-cli.exe')"
Write-Output "path_updated=$pathUpdated"
