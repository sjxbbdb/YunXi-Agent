# Scripts

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

Builds and installs the current YunXi Agent v1.8.9 terminal binaries:

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
