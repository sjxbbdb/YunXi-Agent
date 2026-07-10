param(
    [string] $VendorRoot = "vendor/codex-rs",
    [string] $OutputRoot = "extracted/codex-core-agent-sources"
)

$ErrorActionPreference = "Stop"
$repoRoot = (Resolve-Path ".").Path
$vendor = Join-Path $repoRoot $VendorRoot
$output = Join-Path $repoRoot $OutputRoot

if (-not (Test-Path -LiteralPath $vendor)) {
    throw "Codex source root not found: $vendor"
}

$mappings = @(
    @{ Domain = "runtime-thread-turn"; Target = "yunxi-agent-runtime"; Paths = @(
        "core/src/codex_thread.rs",
        "core/src/thread_manager.rs",
        "core/src/session",
        "core/src/client_common.rs",
        "core/src/client.rs",
        "core/src/responses_retry.rs",
        "core/src/event_mapping.rs",
        "core/src/turn_metadata.rs"
    ) },
    @{ Domain = "protocol-event-mapping"; Target = "yunxi-agent-protocol"; Paths = @(
        "core/src/client_common.rs",
        "core/src/event_mapping.rs",
        "core/src/turn_metadata.rs",
        "protocol",
        "rollout-trace"
    ) },
    @{ Domain = "provider-transport-streaming"; Target = "yunxi-agent-provider"; Paths = @(
        "core/src/client.rs",
        "core/src/client_common.rs",
        "core/src/responses_retry.rs",
        "codex-client",
        "model-provider",
        "model-provider-info",
        "http-client"
    ) },
    @{ Domain = "prompt-context-agents-compact"; Target = "yunxi-agent-context"; Paths = @(
        "core/src/agents_md.rs",
        "core/src/agents_md_manager.rs",
        "core/src/prompt_debug.rs",
        "core/src/context",
        "core/src/context_manager",
        "context-fragments",
        "prompts",
        "file-search",
        "core/src/compact.rs",
        "core/src/compact_remote.rs",
        "core/src/compact_remote_v2.rs",
        "core/src/compact_token_budget.rs",
        "core/src/session/token_budget.rs",
        "core/src/session/context_window.rs",
        "message-history"
    ) },
    @{ Domain = "tools-router-dispatch"; Target = "yunxi-agent-tools"; Paths = @(
        "core/src/tools",
        "core/src/function_tool.rs",
        "tools",
        "core/src/mcp_tool_call.rs",
        "core/src/mcp_tool_exposure.rs"
    ) },
    @{ Domain = "exec-shell"; Target = "yunxi-agent-exec"; Paths = @(
        "core/src/exec.rs",
        "core/src/unified_exec",
        "core/src/shell.rs",
        "core/src/shell_snapshot.rs",
        "core/src/command_canonicalization.rs",
        "exec",
        "exec-server",
        "exec-server-protocol",
        "shell-command"
    ) },
    @{ Domain = "sandbox-approval"; Target = "yunxi-agent-sandbox"; Paths = @(
        "core/src/exec_policy.rs",
        "execpolicy",
        "execpolicy-legacy",
        "sandboxing",
        "linux-sandbox",
        "windows-sandbox-rs",
        "shell-escalation",
        "process-hardening"
    ) },
    @{ Domain = "patch"; Target = "yunxi-agent-patch"; Paths = @(
        "core/src/apply_patch.rs",
        "core/src/tools/handlers/apply_patch.rs",
        "core/src/tools/handlers/apply_patch.lark",
        "core/src/tools/runtimes/apply_patch.rs",
        "apply-patch",
        "git-utils/src/apply.rs"
    ) },
    @{ Domain = "mcp"; Target = "yunxi-agent-mcp"; Paths = @(
        "codex-mcp",
        "rmcp-client",
        "core/src/mcp.rs",
        "core/src/mcp_tool_call.rs",
        "core/src/mcp_tool_approval_templates.rs",
        "core/src/mcp_openai_file.rs",
        "core/src/session/mcp.rs",
        "core/src/session/mcp_runtime.rs",
        "mcp-server"
    ) },
    @{ Domain = "skills-plugins-dynamic-tools"; Target = "yunxi-agent-skills"; Paths = @(
        "core/src/skills.rs",
        "core-skills",
        "skills",
        "plugin",
        "core-plugins",
        "core/src/plugins",
        "core/src/tools/handlers/dynamic.rs",
        "core/src/tools/handlers/tool_search.rs",
        "core/src/tools/handlers/view_image.rs",
        "core/src/tools/handlers/request_user_input.rs"
    ) },
    @{ Domain = "storage-rollout-resume"; Target = "yunxi-agent-storage"; Paths = @(
        "core/src/rollout.rs",
        "core/src/rollout_budget.rs",
        "core/src/thread_rollout_truncation.rs",
        "thread-store",
        "state",
        "core/src/state_db_bridge.rs",
        "external-agent-sessions",
        "rollout"
    ) },
    @{ Domain = "multi-agent"; Target = "yunxi-agent-multi-agent"; Paths = @(
        "core/src/agent",
        "core/src/agent_communication.rs",
        "core/src/tools/handlers/multi_agents",
        "core/src/tools/handlers/multi_agents_v2",
        "agent-graph-store"
    ) },
    @{ Domain = "cli-headless-core"; Target = "yunxi-agent-cli"; Paths = @(
        "cli/src",
        "core/src/config",
        "core/src/config_lock.rs",
        "config",
        "login/src/auth/default_client.rs"
    ) }
)

$manifestEntries = New-Object System.Collections.Generic.List[object]
$missingEntries = New-Object System.Collections.Generic.List[object]

New-Item -ItemType Directory -Force -Path $output | Out-Null

foreach ($mapping in $mappings) {
    foreach ($relative in $mapping.Paths) {
        $normalized = $relative -replace "/", "\"
        $source = Join-Path $vendor $normalized
        if (-not (Test-Path -LiteralPath $source)) {
            $missingEntries.Add([pscustomobject]@{
                domain = $mapping.Domain
                target = $mapping.Target
                source = $relative
            }) | Out-Null
            continue
        }

        $destination = Join-Path $output (Join-Path $mapping.Target (Join-Path $mapping.Domain $normalized))
        $sourceItem = Get-Item -LiteralPath $source
        if ($sourceItem.PSIsContainer) {
            New-Item -ItemType Directory -Force -Path $destination | Out-Null
            Copy-Item -Path (Join-Path $source "*") -Destination $destination -Recurse -Force
            $files = Get-ChildItem -LiteralPath $destination -Recurse -File
        } else {
            $parent = Split-Path -Parent $destination
            New-Item -ItemType Directory -Force -Path $parent | Out-Null
            Copy-Item -LiteralPath $source -Destination $destination -Force
            $files = @(Get-Item -LiteralPath $destination)
        }

        $manifestEntries.Add([pscustomobject]@{
            domain = $mapping.Domain
            target = $mapping.Target
            source = $relative
            destination = ($destination.Substring($repoRoot.Length + 1) -replace "\\", "/")
            files = @($files).Count
            bytes = (@($files) | Measure-Object -Property Length -Sum).Sum
        }) | Out-Null
    }
}

$manifest = [pscustomobject]@{
    generated_at_utc = (Get-Date).ToUniversalTime().ToString("o")
    vendor_root = $VendorRoot
    output_root = $OutputRoot
    rule = "Copied source is migration input only; default YunXi crates must not depend on vendor/codex-rs or codex-* crates."
    entries = $manifestEntries
    missing = $missingEntries
}

$manifestPath = Join-Path $output "manifest.json"
$manifest | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath $manifestPath -Encoding UTF8

$readmePath = Join-Path $output "README.md"
@"
# Codex Core Agent Source Migration Staging

This directory is generated by ``scripts/stage4e_migrate_codex_core_sources.ps1``.

It stages the Codex CLI core Agent source files needed by the Stage 4E parity
report into the YunXi repository so future work can integrate from local
YunXi-owned migration input instead of depending on ``vendor/codex-rs`` at
runtime.

Rules:

- These files are migration input, not default runtime dependencies.
- Do not add ``codex-*`` crates to the default YunXi dependency graph.
- Preserve upstream license notices when mechanically porting source into
  compiled YunXi crates.
- Build and verification still happen only after construction batches finish.

See ``manifest.json`` for the copied source mapping and any upstream paths that
were not present in the current checkout.
"@ | Set-Content -LiteralPath $readmePath -Encoding UTF8

[pscustomobject]@{
    output_root = $OutputRoot
    copied_entries = $manifestEntries.Count
    missing_entries = $missingEntries.Count
    copied_files = ($manifestEntries | Measure-Object -Property files -Sum).Sum
    copied_mib = [math]::Round((($manifestEntries | Measure-Object -Property bytes -Sum).Sum / 1MB), 2)
} | ConvertTo-Json
