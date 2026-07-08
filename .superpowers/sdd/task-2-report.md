Status: DONE_WITH_CONCERNS

Task: Task 2: Core Library Facade Types

Files changed:
- crates/yunxi-agent-core/Cargo.toml
- crates/yunxi-agent-core/src/lib.rs
- crates/yunxi-agent-core/src/config.rs
- crates/yunxi-agent-core/src/input.rs
- crates/yunxi-agent-core/src/event.rs
- crates/yunxi-agent-core/src/error.rs
- crates/yunxi-agent-core/tests/config_tests.rs

Implementation summary:
- Added yunxi-agent-core crate manifest using workspace package metadata and workspace dependencies.
- Added public facade exports for config, input, event, result, status, error, and AgentResult.
- Added AgentConfig with default approval/sandbox modes and builder methods.
- Added AgentInput::text.
- Added AgentEvent, AgentRunStatus, and AgentRunResult.
- Added AgentError and AgentResult.
- Added config tests exactly as specified in the brief.

Commands run:
- cargo test -p yunxi-agent-core config_tests
  - Exit code: 1
  - Output: cargo : The term 'cargo' is not recognized as the name of a cmdlet, function, script file, or operable program.
- cargo fmt
  - Exit code: 1
  - Output: cargo : The term 'cargo' is not recognized as the name of a cmdlet, function, script file, or operable program.
- cargo test -p yunxi-agent-core config_tests
  - Exit code: 1
  - Output: cargo : The term 'cargo' is not recognized as the name of a cmdlet, function, script file, or operable program.

Concerns:
- The current environment does not have cargo available in PATH, so formatting and tests could not be verified here.
- The brief expected the first test run to fail because the crate did not exist, but the actual failure was earlier: cargo was not found.
- An unrelated existing modification to .superpowers/sdd/progress.md was present before this task and was not touched.
