# skills-lint

`slint` is a Rust CLI for linting Agent Skills in monorepos. It recursively scans for `skills` directories and `SKILL.md` files, validates each skill directory, and exits nonzero when errors are found.

## Usage

```sh
cargo run --bin slint -- ./skills
cargo run --bin slint -- --json ./skills
```

Options:

| Flag | Description |
| --- | --- |
| `--json` | Print machine-readable results |
| `--quiet`, `-q` | Suppress warnings in human output |

## Checks

Errors:

| Rule | Description |
| --- | --- |
| Missing `SKILL.md` | Skill directories must contain `SKILL.md` |
| Invalid frontmatter | `SKILL.md` must start with valid YAML frontmatter closed by `---` |
| Missing `name` | `name` is required |
| Name format | `name` must be 1-64 chars, lowercase `[a-z0-9-]`, with no leading, trailing, or consecutive hyphens |
| Name/directory mismatch | `name` must match the parent directory name |
| Missing `description` | `description` is required |
| Description length | `description` must be 1-1024 characters |
| Compatibility length | `compatibility`, when present, must be 1-500 characters |
| Unknown fields | Only `name`, `description`, `license`, `allowed-tools`, `metadata`, and `compatibility` are allowed |

Warnings:

| Rule | Description |
| --- | --- |
| Metadata format | `metadata` should be a mapping of string keys to string values |
| Body line count | `SKILL.md` body should stay under 500 lines |
| Body token estimate | `SKILL.md` body should stay under about 5000 whitespace-delimited tokens |
| Reference depth | Relative file references should be at most one directory level deep |
| Missing reference | Relative file references in the body should exist on disk |
