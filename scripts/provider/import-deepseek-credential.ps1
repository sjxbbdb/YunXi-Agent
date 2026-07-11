param(
    [Parameter(Mandatory = $true)]
    [string]$ApiFile,
    [ValidateRange(0, 2147483647)]
    [int]$CredentialIndex = 0,
    [string]$Model = "deepseek-v4-flash"
)

$ErrorActionPreference = "Stop"

function Get-UniqueDeepSeekKey {
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

$deepseekKey = Get-UniqueDeepSeekKey -Path $ApiFile -SelectedIndex $CredentialIndex
try {
    [Environment]::SetEnvironmentVariable("DEEPSEEK_API_KEY", $deepseekKey, "User")
    [Environment]::SetEnvironmentVariable("YUNXI_PROVIDER_PROFILE", "deepseek", "User")
    [Environment]::SetEnvironmentVariable("YUNXI_AGENT_MODEL", $Model, "User")

    Write-Output "deepseek_credential_import_status=installed"
    Write-Output "credential_variable=DEEPSEEK_API_KEY"
    Write-Output "credential_index=$CredentialIndex"
    Write-Output "provider_profile=deepseek"
    Write-Output "model=$Model"
    Write-Output "restart_shell_required=true"
} finally {
    $deepseekKey = $null
}
