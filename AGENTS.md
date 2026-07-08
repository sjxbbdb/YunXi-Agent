# YunXi Agent Development Instructions

## CodeGraph

If `.codegraph/` exists at the repository root, use CodeGraph before grep/find when locating or understanding code.

## Scope

This repository contains an extracted, runnable Agent CLI and reusable core library based on the Codex CLI source checkout.

Keep first-stage work focused on:

- `yunxi-agent-core`
- `yunxi-agent-cli`
- Documentation needed to build, test, and push this project

Do not add TUI, desktop app, cloud tasks, SDK packaging, update, doctor, completion, marketplace, or full app-server product surfaces unless a later design explicitly includes them.

## Rust

- Use Rust 2024.
- Keep modules small and focused.
- Prefer explicit public facade types over leaking upstream Codex internals.
- Run `cargo fmt` after Rust edits.
- Run `cargo test` for this repository before claiming code is complete.
