# YunXi Agent v2.0.8 Visual Semantics ConPTY Evidence

- Captured: 2026-07-21 17:29:03 to 17:29:16 +08:00
- Binary: `D:\YunXi Agent\target\release\yunxi.exe`
- Version: `yunxi 2.0.8`
- Terminal: real Windows ConPTY through locked `node-pty@1.1.0`
- Provider/model: DeepSeek / `deepseek-chat`
- Collector: `D:\YunXi Agent\scripts\conpty\v208`
- Evidence: `D:\YunXi Agent\docs\reports\evidence\frames\v208-conpty`

## Responsive Density

The collector waited for a completed `[assistant]` response before capturing
80x24, 200x40, and 58x18 frames. Every frame keeps header, transcript, and
Composer in order with an idle `Enter submit` action. The 58-column frame drops
backend, source, and debug diagnostics while retaining product, provider,
conversation, and Composer state.

- Output bytes: 27,165
- Checkpoints: 7
- SHA-256: `00d8620ae74cb3992c2a39918cde7c343fbdbed203e6312ba559c83037bff04b`

## Semantic Low Color

The collector set `NO_COLOR=1`, obtained a real shell Approval, selected the
safe-default Decline path, forced a real invalid-model provider error, restored
`deepseek-chat`, waited for a visible active `[assistant*]` stream, and sent
Ctrl+C. Xterm cell metadata at Approval records zero colored cells and zero
background cells while reverse-video selection and explicit Approval/risk/
safe-default text remain visible.

- Output bytes: 28,503
- Checkpoints: 14
- SHA-256: `3c0159861774cbc16d101a9dabe052c2037cc4d7ce2992f0b9ae3c99e8911063`

## Verification

`npm.cmd run verify --prefix scripts\conpty\v208` independently validates
manifest metadata, file hashes, required checkpoints, responsive dimensions,
diagnostic clipping, monochrome cell metadata, Approval redundancy, Provider
error, cancellation, and secret-pattern absence. Historical
`scripts\conpty\v207-hotfix` and `scripts\conpty\v207` verifiers also pass.

DeepSeek credentials are inherited by the child process and are not persisted
in evidence, logs, scripts, or the manifest.
