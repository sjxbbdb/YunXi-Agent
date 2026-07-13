# YunXi Agent Persona And Transparent Memory

YunXi Agent v1.8.1 hardens the first YunXi-owned persona and transparent memory
foundation. The goal is not to make memory invisible or automatic in a risky
way; the goal is to make persona context stable, local, relevant, inspectable,
and under user control.

## Defaults

- Persona is enabled by default.
- The active built-in profile is `yunxi_companion_strong`.
- Long-term memory writes are disabled by default.
- Memory writes become active only after `yunxi memory on`,
  `YUNXI_MEMORY_ENABLED=1`, or a saved config setting enables them.
- Pending, rejected, and archived memories are not recalled into prompts.

Settings are stored at:

```text
%USERPROFILE%\.yunxi\persona\config.toml
```

`YUNXI_HOME` can override the root directory.

## Storage Paths

Global memory:

```text
%USERPROFILE%\.yunxi\memory\global-memory.jsonl
%USERPROFILE%\.yunxi\memory\pending.jsonl
```

Workspace memory:

```text
<workspace>\.yunxi\memory\workspace-memory.jsonl
<workspace>\.yunxi\memory\pending.jsonl
```

YunXi v1.8.1 stores memory as append-only JSONL. Status changes append a new
record revision rather than silently deleting the old audit entry. Workspace
records use a stable workspace fingerprint instead of exposing the full
workspace path inside the record scope.

v1.8.1 keeps `memory pending` consistent by reading the full latest-record view
and then filtering pending records. Approving, rejecting, or archiving a pending
record removes it from `memory pending` without rewriting the audit ledger.

## CLI

Persona commands:

```powershell
yunxi persona status
yunxi persona profile
yunxi persona set yunxi_companion_strong
yunxi persona off
yunxi persona on
```

Memory commands:

```powershell
yunxi memory status
yunxi memory list --global
yunxi memory list --workspace
yunxi memory show <id>
yunxi memory search <query>
yunxi memory pending
yunxi memory approve <id>
yunxi memory reject <id>
yunxi memory delete <id>
yunxi memory clear --workspace --confirm
yunxi memory off
yunxi memory on
```

Use `--json` for machine-readable management output. `--jsonl` remains reserved
for agent execution event streams and is rejected for persona/memory management
commands.

## Record Schema

Every memory record has:

- `schema_version`: v1.8.1 writes `1`.
- `id`: stable memory id.
- `scope`: `global_user`, `workspace`, `agent_identity`, or `relationship`.
- `kind`: preference, personal fact, correction, project context, and related
  categories.
- `content`: concise memory text.
- `source_session_id`: optional session id.
- `confidence`: 0.0 to 1.0.
- `importance`: 0.0 to 1.0.
- `sensitivity`: `low`, `medium`, or `high`.
- `status`: `active`, `pending`, `rejected`, or `archived`.
- `created_at_millis` and `updated_at_millis`.

## Prompt Injection

Runtime prompt order is:

1. Project and user instructions from AGENTS.md.
2. YunXi persona context.
3. Recalled active memory context.
4. Mentioned file context.
5. Restored session history.
6. Current user prompt.

Memory is marked as context, not instruction. It cannot override project
instructions, sandbox policy, privacy policy, or the user's current turn.

## Write Policy

Provider-backed structured extraction may run after a completed live-provider
turn. It has a timeout, falls back to rule extraction, and never blocks the
assistant final response.

- Low-risk preferences, corrections, and project context can be saved as active
  memories when memory is enabled.
- Personal facts, relationship notes, emotional state, goals, events, and
  medium-risk content require confirmation and go to pending.
- API keys, bearer tokens, authorization headers, passwords, GitHub tokens, and
  similar secrets are discarded or require confirmation; they are not auto-saved
  as active memory.
- When memory is disabled, extraction candidates are not written.
- Language and general interaction preferences are global user memories by
  default. Project hard constraints and workspace facts remain workspace-scoped.
- Recall has a relevance gate. Active memories are not injected only because
  they exist; global language/interaction preferences use a small always-on
  profile budget.

Persona/memory runtime events expose only summary fields such as id, kind,
status, scope, counts, and budget. They do not print full memory content into
JSONL execution streams.

## Non-Goals In v1.8.1

YunXi v1.8.1 does not add SQLite, vector search, graph memory, relationship
state machines, active check-in triggers, or TUI memory inspector screens.
Those can be layered later after this local transparent foundation is stable.
