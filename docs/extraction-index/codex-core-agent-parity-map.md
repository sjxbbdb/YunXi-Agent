# Codex Core Agent Parity Map

This index is the Stage 4E build map. It keeps the migration target concrete:
copy Codex CLI core Agent behavior into YunXi-owned crates without adding
`codex-*` or `vendor/codex-rs` dependencies to the default runtime.

## Stage 4E Migration Staging

The Codex CLI core Agent source references for the 12 parity layers are staged
under `extracted/codex-core-agent-sources`. That directory is migration input
only; compiled YunXi crates must keep using YunXi-owned modules and must not
depend on `vendor/codex-rs`, `codex-*`, or `yunxi-agent-codex` in the default
runtime path.

## Rules

- Use `vendor/codex-rs` as source reference only.
- Do not add `codex-*` crates to YunXi default dependencies.
- Preserve behavior first; rename and de-Codex later.
- Keep model/provider code behind `yunxi-agent-provider`.
- Preserve upstream license notices when copying substantial source text.
- Every migrated capability needs a YunXi fixture or unit test.

## Target Crates

| YunXi crate | Responsibility |
| --- | --- |
| `yunxi-agent-protocol` | Input items, response items, tool calls, runtime events, JSONL shape |
| `yunxi-agent-context` | AGENTS.md, prompt assembly, context fragments, token budget, compaction entry points |
| `yunxi-agent-exec` | Command model, canonicalization, output limits, stdin and exec lifecycle data |
| `yunxi-agent-sandbox` | Approval, sandbox, cwd, network and escalation policy decisions |
| `yunxi-agent-patch` | Codex-style apply_patch parsing and filesystem application |
| `yunxi-agent-tools` | Tool registry, routing, runtime dispatch and file-change reporting |
| `yunxi-agent-mcp` | MCP config, resource operations, tool invocation and elicitation boundary |
| `yunxi-agent-skills` | Skill discovery, metadata, injection and invocation boundary |
| `yunxi-agent-storage` | Thread metadata, rollout, history, resume, archive, fork and pin |
| `yunxi-agent-multi-agent` | Spawn, wait, message, follow-up, interrupt, list and agent graph |
| `yunxi-agent-runtime` | Thread/session/turn loop, provider stream loop and orchestration |

## Runtime / Thread / Turn

| Codex source | YunXi target | Migration mode |
| --- | --- | --- |
| `vendor/codex-rs/core/src/codex_thread.rs` | `yunxi-agent-runtime` | mechanical port |
| `vendor/codex-rs/core/src/thread_manager.rs` | `yunxi-agent-runtime`, `yunxi-agent-storage` | mechanical port |
| `vendor/codex-rs/core/src/session/*.rs` | `yunxi-agent-runtime` | mechanical port by file |
| `vendor/codex-rs/core/src/client_common.rs` | `yunxi-agent-protocol`, `yunxi-agent-provider` | protocol extraction |
| `vendor/codex-rs/core/src/client.rs` | `yunxi-agent-provider` | provider boundary only |
| `vendor/codex-rs/core/src/responses_retry.rs` | `yunxi-agent-provider` | mechanical port without Codex types |
| `vendor/codex-rs/core/src/event_mapping.rs` | `yunxi-agent-protocol` | fixture-backed mapping |
| `vendor/codex-rs/core/src/turn_metadata.rs` | `yunxi-agent-runtime`, `yunxi-agent-protocol` | mechanical port |

## Prompt / Context / AGENTS.md

| Codex source | YunXi target | Migration mode |
| --- | --- | --- |
| `vendor/codex-rs/core/src/agents_md.rs` | `yunxi-agent-context` | mechanical port |
| `vendor/codex-rs/core/src/agents_md_manager.rs` | `yunxi-agent-context` | mechanical port |
| `vendor/codex-rs/core/src/prompt_debug.rs` | `yunxi-agent-context` | prompt fixture parity |
| `vendor/codex-rs/core/src/context/**` | `yunxi-agent-context` | mechanical port |
| `vendor/codex-rs/core/src/context_manager/**` | `yunxi-agent-context` | mechanical port |
| `vendor/codex-rs/context-fragments/**` | `yunxi-agent-context` | mechanical port |
| `vendor/codex-rs/prompts/**` | `yunxi-agent-context` | copy prompt assets |
| `vendor/codex-rs/file-search/**` | `yunxi-agent-context` | mechanical port if needed |

## Compact / History / Token Budget

| Codex source | YunXi target | Migration mode |
| --- | --- | --- |
| `vendor/codex-rs/core/src/compact*.rs` | `yunxi-agent-context` | mechanical port |
| `vendor/codex-rs/core/src/compact_token_budget.rs` | `yunxi-agent-context` | mechanical port |
| `vendor/codex-rs/message-history/**` | `yunxi-agent-storage` | mechanical port |
| `vendor/codex-rs/core/src/session/token_budget.rs` | `yunxi-agent-context` | mechanical port |
| `vendor/codex-rs/core/src/session/context_window.rs` | `yunxi-agent-context` | mechanical port |

## Tools / Router / Dispatch

| Codex source | YunXi target | Migration mode |
| --- | --- | --- |
| `vendor/codex-rs/core/src/tools/mod.rs` | `yunxi-agent-tools` | mechanical port |
| `vendor/codex-rs/core/src/tools/registry.rs` | `yunxi-agent-tools` | mechanical port |
| `vendor/codex-rs/core/src/tools/router.rs` | `yunxi-agent-tools` | mechanical port |
| `vendor/codex-rs/core/src/tools/events.rs` | `yunxi-agent-protocol`, `yunxi-agent-tools` | protocol extraction |
| `vendor/codex-rs/core/src/tools/handlers/**` | domain crates plus `yunxi-agent-tools` | split by handler |
| `vendor/codex-rs/core/src/tools/runtimes/**` | `yunxi-agent-tools`, `yunxi-agent-exec`, `yunxi-agent-patch` | split by runtime |
| `vendor/codex-rs/core/src/function_tool.rs` | `yunxi-agent-tools` | schema parity |

## Shell / Exec / Sandbox / Approval

| Codex source | YunXi target | Migration mode |
| --- | --- | --- |
| `vendor/codex-rs/core/src/exec.rs` | `yunxi-agent-exec` | mechanical port |
| `vendor/codex-rs/core/src/unified_exec/**` | `yunxi-agent-exec` | mechanical port |
| `vendor/codex-rs/core/src/shell.rs` | `yunxi-agent-exec`, `yunxi-agent-tools` | mechanical port |
| `vendor/codex-rs/core/src/shell_snapshot.rs` | `yunxi-agent-exec` | mechanical port |
| `vendor/codex-rs/core/src/command_canonicalization.rs` | `yunxi-agent-exec` | mechanical port |
| `vendor/codex-rs/core/src/exec_policy.rs` | `yunxi-agent-sandbox` | mechanical port |
| `vendor/codex-rs/exec/**` | `yunxi-agent-exec` | mechanical port |
| `vendor/codex-rs/exec-server/**` | `yunxi-agent-exec` | later parity |
| `vendor/codex-rs/execpolicy/**` | `yunxi-agent-sandbox` | mechanical port |
| `vendor/codex-rs/sandboxing/**` | `yunxi-agent-sandbox` | mechanical port |
| `vendor/codex-rs/linux-sandbox/**` | `yunxi-agent-sandbox` | platform port |
| `vendor/codex-rs/windows-sandbox-rs/**` | `yunxi-agent-sandbox` | platform port |
| `vendor/codex-rs/shell-command/**` | `yunxi-agent-exec` | mechanical port |
| `vendor/codex-rs/shell-escalation/**` | `yunxi-agent-sandbox` | platform port |

## Patch

| Codex source | YunXi target | Migration mode |
| --- | --- | --- |
| `vendor/codex-rs/core/src/apply_patch.rs` | `yunxi-agent-patch` | mechanical port |
| `vendor/codex-rs/core/src/tools/handlers/apply_patch.rs` | `yunxi-agent-patch`, `yunxi-agent-tools` | mechanical port |
| `vendor/codex-rs/core/src/tools/handlers/apply_patch.lark` | `yunxi-agent-patch` | grammar asset |
| `vendor/codex-rs/core/src/tools/runtimes/apply_patch.rs` | `yunxi-agent-patch` | mechanical port |
| `vendor/codex-rs/apply-patch/**` | `yunxi-agent-patch` | mechanical port |
| `vendor/codex-rs/git-utils/src/apply.rs` | `yunxi-agent-patch` | mechanical port if needed |

## MCP

| Codex source | YunXi target | Migration mode |
| --- | --- | --- |
| `vendor/codex-rs/codex-mcp/**` | `yunxi-agent-mcp` | mechanical port |
| `vendor/codex-rs/rmcp-client/**` | `yunxi-agent-mcp` | mechanical port |
| `vendor/codex-rs/core/src/mcp.rs` | `yunxi-agent-mcp`, `yunxi-agent-runtime` | mechanical port |
| `vendor/codex-rs/core/src/mcp_tool_call.rs` | `yunxi-agent-mcp`, `yunxi-agent-tools` | mechanical port |
| `vendor/codex-rs/core/src/mcp_tool_approval_templates.rs` | `yunxi-agent-mcp` | template parity |
| `vendor/codex-rs/core/src/session/mcp*.rs` | `yunxi-agent-mcp`, `yunxi-agent-runtime` | mechanical port |

## Skills / Plugins / Dynamic Tools

| Codex source | YunXi target | Migration mode |
| --- | --- | --- |
| `vendor/codex-rs/core/src/skills.rs` | `yunxi-agent-skills` | mechanical port |
| `vendor/codex-rs/core-skills/**` | `yunxi-agent-skills` | mechanical port |
| `vendor/codex-rs/skills/**` | `yunxi-agent-skills` | mechanical port |
| `vendor/codex-rs/plugin/**` | `yunxi-agent-skills` | later plugin crate |
| `vendor/codex-rs/core-plugins/**` | `yunxi-agent-skills` | later plugin crate |
| `vendor/codex-rs/core/src/plugins/**` | `yunxi-agent-skills` | later plugin crate |
| `vendor/codex-rs/core/src/tools/handlers/tool_search.rs` | `yunxi-agent-skills`, `yunxi-agent-tools` | mechanical port |
| `vendor/codex-rs/core/src/tools/handlers/request_user_input.rs` | `yunxi-agent-tools` | mechanical port |
| `vendor/codex-rs/core/src/tools/handlers/view_image.rs` | `yunxi-agent-tools` | mechanical port |

## Storage / Rollout / Resume

| Codex source | YunXi target | Migration mode |
| --- | --- | --- |
| `vendor/codex-rs/core/src/rollout.rs` | `yunxi-agent-storage` | mechanical port |
| `vendor/codex-rs/core/src/rollout_budget.rs` | `yunxi-agent-storage`, `yunxi-agent-multi-agent` | mechanical port |
| `vendor/codex-rs/core/src/thread_rollout_truncation.rs` | `yunxi-agent-storage` | mechanical port |
| `vendor/codex-rs/thread-store/**` | `yunxi-agent-storage` | mechanical port |
| `vendor/codex-rs/state/**` | `yunxi-agent-storage` | mechanical port |
| `vendor/codex-rs/core/src/state_db_bridge.rs` | `yunxi-agent-storage` | mechanical port or YunXi equivalent |
| `vendor/codex-rs/external-agent-sessions/**` | `yunxi-agent-storage` | import only |

## Multi-Agent

| Codex source | YunXi target | Migration mode |
| --- | --- | --- |
| `vendor/codex-rs/core/src/agent/**` | `yunxi-agent-multi-agent` | mechanical port |
| `vendor/codex-rs/core/src/agent_communication.rs` | `yunxi-agent-multi-agent` | mechanical port |
| `vendor/codex-rs/core/src/tools/handlers/multi_agents/**` | `yunxi-agent-multi-agent`, `yunxi-agent-tools` | mechanical port |
| `vendor/codex-rs/core/src/tools/handlers/multi_agents_v2/**` | `yunxi-agent-multi-agent`, `yunxi-agent-tools` | mechanical port |
| `vendor/codex-rs/agent-graph-store/**` | `yunxi-agent-multi-agent`, `yunxi-agent-storage` | mechanical port |

## Non-Core Product Surfaces

These are not part of Stage 4D unless a later report explicitly expands scope:

- `vendor/codex-rs/tui/**`
- `vendor/codex-rs/cloud-tasks/**`
- `vendor/codex-rs/cloud-tasks-client/**`
- `vendor/codex-rs/app-server-daemon/**`
- release installer, update, doctor, completion and marketplace surfaces
