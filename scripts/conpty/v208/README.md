# v2.0.8 Windows ConPTY Evidence Collector

This collector launches the release `yunxi.exe` through the real Windows
ConPTY backend. It verifies v2.0.8 visual semantics without adding Node.js to
the YunXi runtime path.

## Scenarios

- `responsive-density`: runs a real DeepSeek conversation, captures 80x24,
  200x40, and 58x18 frames, and checks that header, transcript, and Composer
  remain ordered while low-priority diagnostics disappear on narrow screens.
- `semantic-low-color`: sets `NO_COLOR`, opens a real shell Approval, verifies
  the visible safe default, exercises a real Provider error, then cancels a
  real streaming turn. Xterm cell metadata proves foreground/background color
  is absent while text, bold, and reverse-video redundancy remains.

## Reproduce

From `D:\YunXi Agent`:

```powershell
cargo build -p yunxi-agent-cli --release --bins
npm ci --prefix scripts\conpty\v208
npm run capture --prefix scripts\conpty\v208
npm run verify --prefix scripts\conpty\v208
```

DeepSeek credentials are inherited by the child process and never persisted.
Evidence is redacted and written to
`docs/reports/evidence/frames/v208-conpty`. The ignored `node_modules` and
`.work` directories are removed only after explicit cleanup confirmation.
