# Scripts

## `link-codex-source.ps1`

Creates the ignored local source link used when refreshing the vendored Codex
Rust source:

```powershell
.\scripts\link-codex-source.ps1 -CodexCheckoutRoot "<path-to-codex-checkout>"
```

The script verifies that the checkout contains `codex-rs` and the core crates
needed by the YunXi live backend. Normal YunXi builds use `vendor/codex-rs` and
do not require this link.
