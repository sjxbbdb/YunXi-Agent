param(
    [Parameter(Mandatory = $true)]
    [string]$ApiFile,
    [ValidateRange(0, 2147483647)]
    [int]$CredentialIndex = 0,
    [string]$Model = "deepseek-v4-flash",
    [switch]$NoStream
)

$ErrorActionPreference = "Stop"

$repoRoot = Resolve-Path (Join-Path $PSScriptRoot "..\..")
$streamEnabled = if ($NoStream) { "0" } else { "1" }
$prompt = if ($NoStream) {
    "Reply exactly: YUNXI_DEEPSEEK_OK"
} else {
    "Reply exactly: YUNXI_DEEPSEEK_STREAM_OK"
}
$tmp = Join-Path $env:TEMP ("yunxi-deepseek-smoke-{0}.jsonl" -f ([guid]::NewGuid().ToString("N")))

function Get-DeepSeekKey {
    param(
        [string]$Path,
        [int]$SelectedIndex
    )
    if (-not (Test-Path -LiteralPath $Path -PathType Leaf)) {
        throw "DeepSeek credential source file was not found"
    }
    $content = Get-Content -LiteralPath $Path -Raw
    $matches = [regex]::Matches(
        $content,
        "(?<![A-Za-z0-9_-])sk-[A-Za-z0-9_-]{20,}(?![A-Za-z0-9_-])"
    )
    $seen = [System.Collections.Generic.HashSet[string]]::new(
        [System.StringComparer]::Ordinal
    )
    $candidates = @(
        foreach ($match in $matches) {
            if ($seen.Add($match.Value)) {
                $match.Value
            }
        }
    )
    if ($candidates.Count -eq 0) {
        throw "No DeepSeek credential candidate was found"
    }
    if ($candidates.Count -eq 1) {
        return $candidates[0]
    }
    if ($SelectedIndex -eq 0) {
        throw "Multiple credential candidates were found; specify -CredentialIndex explicitly"
    }
    if ($SelectedIndex -gt $candidates.Count) {
        throw "CredentialIndex is outside the available candidate range"
    }
    return $candidates[$SelectedIndex - 1]
}

try {
    $deepseekKey = Get-DeepSeekKey -Path $ApiFile -SelectedIndex $CredentialIndex

    $env:YUNXI_PROVIDER_API_KEY = $deepseekKey
    $env:YUNXI_PROVIDER_PROFILE = "deepseek"
    $env:YUNXI_PROVIDER_BASE_URL = "https://api.deepseek.com"
    $env:YUNXI_AGENT_MODEL = $Model
    $env:YUNXI_PROVIDER_STREAM = $streamEnabled

    Push-Location $repoRoot
    try {
        cargo run -p yunxi-agent-cli -- --backend yunxi --provider-live --jsonl --model $Model $prompt > $tmp
        $exitCode = $LASTEXITCODE
    } finally {
        Pop-Location
    }

    $lines = @()
    if (Test-Path -LiteralPath $tmp) {
        $lines = Get-Content -LiteralPath $tmp
    }
    $eventCounts = @{}
    foreach ($line in $lines) {
        try {
            $event = $line | ConvertFrom-Json
            $type = [string]$event.type
            if ([string]::IsNullOrWhiteSpace($type)) {
                $type = "unknown"
            }
            if (-not $eventCounts.ContainsKey($type)) {
                $eventCounts[$type] = 0
            }
            $eventCounts[$type] += 1
        } catch {
            if (-not $eventCounts.ContainsKey("invalid_json")) {
                $eventCounts["invalid_json"] = 0
            }
            $eventCounts["invalid_json"] += 1
        }
    }

    $joinedOutput = ($lines -join "`n")
    $leakDetected = $joinedOutput -match "sk-[A-Za-z0-9_-]{20,}|Bearer [A-Za-z0-9._-]{20,}|Authorization"

    Write-Output ("deepseek_key_present={0}" -f (-not [string]::IsNullOrWhiteSpace($deepseekKey)))
    Write-Output ("credential_index={0}" -f $CredentialIndex)
    Write-Output ("model={0}" -f $Model)
    Write-Output ("base_url=https://api.deepseek.com")
    Write-Output ("stream={0}" -f $streamEnabled)
    Write-Output ("exit_code={0}" -f $exitCode)
    Write-Output ("jsonl_lines={0}" -f $lines.Count)
    Write-Output ("event_counts={0}" -f (($eventCounts.GetEnumerator() | Sort-Object Name | ForEach-Object { "$($_.Name):$($_.Value)" }) -join ","))
    Write-Output ("secret_leak_detected={0}" -f $leakDetected)

    if ($leakDetected) {
        exit 90
    }
    exit $exitCode
} finally {
    Remove-Item Env:\YUNXI_PROVIDER_API_KEY -ErrorAction SilentlyContinue
    Remove-Item Env:\YUNXI_PROVIDER_PROFILE -ErrorAction SilentlyContinue
    Remove-Item Env:\YUNXI_PROVIDER_BASE_URL -ErrorAction SilentlyContinue
    Remove-Item Env:\YUNXI_AGENT_MODEL -ErrorAction SilentlyContinue
    Remove-Item Env:\YUNXI_PROVIDER_STREAM -ErrorAction SilentlyContinue
    if (Test-Path -LiteralPath $tmp) {
        Remove-Item -LiteralPath $tmp -Force
    }
}
