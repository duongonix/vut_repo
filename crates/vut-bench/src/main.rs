//! `vut-bench` command line: `list`, `run`, `compare`.

use clap::{Parser, Subcommand};
use std::path::{Path, PathBuf};
use vut_bench::{
    BenchReport, Level, Runner, competitors,
    competitors::Language,
    corpus,
    report::{self, Comparison},
};

#[derive(Parser, Debug)]
#[command(
    name = "vut-bench",
    version,
    about = "Vut benchmark and measurement harness"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// List the workloads in the corpus.
    List {
        /// Corpus directory (defaults to `benchmarks/corpus`).
        #[arg(long)]
        corpus: Option<PathBuf>,
    },
    /// Compile and measure the corpus, writing a JSON report.
    Run {
        #[arg(long)]
        corpus: Option<PathBuf>,
        /// Comma-separated optimization levels, e.g. `0,1,2,3`.
        #[arg(long, default_value = "0,1,2,3")]
        levels: String,
        #[arg(long, default_value_t = 20)]
        iterations: usize,
        #[arg(long, default_value_t = 3)]
        warmup: usize,
        /// Report output path (defaults to `target/vut-bench/report.json`).
        #[arg(long)]
        out: Option<PathBuf>,
        #[arg(long)]
        work_dir: Option<PathBuf>,
        /// Target triple (defaults to the host; portable CPU baseline).
        #[arg(long)]
        target: Option<String>,
        /// Also compare against this committed baseline and print a diff.
        #[arg(long)]
        baseline: Option<PathBuf>,
    },
    /// Compare two reports.
    Compare {
        baseline: PathBuf,
        current: PathBuf,
        #[arg(long, default_value_t = 5.0)]
        threshold: f64,
    },
    /// Measure C/Rust (Zig/Go best-effort) counterparts against the Vut baseline.
    Competitors {
        #[arg(long)]
        corpus: Option<PathBuf>,
        /// Directory holding `<workload>.<ext>` counterpart sources.
        #[arg(long)]
        competitors: Option<PathBuf>,
        #[arg(long, default_value_t = 20)]
        iterations: usize,
        #[arg(long, default_value_t = 3)]
        warmup: usize,
        #[arg(long)]
        work_dir: Option<PathBuf>,
        /// Vut baseline used for reference timings.
        #[arg(long)]
        baseline: Option<PathBuf>,
        /// Optimization level to compare (defaults to O2).
        #[arg(long, default_value = "2")]
        level: String,
    },
}

fn workspace(relative: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(relative)
}

fn default_corpus() -> PathBuf {
    workspace("benchmarks/corpus")
}

fn default_work_dir() -> PathBuf {
    workspace("target/vut-bench/work")
}

fn default_out() -> PathBuf {
    workspace("target/vut-bench/report.json")
}

fn parse_levels(value: &str) -> Result<Vec<Level>, Box<dyn std::error::Error>> {
    let mut levels = Vec::new();
    for part in value.split(',') {
        let part = part.trim();
        let number: u8 = part
            .parse()
            .map_err(|_| format!("invalid optimization level `{part}`"))?;
        levels.push(Level::from_u8(number).ok_or_else(|| format!("unsupported level `{part}`"))?);
    }
    levels.sort();
    levels.dedup();
    if levels.is_empty() {
        return Err("no optimization levels selected".into());
    }
    Ok(levels)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    match cli.command {
        Command::List { corpus } => {
            let corpus = corpus.unwrap_or_else(default_corpus);
            for workload in vut_bench::corpus::discover(&corpus)? {
                println!("{}", workload.name);
            }
        }
        Command::Run {
            corpus,
            levels,
            iterations,
            warmup,
            out,
            work_dir,
            target,
            baseline,
        } => {
            let corpus_dir = corpus.unwrap_or_else(default_corpus);
            let parsed_levels = parse_levels(&levels)?;
            let out = out.unwrap_or_else(default_out);
            run(
                &corpus_dir,
                &parsed_levels,
                iterations,
                warmup,
                &out,
                work_dir.unwrap_or_else(default_work_dir),
                target,
                baseline.as_deref(),
            )?;
        }
        Command::Compare {
            baseline,
            current,
            threshold,
        } => {
            let comparison =
                Comparison::between(&report::load(&baseline)?, &report::load(&current)?);
            print!("{comparison}");
            let regressions = comparison.regressions(threshold);
            if !regressions.is_empty() {
                eprintln!(
                    "{} workload(s) regressed by more than {threshold:.1}%",
                    regressions.len()
                );
            }
        }
        Command::Competitors {
            corpus,
            competitors,
            iterations,
            warmup,
            work_dir,
            baseline,
            level,
        } => {
            let level = Level::from_u8(
                level
                    .parse()
                    .map_err(|_| format!("invalid level `{level}`"))?,
            )
            .ok_or_else(|| format!("unsupported level `{level}`"))?;
            let corpus_dir = corpus.unwrap_or_else(default_corpus);
            let competitors_dir =
                competitors.unwrap_or_else(|| workspace("benchmarks/competitors"));
            let work_dir = work_dir.unwrap_or_else(default_work_dir);
            let baseline_path = baseline.unwrap_or_else(|| workspace("benchmarks/baseline.json"));
            competitors_command(
                &corpus_dir,
                &competitors_dir,
                &work_dir,
                &baseline_path,
                level,
                iterations,
                warmup,
            )?;
        }
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn competitors_command(
    corpus_dir: &Path,
    competitors_dir: &Path,
    work_dir: &Path,
    baseline_path: &Path,
    level: Level,
    iterations: usize,
    warmup: usize,
) -> Result<(), Box<dyn std::error::Error>> {
    let workloads = corpus::discover(corpus_dir)?;
    let available: Vec<Language> = Language::ALL
        .into_iter()
        .filter(|language| language.available())
        .collect();
    if available.is_empty() {
        return Err("no competitor compilers found (cc/rustc/zig/go)".into());
    }
    let reference = report::load(baseline_path).ok();

    println!(
        "{:<24} {:>10} {}",
        "workload",
        format!("vut-{}", level.label()),
        available
            .iter()
            .map(|language| format!("{:>12}", language.label()))
            .collect::<Vec<_>>()
            .join(" ")
    );
    for workload in &workloads {
        let vut_ms = reference
            .as_ref()
            .and_then(|report| report.find(&workload.name, level))
            .map(|result| result.run.median_ms);
        let mut cells = Vec::new();
        for language in &available {
            let Some(source) = competitors::source_for(competitors_dir, &workload.name, *language)
            else {
                continue;
            };
            match competitors::measure(
                *language,
                &workload.name,
                &source,
                workload.expected_trimmed(),
                work_dir,
                iterations,
                warmup,
            ) {
                Ok(result) => cells.push(format!("{:>12.3}", result.run.median_ms)),
                Err(error) => {
                    eprintln!("{error}");
                    cells.push(format!("{:>12}", "failed"));
                }
            }
        }
        if cells.is_empty() {
            continue;
        }
        let vut_cell = vut_ms.map_or_else(|| "n/a".to_owned(), |ms| format!("{ms:>10.3}"));
        println!("{:<24} {vut_cell} {}", workload.name, cells.join(" "));
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn run(
    corpus_dir: &Path,
    levels: &[Level],
    iterations: usize,
    warmup: usize,
    out: &Path,
    work_dir: PathBuf,
    target: Option<String>,
    baseline: Option<&Path>,
) -> Result<(), Box<dyn std::error::Error>> {
    let workloads = corpus::discover(corpus_dir)?;
    let target = target.unwrap_or_else(|| vut_compiler::CompilerConfig::default().target);
    let mut runner = Runner::new(target.clone(), work_dir);
    runner.iterations = iterations;
    runner.warmup = warmup;

    let mut report = BenchReport::new(target);
    for level in levels {
        for workload in &workloads {
            let result = runner.measure(workload, *level)?;
            let rss = if result.run.peak_rss_bytes == 0 {
                "n/a".to_owned()
            } else {
                format!("{}KB", result.run.peak_rss_bytes / 1024)
            };
            println!(
                "{:<24} {:<3} median={:>9.3}ms (raw {:>9.3}) compile={:>8.3}ms rss={rss:>9}",
                result.name,
                level.label(),
                result.run.trimmed_median_ms,
                result.run.median_ms,
                result.compile_ms,
            );
            report.results.push(result);
        }
    }
    report::save(&report, out)?;
    println!("wrote {}", out.display());

    if let Some(baseline_path) = baseline {
        let comparison = Comparison::between(&report::load(baseline_path)?, &report);
        print!("{comparison}");
    }
    Ok(())
}
