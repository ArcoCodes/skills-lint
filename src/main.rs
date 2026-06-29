use clap::Parser;
use skills_lint::{
    Diagnostic, LintConfig, LintOptions, RULES, Severity, SkillResult, lint_skills_with_options,
};
use std::fs;
use std::io::IsTerminal;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

#[derive(Debug, Parser)]
#[command(
    name = "slint",
    about = "Lint Agent Skills directories for agentskills.io compatibility"
)]
struct Args {
    #[arg(default_value = ".")]
    path: PathBuf,

    #[arg(long)]
    config: Option<PathBuf>,

    #[arg(long)]
    json: bool,

    #[arg(long)]
    list_rules: bool,

    #[arg(long, value_delimiter = ',')]
    ignore: Vec<String>,

    #[arg(long, value_delimiter = ',')]
    select: Vec<String>,
}

fn main() -> ExitCode {
    let args = Args::parse();

    if args.list_rules {
        print_rules();
        return ExitCode::SUCCESS;
    }

    let options = match load_options(&args) {
        Ok(options) => options,
        Err(error) => {
            eprintln!("{error}");
            return ExitCode::from(2);
        }
    };

    if let Err(unknown) = options.validate() {
        eprintln!("Unknown rule ID(s): {}", unknown.join(", "));
        return ExitCode::from(2);
    }

    let result = lint_skills_with_options(&args.path, &options);

    if args.json {
        println!("{}", serde_json::to_string_pretty(&result).unwrap());
    } else {
        let printed = print_human(&result.skills);
        if printed {
            println!();
        }
        println!("{} error(s)", result.error_count);
    }

    if result.error_count > 0 {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}

fn print_human(skills: &[SkillResult]) -> bool {
    let color = Color::detect();
    let mut printed = false;

    for skill in skills {
        for diagnostic in &skill.diagnostics {
            if printed {
                println!();
            }
            print_diagnostic(skill, diagnostic, color);
            printed = true;
        }
    }

    printed
}

fn print_diagnostic(skill: &SkillResult, diagnostic: &Diagnostic, color: Color) {
    let severity = color.red("error");
    let rule_id = color.cyan(diagnostic.rule_id);
    let location = skill.skill_file.as_ref().unwrap_or(&skill.path);

    println!("{}({}): {}", severity, rule_id, diagnostic.message);
    println!(" {} {}", color.blue("-->"), location.display());
}

fn print_rules() {
    for rule in RULES.iter().filter(|rule| rule.severity == Severity::Error) {
        println!("  {}: {}", rule.id, rule.summary);
    }
}

fn load_options(args: &Args) -> Result<LintOptions, String> {
    let mut options = match config_path(args) {
        Some(path) => {
            let source = fs::read_to_string(&path)
                .map_err(|error| format!("Could not read config {}: {error}", path.display()))?;
            let config = toml::from_str::<LintConfig>(&source)
                .map_err(|error| format!("Could not parse config {}: {error}", path.display()))?;
            LintOptions::from_config(config)
        }
        None => LintOptions::default(),
    };

    options.merge(cli_options(args));
    Ok(options)
}

fn config_path(args: &Args) -> Option<PathBuf> {
    if let Some(path) = &args.config {
        return Some(path.clone());
    }

    ["slint.toml", ".slint.toml"]
        .into_iter()
        .map(Path::new)
        .find(|path| path.is_file())
        .map(Path::to_path_buf)
}

fn cli_options(args: &Args) -> LintOptions {
    LintOptions {
        ignore: args.ignore.iter().cloned().collect(),
        select: args.select.iter().cloned().collect(),
    }
}

#[derive(Clone, Copy)]
struct Color {
    enabled: bool,
}

impl Color {
    fn detect() -> Self {
        let forced = std::env::var_os("FORCE_COLOR").is_some();
        let disabled = std::env::var_os("NO_COLOR").is_some();

        Self {
            enabled: !disabled && (forced || std::io::stdout().is_terminal()),
        }
    }

    fn red(self, value: &str) -> String {
        self.paint("31;1", value)
    }

    fn cyan(self, value: &str) -> String {
        self.paint("36", value)
    }

    fn blue(self, value: &str) -> String {
        self.paint("34", value)
    }

    fn paint(self, code: &str, value: &str) -> String {
        if self.enabled {
            format!("\x1b[{code}m{value}\x1b[0m")
        } else {
            value.to_string()
        }
    }
}
