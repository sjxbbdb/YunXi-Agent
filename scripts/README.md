# Scripts

## link-codex-source.ps1

Creates the ignored local source link used by stage-2 Codex path dependencies:

```powershell
.\scripts\link-codex-source.ps1 -CodexCheckoutRoot "<path-to-codex-checkout>"
```

The script verifies that the checkout contains `codex-rs` and the core crates
needed by the YunXi live backend.
