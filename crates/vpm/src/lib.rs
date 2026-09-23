//! VPM project workflow and package metadata foundation.
#![allow(clippy::missing_errors_doc)]
mod artifact;
pub mod compatibility;
mod global;
mod lockfile;
mod manifest;
mod messages;
mod native;
mod native_plan;
mod package_validation;
mod project;
mod provider;
mod publish;
mod resolver;
mod source;
mod store;
mod testing;
mod tooling;
mod version;

use std::path::Path;

use clap::{Parser, Subcommand};
pub use manifest::{Dependency, Manifest};
use messages::{ColorMode, Style};
pub use project::{
    AddReport, AnalysisSourceRoot, InstallReport, add, analysis_source_roots, build, check, init,
    install, new, remove, update,
};
pub use provider::{
    GitHubProvider, GitLabProvider, PackageProvider, PackageSnapshot, ProviderError,
};
pub use resolver::{PackageId, ResolvedGraph, Resolver};
pub use source::{PackageSource, ProviderKind};

#[derive(Parser)]
#[command(name = "vpm", version, about = "Vut project and package manager")]
struct Cli {
    /// Suppress all standard output; errors and warnings still go to stderr.
    #[arg(short, long, global = true)]
    quiet: bool,
    /// Colour policy for status, warnings, and errors.
    #[arg(long, value_enum, default_value = "auto", global = true)]
    color: ColorMode,
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    New {
        name: String,
    },
    Init,
    Add {
        package: String,
    },
    Remove {
        package: String,
    },
    Install {
        /// Install a published CLI package globally when present; otherwise
        /// install the current project's dependencies.
        package: Option<String>,
    },
    Uninstall {
        package: String,
    },
    Exec {
        package: String,
        #[arg(long)]
        bin: Option<String>,
        #[arg(last = true)]
        args: Vec<String>,
    },
    Publish {
        /// Publication target; defaults to the official registry.
        registry: Option<String>,
        #[arg(long)]
        dry_run: bool,
    },
    Update,
    Build {
        #[arg(long)]
        release: bool,
    },
    Check,
    Test {
        filter: Option<String>,
        #[arg(long)]
        release: bool,
    },
    Fmt {
        #[arg(long)]
        check: bool,
    },
    Lint,
    Doc,
    Clean,
    Tree,
    Outdated,
    Search {
        query: String,
    },
    Info {
        package: String,
    },
    Run {
        #[arg(last = true)]
        args: Vec<String>,
        #[arg(long)]
        release: bool,
    },
}

/// A command outcome that has already been reported to the user (for example
/// lint warnings) or that wraps a failure to be printed by the CLI boundary.
enum CliFailure {
    Presented,
    Error(Box<dyn std::error::Error>),
}

impl From<Box<dyn std::error::Error>> for CliFailure {
    fn from(error: Box<dyn std::error::Error>) -> Self {
        Self::Error(error)
    }
}

impl From<std::io::Error> for CliFailure {
    fn from(error: std::io::Error) -> Self {
        Self::Error(Box::new(error))
    }
}

fn emit(quiet: bool, line: &str) {
    if !quiet {
        println!("{line}");
    }
}

#[must_use]
pub fn run_cli() -> i32 {
    let cli = Cli::parse();
    let style = Style::new(cli.color);
    match execute(&cli, style) {
        Ok(code) => code,
        Err(CliFailure::Presented) => 1,
        Err(CliFailure::Error(error)) => {
            eprintln!("{}", messages::error_line(style, &error.to_string()));
            1
        }
    }
}

fn execute(cli: &Cli, style: Style) -> Result<i32, CliFailure> {
    let quiet = cli.quiet;
    let cwd = std::env::current_dir()?;
    match &cli.command {
        Command::New { .. }
        | Command::Init
        | Command::Add { .. }
        | Command::Remove { .. }
        | Command::Update
        | Command::Install { package: None } => setup(&cli.command, &cwd, style, quiet),
        Command::Install { package: Some(_) }
        | Command::Uninstall { .. }
        | Command::Exec { .. } => global_commands(&cli.command, style, quiet),
        Command::Publish { .. } => publish_command(&cli.command, &cwd, style, quiet),
        Command::Build { .. } | Command::Check | Command::Test { .. } | Command::Run { .. } => {
            build_commands(&cli.command, &cwd, style, quiet)
        }
        Command::Fmt { .. }
        | Command::Lint
        | Command::Doc
        | Command::Clean
        | Command::Tree
        | Command::Outdated
        | Command::Search { .. }
        | Command::Info { .. } => tooling_commands(&cli.command, &cwd, style, quiet),
    }
}

/// Project lifecycle commands: `new`, `init`, `add`, `remove`, `install`, `update`.
fn setup(command: &Command, cwd: &Path, style: Style, quiet: bool) -> Result<i32, CliFailure> {
    match command {
        Command::New { name } => {
            let root = cwd.join(name);
            new(&root, name)?;
            emit(
                quiet,
                &messages::created(style, name, &root.display().to_string()),
            );
        }
        Command::Init => {
            let name = init(cwd)?;
            emit(
                quiet,
                &messages::initialized(style, &name, &cwd.display().to_string()),
            );
        }
        Command::Add { package } => {
            let report = add(cwd, package)?;
            emit(
                quiet,
                &messages::added(style, &report.import_name, &report.version, &report.source),
            );
        }
        Command::Remove { package } => {
            remove(cwd, package)?;
            emit(quiet, &messages::removed(style, package));
        }
        Command::Install { package: None } => {
            let report = install(cwd)?;
            emit(quiet, &install_line(style, &report));
        }
        Command::Update => {
            let report = update(cwd)?;
            let line = if report.up_to_date {
                messages::already_up_to_date(style, report.total)
            } else {
                messages::updated(style, report.installed)
            };
            emit(quiet, &line);
        }
        _ => unreachable!("setup only handles project lifecycle commands"),
    }
    Ok(0)
}

fn install_line(style: Style, report: &InstallReport) -> String {
    if report.up_to_date {
        messages::already_up_to_date(style, report.total)
    } else {
        messages::installed(style, report.installed)
    }
}

/// Global package commands: `install <package>`, `uninstall`, `exec`.
fn global_commands(command: &Command, style: Style, quiet: bool) -> Result<i32, CliFailure> {
    match command {
        Command::Install {
            package: Some(package),
        } => {
            let report = global::install(package)?;
            emit(
                quiet,
                &messages::global_installed(style, &report.package, &report.bins),
            );
        }
        Command::Uninstall { package } => {
            global::uninstall(package)?;
            emit(quiet, &messages::uninstalled(style, package));
        }
        Command::Exec { package, bin, args } => {
            let code = global::exec(package, bin.as_deref(), args)?;
            if code != 0 {
                return Ok(code);
            }
        }
        _ => unreachable!("global_commands only handles global package commands"),
    }
    Ok(0)
}

/// Publication command: `publish [registry] [--dry-run]`.
fn publish_command(
    command: &Command,
    root: &Path,
    style: Style,
    quiet: bool,
) -> Result<i32, CliFailure> {
    let Command::Publish { registry, dry_run } = command else {
        unreachable!("publish_command only handles `publish`");
    };
    let report = publish::publish(root, registry.as_deref(), *dry_run)?;
    emit(
        quiet,
        &messages::published(
            style,
            &report.package,
            &report.version,
            &report.registry,
            report.files,
            report.dry_run,
        ),
    );
    Ok(0)
}

/// Compiler-backed commands: `build`, `check`, `test`, `run`.
fn build_commands(
    command: &Command,
    cwd: &Path,
    style: Style,
    quiet: bool,
) -> Result<i32, CliFailure> {
    match command {
        Command::Build { release } => {
            let path = build(cwd, *release)?;
            emit(
                quiet,
                &messages::built(style, &path.display().to_string(), *release),
            );
        }
        Command::Check => {
            let name = check(cwd)?;
            emit(quiet, &messages::checked(style, &name));
        }
        Command::Test { filter, release } => {
            let report = testing::test_project(cwd, filter.as_deref(), *release)?;
            let text = messages::test_report(style, &report);
            if report.failed == 0 {
                emit(quiet, &text);
            } else {
                eprintln!("{text}");
                return Err(CliFailure::Presented);
            }
        }
        Command::Run { args, release } => {
            let path = build(cwd, *release)?;
            emit(
                quiet,
                &messages::built(style, &path.display().to_string(), *release),
            );
            emit(
                quiet,
                &messages::running(style, &path.display().to_string()),
            );
            let status = std::process::Command::new(&path).args(args).status()?;
            if !status.success() {
                return Ok(status.code().unwrap_or(1));
            }
        }
        _ => unreachable!("build_commands only handles compiler-backed commands"),
    }
    Ok(0)
}

/// Tooling commands: `fmt`, `lint`, `doc`, `clean`, `tree`, `outdated`, `search`, `info`.
fn tooling_commands(
    command: &Command,
    cwd: &Path,
    style: Style,
    quiet: bool,
) -> Result<i32, CliFailure> {
    match command {
        Command::Fmt { check } => {
            let changed = tooling::format_project(cwd, *check)?;
            if *check {
                if changed != 0 {
                    eprintln!("{}", messages::require_formatting(style, changed));
                    return Err(CliFailure::Presented);
                }
                emit(quiet, &messages::all_formatted(style));
            } else {
                emit(quiet, &messages::formatted(style, changed));
            }
        }
        Command::Lint => {
            let warnings = tooling::lint_project(cwd)?;
            if warnings.is_empty() {
                emit(quiet, &messages::no_lint_warnings(style));
            } else {
                for warning in &warnings {
                    eprintln!(
                        "{}",
                        messages::lint_warning(
                            style,
                            &warning.path,
                            warning.line,
                            &warning.code,
                            &warning.message
                        )
                    );
                }
                eprintln!("{}", messages::lint_summary(style, warnings.len()));
                return Err(CliFailure::Presented);
            }
        }
        Command::Doc => {
            let path = tooling::generate_docs(cwd)?;
            emit(
                quiet,
                &messages::generated(style, &path.display().to_string()),
            );
        }
        Command::Clean => {
            tooling::clean(cwd)?;
            emit(quiet, &messages::cleaned(style));
        }
        Command::Tree => emit_data(quiet, style, &tooling::dependency_tree(cwd)?),
        Command::Outdated => emit_data(quiet, style, &tooling::outdated(cwd)?),
        Command::Search { query } => emit_data(quiet, style, &tooling::search(query)?),
        Command::Info { package } => emit_data(quiet, style, &tooling::package_info(package)?),
        _ => unreachable!("tooling_commands only handles tooling commands"),
    }
    Ok(0)
}

fn emit_data(quiet: bool, style: Style, output: &str) {
    if !quiet {
        print!("{}", messages::highlight_header(style, output));
    }
}

#[cfg(test)]
mod cli_tests {
    use super::*;
    #[test]
    fn phase_twenty_commands_have_stable_cli_contracts() {
        for command in [
            vec!["vpm", "test"],
            vec!["vpm", "test", "math", "--release"],
            vec!["vpm", "fmt", "--check"],
            vec!["vpm", "lint"],
            vec!["vpm", "doc"],
            vec!["vpm", "clean"],
            vec!["vpm", "tree"],
            vec!["vpm", "outdated"],
            vec!["vpm", "search", "math"],
            vec!["vpm", "info", "owner/repo/math"],
        ] {
            assert!(Cli::try_parse_from(command).is_ok());
        }
        for deferred in ["login", "yank"] {
            assert!(Cli::try_parse_from(["vpm", deferred]).is_err());
        }
    }

    #[test]
    fn global_and_publish_commands_have_stable_cli_contracts() {
        for command in [
            vec!["vpm", "install"],
            vec!["vpm", "install", "math"],
            vec!["vpm", "install", "math@1.0.0"],
            vec!["vpm", "uninstall", "math"],
            vec!["vpm", "exec", "math"],
            vec!["vpm", "exec", "math", "--bin", "math-tool", "--", "arg"],
            vec!["vpm", "publish"],
            vec!["vpm", "publish", "alice/vut-packages"],
            vec!["vpm", "publish", "--dry-run"],
            vec!["vpm", "publish", "alice/vut-packages", "--dry-run"],
        ] {
            assert!(Cli::try_parse_from(command.clone()).is_ok(), "{command:?}");
        }
        assert!(matches!(
            Cli::try_parse_from(["vpm", "install"]).unwrap().command,
            Command::Install { package: None }
        ));
        assert!(matches!(
            Cli::try_parse_from(["vpm", "install", "math"])
                .unwrap()
                .command,
            Command::Install { package: Some(_) }
        ));
    }

    #[test]
    fn quiet_and_colour_are_global_flags() {
        let before = Cli::try_parse_from(["vpm", "--quiet", "build"]).unwrap();
        assert!(before.quiet);
        let after = Cli::try_parse_from(["vpm", "build", "--quiet"]).unwrap();
        assert!(after.quiet);
        let colour = Cli::try_parse_from(["vpm", "--color", "never", "install"]).unwrap();
        assert_eq!(colour.color, ColorMode::Never);
        assert!(Cli::try_parse_from(["vpm", "--color", "rainbow", "install"]).is_err());
    }
}
