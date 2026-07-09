# Local Patches

This directory records YunXi-local changes applied to the vendored Codex Rust
source.

## Windows PTY raw handle cast

- File: `utils/pty/src/win/conpty.rs`
- Reason: the Windows build needs the pseudoconsole handle expression to match
  `std::os::windows::io::RawHandle`.
- Change:

```rust
self.con.raw_handle() as RawHandle
```

This change was present in the local `external/codex-rs` checkout when the
2026-07-10 vendored snapshot was imported. Reapply it after future upstream
refreshes if upstream has not adopted an equivalent fix.
