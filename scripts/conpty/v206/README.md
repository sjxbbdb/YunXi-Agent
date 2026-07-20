# v2.0.6 Windows ConPTY Evidence Collector

This collector launches the release `yunxi.exe` through the real Windows
ConPTY backend exposed by `node-pty`. `@xterm/headless` interprets terminal
frames; it does not synthesize YunXi input or agent events.

## Prerequisites

- Windows with ConPTY support
- Node.js 20 or newer
- Rust toolchain used by this repository
- DeepSeek credentials already available through YunXi's normal local
  provider configuration or environment

Credential values are inherited by the child process. The collector never
prints or persists them. Saved frames are scanned and redacted for GitHub,
bearer, API-key, and `sk-` token shapes.

## Locked native dependency

`node-pty@1.1.0` contains the Windows native ConPTY binding. Its local install
script is explicitly allowed only for that pinned version in `package.json`.
`package-lock.json` also pins `@xterm/headless@5.5.0` and every transitive
archive hash. Installation remains below `scripts/conpty/v206/node_modules`;
it does not install a global package, modify `PATH`, or change the registry.

## Reproduce

From `D:\YunXi Agent`:

```powershell
cargo build -p yunxi-agent-cli --release --bins
npm ci --prefix scripts\conpty\v206
node -e "require('./scripts/conpty/v206/node_modules/node-pty'); console.log('node-pty binding ready')"
npm run capture --prefix scripts\conpty\v206
npm run verify --prefix scripts\conpty\v206
```

The capture runs eight independent, real DeepSeek sessions:

- ordinary composer input;
- bracketed multi-line paste;
- CRLF and lone-CR normalization;
- bounded 80x24 rendering of a paste longer than 1,800 characters;
- committed Chinese IME-style text, an emoji ZWJ sequence, and a combining
  grapheme;
- drafting during a streamed turn, Ctrl+C cancellation, restoration, and
  next-turn submission;
- drafting before a tool approval overlay, safe decline, restoration, and
  next-turn submission;
- streamed delta/final completion rendered as exactly one assistant cell.

Sanitized checkpoints and their SHA-256 manifest are written to:

```text
docs/reports/evidence/frames/v206-conpty
```

To diagnose one scenario:

```powershell
npm run capture:scenario --prefix scripts\conpty\v206 -- stream-cancel
```

Valid modes are `ordinary`, `multiline`, `crlf`, `long`, `ime`,
`stream-cancel`, `approval-restore`, and `final-single`.

After individually diagnosing and recapturing every required scenario,
`npm run manifest --prefix scripts\conpty\v206` rebuilds only the SHA-256
manifest without making another Provider request. The normal `capture` command
always recaptures all eight scenarios before rebuilding the manifest.

The ignored `scripts/conpty/v206/.work` directory contains only temporary
fixtures and isolated session state. The ignored `node_modules` directory
contains the locked local collector dependencies. `npm run verify` is offline:
it recomputes all evidence hashes, checks scenario-specific interaction
markers and metadata, and rejects secret-like values.
