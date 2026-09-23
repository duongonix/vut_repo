use clap::{Parser, Subcommand, ValueEnum};
use std::path::PathBuf;
use vut_compiler::{BuildMode, CompilerConfig, CompilerSession, OptimizationLevel};

#[derive(Parser, Debug)]
#[command(name = "vut", version, about = "The Vut compiler")]
struct Cli {
    #[arg(long, value_enum, global = true, default_value_t = Color::Auto)]
    color: Color,
    #[arg(long, value_enum, global = true, default_value_t = DiagnosticFormat::Human)]
    diagnostic_format: DiagnosticFormat,
    #[command(subcommand)]
    command: Command,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum Color {
    Auto,
    Always,
    Never,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum DiagnosticFormat {
    Human,
    Json,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Compile and execute a Vut program.
    Run(Options),
    /// Compile a Vut program to a native executable.
    Build(Options),
    /// Diagnose the local toolchain, runtime archives and linker backend.
    Doctor {
        /// Target triple to diagnose; defaults to the host.
        #[arg(long)]
        target: Option<String>,
        /// Emit a machine-readable JSON report.
        #[arg(long)]
        json: bool,
    },
}

#[derive(clap::Args, Debug)]
struct Options {
    /// Standalone source file; defaults to src/main.vut.
    source: Option<PathBuf>,
    #[arg(long)]
    release: bool,
    /// Optimization level: 0, 1, 2, or 3. `--release` implies `-O2`.
    #[arg(short = 'O', long = "opt-level", value_name = "LEVEL")]
    opt_level: Option<String>,
    #[arg(long)]
    target: Option<String>,
    /// Target CPU: `native` or a preset. Portable baseline by default.
    #[arg(long = "target-cpu", value_name = "CPU")]
    target_cpu: Option<String>,
    /// Cranelift feature override (`avx2`, `+avx2`, `-avx2`); repeatable.
    #[arg(
        long = "target-feature",
        value_name = "FEATURE",
        allow_hyphen_values = true
    )]
    target_feature: Vec<String>,
    #[arg(long, short)]
    output: Option<PathBuf>,
    /// Native static library (`.lib`/`.a`) to link; repeatable.
    #[arg(long = "native-lib", value_name = "PATH")]
    native_lib: Vec<PathBuf>,
    /// Platform system library to link by name; repeatable.
    #[arg(long = "system-lib", value_name = "NAME")]
    system_lib: Vec<String>,
    /// Arguments passed to the compiled program (after --).
    #[arg(last = true)]
    args: Vec<String>,
}

pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    match cli.command {
        Command::Build(options) => {
            build(&options, cli.color, cli.diagnostic_format)?;
        }
        Command::Run(options) => {
            let executable = build(&options, cli.color, cli.diagnostic_format)?;
            let status = std::process::Command::new(&executable)
                .args(&options.args)
                .status()?;
            if !status.success() {
                std::process::exit(status.code().unwrap_or(1));
            }
        }
        Command::Doctor { target, json } => {
            if !crate::doctor::run(target, json)? {
                std::process::exit(1);
            }
        }
    }
    Ok(())
}

fn build(
    options: &Options,
    color: Color,
    diagnostic_format: DiagnosticFormat,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let source = options
        .source
        .clone()
        .unwrap_or_else(|| PathBuf::from("src/main.vut"));
    if !source.is_file() {
        return Err(format!("source file `{}` does not exist", source.display()).into());
    }
    let mode = if options.release {
        BuildMode::Release
    } else {
        BuildMode::Debug
    };
    let mut config = CompilerConfig::for_target(
        options
            .target
            .clone()
            .unwrap_or_else(|| CompilerConfig::default().target),
        mode,
    );
    config.runtime_library = vut_paths::runtime_library(&config.target);
    config.startup_object = vut_paths::startup_object(&config.target);
    config.optimization = options
        .opt_level
        .as_deref()
        .map(parse_opt_level)
        .transpose()?;
    config.target_cpu.clone_from(&options.target_cpu);
    config.target_features.clone_from(&options.target_feature);
    config.native_libraries.clone_from(&options.native_lib);
    config.system_libraries.clone_from(&options.system_lib);
    let output = options
        .output
        .clone()
        .unwrap_or_else(|| default_output(options.release));
    if let Some(parent) = output.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut compiler = CompilerSession::new(config);
    let checked = compiler.check_source_file(&source)?;
    let diagnostics = checked
        .resolution
        .diagnostics
        .as_slice()
        .iter()
        .chain(checked.semantics.diagnostics.as_slice());
    let renderer = vut_diagnostics::Renderer::new(match color {
        Color::Auto => vut_diagnostics::ColorChoice::Auto,
        Color::Always => vut_diagnostics::ColorChoice::Always,
        Color::Never => vut_diagnostics::ColorChoice::Never,
    });
    for diagnostic in diagnostics {
        let output = match diagnostic_format {
            DiagnosticFormat::Human => renderer.render(diagnostic, compiler.sources()),
            DiagnosticFormat::Json => renderer.render_json(diagnostic, compiler.sources()),
        };
        eprintln!("{output}");
    }
    if checked.resolution.diagnostics.has_errors() || checked.semantics.diagnostics.has_errors() {
        return Err("compilation failed".into());
    }
    compiler.emit_source_file_executable(&source, &output)?;
    println!("Built {}", output.display());
    Ok(output)
}

fn default_output(release: bool) -> PathBuf {
    let directory = if release { "release" } else { "debug" };
    PathBuf::from("build")
        .join(directory)
        .join(if cfg!(windows) { "app.exe" } else { "app" })
}

fn parse_opt_level(value: &str) -> Result<OptimizationLevel, Box<dyn std::error::Error>> {
    match value {
        "0" => Ok(OptimizationLevel::O0),
        "1" => Ok(OptimizationLevel::O1),
        "2" => Ok(OptimizationLevel::O2),
        "3" => Ok(OptimizationLevel::O3),
        other => {
            Err(format!("invalid optimization level `{other}`; expected 0, 1, 2, or 3").into())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;
    #[test]
    fn command_contract_is_valid() {
        Cli::command().debug_assert();
    }
    #[test]
    fn accepts_program_arguments_after_separator() {
        let parsed = Cli::try_parse_from(["vut", "run", "app.vut", "--", "one", "two"]).unwrap();
        let Command::Run(options) = parsed.command else {
            panic!()
        };
        assert_eq!(options.args, ["one", "two"]);
    }
    #[test]
    fn accepts_optimization_level() {
        let parsed = Cli::try_parse_from(["vut", "build", "app.vut", "-O3"]).unwrap();
        let Command::Build(options) = parsed.command else {
            panic!()
        };
        assert_eq!(options.opt_level.as_deref(), Some("3"));
        assert_eq!(parse_opt_level("2").unwrap(), OptimizationLevel::O2);
        assert!(parse_opt_level("9").is_err());
    }

    #[test]
    fn accepts_target_cpu_and_features() {
        let parsed = Cli::try_parse_from([
            "vut",
            "build",
            "app.vut",
            "--target-cpu",
            "native",
            "--target-feature",
            "+avx2",
            "--target-feature",
            "-sse3",
        ])
        .unwrap();
        let Command::Build(options) = parsed.command else {
            panic!()
        };
        assert_eq!(options.target_cpu.as_deref(), Some("native"));
        assert_eq!(options.target_feature, ["+avx2", "-sse3"]);
    }

    #[test]
    fn accepts_all_build_controls() {
        let parsed = Cli::try_parse_from([
            "vut",
            "--color",
            "never",
            "build",
            "app.vut",
            "--release",
            "--target",
            "x86_64-pc-windows-msvc",
            "--output",
            "custom.exe",
        ])
        .unwrap();
        let Command::Build(options) = parsed.command else {
            panic!()
        };
        assert!(options.release);
        assert_eq!(options.target.as_deref(), Some("x86_64-pc-windows-msvc"));
        assert_eq!(options.output, Some(PathBuf::from("custom.exe")));
    }
}
