# v2.0.7-hotfix Windows ConPTY Evidence Collector

This collector launches the release `yunxi.exe` through the real Windows
ConPTY backend exposed by `node-pty`. It exercises the focus-routing repair
with real terminal mouse input; `@xterm/headless` only interprets the frames.

## Prerequisites

- Windows with ConPTY support
- Node.js 20 or newer
- Rust toolchain used by this repository
- DeepSeek credentials available through YunXi's normal local provider configuration

Credential values are inherited by the child process. The collector never
prints or persists them. Saved frames are scanned and redacted for GitHub,
bearer, API-key, and `sk-` token shapes.

## Reproduce

From `D:\YunXi Agent`:

```powershell
cargo build -p yunxi-agent-cli --release --bins
npm ci --prefix scripts\conpty\v207-hotfix
node -e "require('./scripts/conpty/v207-hotfix/node_modules/node-pty'); console.log('node-pty binding ready')"
npm run capture --prefix scripts\conpty\v207-hotfix
npm run verify --prefix scripts\conpty\v207-hotfix
```

The capture opens two independent real DeepSeek/Windows ConPTY sessions:

- `approval-freeze`: creates a scrollable transcript, opens an Approval
  overlay, sends wheel, scrollbar click, drag, and mouse-up events, verifies
  the transcript frame is unchanged, and captures the 58x18 narrow approval
  footer before restoring the turn.
- `details-scroll`: exercises Composer and History focus hints, runs a safe
  local PowerShell fixture to create long debug details, opens Details, sends
  a real mouse wheel and PgDown, resizes to 58x18, then closes Details and
  verifies the transcript frame is restored.

Mouse input uses SGR sequences delivered through ConPTY. The collector does
not synthesize application state or bypass the approval dialog.

Evidence and a SHA-256 manifest are written to:

```text
docs/reports/evidence/frames/v207-hotfix-conpty
```

`npm run verify` is offline after capture. It checks metadata, checkpoint
coverage, focus-specific markers, unchanged/restored frame assertions, hashes,
and secret-like values. The ignored `.work` and `node_modules` directories
remain below this collector and are removed only after explicit cleanup
confirmation.
