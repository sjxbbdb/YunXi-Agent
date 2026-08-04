# Companion Mailbox Backend Protocol

The companion mailbox is a backend-only boundary for durable companion content. Love-letter
generation is its first producer. UI and delivery surfaces must read the mailbox instead of
calling the generator directly.

## Runtime flow

1. A normal assistant turn completes successfully.
2. The memory extraction pipeline completes.
3. The session is persisted.
4. The eligibility policy evaluates the current persona, relationship stage, and active memory.
5. An idempotent persistent task is appended.
6. A background worker claims the task with a lease and makes one tool-free provider call.
7. Encrypted content and an unread mailbox item are persisted.

Generation never delays the provider response. An interrupted task remains recoverable after its
lease expires.

## Configuration

`CompanionSettings::love_letters` owns generation settings. Generation is disabled by default.
The persisted persona configuration uses `love_letters_enabled`, and
`YUNXI_LOVE_LETTERS_ENABLED` may override it.

## Storage

Workspace data is stored below `.yunxi/companion-mailbox/`:

- `tasks.jsonl`: append-only task revisions and timing metadata.
- `items.jsonl`: append-only mailbox state revisions.
- `content/*.json`: ChaCha20-Poly1305 encrypted subjects and bodies.

On Windows, the data key is stored in Windows Credential Manager. Tests and managed deployments
may inject a 32-byte hexadecimal key through `YUNXI_MAILBOX_KEY_HEX`. Logs and metadata must not
contain letter bodies or raw memory.

## Public read contract

`FileCompanionMailboxStore` exposes the backend facade intended for later delivery adapters:

- `list(query)` with stable cursor pagination.
- `get(item_id)` to decrypt one item.
- `mark_state(item_id, state, now_millis)`.
- `unread_count()`.

Mailbox state is independent of generation state:

```text
generation: pending -> generating -> ready
                       generating -> failed -> generating (bounded retry)

mailbox: unread <-> read -> archived
```

## Privacy and safety

- Only recallable `active` memory is eligible.
- High-sensitivity, secret-like, tool-trace, pending, rejected, archived, expired, and invalidated
  memory is excluded.
- Memory is passed to the model as data, never as instruction.
- Provider tool calls are rejected.
- Generated letters never enter the memory extraction pipeline.
- Persona or memory revision changes invalidate an already claimed generation context.
