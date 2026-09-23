//! Compiles and measures one Vut workload at one optimization level.
//!
//! Optimization level and CPU target are independent: the target defaults to
//! the portable host triple and native codegen is never enabled implicitly.

use crate::{
    corpus::Workload,
    metrics,
    model::{Level, RunStats, StructureMetrics, WorkloadResult},
    process,
};
use std::{
    path::{Path, PathBuf},
    time::Instant,
};
use vut_compiler::{CompilerConfig, CompilerSession};

pub struct Runner {
    pub target: String,
    pub work_dir: PathBuf,
    pub iterations: usize,
    pub warmup: usize,
}

impl Runner {
    #[must_use]
    pub fn new(target: impl Into<String>, work_dir: PathBuf) -> Self {
        Self {
            target: target.into(),
            work_dir,
            iterations: 20,
            warmup: 3,
        }
    }

    pub fn measure(&self, workload: &Workload, level: Level) -> Result<WorkloadResult, String> {
        let out_dir = self.work_dir.join(&workload.name).join(level.label());
        std::fs::create_dir_all(&out_dir).map_err(|error| error.to_string())?;
        let executable = out_dir.join(if cfg!(windows) { "bench.exe" } else { "bench" });

        let config = self.config(level);
        let structure = probe_structure(workload, &config, level)?;

        let compile_start = Instant::now();
        let mut session = CompilerSession::new(config);
        session
            .emit_source_file_executable(&workload.source, &executable)
            .map_err(|error| format!("{}: build failed: {error}", workload.name))?;
        let compile_ms = metrics::millis(compile_start.elapsed());
        let binary_bytes = std::fs::metadata(&executable).map_or(0, |meta| meta.len());

        let (run, exit_code) = self.time(&executable, workload)?;
        Ok(WorkloadResult {
            name: workload.name.clone(),
            level,
            structure,
            compile_ms,
            binary_bytes,
            exit_code,
            verified: true,
            run,
        })
    }

    fn config(&self, level: Level) -> CompilerConfig {
        let mut config = CompilerConfig::for_target(self.target.clone(), level.build_mode());
        config.optimization = Some(level.optimization());
        config.runtime_library = vut_paths::runtime_library(&self.target);
        config.startup_object = vut_paths::startup_object(&self.target);
        config
    }

    fn time(&self, executable: &Path, workload: &Workload) -> Result<(RunStats, i32), String> {
        let expected = workload.expected_trimmed();
        for _ in 0..self.warmup {
            let run = process::run_once(executable, workload.stdin.as_deref())?;
            if run.stdout.trim_end() != expected {
                return Err(mismatch(workload, expected, &run.stdout));
            }
        }
        let mut samples = Vec::with_capacity(self.iterations);
        let mut cpu_ms = 0.0_f64;
        let mut peak = 0_u64;
        let mut exit_code = 0;
        for _ in 0..self.iterations {
            let run = process::run_once(executable, workload.stdin.as_deref())?;
            if run.exit_code != 0 {
                return Err(format!(
                    "{}: program exited with code {}",
                    workload.name, run.exit_code
                ));
            }
            if run.stdout.trim_end() != expected {
                return Err(mismatch(workload, expected, &run.stdout));
            }
            samples.push(run.wall);
            cpu_ms = cpu_ms.max(run.cpu_ms);
            peak = peak.max(run.peak_rss_bytes);
            exit_code = run.exit_code;
        }
        Ok((metrics::summarize(samples, cpu_ms, peak), exit_code))
    }
}

/// Type-checks a workload, runs the MIR optimizer at `level`, and collects the
/// deterministic structure metrics of the resulting MIR.
fn probe_structure(
    workload: &Workload,
    config: &CompilerConfig,
    level: Level,
) -> Result<StructureMetrics, String> {
    let mut session = CompilerSession::new(config.clone());
    let mut checked = session
        .check_source_file(&workload.source)
        .map_err(|error| format!("{}: {error}", workload.name))?;
    if checked.resolution.diagnostics.has_errors() || checked.semantics.diagnostics.has_errors() {
        return Err(format!(
            "{}: compilation failed\n{}",
            workload.name,
            session.render_diagnostics(&checked)
        ));
    }
    if level.optimization().is_optimizing() {
        vut_mir::optimize(&mut checked.mir, level.optimization());
    }
    Ok(crate::structure::collect(&checked.mir))
}

fn mismatch(workload: &Workload, expected: &str, actual: &str) -> String {
    format!(
        "{}: output mismatch\n--- expected ---\n{expected}\n--- actual ---\n{}",
        workload.name,
        actual.trim_end()
    )
}
