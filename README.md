# skills-lint

`slint` is a Rust CLI for linting Agent Skills in monorepos. It recursively scans for `skills` directories and `SKILL.md` files, validates each skill directory, and exits nonzero when errors are found.

## Install

Linux and macOS:

```sh
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/ArcoCodes/skills-lint/releases/latest/download/skills-lint-installer.sh | sh
```

Windows PowerShell:

```powershell
powershell -c "irm https://github.com/ArcoCodes/skills-lint/releases/latest/download/skills-lint-installer.ps1 | iex"
```

## CI

Use the GitHub Action to install a prebuilt release binary and fail the job when `slint` finds errors:

```yaml
name: Skills Lint

on: [push, pull_request]

jobs:
  skills-lint:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v7.0.0
      - uses: ArcoCodes/skills-lint@v0.1.0
        with:
          path: .                                      # Directory to lint (default: .)
          config:                                      # Path to a slint config file
          select:                                      # Comma-separated rule IDs to run
          ignore: body-line-count,body-token-estimate  # Comma-separated rule IDs to skip
          version: latest                              # Release tag to install, or "latest"
```

## Lint Rules

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
