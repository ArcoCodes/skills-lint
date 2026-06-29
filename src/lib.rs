use regex::Regex;
use serde::Serialize;
use serde_yaml::{Mapping, Value};
use std::collections::{BTreeSet, HashSet};
use std::fs;
use std::path::{Component, Path, PathBuf};
use walkdir::{DirEntry, WalkDir};

#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct LintResult {
    pub root: PathBuf,
    pub error_count: usize,
    pub warning_count: usize,
    pub skills: Vec<SkillResult>,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct SkillResult {
    pub path: PathBuf,
    pub skill_file: Option<PathBuf>,
    pub diagnostics: Vec<Diagnostic>,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct Diagnostic {
    pub severity: Severity,
    pub code: &'static str,
    pub message: String,
}

#[derive(Debug, Serialize, PartialEq, Eq, Clone, Copy)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Error,
    Warning,
}

pub fn lint_skills(root: impl AsRef<Path>) -> LintResult {
    let root = root.as_ref().to_path_buf();
    let mut skills = discover_skill_dirs(&root)
        .into_iter()
        .map(lint_skill_dir)
        .collect::<Vec<_>>();

    skills.sort_by(|left, right| left.path.cmp(&right.path));

    let error_count = skills
        .iter()
        .flat_map(|skill| &skill.diagnostics)
        .filter(|diagnostic| diagnostic.severity == Severity::Error)
        .count();
    let warning_count = skills
        .iter()
        .flat_map(|skill| &skill.diagnostics)
        .filter(|diagnostic| diagnostic.severity == Severity::Warning)
        .count();

    LintResult {
        root,
        error_count,
        warning_count,
        skills,
    }
}

pub fn lint_skill_dir(path: impl AsRef<Path>) -> SkillResult {
    let path = path.as_ref().to_path_buf();
    let skill_file = path.join("SKILL.md");

    if !skill_file.is_file() {
        return SkillResult {
            path,
            skill_file: None,
            diagnostics: vec![error(
                "missing-skill-md",
                "Skill directory must contain a SKILL.md file",
            )],
        };
    }

    let mut diagnostics = Vec::new();
    let source = match fs::read_to_string(&skill_file) {
        Ok(source) => source,
        Err(read_error) => {
            return SkillResult {
                path,
                skill_file: Some(skill_file),
                diagnostics: vec![error(
                    "read-error",
                    format!("Could not read SKILL.md: {read_error}"),
                )],
            };
        }
    };

    let parsed = parse_skill_markdown(&source);
    match parsed {
        Ok((frontmatter, body)) => {
            validate_frontmatter(&path, &frontmatter, &mut diagnostics);
            validate_body(&path, body, &mut diagnostics);
        }
        Err(message) => diagnostics.push(error("invalid-frontmatter", message)),
    }

    SkillResult {
        path,
        skill_file: Some(skill_file),
        diagnostics,
    }
}

fn discover_skill_dirs(root: &Path) -> Vec<PathBuf> {
    if root.is_file() {
        return root
            .file_name()
            .filter(|name| *name == "SKILL.md")
            .and_then(|_| root.parent())
            .map(Path::to_path_buf)
            .into_iter()
            .collect();
    }

    let mut candidates = BTreeSet::new();

    for entry in WalkDir::new(root)
        .follow_links(false)
        .into_iter()
        .filter_entry(should_descend)
        .filter_map(Result::ok)
    {
        let path = entry.path();
        if entry.file_type().is_file()
            && entry.file_name() == "SKILL.md"
            && let Some(parent) = path.parent()
        {
            candidates.insert(parent.to_path_buf());
        }

        if entry.file_type().is_dir() && entry.file_name() == "skills" {
            add_child_dirs(path, &mut candidates);
        }
    }

    if root.file_name().is_some_and(|name| name == "skills") {
        add_child_dirs(root, &mut candidates);
    }

    candidates.into_iter().collect()
}

fn should_descend(entry: &DirEntry) -> bool {
    let name = entry.file_name().to_string_lossy();
    !matches!(
        name.as_ref(),
        ".git" | "node_modules" | "target" | ".next" | "dist" | "build"
    )
}

fn add_child_dirs(path: &Path, candidates: &mut BTreeSet<PathBuf>) {
    if let Ok(entries) = fs::read_dir(path) {
        for entry in entries.filter_map(Result::ok) {
            let path = entry.path();
            if path.is_dir() && should_include_child(&path) {
                candidates.insert(path);
            }
        }
    }
}

fn should_include_child(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| !name.starts_with('.'))
}

fn parse_skill_markdown(source: &str) -> Result<(Mapping, &str), String> {
    if !source.starts_with("---\n") && source.trim_end() != "---" {
        return Err("SKILL.md must start with YAML frontmatter delimited by ---".to_string());
    }

    let mut lines = source.lines();
    if lines.next() != Some("---") {
        return Err("SKILL.md must start with YAML frontmatter delimited by ---".to_string());
    }

    let mut frontmatter = String::new();
    let mut consumed = 4usize;
    let mut closed = false;

    for line in lines {
        if line == "---" {
            closed = true;
            break;
        }
        consumed += line.len() + 1;
        frontmatter.push_str(line);
        frontmatter.push('\n');
    }

    if !closed {
        return Err("YAML frontmatter must be closed with ---".to_string());
    }

    let value: Value = serde_yaml::from_str(&frontmatter)
        .map_err(|error| format!("YAML frontmatter is invalid: {error}"))?;
    let Value::Mapping(mapping) = value else {
        return Err("YAML frontmatter must be a mapping".to_string());
    };

    let body = source.get(consumed..).unwrap_or_default();
    let body = body.strip_prefix("---\n").unwrap_or(body);
    Ok((mapping, body))
}

fn validate_frontmatter(path: &Path, frontmatter: &Mapping, diagnostics: &mut Vec<Diagnostic>) {
    let allowed = [
        "name",
        "description",
        "license",
        "allowed-tools",
        "metadata",
        "compatibility",
    ]
    .into_iter()
    .collect::<HashSet<_>>();

    for key in frontmatter.keys() {
        match key.as_str() {
            Some(key) if allowed.contains(key) => {}
            Some(key) => diagnostics.push(error(
                "unknown-field",
                format!("Unknown frontmatter field `{key}`"),
            )),
            None => diagnostics.push(error("unknown-field", "Frontmatter keys must be strings")),
        }
    }

    let name = get_string(frontmatter, "name");
    match name {
        Some(name) => validate_name(path, name, diagnostics),
        None => diagnostics.push(error("missing-name", "Required field `name` is missing")),
    }

    let description = get_string(frontmatter, "description");
    match description {
        Some(description) => validate_description(description, diagnostics),
        None => diagnostics.push(error(
            "missing-description",
            "Required field `description` is missing",
        )),
    }

    if has_key(frontmatter, "name") && name.is_none() {
        diagnostics.push(error("invalid-name", "`name` must be a string"));
    }

    if has_key(frontmatter, "description") && description.is_none() {
        diagnostics.push(error(
            "invalid-description",
            "`description` must be a string",
        ));
    }

    if let Some(compatibility) = get_string(frontmatter, "compatibility") {
        if compatibility.is_empty() || compatibility.chars().count() > 500 {
            diagnostics.push(error(
                "invalid-compatibility",
                "`compatibility` must be 1-500 characters",
            ));
        }
    } else if has_key(frontmatter, "compatibility") {
        diagnostics.push(error(
            "invalid-compatibility",
            "`compatibility` must be a string",
        ));
    }

    validate_metadata(frontmatter, diagnostics);
}

fn validate_name(path: &Path, name: &str, diagnostics: &mut Vec<Diagnostic>) {
    let name_pattern = Regex::new(r"^[a-z0-9](?:[a-z0-9-]{0,62}[a-z0-9])?$").unwrap();
    if name.is_empty() || name.chars().count() > 64 || !name_pattern.is_match(name) {
        diagnostics.push(error(
            "invalid-name",
            "`name` must be 1-64 characters, lowercase [a-z0-9-], and cannot use leading, trailing, or consecutive hyphens",
        ));
    }

    if name.contains("--") {
        diagnostics.push(error(
            "invalid-name",
            "`name` cannot contain consecutive hyphens",
        ));
    }

    if let Some(directory_name) = path.file_name().and_then(|name| name.to_str())
        && directory_name != name
    {
        diagnostics.push(error(
            "name-directory-mismatch",
            format!("`name` must match parent directory `{directory_name}`"),
        ));
    }
}

fn validate_description(description: &str, diagnostics: &mut Vec<Diagnostic>) {
    let length = description.chars().count();
    if length == 0 || length > 1024 {
        diagnostics.push(error(
            "invalid-description",
            "`description` must be 1-1024 characters",
        ));
    }
}

fn validate_metadata(frontmatter: &Mapping, diagnostics: &mut Vec<Diagnostic>) {
    let Some(metadata) = get_value(frontmatter, "metadata") else {
        return;
    };

    let Value::Mapping(metadata) = metadata else {
        diagnostics.push(warning(
            "invalid-metadata",
            "`metadata` should be a mapping of string keys to string values",
        ));
        return;
    };

    for (key, value) in metadata {
        if key.as_str().is_none() || value.as_str().is_none() {
            diagnostics.push(warning(
                "invalid-metadata",
                "`metadata` should be a mapping of string keys to string values",
            ));
            return;
        }
    }
}

fn validate_body(path: &Path, body: &str, diagnostics: &mut Vec<Diagnostic>) {
    let line_count = body.lines().count();
    if line_count > 500 {
        diagnostics.push(warning(
            "body-line-count",
            format!("SKILL.md body should stay under 500 lines, found {line_count}"),
        ));
    }

    let token_estimate = body.split_whitespace().count();
    if token_estimate > 5000 {
        diagnostics.push(warning(
            "body-token-estimate",
            format!(
                "SKILL.md body should stay under about 5000 tokens, estimated {token_estimate}"
            ),
        ));
    }

    for reference in find_relative_references(body) {
        validate_reference(path, &reference, diagnostics);
    }
}

fn find_relative_references(body: &str) -> Vec<String> {
    let markdown_link = Regex::new(r"\[[^\]]*]\(([^)]+)\)").unwrap();
    let angle_reference = Regex::new(r"<([A-Za-z0-9_./-]+\.[A-Za-z0-9][A-Za-z0-9_-]*)>").unwrap();
    let mut references = BTreeSet::new();

    for captures in markdown_link.captures_iter(body) {
        let target = captures
            .get(1)
            .map(|match_| match_.as_str())
            .unwrap_or_default();
        if let Some(reference) = normalize_reference(target) {
            references.insert(reference);
        }
    }

    for captures in angle_reference.captures_iter(body) {
        let target = captures
            .get(1)
            .map(|match_| match_.as_str())
            .unwrap_or_default();
        if let Some(reference) = normalize_reference(target) {
            references.insert(reference);
        }
    }

    references.into_iter().collect()
}

fn normalize_reference(target: &str) -> Option<String> {
    let target = target
        .split('#')
        .next()
        .unwrap_or_default()
        .split('?')
        .next()
        .unwrap_or_default()
        .trim();

    if target.is_empty()
        || target.starts_with('/')
        || target.starts_with('#')
        || target.starts_with("http://")
        || target.starts_with("https://")
        || target.starts_with("mailto:")
    {
        return None;
    }

    Some(target.to_string())
}

fn validate_reference(path: &Path, reference: &str, diagnostics: &mut Vec<Diagnostic>) {
    let depth = Path::new(reference)
        .components()
        .filter(|component| matches!(component, Component::Normal(_)))
        .count();

    if depth > 2 {
        diagnostics.push(warning(
            "reference-depth",
            format!(
                "Relative file reference `{reference}` should be at most one directory level deep"
            ),
        ));
    }

    if !path.join(reference).exists() {
        diagnostics.push(warning(
            "missing-reference",
            format!("Relative file reference `{reference}` does not exist"),
        ));
    }
}

fn get_string<'a>(mapping: &'a Mapping, key: &str) -> Option<&'a str> {
    get_value(mapping, key).and_then(Value::as_str)
}

fn get_value<'a>(mapping: &'a Mapping, key: &str) -> Option<&'a Value> {
    mapping.get(Value::String(key.to_string()))
}

fn has_key(mapping: &Mapping, key: &str) -> bool {
    mapping.contains_key(Value::String(key.to_string()))
}

fn error(code: &'static str, message: impl Into<String>) -> Diagnostic {
    Diagnostic {
        severity: Severity::Error,
        code,
        message: message.into(),
    }
}

fn warning(code: &'static str, message: impl Into<String>) -> Diagnostic {
    Diagnostic {
        severity: Severity::Warning,
        code,
        message: message.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn valid_skill_has_no_diagnostics() {
        let temp = TestDir::new();
        let skill = temp.path.join("skills").join("hello-world");
        fs::create_dir_all(skill.join("references")).unwrap();
        fs::write(skill.join("references").join("guide.md"), "details").unwrap();
        fs::write(
            skill.join("SKILL.md"),
            "---\nname: hello-world\ndescription: Greets the world\nmetadata:\n  owner: platform\n---\nSee [guide](references/guide.md).\n",
        )
        .unwrap();

        let result = lint_skills(temp.path.join("skills"));

        assert_eq!(result.error_count, 0);
        assert_eq!(result.warning_count, 0);
    }

    #[test]
    fn catches_frontmatter_errors() {
        let temp = TestDir::new();
        let skill = temp.path.join("bad-name");
        fs::create_dir_all(&skill).unwrap();
        fs::write(
            skill.join("SKILL.md"),
            "---\nname: Bad--Name\ndescription: \"\"\nextra: true\nmetadata:\n  owner: [team]\n---\n",
        )
        .unwrap();

        let result = lint_skill_dir(&skill);
        let codes = codes(&result);

        assert!(codes.contains(&"unknown-field"));
        assert!(codes.contains(&"invalid-name"));
        assert!(codes.contains(&"name-directory-mismatch"));
        assert!(codes.contains(&"invalid-description"));
        assert!(codes.contains(&"invalid-metadata"));
    }

    #[test]
    fn catches_missing_skill_md_in_skills_root_child() {
        let temp = TestDir::new();
        let skill = temp.path.join("skills").join("missing-file");
        fs::create_dir_all(&skill).unwrap();

        let result = lint_skills(temp.path.join("skills"));

        assert_eq!(result.error_count, 1);
        assert_eq!(result.skills[0].diagnostics[0].code, "missing-skill-md");
    }

    #[test]
    fn catches_reference_warnings() {
        let temp = TestDir::new();
        let skill = temp.path.join("reference-test");
        fs::create_dir_all(&skill).unwrap();
        fs::write(
            skill.join("SKILL.md"),
            "---\nname: reference-test\ndescription: Tests references\n---\nRead [deep](a/b/c.md) and <missing.md>.\n",
        )
        .unwrap();

        let result = lint_skill_dir(&skill);
        let codes = codes(&result);

        assert!(codes.contains(&"reference-depth"));
        assert!(codes.contains(&"missing-reference"));
    }

    fn codes(result: &SkillResult) -> HashSet<&'static str> {
        result
            .diagnostics
            .iter()
            .map(|diagnostic| diagnostic.code)
            .collect()
    }

    struct TestDir {
        path: PathBuf,
    }

    impl TestDir {
        fn new() -> Self {
            let id = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let path = std::env::temp_dir().join(format!("skills-lint-test-{id}"));
            fs::create_dir_all(&path).unwrap();
            Self { path }
        }
    }

    impl Drop for TestDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.path);
        }
    }
}
