# YunXi Agent v1.8.4 English Memory Preference Development Report

## Goal

YunXi Agent v1.8.4 fixes two v1.8.3 audit findings in the persona and transparent memory layer:

- P1: local rule extraction recognized Chinese language preferences but did not recognize English language preferences in offline/rule-only mode.
- P2: after `promote_incoming` or `combine_non_conflicting`, the single `source_session_id` audit field could stay attached to the old record even when the final content came from the incoming record.

This version stays inside the v1.8 persona/memory foundation. It does not add SQLite, vector search, graph memory, relationship state machines, TUI memory management UI, or an external memory runtime.

## Constraints

- Build the full source slice first.
- Do not run mid-construction validation loops.
- Run unified verification only after source construction is complete.
- Preserve v1.8.3 merge fidelity: generic language preferences must not overwrite richer existing memory.
- Low-risk language preference candidates can be auto-saved.
- Conflicting language preferences must go to pending review.
- Memory extraction/write failure must not block the main final response.
- Write the desktop development log at task end.
- Clean Rust build artifacts before release completion.
- Publish by GitHub REST API only.
- Create a new immutable annotated `v1.8.4` tag. Do not delete or move old tags.

## Construction Scope

Modified source:

- `crates/yunxi-agent-persona/src/extractor.rs`
- `crates/yunxi-agent-persona/src/merge.rs`

Modified tests:

- `crates/yunxi-agent-persona/tests/extractor_tests.rs`
- `crates/yunxi-agent-storage/tests/storage_tests.rs`
- `crates/yunxi-agent-cli/tests/cli_tests.rs`

Modified version/documentation surfaces:

- `Cargo.toml`
- `Cargo.lock`
- `crates/yunxi-agent-cli/src/main.rs`
- `crates/yunxi-agent-cli/src/render.rs`
- `crates/yunxi-agent-cli/tests/cli_tests.rs`
- `crates/yunxi-agent-persona/src/compiler.rs`
- `crates/yunxi-agent-persona/src/profile.rs`
- `crates/yunxi-agent-tui/src/app.rs`
- `crates/yunxi-agent-tui/src/render.rs`
- `docs/persona-memory.md`
- `docs/extraction-status.md`

## Implemented Behavior

English language preference rule extraction now recognizes:

- `以后请用英文回答`
- `以后请用英语回答`
- `Please answer in English from now on`
- `reply in English from now on`
- `use English by default`
- direct forms such as `answer in English`, `reply in English`, `respond in English`, `use English`, and `English please`

These candidates normalize to:

```text
global_user|preference|language:en
```

Chinese and English language preferences remain separate dedup slots but share the same language conflict family, so an active Chinese preference followed by an English preference is routed to pending review instead of being ignored or silently overwriting the active record.

Merge source audit behavior is now strategy-aware:

- `promote_incoming`: prefer incoming `source_session_id` because incoming content becomes the final durable content.
- `preserve_existing`: prefer existing `source_session_id`.
- `combine_non_conflicting`: prefer existing `source_session_id` until a future schema can represent multi-source provenance.
- `conflict_requires_confirmation`: prefer existing `source_session_id`.

## Tests Added

Persona extractor tests:

- English preference routes to `GlobalUser`, active status, and `language:en`.
- Equivalent English preference expressions deduplicate to one candidate.
- Chinese and English language preferences keep distinct dedup keys.
- `promote_incoming` uses incoming source session.
- `preserve_existing` keeps existing source session.

Storage tests:

- Promoted rich content tracks incoming source session through `FilePersonaMemoryStore::append_or_merge`.

CLI tests:

- `--offline --memory-extraction rule-only "以后请用英文回答"` writes active global `language:en`.
- Existing active Chinese preference plus English preference emits `pending_confirmation`, `conflict_requires_confirmation`, and a pending `language:en` record.

## Unified Verification Plan

After construction, run:

```powershell
cargo fmt
cargo fmt --check
cargo test -p yunxi-agent-persona -p yunxi-agent-storage -p yunxi-agent-runtime -p yunxi-agent-cli -p yunxi-agent-tui
cargo test
cargo check --workspace
cargo build -p yunxi-agent-cli --release --bins
target\release\yunxi.exe --version
target\release\yunxi-agent-cli.exe --version
git diff --check
```

Black-box validation with isolated `YUNXI_HOME`:

- English preference writes `language:en` through debug/release CLI.
- Chinese active then English preference routes English to pending.
- JSONL exposes `memory_candidate`, `memory_write`, `pending_confirmation`, and `conflict_requires_confirmation` without leaking secrets.

Release completion:

- `codegraph sync .`
- `codegraph status .`
- install refresh with `scripts\install\install-yunxi.ps1 -AddToPath -SkipBuild`
- PATH smoke for `yunxi --version` and `yunxi-agent-cli --version`
- owned-source secret scan
- `cargo clean`
- local commit
- GitHub REST API publish to `master`
- GitHub REST API annotated tag `v1.8.4`
- GitHub REST API verification of remote `master`, tag ref, and tag target

## Unified Verification Results

Unified verification was performed after source construction, following the
project hard constraint.

Verified in this slice:

- `cargo fmt`: pass
- `cargo fmt --check`: pass
- `cargo check --workspace`: pass
- targeted package tests for persona/storage/runtime/cli/tui: pass
- `cargo test`: pass; workspace unit tests, integration tests, and doc tests
  completed with zero failures
- `cargo build -p yunxi-agent-cli --release --bins`: pass
- `target\release\yunxi.exe --version`: pass; `yunxi 1.8.4`
- `target\release\yunxi-agent-cli.exe --version`: pass; `yunxi 1.8.4`
- isolated `YUNXI_HOME` black-box English memory write: pass; JSONL emitted
  `memory_candidate` and `memory_write action=auto_saved`, and global memory
  contained `global_user|preference|language:en`
- isolated `YUNXI_HOME` black-box language conflict: pass; English preference
  after active Chinese preference emitted `pending_confirmation` with
  `merge_strategy=conflict_requires_confirmation`, pending memory contained
  `global_user|preference|language:en`, and active
  `global_user|preference|language:zh` was preserved
- `git diff --check`: pass
- owned-source secret scan excluding `target`, `vendor`, `extracted`, `.git`,
  `.codegraph`, and `Cargo.lock`: pass; no live key-shaped matches
- `codegraph sync .`: pass; 11 changed Rust files synced
- `codegraph status .`: pass; index is up to date
- `scripts\install\install-yunxi.ps1 -AddToPath -SkipBuild`: pass
- PATH smoke: pass; `yunxi --version` and `yunxi-agent-cli --version` both
  returned `yunxi 1.8.4`

## Completion Definition

v1.8.4 is complete only when:

- English preference extraction works in offline/rule-only mode.
- English preferences persist as `global_user|preference|language:en`.
- Chinese/English language conflicts route to pending review.
- `promote_incoming` source audit points at incoming session.
- v1.8.3 memory merge fidelity tests still pass.
- Full unified verification passes.
- CodeGraph is up to date.
- Desktop development log is updated.
- Build artifacts are cleaned.
- GitHub REST API publish succeeds and `v1.8.4` is available as a rollback tag.
