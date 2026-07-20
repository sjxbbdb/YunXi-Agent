# v2.0.5 Windows ConPTY Evidence Collector

This collector launches the release `yunxi.exe` through the real Windows
ConPTY backend exposed by `node-pty`. `@xterm/headless` interprets terminal
frames; it does not synthesize YunXi events.

## Prerequisites

- Windows with ConPTY support
- Node.js 20 or newer
- Rust toolchain used by this repository
- DeepSeek credentials already available to YunXi through the normal local
  provider configuration or environment

Credential values are inherited by the child process. The collector never
prints or persists them. Saved frames are scanned and redacted for GitHub,
bearer, API-key, and `sk-` token shapes.

## Native dependency permission

`node-pty@1.1.0` contains the Windows native ConPTY binding and therefore has
an npm install script. This project explicitly grants that one version local
install-script permission in `package.json`:

```json
"allowScripts": {
  "node-pty@1.1.0": true
}
```

`npm ci` consumes this committed permission while installing the locked
dependencies below `scripts/conpty/v205/node_modules`. No separate
`npm approve-scripts` command should be required. Treat a pending or blocked
`node-pty` build as an installation failure instead of bypassing it with an
uncommitted local approval.

This permission is project-local and version-specific. It does not install a
global package or service, and it does not change `PATH`, the Windows registry,
or system configuration. On a machine without a usable prebuilt binding, npm
may invoke the local compiler toolchain to build the binding inside this
project directory. The exact package archive and transitive dependency hashes
remain pinned by `package-lock.json`.

## Reproduce

From `D:\YunXi Agent`:

```powershell
cargo build -p yunxi-agent-cli --release --bins
npm ci --prefix scripts\conpty\v205
node -e "require('./scripts/conpty/v205/node_modules/node-pty'); console.log('node-pty binding ready')"
npm run capture --prefix scripts\conpty\v205
npm run verify --prefix scripts\conpty\v205
```

The full run uses independent DeepSeek sessions for:

- responsive 80x24, 100x30, 120x40, and 200x50 frames;
- approval decline;
- approval approve;
- approval Ctrl+C cancellation;
- non-zero exit and next input;
- invalid UTF-8 plus `/details`;
- binary output plus `/details`;
- 2000-line output, bounded details, PageUp metadata, and next input.

Sanitized raw checkpoints and a SHA-256 manifest are written to:

```text
docs/reports/evidence/frames/v205-conpty
```

To rerun one scenario while diagnosing a failure:

```powershell
npm run capture:scenario --prefix scripts\conpty\v205 -- cancel
```

Valid modes are `responsive`, `decline`, `approve`, `cancel`, `nonzero`,
`invalid`, `binary`, and `long`.

The ignored `scripts/conpty/v205/.work` directory contains only temporary
fixture scripts and session state. Removing it does not remove the committed
collector, dependency lock, or sanitized evidence frames.

`npm run verify` is offline. It recomputes every frame file hash from the
manifest, checks required checkpoints and interaction markers, and rejects
secret-like token shapes.
