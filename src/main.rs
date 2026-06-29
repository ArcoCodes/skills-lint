use clap::Parser;
use skills_lint::{Diagnostic, Severity, SkillResult, lint_skills};
use std::path::PathBuf;
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
    json: bool,

    #[arg(short, long)]
    quiet: bool,
}

fn main() -> ExitCode {
    let args = Args::parse();
    let result = lint_skills(&args.path);

    if args.json {
        println!("{}", serde_json::to_string_pretty(&result).unwrap());
    } else {
        print_human(&result.skills, args.quiet);
        println!(
            "{} error(s), {} warning(s)",
            result.error_count, result.warning_count
        );
    }

    if result.error_count > 0 {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}

fn print_human(skills: &[SkillResult], quiet: bool) {
    for skill in skills {
        let diagnostics = skill
            .diagnostics
            .iter()
            .filter(|diagnostic| !quiet || diagnostic.severity == Severity::Error)
            .collect::<Vec<_>>();

        if diagnostics.is_empty() {
            continue;
        }

        println!("{}", skill.path.display());
        for diagnostic in diagnostics {
            print_diagnostic(diagnostic);
        }
    }
}

fn print_diagnostic(diagnostic: &Diagnostic) {
    println!(
        "  {} {}: {}",
        match diagnostic.severity {
            Severity::Error => "error",
            Severity::Warning => "warning",
        },
        diagnostic.code,
        diagnostic.message
    );
}
