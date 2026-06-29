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

| Rule ID | Description | Help |
| --- | --- | --- |
| `missing-skill-md` | Skill directories must contain `SKILL.md` | Add a `SKILL.md` file to the skill directory. |
| `read-error` | `SKILL.md` must be readable | Check the file permissions and make sure `SKILL.md` can be read. |
| `invalid-frontmatter` | `SKILL.md` must start with valid YAML frontmatter closed by `---` | Start `SKILL.md` with a YAML mapping delimited by opening and closing `---` lines. |
| `unknown-field` | Only `name`, `description`, `license`, `allowed-tools`, `metadata`, and `compatibility` are allowed | Remove unsupported fields or move custom data under `metadata`. |
| `missing-name` | `name` is required | Add a lowercase kebab-case `name` field to the frontmatter. |
| `invalid-name` | `name` must be 1-64 chars, lowercase `[a-z0-9-]`, with no leading, trailing, or consecutive hyphens | Use 1-64 lowercase letters, numbers, or single hyphens, with no leading or trailing hyphen. |
| `name-directory-mismatch` | `name` must match the parent directory name | Rename the skill directory or update the `name` field so they match exactly. |
| `missing-description` | `description` is required | Add a `description` field that explains when the skill should be used. |
| `invalid-description` | `description` must be a string and 1-1024 characters | Use a non-empty string description no longer than 1024 characters. |
| `invalid-compatibility` | `compatibility`, when present, must be a string and 1-500 characters | Use a non-empty compatibility string no longer than 500 characters, or remove the field. |
| `invalid-metadata` | `metadata` should be a mapping of string keys to string values | Use string keys and string values for metadata entries. |
| `body-line-count` | `SKILL.md` body should stay under 500 lines | Move detailed material into referenced files and keep `SKILL.md` focused. |
| `body-token-estimate` | `SKILL.md` body should stay under about 5000 whitespace-delimited tokens | Shorten `SKILL.md` or move long reference material into separate files. |
| `reference-depth` | Relative file references should be at most one directory level deep | Keep referenced files in the skill directory or one nested directory. |
| `missing-reference` | Relative file references in the body should exist on disk | Create the referenced file or update the link target. |
