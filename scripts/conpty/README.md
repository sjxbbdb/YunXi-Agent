# Windows ConPTY Gates

Each versioned directory is a locked, evidence-only Windows terminal gate.
The latest gate is `v209`, covering CLI path isolation, terminal restoration,
Provider recovery, and bounded-stream cancellation. Historical gates remain
unchanged for release rollback and regression verification.

Node.js dependencies are confined to these collectors. YunXi Agent's default
runtime and UI remain Rust-only.
