# Task 6 Fix Report: Codex Source Verification Wording

Status: DONE

## Changes

- Updated `README.md` so Current Capabilities says the project provides a boundary for verifying the local Codex CLI checkout shape.
- Rewrote `docs/extraction-status.md` Verified Codex Source wording to avoid hardcoding the machine-specific mojibake path.
- Kept the three expected checkout files documented as stable relative paths verified by the `CodexSource` boundary.

## Verification

- `git diff --check`: PASS

## Concerns

- None.
