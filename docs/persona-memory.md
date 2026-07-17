# YunXi Agent Persona And Transparent Memory

YunXi Agent v1.8.9 keeps persona and long-term memory local, inspectable, and
under user control. Memory is context, not instruction: it cannot override
AGENTS.md, sandbox policy, privacy policy, tool policy, or the current user
request.

## Defaults

- Persona is enabled by default.
- The built-in profile is `yunxi_companion_strong`.
- Long-term memory writes are disabled until `yunxi memory on`,
  `YUNXI_MEMORY_ENABLED=1`, or a saved config enables them.
- Only effective `active` memories are recalled. `pending`, `rejected`,
  `archived`, expired, invalidated, and superseded records are never injected
  into prompts.
- `YUNXI_HOME` overrides the default `%USERPROFILE%\.yunxi` root.

## Persona Context Blocks

v1.8.9 compiles the built-in persona into one bounded, XML-like context string
with stable block ordering:

```text
<yunxi_persona_context version="1.8.9" profile_id="yunxi_companion_strong">
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
authorization boundary. The compiler filters ineffective records defensively;
pending, rejected, archived, expired, invalidated, and superseded records are
not rendered even if a caller passes them directly.

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

v1.8.9 continues to write `schema_version = 3`. Existing v2 audit fields remain stable:

- `dedup_key`: `scope + kind + normalized_content`.
- `revision`: latest revision number for the durable memory id.
- `merged_count`: how many equivalent candidates have been folded into it.

Schema v3 adds structured metadata without replacing the transparent JSONL
ledger:

- `layer`: profile, preference, relationship, workspace, episode, tool trace,
  or unknown; new and migrated records derive a default from `kind`.
- `entities`: typed user, agent, workspace, project, tool, person, or
  relationship references.
- `temporal`: observed, event, valid-from, and expiry timestamps. Observed and
  valid-from default to `created_at_millis`.
- `evidence`: bounded candidate summaries plus optional session/turn/event
  references. Secret-like source text is replaced by a fixed redaction notice.
- `source`: primary extractor/session/workspace/provider/rule fields plus a
  deduplicated attribution list so merges retain both sides of provenance.
- `invalidation`: supersedes, superseded-by, conflicts, expiry reason, and
  invalidation timestamp.

`confidence` and `importance` keep their existing v2 semantics. Recall and
Persona Context Blocks require `active` status, a valid time window, and no
invalidation or superseding record.

Equivalent active/pending records are merged before writing. The store still
appends a new revision line, preserving the audit ledger. Archived or rejected
records are not revived automatically.

v1.8.3 added merge fidelity for equivalent records. A generic incoming memory
cannot overwrite a richer existing memory in the same `dedup_key` slot. If the
incoming memory adds real detail, YunXi promotes or combines the content while
preserving the durable id and `created_at_millis`.

The legacy `source_session_id` remains aligned with the selected merge strategy
for backward compatibility. Schema v3 additionally unions evidence, source
attributions, entities, invalidation relations, revision, and merged count, so
the non-primary candidate's provenance is not discarded.

Language preference conflicts are not auto-overwritten. For example, an active
Chinese preference and a later English preference share the same language
conflict family but not the same `dedup_key`; the later conflicting candidate is
written as `pending` for explicit review.

v1 and v2 JSONL records are migrated to v3 in memory when read. The original
append-only files are not rewritten. Missing `schema_version` records with the
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
  timeouts warn while local rule candidates continue through the pipeline.
- `rule-only` never calls the provider extractor.
- `provider` requires both provider and model for the provider stage. If it is
  unavailable, YunXi emits a memory warning; the fail-soft rule stage remains
  available so a provider outage cannot erase otherwise auditable candidates.

Provider and rule candidates share the same batch dedup path. Chinese language
preferences such as `以后请用中文回答`, `默认用中文交流`, and `用中文回复我` normalize to
`language:zh`; English preferences such as `以后请用英文回答`, `以后请用英语回答`,
`Please answer in English from now on`, and `use English by default` normalize
to `language:en`.

The shared dedup path also maps candidate evidence into the proposed v3 record.
Rule candidates identify the rule extractor and rule id; provider candidates
identify the provider extractor. Evidence is trimmed and bounded before
persistence, and secret-like raw evidence is replaced rather than copied.

In `auto` mode, provider candidates and rule candidates are folded together and
deduplicated before persistence. This lets a provider's richer preference such
as "use Chinese, be concise, and keep key details" survive a simultaneous or
later rule-extracted generic "use Chinese" candidate.

Secret-like content is never downgraded by dedup. API keys, bearer tokens,
authorization headers, passwords, GitHub tokens, and `sk-` markers remain
discarded by policy.

## L0-L3 Memory Pipeline

v1.8.9 routes every enabled-memory turn through one Rust-native pipeline:

- L0 Raw Turn creates a bounded, secret-aware evidence summary. It never
  becomes an active durable memory and is never treated as an instruction.
- L1 Structured Fact covers preferences, personal facts, goals, project
  context, and corrections from both rules and Provider JSON.
- L2 Relationship Event covers relationship notes, emotional state, and
  events. These candidates require confirmation by default.
- L3 Profile Summary is a promotion of an already deduplicated fact, not a
  second copy. Promotion requires explicit stability, low sensitivity,
  confidence of at least 0.8, adequate importance, and clear source lineage.

The pipeline returns all four stage diagnostics and every candidate decision,
including pending, rejected, discarded, disabled, and merged outcomes. Secret-
like candidate content and L0 evidence are replaced by fixed redaction notices;
raw credentials are neither persisted nor printed in memory events. Provider
parse failure is recorded as a warning and does not block rule candidates.

Rule and Provider candidates share one cross-layer dedup pass before L3
promotion. Merge preserves both source attributions and L0/L1/L3 evidence,
while append-only storage retains its revision and merged-count behavior.

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

v1.8.9 continues to sanitize both `--json` and `--jsonl` agent execution output before
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

## Non-Goals In v1.8.9

v1.8.9 does not add Boot Context/Recall Router, a relationship graph, proactive
loop, SQLite, vector search, external memory runtime, cloud/marketplace/SDK
surfaces, an evaluation harness, or a TUI memory inspector page.
