use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_yaml::{Mapping, Value};
use std::collections::{BTreeSet, HashSet};
use std::fs;
use std::path::{Component, Path, PathBuf};
use walkdir::{DirEntry, WalkDir};

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(default, deny_unknown_fields, rename_all = "kebab-case")]
pub struct LintConfig {
    pub ignore: Vec<String>,
    pub select: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LintOptions {
    pub ignore: BTreeSet<String>,
    pub select: BTreeSet<String>,
}

impl LintOptions {
    pub fn from_config(config: LintConfig) -> Self {
        Self {
            ignore: config.ignore.into_iter().collect(),
            select: config.select.into_iter().collect(),
        }
    }

    pub fn merge(&mut self, other: Self) {
        self.ignore.extend(other.ignore);
        self.select.extend(other.select);
    }

    pub fn validate(&self) -> Result<(), Vec<String>> {
        let valid = valid_rule_ids();
        let mut unknown = BTreeSet::new();

        for rule_id in self.ignore.iter().chain(&self.select) {
            if !valid.contains(rule_id.as_str()) {
                unknown.insert(rule_id.clone());
            }
        }

        if unknown.is_empty() {
            Ok(())
        } else {
            Err(unknown.into_iter().collect())
        }
    }

    fn includes(&self, diagnostic: &Diagnostic) -> bool {
        let rule_id = diagnostic.rule_id;

        let selected = self.select.is_empty() || self.select.contains(rule_id);
        let ignored = self.ignore.contains(rule_id);

        selected && !ignored
    }
}

#[derive(Debug, Serialize, PartialEq, Eq, Clone, Copy)]
pub struct Rule {
    pub id: &'static str,
    pub severity: Severity,
    pub summary: &'static str,
    pub help: &'static str,
}

pub const RULES: &[Rule] = &[
    Rule {
        id: "missing-skill-md",
        severity: Severity::Error,
        summary: "Skill directories must contain SKILL.md",
        help: "Add a SKILL.md file to the skill directory.",
    },
    Rule {
        id: "read-error",
        severity: Severity::Error,
        summary: "SKILL.md must be readable",
        help: "Check the file permissions and make sure SKILL.md can be read.",
    },
    Rule {
        id: "invalid-frontmatter",
        severity: Severity::Error,
        summary: "SKILL.md must start with valid YAML frontmatter closed by ---",
        help: "Start SKILL.md with a YAML mapping delimited by opening and closing --- lines.",
    },
    Rule {
        id: "unknown-field",
        severity: Severity::Error,
        summary: "Frontmatter can only use supported fields",
        help: "Remove unsupported fields or move custom data under metadata.",
    },
    Rule {
        id: "missing-name",
        severity: Severity::Error,
        summary: "`name` is required",
        help: "Add a lowercase kebab-case name field to the frontmatter.",
    },
    Rule {
        id: "invalid-name",
        severity: Severity::Error,
        summary: "`name` must be lowercase kebab-case and 1-64 characters",
        help: "Use 1-64 lowercase letters, numbers, or single hyphens, with no leading or trailing hyphen.",
    },
    Rule {
        id: "name-directory-mismatch",
        severity: Severity::Error,
        summary: "`name` must match the parent directory name",
        help: "Rename the skill directory or update the name field so they match exactly.",
    },
    Rule {
        id: "missing-description",
        severity: Severity::Error,
        summary: "`description` is required",
        help: "Add a description field that explains when the skill should be used.",
    },
    Rule {
        id: "invalid-description",
        severity: Severity::Error,
        summary: "`description` must be a string and 1-1024 characters",
        help: "Use a non-empty string description no longer than 1024 characters.",
    },
    Rule {
        id: "invalid-compatibility",
        severity: Severity::Error,
        summary: "`compatibility` must be a string and 1-500 characters",
        help: "Use a non-empty compatibility string no longer than 500 characters, or remove the field.",
    },
    Rule {
        id: "invalid-metadata",
        severity: Severity::Error,
        summary: "`metadata` should be a mapping of string keys to string values",
        help: "Use string keys and string values for metadata entries.",
    },
    Rule {
        id: "body-line-count",
        severity: Severity::Error,
        summary: "SKILL.md body should stay under 500 lines",
        help: "Move detailed material into referenced files and keep SKILL.md focused.",
    },
    Rule {
        id: "body-token-estimate",
        severity: Severity::Error,
        summary: "SKILL.md body should stay under about 5000 tokens",
        help: "Shorten SKILL.md or move long reference material into separate files.",
    },
    Rule {
        id: "reference-depth",
        severity: Severity::Error,
        summary: "Relative file references should be at most one directory level deep",
        help: "Keep referenced files in the skill directory or one nested directory.",
    },
    Rule {
        id: "missing-reference",
        severity: Severity::Error,
        summary: "Relative file references in the body should exist on disk",
        help: "Create the referenced file or update the link target.",
    },
];

#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct LintResult {
    pub root: PathBuf,
    pub error_count: usize,
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
    pub rule_id: &'static str,
    pub message: String,
    pub help: &'static str,
}

#[derive(Debug, Serialize, PartialEq, Eq, Clone, Copy)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Error,
}

pub fn lint_skills(root: impl AsRef<Path>) -> LintResult {
    lint_skills_with_options(root, &LintOptions::default())
}

pub fn lint_skills_with_options(root: impl AsRef<Path>, options: &LintOptions) -> LintResult {
    let root = root.as_ref().to_path_buf();
    let mut skills = discover_skill_dirs(&root)
        .into_iter()
        .map(|path| lint_skill_dir_with_options(path, options))
        .collect::<Vec<_>>();

    skills.sort_by(|left, right| left.path.cmp(&right.path));

    let error_count = skills
        .iter()
        .flat_map(|skill| &skill.diagnostics)
        .filter(|diagnostic| diagnostic.severity == Severity::Error)
        .count();
    LintResult {
        root,
        error_count,
        skills,
    }
}

pub fn lint_skill_dir(path: impl AsRef<Path>) -> SkillResult {
    lint_skill_dir_with_options(path, &LintOptions::default())
}

pub fn lint_skill_dir_with_options(path: impl AsRef<Path>, options: &LintOptions) -> SkillResult {
    let path = path.as_ref().to_path_buf();
    let skill_file = path.join("SKILL.md");

    if !skill_file.is_file() {
        let mut diagnostics = vec![error(
            "missing-skill-md",
            "Skill directory must contain a SKILL.md file",
        )];
        diagnostics.retain(|diagnostic| options.includes(diagnostic));
        return SkillResult {
            path,
            skill_file: None,
            diagnostics,
        };
    }

    let mut diagnostics = Vec::new();
    let source = match fs::read_to_string(&skill_file) {
        Ok(source) => source,
        Err(read_error) => {
            let mut diagnostics = vec![error(
                "read-error",
                format!("Could not read SKILL.md: {read_error}"),
            )];
            diagnostics.retain(|diagnostic| options.includes(diagnostic));
            return SkillResult {
                path,
                skill_file: Some(skill_file),
                diagnostics,
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

    diagnostics.retain(|diagnostic| options.includes(diagnostic));

    SkillResult {
        path,
        skill_file: Some(skill_file),
        diagnostics,
    }
}

pub fn valid_rule_ids() -> BTreeSet<&'static str> {
    RULES.iter().map(|rule| rule.id).collect()
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
        diagnostics.push(error(
            "invalid-metadata",
            "`metadata` should be a mapping of string keys to string values",
        ));
        return;
    };

    for (key, value) in metadata {
        if key.as_str().is_none() || value.as_str().is_none() {
            diagnostics.push(error(
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
        diagnostics.push(error(
            "body-line-count",
            format!("SKILL.md body should stay under 500 lines, found {line_count}"),
        ));
    }

    let token_estimate = body.split_whitespace().count();
    if token_estimate > 5000 {
        diagnostics.push(error(
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
        diagnostics.push(error(
            "reference-depth",
            format!(
                "Relative file reference `{reference}` should be at most one directory level deep"
            ),
        ));
    }

    if !path.join(reference).exists() {
        diagnostics.push(error(
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
        rule_id: code,
        message: message.into(),
        help: rule_help(code),
    }
}

fn rule_help(rule_id: &str) -> &'static str {
    RULES
        .iter()
        .find(|rule| rule.id == rule_id)
        .map(|rule| rule.help)
        .unwrap_or("Fix the reported issue and run slint again.")
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
        assert_eq!(result.skills[0].diagnostics[0].rule_id, "missing-skill-md");
    }

    #[test]
    fn catches_reference_errors() {
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

    #[test]
    fn filters_with_ignore_and_select_rules() {
        let temp = TestDir::new();
        let skill = temp.path.join("filtered");
        fs::create_dir_all(&skill).unwrap();
        fs::write(
            skill.join("SKILL.md"),
            "---\nname: Filtered\ndescription: \"\"\nextra: true\n---\n",
        )
        .unwrap();

        let mut options = LintOptions::default();
        options.select.insert("invalid-name".to_string());
        options.ignore.insert("invalid-name".to_string());

        let result = lint_skill_dir_with_options(&skill, &options);

        assert!(result.diagnostics.is_empty());
    }

    #[test]
    fn filters_with_select_rules() {
        let temp = TestDir::new();
        let skill = temp.path.join("reference-test");
        fs::create_dir_all(&skill).unwrap();
        fs::write(
            skill.join("SKILL.md"),
            "---\nname: Wrong\ndescription: Tests references\n---\nRead [deep](a/b/c.md).\n",
        )
        .unwrap();

        let mut options = LintOptions::default();
        options.select.insert("invalid-name".to_string());
        options.select.insert("reference-depth".to_string());

        let result = lint_skill_dir_with_options(&skill, &options);
        let codes = codes(&result);

        assert_eq!(codes.len(), 2);
        assert!(codes.contains(&"invalid-name"));
        assert!(codes.contains(&"reference-depth"));
    }

    #[test]
    fn validates_unknown_rule_ids() {
        let mut options = LintOptions::default();
        options.ignore.insert("not-a-rule".to_string());

        assert_eq!(options.validate(), Err(vec!["not-a-rule".to_string()]));
    }

    fn codes(result: &SkillResult) -> HashSet<&'static str> {
        result
            .diagnostics
            .iter()
            .map(|diagnostic| diagnostic.rule_id)
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
