# YunXi Agent v2.0.9 Windows ConPTY Resilience Evidence

- Collected at: `2026-07-21T12:54:19.874Z`
- Release binary: `target\release\yunxi.exe`, version `yunxi 2.0.9`
- Terminal: real Windows ConPTY through `node-pty@1.1.0`
- Terminal parser: `@xterm/headless@5.5.0`
- Collector: `scripts/conpty/v209/verify.js`
- Manifest SHA-256: `714b9c2d9c01b1616ffc8789e940ea778559335e20998679a904b5c335fcfdc8`

## Results

- Plain one-shot, piped interactive, CI, `--no-tui`, forced-TUI fallback,
  JSON, and JSONL all exited with code 0. The verifier rejected ANSI escape
  bytes, alternate-screen output, TUI footer text, malformed JSON/JSONL, and
  secret-like values on these paths.
- Normal `/exit` and idle Ctrl+C both exited with code 0 and emitted observable
  alternate-screen, bracketed-paste, mouse-capture, and cursor enter/leave
  sequences. Windows ConPTY did not echo `DisableFocusChange`; the Rust
  lifecycle recorder separately asserts that action and its exact reverse
  order for both complete and partial entry.
- A deterministic loopback Provider returned two HTTP 400 responses and then a
  successful response. The TUI showed `YX-PROVIDER-001`, returned to Composer,
  accepted the next prompt, displayed `SECOND_TURN_OK`, and exited cleanly.
- The streaming fixture delivered 371,375 SSE bytes. YunXi remained responsive,
  accepted Ctrl+C, displayed cancellation, issued a second request, displayed
  `RECOVERED_AFTER_CANCEL`, and exited with code 0.

## Artifacts

- `docs/reports/evidence/frames/v209-conpty/resilience.json`
- `docs/reports/evidence/frames/v209-conpty/manifest.json`

Node.js is used only for evidence collection. The default YunXi runtime remains
Rust-only and does not depend on this collector or its packages.
