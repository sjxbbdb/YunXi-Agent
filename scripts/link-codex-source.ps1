param(
    [Parameter(Mandatory = $true)]
    [string] $CodexCheckoutRoot
)

$ErrorActionPreference = 'Stop'

$repoRoot = Split-Path -Parent $PSScriptRoot
$sourceRoot = Resolve-Path -LiteralPath $CodexCheckoutRoot
$codexRs = Join-Path $sourceRoot 'codex-rs'

if (-not (Test-Path -LiteralPath $codexRs -PathType Container)) {
    throw "Codex checkout does not contain codex-rs: $codexRs"
}

foreach ($required in @(
    'Cargo.toml',
    'exec/src/lib.rs',
    'app-server-client/Cargo.toml',
    'app-server-protocol/Cargo.toml',
    'core/Cargo.toml',
    'protocol/Cargo.toml'
)) {
    $path = Join-Path $codexRs $required
    if (-not (Test-Path -LiteralPath $path)) {
        throw "Codex source is missing required path: $path"
    }
}

$externalDir = Join-Path $repoRoot 'external'
$target = Join-Path $externalDir 'codex-rs'

New-Item -ItemType Directory -Force -Path $externalDir | Out-Null

if (Test-Path -LiteralPath $target) {
    $item = Get-Item -LiteralPath $target -Force
    if ($item.LinkType -eq 'Junction' -or $item.LinkType -eq 'SymbolicLink') {
        Remove-Item -LiteralPath $target -Force
    } else {
        throw "Refusing to replace non-link path: $target"
    }
}

New-Item -ItemType Junction -Path $target -Target $codexRs | Out-Null
Write-Output "Linked $target -> $codexRs"
