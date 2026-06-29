# skills-lint

`slint` is a Rust CLI for linting Agent Skills in monorepos. It recursively scans for `skills` directories and `SKILL.md` files, validates each skill directory, and exits nonzero when errors are found.

## Usage

```sh
cargo run --bin slint -- ./skills
cargo run --bin slint -- --json ./skills
cargo run --bin slint -- --select missing-name,invalid-name ./skills
cargo run --bin slint -- --ignore body-line-count ./skills
```

Options:

| Flag | Description |
| --- | --- |
| `--config <PATH>` | Read configuration from a TOML file |
| `--json` | Print machine-readable results |
| `--list-rules` | Print all rule IDs |
| `--select <ID>` | Only run matching rule IDs |
| `--ignore <ID>` | Ignore matching rule IDs |

Rule filters accept repeated flags or comma-separated values.

## Configuration

`slint` automatically reads `slint.toml` or `.slint.toml` from the current directory. Use `--config <PATH>` to choose a specific file.

```toml
select = ["missing-name", "invalid-name"]
ignore = ["body-line-count"]
```

Command-line filters are merged with configuration file filters. Unknown rule IDs are reported before linting starts.

## Checks

| Rule ID | Description |
| --- | --- |
| `missing-skill-md` | Skill directories must contain `SKILL.md` |
| `read-error` | `SKILL.md` must be readable |
| `invalid-frontmatter` | `SKILL.md` must start with valid YAML frontmatter closed by `---` |
| `unknown-field` | Only `name`, `description`, `license`, `allowed-tools`, `metadata`, and `compatibility` are allowed |
| `missing-name` | `name` is required |
| `invalid-name` | `name` must be 1-64 chars, lowercase `[a-z0-9-]`, with no leading, trailing, or consecutive hyphens |
| `name-directory-mismatch` | `name` must match the parent directory name |
| `missing-description` | `description` is required |
| `invalid-description` | `description` must be a string and 1-1024 characters |
| `invalid-compatibility` | `compatibility`, when present, must be a string and 1-500 characters |
| `invalid-metadata` | `metadata` should be a mapping of string keys to string values |
| `body-line-count` | `SKILL.md` body should stay under 500 lines |
| `body-token-estimate` | `SKILL.md` body should stay under about 5000 whitespace-delimited tokens |
| `reference-depth` | Relative file references should be at most one directory level deep |
| `missing-reference` | Relative file references in the body should exist on disk |
