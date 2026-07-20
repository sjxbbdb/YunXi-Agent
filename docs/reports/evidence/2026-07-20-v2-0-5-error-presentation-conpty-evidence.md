# YunXi Agent v2.0.5 Error Presentation And Windows ConPTY Evidence

- Evidence date: `2026-07-20` (`Asia/Shanghai`, UTC+08:00)
- Binary: `D:\YunXi Agent\target\release\yunxi.exe`
- Version: `yunxi 2.0.5`
- Terminal host: Windows ConPTY through `node-pty` with `useConpty: true`
- Frame interpreter: `@xterm/headless`
- Backend: `yunxi`
- Provider: DeepSeek live
- Model: `deepseek-chat`
- Workspace: `D:\YunXi Agent\.tmp\v205-conpty-workspace`
- Credential handling: inherited from the local environment; values were not printed, persisted in evidence, or written to Git configuration

## Evidence Method

The release binary was launched as a real ConPTY child process. The driver sent terminal resize, text, `Y`, `N`, `Ctrl+C`, PageUp, End, and `/details` input through the PTY. It did not invoke presentation functions or inject synthetic `AgentEvent` values.

The approval, cancel, decoder, long-output, and non-zero checks used separate live Provider sessions. This prevents a previous declined tool result from influencing whether the model requests the next tool. Every recorded tool case required a real approval pane before a decision key was sent. Screens were sanitized for GitHub token, bearer token, API-key, and `sk-` token shapes before inspection.

## Independent Reproduction

The 11:34 re-audit required the evidence to be independently replayable. The committed collector and exact dependency lock are now stored in `scripts/conpty/v205`. From the repository root on Windows:

```powershell
cargo build -p yunxi-agent-cli --release --bins
npm ci --prefix scripts\conpty\v205
npm run capture --prefix scripts\conpty\v205
npm run verify --prefix scripts\conpty\v205
```

The full collector launches eight independent real DeepSeek/ConPTY sessions: `responsive`, `decline`, `approve`, `cancel`, `nonzero`, `invalid`, `binary`, and `long`. It records timestamped actions, terminal dimensions, sanitized screen frames, output byte counts, and checkpoint results. A failed or timed-out checkpoint exits non-zero.

The offline verifier recomputes every JSON SHA-256, checks required checkpoint labels and interaction markers, validates error/decoder fields, and rejects GitHub, bearer, API-key, and `sk-` token shapes. It does not require Provider access.

## Raw Frame Manifest

The successful full replay ran from `2026-07-20 12:01:29 +08:00` through `12:02:15 +08:00`. Raw sanitized evidence is committed under `docs/reports/evidence/frames/v205-conpty`:

| Scenario | Checkpoints | SHA-256 |
| --- | ---: | --- |
| `responsive.json` | 7 | `b41c6e0d121d307b2768b57c4b1e43bb6df8bfb8f2379f69a4bdfefb723d30c4` |
| `decline.json` | 6 | `87327b29e98b3586573bd39ca0b5955d32dfdd53a8195c18f3b5fd8a5d485704` |
| `approve.json` | 6 | `ad6f9ab87b44ca93c977cc7f930d4fac97ddd43a8f4885fca8d22c2c038d1326` |
| `cancel.json` | 6 | `a4fe92af7c2451641f1cef9820866928ee89ddc5c959e98d5c551111d46d6cee` |
| `nonzero.json` | 6 | `f97c6b4f655fb9d5341ff38597b56bed33450fe70156195730497a49ab82751d` |
| `invalid.json` | 8 | `a4eb70a315c7c06ce9d7ae573067fe5afe9db05dafb5bc3f4f6fcfef7fe7a42e` |
| `binary.json` | 8 | `dfe631259be063d9ac5488b076bc11850bba1e940046cafe74562742d4514537` |
| `long.json` | 9 | `9215b739d2d66d7e2cef07317e438917888bc09c564d0019aa49606d4f607c40` |

`manifest.json` is generated only after all scenarios succeed. `npm run verify --prefix scripts\conpty\v205` returns `{"ok":true,"scenarios":8}` for these committed files.

## Responsive Frames

The live session started at 80x24, resized through 100x30, 120x40, and 200x50, then returned to 80x24 before the online response and approval checks.

| Local time | Size | Header result | Layout result |
| --- | --- | --- | --- |
| `2026-07-20 10:19:54 +08:00` | 80x24 | `YunXi v2.0.5 | deepseek live` | Transcript and composer visible; no overlap |
| `2026-07-20 10:19:54 +08:00` | 100x30 | Product, live Provider, and model visible | Transcript and composer visible; no overlap |
| `2026-07-20 10:19:55 +08:00` | 120x40 | Model and compact workspace path visible | Transcript and composer visible; no overlap |
| `2026-07-20 10:19:56 +08:00` | 200x50 | Full responsive header visible | Transcript and composer visible; no overlap |

The first live round trip at 80x24 returned exactly `YUNXI_V205_CONPTY_OK`.

## Approval Pane

At `2026-07-20 10:19:59 +08:00`, the harmless `probe-decline.ps1` request opened the real bottom pane at 80x24. The selected row was Decline, confirming the safe default:

```text
Approval
approval shell in D:\YunXi Agent\.tmp\v205-conpty-workspace
reason   tool execution requires approval
risk     risk: low
command  powershell -NoProfile -File .\probe-decline.ps1

  Approve  Enter/Y
> Decline  N/Esc
Tab changes selection
```

Sending `N` did not execute the script. One shell activity cell was updated in place:

```text
[tool] shell: declined; path=approval required -> running -> declined;
    YX-APPROVAL-001 approval was not granted: command request was declined;
    retryable=no; next: Review the request and choose an explicit safe action.
```

At `2026-07-20 10:20:02 +08:00`, a separate `probe-approve.ps1` request was explicitly approved with `Y`. The same activity reached completion and exposed only a quiet output summary:

```text
[tool] shell: completed; path=approval required -> running -> completed;
    output captured: 1 line(s), 36 char(s)
```

No extra `[notice] approved` or `[notice] declined` cell was present.

## Ctrl+C Cancellation

An independent 80x24 live session opened the decline-by-default approval pane for `probe-cancel.ps1`. At `2026-07-20 10:26:35 +08:00`, raw-mode `Ctrl+C` was sent instead of approving or declining.

The initial generic decline was refined by the later structured approval decision in the same activity cell. The tool was not executed and the terminal state was cancellation:

```text
[tool] shell: cancelled;
    path=approval required -> running -> declined -> cancelled;
    YX-CANCEL-001 operation cancelled: approval was cancelled;
    retryable=yes; next: Enter a new request when ready.
```

The next input succeeded in the same TUI session:

```text
[user] Reply exactly: YUNXI_CANCEL_NEXT_OK
[assistant] YUNXI_CANCEL_NEXT_OK
```

## Non-Zero Exit

At `2026-07-20 10:44:04 +08:00`, `cmd /c exit 7` was explicitly approved in an independent 120x40 live session. The one shell activity showed the stable tool error contract:

```text
[tool] shell: failed; path=approval required -> running -> failed;
    YX-TOOL-001 tool execution failed: command returned a non-zero status;
    retryable=yes; next: Review tool details and retry when safe.
```

The next Provider round trip succeeded:

```text
[user] Reply exactly: YUNXI_NONZERO_NEXT_OK
[assistant] YUNXI_NONZERO_NEXT_OK
```

## Invalid UTF-8

At `2026-07-20 10:34:23 +08:00`, `invalid-output.ps1` wrote one invalid byte to stdout. The normal tool cell did not render the raw byte or a decoder exception:

```text
[tool] shell: completed; path=approval required -> running -> completed;
    output captured: 1 line(s), 1 char(s)
```

Debug was then enabled explicitly and `/details 50` was opened. The details boundary recorded:

```text
[details] debug #50 shell result
command=powershell -NoProfile -File .\invalid-output.ps1
status=completed exit_code=0
execution_details=duration_millis=Some(195) timed_out=false
stdout{original_bytes=1 displayed_bytes=3 replacement_count=1
truncated=false integrity=Lossy display=�}
stderr{original_bytes=0 displayed_bytes=0 replacement_count=0
truncated=false integrity=Clean display=}
```

The normal transcript never displayed `stream did not contain valid UTF-8`.

## Binary Output

At `2026-07-20 10:34:26 +08:00`, `binary-output.ps1` wrote three binary bytes. The normal tool cell remained a count-only summary:

```text
[tool] shell: completed; path=approval required -> running -> completed;
    output captured: 1 line(s), 61 char(s)
```

The explicitly opened `/details 88` view recorded a safe decoder placeholder rather than raw binary:

```text
[details] debug #88 shell result
command=powershell -NoProfile -File .\binary-output.ps1
status=completed exit_code=0
execution_details=duration_millis=Some(152) timed_out=false
stdout{original_bytes=3 displayed_bytes=61 replacement_count=1
truncated=false integrity=Lossy
display=[binary output omitted: 3 bytes, 1 invalid UTF-8 sequence(s)]}
stderr{original_bytes=0 displayed_bytes=0 replacement_count=0
truncated=false integrity=Clean display=}
```

## Long Output

At `2026-07-20 10:41:50 +08:00`, `long-output.ps1` produced 2000 lines. The normal view stayed compact:

```text
[tool] shell: completed; path=approval required -> running -> completed;
    output captured: 1001 line(s), 12018 char(s)
```

The details tail showed the bounded display contract:

```text
... 1966 line(s) hidden
```

PageUp moved the real transcript viewport to the start of `/details 50`, where the truncation metadata was visible:

```text
[details] debug #50 shell result
command=powershell -NoProfile -File .\long-output.ps1
status=completed exit_code=0
execution_details=duration_millis=Some(218) timed_out=false
stdout{original_bytes=26000 displayed_bytes=12019 replacement_count=0
truncated=true integrity=Partial display=LONG_OUTPUT ...}
```

After returning to the tail, the next Provider round trip succeeded:

```text
[assistant] YUNXI_LONG_NEXT_OK
```

## Error Presentation Boundary

The ConPTY evidence directly covers approval, cancel, tool failure, decoder loss, binary fallback, and long-output truncation. Rust integration tests additionally cover provider, tool, approval, cancel, terminal, and unknown categories through the same `ErrorPresentation` boundary. Each normal error summary contains a stable `YX-*-001` code, `retryable=yes|no`, and an actionable `next:` field. Provider wire bodies, internal stacks, commands, raw decoder data, and full output remain in details/debug.

## Result

The Windows ConPTY remediation gate passed for the selected successful checkpoints:

- DeepSeek live TUI and `deepseek-chat` were visible in every session.
- 80, 100, 120, and 200-column layouts rendered without overlap.
- Approval defaulted to Decline and required explicit input.
- Approve, decline, and `Ctrl+C` did not create duplicate activity cells.
- `Ctrl+C` produced `YX-CANCEL-001` and did not execute the tool.
- Non-zero exit produced `YX-TOOL-001` and the session accepted another turn.
- Invalid UTF-8 and binary output used loss-aware details without raw-byte leakage in the tool summary.
- Long output remained compact, recorded `truncated=true / integrity=Partial`, and accepted another turn.
- The committed collector can replay all eight sessions, and the committed raw frames pass offline SHA-256/checkpoint/secret verification.

署名：开发者
