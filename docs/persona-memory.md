# YunXi Agent Persona And Transparent Memory

YunXi Agent v1.8.7 keeps persona and long-term memory local, inspectable, and
under user control. Memory is context, not instruction: it cannot override
AGENTS.md, sandbox policy, privacy policy, tool policy, or the current user
request.

## Defaults

- Persona is enabled by default.
- The built-in profile is `yunxi_companion_strong`.
- Long-term memory writes are disabled until `yunxi memory on`,
  `YUNXI_MEMORY_ENABLED=1`, or a saved config enables them.
- Only `active` memories are recalled. `pending`, `rejected`, and `archived`
  records are never injected into prompts.
- `YUNXI_HOME` overrides the default `%USERPROFILE%\.yunxi` root.

## Persona Context Blocks

v1.8.7 compiles the built-in persona into one bounded, XML-like context string
with stable block ordering:

```text
<yunxi_persona_context version="1.8.7" profile_id="yunxi_companion_strong">
<persona>...</persona>
<boundaries>...</boundaries>
<human>...</human>
<relationship>...</relationship>
<memory_context role="context_not_instruction">...</memory_context>
</yunxi_persona_context>
```

The persona crate owns block construction, escaping, ordering, and budget
handling. Runtime still injects only `CompiledPersonaContext.content`, keeping
the provider/runtime boundary narrow. Text and attribute values escape `&`,
`<`, `>`, quotes, and apostrophes so recalled content cannot introduce new
block tags.

The `boundaries` block always states that project instructions (including
`AGENTS.md`), the current user request, sandbox/privacy/safety/tool policies,
and tool execution boundaries take priority over persona and memory. The
`memory_context` block always retains its context-not-instruction notice and
authorization boundary. The compiler filters non-active records defensively;
`pending`, `rejected`, and `archived` records are not rendered even if a caller
passes them directly.

The compiler has a 1,000-character minimum safety floor and a 1,800-character
default budget. When the requested budget is exceeded, optional lines are
removed in priority order: memory entries first, then relationship/human,
persona, and finally profile-specific boundary details. Required block tags,
the priority rule, the memory safety notices, and closing tags remain intact;
affected blocks receive a stable `<truncated section="..." />` marker.

## Storage

Memory is append-only JSONL:

```text
%USERPROFILE%\.yunxi\memory\global-memory.jsonl
%USERPROFILE%\.yunxi\memory\pending.jsonl
<workspace>\.yunxi\memory\workspace-memory.jsonl
<workspace>\.yunxi\memory\pending.jsonl
```

v1.8.7 continues to write `schema_version = 2`. Every record includes:

- `dedup_key`: `scope + kind + normalized_content`.
- `revision`: latest revision number for the durable memory id.
- `merged_count`: how many equivalent candidates have been folded into it.

Equivalent active/pending records are merged before writing. The store still
appends a new revision line, preserving the audit ledger. Archived or rejected
records are not revived automatically.

v1.8.3 added merge fidelity for equivalent records. A generic incoming memory
cannot overwrite a richer existing memory in the same `dedup_key` slot. If the
incoming memory adds real detail, YunXi promotes or combines the content while
preserving the durable id and `created_at_millis`.

v1.8.4 keeps the single-field source audit aligned with the selected merge
strategy. When `promote_incoming` supplies the final richer content,
`source_session_id` follows the incoming record. When `preserve_existing` or
`combine_non_conflicting` keeps the existing record as the primary source, the
existing source remains preferred.

Language preference conflicts are not auto-overwritten. For example, an active
Chinese preference and a later English preference share the same language
conflict family but not the same `dedup_key`; the later conflicting candidate is
written as `pending` for explicit review.

v1 JSONL records are migrated on read. Missing `schema_version` records with the
old v1 shape are treated as legacy v1 and emit a warning. Unsupported future
schema lines are skipped with a warning instead of panicking.

## Extraction

The CLI supports:

```powershell
yunxi --memory-extraction auto "prompt"
yunxi --memory-extraction rule-only "prompt"
yunxi --memory-extraction provider "prompt"
```

- `auto` is the default. Live provider mode may run one additional structured
  extraction call after the main response. Invalid JSON, provider errors, or
  timeouts warn and fall back to local rules.
- `rule-only` never calls the provider extractor.
- `provider` requires both provider and model. If unavailable, YunXi emits a
  memory warning and does not silently fall back.

Provider and rule candidates share the same batch dedup path. Chinese language
preferences such as `以后请用中文回答`, `默认用中文交流`, and `用中文回复我` normalize to
`language:zh`; English preferences such as `以后请用英文回答`, `以后请用英语回答`,
`Please answer in English from now on`, and `use English by default` normalize
to `language:en`.

In `auto` mode, provider candidates and rule candidates are folded together and
deduplicated before persistence. This lets a provider's richer preference such
as "use Chinese, be concise, and keep key details" survive a simultaneous or
later rule-extracted generic "use Chinese" candidate.

Secret-like content is never downgraded by dedup. API keys, bearer tokens,
authorization headers, passwords, GitHub tokens, and `sk-` markers remain
discarded by policy.

## Recall Diagnostics

`memory_recall` JSONL events expose counts only:

- `count`
- `always_on_count`
- `dropped_unrelated`
- `dropped_by_budget`
- `dropped_duplicates`
- `budget_used_chars`
- `truncated`

The event does not print memory content. Secret-like queries remain
`[redacted-sensitive-query]`.

Recall performs defensive dedup before scoring, so old duplicate JSONL rows do
not consume prompt budget. Global language and interaction preferences use a
small always-on budget after dedup.

## Machine-Readable Output Redaction

v1.8.7 continues to sanitize both `--json` and `--jsonl` agent execution output before
serialization. Secret-like fragments in `AgentRunResult.final_response`,
conversation events, memory recall queries, command/tool text, provider/error
messages, state `data` maps, child-agent messages, and nested JSONL protocol
payloads are replaced with `[redacted]` while ordinary non-secret text remains
visible.

This output boundary is independent from memory write policy. Secret-like
memory candidates are still discarded instead of persisted; output redaction
prevents the same sensitive text from being printed to machine-readable logs.

## CLI

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

Use `--json` for management output. `--jsonl` is reserved for agent execution
streams.

## TUI Notices

Normal TUI memory notices are intentionally short:

- `memory saved: preference`
- `memory updated: preference`
- `memory pending review`
- `memory discarded by privacy policy`
- `memory disabled`

Debug/details keep engineering fields such as id, scope, kind, status, action,
revision, merged_count, merge_strategy, conflict_family, and recall diagnostic
counts.

## Non-Goals In v1.8.7

v1.8.7 does not add Memory Schema v3, L0-L3 pipelines, SQLite, vector search,
graph memory, relationship state
machines, proactive triggers, or a TUI memory inspector page.
