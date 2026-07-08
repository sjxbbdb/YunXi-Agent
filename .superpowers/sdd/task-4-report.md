Status: DONE_WITH_CONCERNS

Implemented Task 4: Minimal CLI Over The Core.

Files changed:
- `crates/yunxi-agent-cli/Cargo.toml`
- `crates/yunxi-agent-cli/src/main.rs`
- `crates/yunxi-agent-cli/tests/cli_tests.rs`

Red step:
- Command: `cargo test -p yunxi-agent-cli cli_tests`
- Exit code: 1
- Output:

```text
cargo : The term 'cargo' is not recognized as the name of a cmdlet, function, script file, or operable program. Check t
he spelling of the name, or if a path was included, verify that the path is correct and try again.
At line:2 char:1
+ cargo test -p yunxi-agent-cli cli_tests
+ ~~~~~
    + CategoryInfo          : ObjectNotFound: (cargo:String) [], CommandNotFoundException
    + FullyQualifiedErrorId : CommandNotFoundException
```

Verification commands:

1. `cargo fmt`
   - Exit code: 1
   - Output:

```text
cargo : The term 'cargo' is not recognized as the name of a cmdlet, function, script file, or operable program. Check t
he spelling of the name, or if a path was included, verify that the path is correct and try again.
At line:2 char:1
+ cargo fmt
+ ~~~~~
    + CategoryInfo          : ObjectNotFound: (cargo:String) [], CommandNotFoundException
    + FullyQualifiedErrorId : CommandNotFoundException
```

2. `cargo test -p yunxi-agent-cli cli_tests`
   - Exit code: 1
   - Output:

```text
cargo : The term 'cargo' is not recognized as the name of a cmdlet, function, script file, or operable program. Check t
he spelling of the name, or if a path was included, verify that the path is correct and try again.
At line:2 char:1
+ cargo test -p yunxi-agent-cli cli_tests
+ ~~~~~
    + CategoryInfo          : ObjectNotFound: (cargo:String) [], CommandNotFoundException
    + FullyQualifiedErrorId : CommandNotFoundException
```

3. `cargo run -p yunxi-agent-cli -- "explain this project"`
   - Exit code: 1
   - Output:

```text
cargo : The term 'cargo' is not recognized as the name of a cmdlet, function, script file, or operable program. Check t
he spelling of the name, or if a path was included, verify that the path is correct and try again.
At line:2 char:1
+ cargo run -p yunxi-agent-cli -- "explain this project"
+ ~~~~~
    + CategoryInfo          : ObjectNotFound: (cargo:String) [], CommandNotFoundException
    + FullyQualifiedErrorId : CommandNotFoundException
```

4. `cargo run -p yunxi-agent-cli -- --json "explain this project"`
   - Exit code: 1
   - Output:

```text
cargo : The term 'cargo' is not recognized as the name of a cmdlet, function, script file, or operable program. Check t
he spelling of the name, or if a path was included, verify that the path is correct and try again.
At line:2 char:1
+ cargo run -p yunxi-agent-cli -- --json "explain this project"
+ ~~~~~
    + CategoryInfo          : ObjectNotFound: (cargo:String) [], CommandNotFoundException
    + FullyQualifiedErrorId : CommandNotFoundException
```

Concern:
- Current environment does not have `cargo` available in PATH, so formatting, tests, and manual smoke checks could not be executed successfully.
