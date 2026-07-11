# Scripts

## `install\install-yunxi.ps1`

Builds and installs the YunXi Agent v1.0 terminal binaries:

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
