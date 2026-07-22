# Scripts

## `conpty\v210`

Contains the integrated Windows ConPTY release gate for v2.1.0. Capture writes
to an explicit `.tmp` directory; verification is read-only and binds the
non-TUI matrix, terminal recovery, Provider/stream recovery, wide-character
resize/mouse/copy smoke, and Rust golden hashes.

```powershell
npm.cmd ci --prefix scripts\conpty\v210
npm.cmd run capture --prefix scripts\conpty\v210 -- --output-dir .tmp\conpty\v210-release
npm.cmd run verify --prefix scripts\conpty\v210 -- --input-dir .tmp\conpty\v210-release
```

## `conpty\v209`

Contains the Windows ConPTY release gate for v2.0.9 cross-path mode isolation,
terminal lifecycle restoration, Provider failure recovery, and oversized
stream cancellation followed by a successful next turn.

```powershell
npm.cmd ci --prefix scripts\conpty\v209
npm.cmd run capture --prefix scripts\conpty\v209 -- --output-dir .tmp\conpty\v209-capture
npm.cmd run verify --prefix scripts\conpty\v209
```

Capture writes sanitized evidence and its SHA-256 manifest only to the explicit
`.tmp` directory. The formal `docs/reports/evidence/frames/v209-conpty` gate is
read-only. The loopback Provider fixtures reproduce deterministic HTTP/SSE
failures. Node.js remains an evidence dependency and is not part of the YunXi
runtime path.

## `conpty\v208`

Contains the real Windows ConPTY gate for the v2.0.8 semantic-style and
responsive information-density release. It captures completed DeepSeek
conversation frames at 80x24, 200x40, and 58x18, then runs a `NO_COLOR`
Approval/default-Decline, provider-error, and active-stream-cancel scenario.

```powershell
npm ci --prefix scripts\conpty\v208
npm run capture --prefix scripts\conpty\v208
npm run verify --prefix scripts\conpty\v208
```

Sanitized frames and the SHA-256 manifest are stored under
`docs/reports/evidence/frames/v208-conpty`. Node.js is used only by this
evidence collector and is not part of the YunXi runtime path.

## `conpty\v207-hotfix`

Contains the real Windows ConPTY gate for the v2.0.7-hotfix focus-routing
remediation. It verifies Approval wheel/scrollbar freeze and Details
wheel/PgDown/close behavior, including 58x18 responsive frames, against real
DeepSeek sessions.

```powershell
npm ci --prefix scripts\conpty\v207-hotfix
npm run capture --prefix scripts\conpty\v207-hotfix
npm run verify --prefix scripts\conpty\v207-hotfix
```

Sanitized frames and the SHA-256 manifest are stored under
`docs/reports/evidence/frames/v207-hotfix-conpty`.

## `conpty\v207`

Contains the reproducible Windows ConPTY gate for the v2.0.7 interaction-focus,
details-layer, and shortcut-consistency release. The locked collector preserves
the eight v2.0.6 composer/streaming regressions while verifying the final
v2.0.7 release binary against real DeepSeek sessions.

```powershell
npm ci --prefix scripts\conpty\v207
npm run capture --prefix scripts\conpty\v207
npm run verify --prefix scripts\conpty\v207
```

Sanitized frames and the SHA-256 manifest are stored under
`docs/reports/evidence/frames/v207-conpty`.

## `conpty\v206`

Contains the reproducible Windows ConPTY gate for the v2.0.6 Composer, input
recovery, and dialog-consistency release. Eight real DeepSeek sessions cover
ordinary, multi-line, CRLF, long, IME-style, stream-cancel,
approval-restore, and final-single behavior.

```powershell
npm ci --prefix scripts\conpty\v206
npm run capture --prefix scripts\conpty\v206
npm run verify --prefix scripts\conpty\v206
```

The collector writes sanitized frames and a SHA-256 manifest to
`docs/reports/evidence/frames/v206-conpty`. See
`scripts/conpty/v206/README.md` for locked dependency, credential, individual
scenario, and offline verification details.

## `conpty\v205`

Contains the reproducible Windows ConPTY evidence collector for the v2.0.5
error-presentation remediation. The collector uses locked `node-pty` and
`@xterm/headless` dependencies, launches the real release binary and DeepSeek
provider, and writes sanitized raw frames plus a SHA-256 manifest under
`docs/reports/evidence/frames/v205-conpty`.

```powershell
npm ci --prefix scripts\conpty\v205
node -e "require('./scripts/conpty/v205/node_modules/node-pty'); console.log('node-pty binding ready')"
npm run capture --prefix scripts\conpty\v205
npm run verify --prefix scripts\conpty\v205
```

The collector manifest grants project-local install-script permission only to
the locked `node-pty@1.1.0` native dependency. See
`scripts/conpty/v205/README.md` for the permission boundary, prerequisites,
and scenario details.

## `install\install-yunxi.ps1`

Builds and installs the current YunXi Agent v2.0.9 terminal binaries:

```powershell
.\scripts\install\install-yunxi.ps1 -AddToPath
```

The installer copies `yunxi.exe` and the compatibility
`yunxi-agent-cli.exe` into a user-local bin directory. It does not persist API
keys or require upstream Codex runtime dependencies.

## `link-codex-source.ps1`

Creates the ignored local source link used when refreshing the vendored Codex
Rust source:

```powershell
.\scripts\link-codex-source.ps1 -CodexCheckoutRoot "<path-to-codex-checkout>"
```

The script verifies that the checkout contains `codex-rs` and the core crates
needed by the YunXi live backend. Normal YunXi builds use `vendor/codex-rs` and
do not require this link.
