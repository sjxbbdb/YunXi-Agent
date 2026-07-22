# v2.0.9 Windows ConPTY Resilience Verifier

This verifier launches the release `yunxi.exe` through real Windows ConPTY.
Node.js, `node-pty`, and `@xterm/headless` are evidence-only dependencies and
are not part of the YunXi runtime path.

It verifies:

- normal exit and Ctrl+C restore alternate screen, cursor, mouse capture,
  bracketed paste, focus tracking, and raw terminal mode;
- plain, pipe, CI, JSON, JSONL, `--no-tui`, and forced-TUI fallback output do
  not contain ANSI or TUI footer bytes;
- a Provider failure freezes the partial turn and the next prompt succeeds;
- an oversized active SSE stream remains responsive, can be cancelled, and
  does not prevent the next turn.

From `D:\YunXi Agent`:

```powershell
cargo build -p yunxi-agent-cli --release --bins
npm.cmd ci --prefix scripts\conpty\v209
npm.cmd run capture --prefix scripts\conpty\v209 -- --output-dir .tmp\conpty\v209-capture
npm.cmd run verify --prefix scripts\conpty\v209
```

`capture` writes only to the explicit output directory (the default is
`.tmp/conpty/v209-capture`). It never overwrites formal evidence. `verify` is a
pure read-only gate over `docs/reports/evidence/frames/v209-conpty` and checks
the evidence directory fingerprint before and after verification. The ignored
`node_modules`, capture output, and `.work` directories are removed only after
explicit cleanup confirmation.
