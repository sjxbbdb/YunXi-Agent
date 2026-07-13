# Sandbox Event Protocol

YunXi Agent v1.7.8 emits `sandbox_attempt` events for tool execution policy and
runner diagnostics. These events describe policy evaluation and runner
selection; they do not claim OS filesystem isolation unless the machine fields
explicitly say so.

## Stable Fields

`schema_version`: integer. v1.7.8 emits `1`.

`backend_id`: stable snake_case backend identifier. Consumers should use this
instead of parsing the human `backend` string.

`backend_label`: human-readable backend description for CLI/TUI display.

`backend`: compatibility human-readable backend string. This field remains for
older consumers but is not the preferred machine key.

`enforcement`: canonical machine-readable enforcement level. Allowed values are
`no_policy_guard`, `policy_only`, `process_lifecycle`, `os_restricted`, and
`policy_bypass`.

`enforcement_level`: compatibility alias for `enforcement`. v1.7.8 keeps this
field equal to `enforcement`.

`runner`: stable runner identifier. Current Windows policy-guarded execution
uses `windows_process_lifecycle`.

`os_isolation`: boolean. `false` means YunXi is not reporting OS-enforced
filesystem isolation for that attempt.

`unsupported_reason`: human-readable reason when the selected backend cannot
claim OS filesystem isolation.

## Current Windows Boundary

The default Windows runner in v1.7.8 is policy guard plus child process
lifecycle management. It is not OS filesystem isolation.

Expected default Windows workspace-write/read-only fields:

```json
{
  "schema_version": 1,
  "backend_id": "windows_process_lifecycle",
  "enforcement": "process_lifecycle",
  "enforcement_level": "process_lifecycle",
  "runner": "windows_process_lifecycle",
  "os_isolation": false
}
```

When `danger-full-access` is selected, the attempt is a visible policy bypass:

```json
{
  "schema_version": 1,
  "backend_id": "direct_process_policy_bypass",
  "enforcement": "policy_bypass",
  "enforcement_level": "policy_bypass",
  "os_isolation": false
}
```

## Execution Policy Invariant

Every execution-capable entrypoint must evaluate `ExecutionPolicy`,
`SandboxRequirement`, and `NetworkPolicy` before it can spawn a child process,
write files, access network-capable commands, or delegate to a specialized tool
runtime.

If an entrypoint is registered but its specialized runtime is not attached, it
must still emit policy/runtime diagnostics before returning a declined response.

## v1.7.8 Execution Entry Matrix

| Entry | Current behavior | Policy source | Event evidence |
| --- | --- | --- | --- |
| `shell` | Runs through `ExecManager` after policy allows it | `ToolPolicy::execution_policy_for()` | `sandbox_decision`, `sandbox_runner`, `sandbox_attempt` |
| `patch` | Applies constrained workspace patch after policy allows it | `ToolPolicy::execution_policy_for()` | `sandbox_decision`, `sandbox_runner`, patch diagnostics |
| `mcp` | Calls attached in-memory or long-lived MCP session after policy allows it | `ToolPolicy::execution_policy_for()` | `sandbox_decision`, `sandbox_runner`, `mcp_session` |
| `skill` | Loads registered skill content after policy allows it | `ToolPolicy::execution_policy_for()` | `sandbox_decision`, `sandbox_runner` |
| `multi_agent` | Executes in-memory child lifecycle after policy allows it | `ToolPolicy::execution_policy_for()` | `sandbox_decision`, `sandbox_runner`, `multi_agent`, `child_agent` |
| `tool_search` | Reads workspace metadata after policy allows it | `ToolPolicy::execution_policy_for()` | `sandbox_decision`, `sandbox_runner` |
| `view_image` | Checks local image path after policy allows it | `ToolPolicy::execution_policy_for()` | `sandbox_decision`, `sandbox_runner` |
| `request_user_input` | Declines unless an interactive host handles it, after policy allows it | `ToolPolicy::execution_policy_for()` | `sandbox_decision`, `sandbox_runner` |
