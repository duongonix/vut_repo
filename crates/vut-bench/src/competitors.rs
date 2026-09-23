//! Cross-language competitor harness.
//!
//! The hard comparison set is C and Rust. Zig and Go are best-effort: a language
//! is used only when its compiler is present and a counterpart source exists.
//! Every language is measured with the same process sampler as Vut.

use crate::{metrics, model::RunStats, process};
use std::{
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Language {
    C,
    Rust,
    Zig,
    Go,
}

impl Language {
    pub const ALL: [Self; 4] = [Self::C, Self::Rust, Self::Zig, Self::Go];

    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::C => "C",
            Self::Rust => "Rust",
            Self::Zig => "Zig",
            Self::Go => "Go",
        }
    }

    #[must_use]
    pub fn extension(self) -> &'static str {
        match self {
            Self::C => "c",
            Self::Rust => "rs",
            Self::Zig => "zig",
            Self::Go => "go",
        }
    }

    fn program(self) -> String {
        match self {
            Self::C => std::env::var("CC").unwrap_or_else(|_| "cc".to_owned()),
            Self::Rust => "rustc".to_owned(),
            Self::Zig => "zig".to_owned(),
            Self::Go => "go".to_owned(),
        }
    }

    fn probe_arguments(self) -> &'static [&'static str] {
        match self {
            Self::Zig | Self::Go => &["version"],
            Self::C | Self::Rust => &["--version"],
        }
    }

    #[must_use]
    pub fn available(self) -> bool {
        Command::new(self.program())
            .args(self.probe_arguments())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .is_ok_and(|status| status.success())
    }

    fn build(self, source: &Path, output: &Path) -> Result<(), String> {
        let mut command = Command::new(self.program());
        match self {
            Self::C => {
                command.arg("-O3").arg("-o").arg(output).arg(source);
            }
            Self::Rust => {
                command
                    .args(["-C", "opt-level=3", "-C", "debuginfo=0", "-o"])
                    .arg(output)
                    .arg(source);
            }
            Self::Zig => {
                command
                    .arg("build-exe")
                    .arg("-O")
                    .arg("ReleaseFast")
                    .arg(format!("-femit-bin={}", output.display()))
                    .arg(source);
            }
            Self::Go => {
                command.arg("build").arg("-o").arg(output).arg(source);
            }
        }
        let result = command.output().map_err(|error| error.to_string())?;
        if !result.status.success() {
            return Err(format!(
                "{}: build failed: {}",
                self.label(),
                String::from_utf8_lossy(&result.stderr).trim()
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Debug)]
pub struct CompetitorResult {
    pub name: String,
    pub language: Language,
    pub run: RunStats,
    pub exit_code: i32,
}

/// Builds and measures a competitor binary, verifying its output.
pub fn measure(
    language: Language,
    name: &str,
    source: &Path,
    expected: &str,
    work_dir: &Path,
    iterations: usize,
    warmup: usize,
) -> Result<CompetitorResult, String> {
    let out_dir = work_dir.join("competitors").join(language.label());
    std::fs::create_dir_all(&out_dir).map_err(|error| error.to_string())?;
    let executable = out_dir.join(format!("{name}{}", std::env::consts::EXE_SUFFIX));
    language.build(source, &executable)?;

    for _ in 0..warmup {
        let run = process::run_once(&executable, None)?;
        if run.stdout.trim_end() != expected {
            return Err(format!(
                "{} {}: output mismatch (got `{}`)",
                language.label(),
                name,
                run.stdout.trim_end()
            ));
        }
    }
    let mut samples = Vec::with_capacity(iterations);
    let mut cpu_ms = 0.0_f64;
    let mut peak = 0_u64;
    let mut exit_code = 0;
    for _ in 0..iterations {
        let run = process::run_once(&executable, None)?;
        if run.exit_code != 0 {
            return Err(format!(
                "{} {}: exited with code {}",
                language.label(),
                name,
                run.exit_code
            ));
        }
        samples.push(run.wall);
        cpu_ms = cpu_ms.max(run.cpu_ms);
        peak = peak.max(run.peak_rss_bytes);
        exit_code = run.exit_code;
    }
    Ok(CompetitorResult {
        name: name.to_owned(),
        language,
        run: metrics::summarize(samples, cpu_ms, peak),
        exit_code,
    })
}

/// Source path for `name` in `language` under `competitors_dir`, if it exists.
#[must_use]
pub fn source_for(competitors_dir: &Path, name: &str, language: Language) -> Option<PathBuf> {
    let path = competitors_dir.join(format!("{name}.{}", language.extension()));
    path.is_file().then_some(path)
}
