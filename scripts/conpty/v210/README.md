# v2.1.0 Integrated Windows ConPTY Release Gate

This evidence-only gate binds the v2.0.1-v2.0.9 terminal behavior into one
v2.1.0 release regression. It runs the complete non-TUI mode matrix, normal and
Ctrl+C restoration, loopback live Provider recovery, long-stream cancellation,
and a 100x30 to 58x18 wide-character, mouse, copy-boundary, ANSI-reset smoke.
It also records the hashes of the Rust main/Details and VT100 goldens.

From `D:\YunXi Agent`:

```powershell
cargo build -p yunxi-agent-cli --release --bins
npm.cmd ci --prefix scripts\conpty\v210
npm.cmd run capture --prefix scripts\conpty\v210 -- --output-dir .tmp\conpty\v210-release
npm.cmd run verify --prefix scripts\conpty\v210 -- --input-dir .tmp\conpty\v210-release
```

`capture` writes only to an explicit `.tmp` path. Formal evidence is copied to
`docs/reports/evidence/frames/v210-conpty` only at release close. `verify` is
read-only and rejects any evidence or golden hash drift.
