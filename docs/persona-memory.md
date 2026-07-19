# YunXi Agent Persona And Transparent Memory

YunXi Agent v2.0.2 keeps persona and long-term memory local, inspectable, and
under user control. Memory is context, not instruction: it cannot override
AGENTS.md, sandbox policy, privacy policy, tool policy, or the current user
request.

## Defaults

- Persona is enabled by default.
- The built-in profile is `yunxi_companion_strong`.
- Long-term memory writes are disabled until `yunxi memory on`,
  `YUNXI_MEMORY_ENABLED=1`, or a saved config enables them.
- Proactive companion planning is disabled until `yunxi companion on`, the
  one-turn `--companion` flag, or `YUNXI_COMPANION_ENABLED=1` enables it.
- Cloud control is a distinct persisted status field, defaults to false, and
  does not activate any cloud runtime.
- Only effective `active` memories are recalled. `pending`, `rejected`,
  `archived`, expired, invalidated, and superseded records are never injected
  into prompts.
- `YUNXI_HOME` overrides the default `%USERPROFILE%\.yunxi` root.

## Persona Context Blocks

v2.0.2 compiles the built-in persona into one bounded, XML-like context string
with stable block ordering:

```text
<yunxi_persona_context version="2.0.2" profile_id="yunxi_companion_strong" mode="routed_memory">
<persona>...</persona>
<boundaries>...</boundaries>
<human>...</human>
<relationship>...</relationship>
<boot_memory_context role="context_not_instruction">...</boot_memory_context>
<dynamic_memory_context role="context_not_instruction">...</dynamic_memory_context>
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
boot and dynamic memory blocks always retain their context-not-instruction
notice and authorization boundary. The compiler filters ineffective records defensively;
pending, rejected, archived, expired, invalidated, and superseded records are
not rendered even if a caller passes them directly.

The compiler has a 1,000-character minimum safety floor and a 3,200-character
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

v1.9.1 continues to write `schema_version = 3`. Existing v2 audit fields remain stable:

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

Language preference changes are never destructive. A later active English
preference can supersede an active Chinese preference in the same language
conflict family: storage appends an updated old record with `superseded_by` and
an `invalidated_at_millis`, then appends the new record with `supersedes`.
Pending or high-sensitivity conflicts remain pending and use `conflicts_with`
instead of invalidating an active fact.

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

v1.9.0 routes recall through two independent budgets:

- Boot Context runs before the first turn of a new session with a default
  1,000-character budget and six-record limit. It selects stable, sufficiently
  confident global preferences, user/agent and relationship baselines, and
  matching-workspace facts or constraints.
- Dynamic Recall runs on every turn with a default 1,200-character budget and
  eight-record limit. It scores the current prompt plus at most four recent
  user/assistant messages under a separate 600-character query-context cap,
  and does not inherit the legacy always-on preference path.
- One dedup key cannot appear in both routes. Resume turns skip Boot Context,
  while a stable record may still enter Dynamic Recall when the current prompt
  is genuinely relevant.
- Both routes exclude ineffective records, wrong-workspace records, and high-
  sensitivity memory. Low-confidence, low-importance, episodic noise is not
  admitted into Boot Context.

The Rust facade exposes `MemoryRecallExplanation` entries containing only the
memory id, route, score, selected flag, fixed reason, safe source category,
layer, scope, and kind. It never copies `MemoryRecord.content`, evidence text,
provider output, or a secret into the explanation. Routes are `boot`,
`dynamic`, `dropped_duplicate`, `dropped_unrelated`, `dropped_budget`, and
`dropped_invalid`.

Runtime emits separate `memory_recall` summaries with `scope=boot` and
`scope=dynamic`. JSON/JSONL summaries expose counts only:

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
not consume either route's prompt budget.

## Relationship Graph Lite

v1.9.1 derives `RelationshipGraphLite` from Schema v3 `MemoryRecord` values.
It is an in-memory view, not a second persistence system: append-only JSONL
remains the durable ledger, and no SQLite, vector store, graph database, Python
runtime, or external memory service is required.

- Nodes retain typed user, agent, workspace, project, tool, person, and
  relationship entity references. Records without explicit endpoints receive
  stable scope and memory nodes so every retained fact has an inspectable edge.
- Fact edges represent preference, correction, relationship note, project
  context, goal, emotional state, event, and related-to semantics. Schema v3
  `supersedes` and `conflicts_with` values produce explicit graph edges.
- Event ordering is descending `event_at_millis`, then
  `observed_at_millis`, then `updated_at_millis`, then
  `created_at_millis`. Active-edge queries additionally enforce valid-from,
  expiry, invalidation, supersession, and status checks.
- Clear active preference/correction changes append a bidirectional
  supersession chain. The old record stays in history but cannot enter Boot
  Context or ordinary active recall. Pending/high-sensitivity conflicts never
  invalidate an active fact automatically.
- Relationship, emotion, before/after, changed, timeline, and history queries
  use the time-ordered dynamic route. Historical expired or superseded facts
  may appear there with fixed temporal reasons, while normal current queries
  continue to exclude them.

`MemoryRecallExplanation` now adds optional `relation` and `temporal_reason`
metadata. Fixed graph reasons include `active_relation_edge`,
`temporal_event_match`, `superseded_by_newer_fact`,
`expired_relation_edge`, and `conflict_pending_confirmation`. These fields
still never contain raw memory content, evidence, provider output, or secrets.

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

## Companion UX & Controls In v1.9.3

CLI and TUI use the same runtime-generated control snapshot. It exposes local
companion state, the separate default-off cloud-control field, quiet hours,
persona profile summary, transparent memory counts, Relationship Graph Lite
counts, each scope's source, clear effects, and the most recent audited change.

```powershell
yunxi controls status
yunxi controls show companion
yunxi controls show memory
yunxi controls show persona
yunxi controls show relationship
yunxi controls enable companion
yunxi controls disable companion
yunxi controls clear companion --confirm
yunxi controls clear memory --confirm
yunxi controls audit
```

In interactive/TUI mode, `/controls` opens the shared control panel.
`/controls clear companion` and `/controls clear memory` open an explicit
confirmation input and require `CLEAR COMPANION` or `CLEAR MEMORY`. Persona and
relationship scopes are review-only and cannot be cleared through the UI.

Control audit and companion plan history are separate local JSONL ledgers:

```text
<workspace>/.yunxi/controls/audit.jsonl
<workspace>/.yunxi/controls/companion-history.jsonl
```

Memory clear remains append-only by writing archived revisions for active and
pending workspace records. Companion clear truncates only the companion
history ledger. Neither operation changes persona profile definitions or the
derived relationship graph directly.

## General Companion Integration In v2.0.0

The runtime exposes one `GeneralCompanionSnapshot` over persona, Memory Schema
v3, read-only relationship history, proactive settings, cloud-control state,
and the shared control snapshot. It explicitly identifies the runtime as
YunXi-owned and records that the default path does not require upstream Codex.
Companion planning and cloud control remain disabled by default.

The v2 cross-session regression writes an auditable preference in one session,
then starts an independent session and verifies that the preference is recalled
through the persona context. It also links an older relationship fact to its
replacement: the old revision remains in append-only storage, while only the
new fact is recalled and counted as active. This keeps memory review, reject,
archive, confirmation, validity, and supersession semantics visible instead of
collapsing history into an opaque current-state record.

CLI/TUI controls continue to consume the same runtime boundary. Relationship
operations remain review-only, proactive tool ideas remain confirmation
requests, and the 31-scenario offline evaluation harness continues to enforce
the unchanged golden thresholds.

## Evaluation Harness In v1.9.4

`yunxi eval companion` runs the local persona, memory, relationship, proactive,
and control regression suite from `evals/companion`. The suite is deterministic
and offline: no provider API key, live model, cloud judge, Python runtime, or
external service is used.

The memory metrics distinguish correct writes, false-positive writes, missed
writes, and forbidden secret-like writes. Relationship checks cover
supersession links, retained history, expiry, future validity, timeline order,
and read-only control state. Proactive checks cover default/explicit silence,
quiet hours, session/day limits, reason visibility, and tool confirmation.

Use `--json` for a complete structured report or `--jsonl` for one compact
report line. The golden thresholds and scenario/result schemas remain small,
reviewable files under `evals/companion`; generated result files are not
committed.

## Proactive Companion In v1.9.2

The v1.9.2 companion planner consumes persona, memory recall, and relationship
signals as read-only context. It does not write memory, schedule background
jobs, call tools, or bypass approval. `AgentConfig.companion.enabled` defaults
to `false`; every generated plan includes a reason, quiet hours are honored,
and per-session/day limits are enforced in the planner.

## Non-Goals In v1.9.2

v1.9.2 does not add SQLite, vector search, an external graph
database or memory runtime, cloud/marketplace/SDK surfaces, an evaluation
harness, or a TUI memory inspector page.
