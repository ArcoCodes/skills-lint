# Repository Guidelines

## Project Structure & Module Organization

```text
.
├── src/
│   ├── lib.rs      # core linting logic and unit tests
│   └── main.rs     # `slint` CLI argument parsing and output
├── action.yml      # GitHub Action wrapper
├── dist-workspace.toml
│                  # cargo-dist release configuration
├── Cargo.toml      # Rust package and binary metadata
└── README.md       # user-facing usage and CI docs
```

## Build, Test, and Development Commands

- `cargo run --bin slint -- ./skills` runs the CLI against a local skills directory.
- `cargo run --bin slint -- --json .` prints machine-readable diagnostics.
- `cargo test` runs the Rust test suite.
- `cargo fmt` formats Rust source files.
- `cargo clippy --all-targets -- -D warnings` runs lint checks and treats warnings as errors.

## Release

Release artifacts are built by `dist` when a version tag is pushed:

```sh
git tag v0.1.0
git push origin v0.1.0
```
