//! Serialisable benchmark data model.
//!
//! Reports are plain JSON so they can be committed as a baseline, diffed, and
//! compared across machines without a database.

use serde::{Deserialize, Serialize};

/// A build optimization level. Independent of the CPU target (which is portable
/// by default; native codegen requires an explicit opt-in).
#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum Level {
    O0,
    O1,
    O2,
    O3,
}

impl Level {
    pub const ALL: [Self; 4] = [Self::O0, Self::O1, Self::O2, Self::O3];

    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::O0 => "O0",
            Self::O1 => "O1",
            Self::O2 => "O2",
            Self::O3 => "O3",
        }
    }

    #[must_use]
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0 => Some(Self::O0),
            1 => Some(Self::O1),
            2 => Some(Self::O2),
            3 => Some(Self::O3),
            _ => None,
        }
    }

    #[must_use]
    pub fn optimization(self) -> vut_compiler::OptimizationLevel {
        match self {
            Self::O0 => vut_compiler::OptimizationLevel::O0,
            Self::O1 => vut_compiler::OptimizationLevel::O1,
            Self::O2 => vut_compiler::OptimizationLevel::O2,
            Self::O3 => vut_compiler::OptimizationLevel::O3,
        }
    }

    #[must_use]
    pub fn build_mode(self) -> vut_compiler::BuildMode {
        match self {
            Self::O0 => vut_compiler::BuildMode::Debug,
            _ => vut_compiler::BuildMode::Release,
        }
    }
}

/// Deterministic MIR shape metrics. These are noise-free and suitable for PR
/// regression gating (unlike wall-clock timings).
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Eq, Serialize)]
pub struct StructureMetrics {
    pub functions: usize,
    pub blocks: usize,
    pub instructions: usize,
    pub allocations: usize,
    pub retains: usize,
    pub releases: usize,
    pub make_unique: usize,
    pub drops: usize,
    pub runtime_calls: usize,
    pub list_at: usize,
    pub list_at_unchecked: usize,
    pub array_at: usize,
    pub array_at_unchecked: usize,
    pub variadic_at: usize,
    pub variadic_at_in_bounds: usize,
    pub interface_calls: usize,
    pub indirect_calls: usize,
    pub direct_calls: usize,
    pub spawn: usize,
    pub make_closure: usize,
    pub frame_count: usize,
    pub frame_bytes: usize,
}

/// Timing and resource statistics for one `(workload, level)` pair.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct RunStats {
    pub iterations: usize,
    pub median_ms: f64,
    pub p95_ms: f64,
    pub min_ms: f64,
    pub max_ms: f64,
    /// Median after discarding samples slower than 2x the minimum. This is the
    /// primary comparison metric: it is robust to intermittent system
    /// interference while a genuine regression still raises the minimum.
    pub trimmed_median_ms: f64,
    /// Approximate CPU time from process sampling (0 = not sampled).
    pub cpu_ms_estimate: f64,
    /// Peak resident set size in bytes (0 = not sampled; enable with
    /// `VUT_BENCH_SAMPLE=1`).
    pub peak_rss_bytes: u64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct WorkloadResult {
    pub name: String,
    pub level: Level,
    pub structure: StructureMetrics,
    pub compile_ms: f64,
    pub binary_bytes: u64,
    pub exit_code: i32,
    pub verified: bool,
    pub run: RunStats,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct BenchReport {
    pub schema: u32,
    pub tool: String,
    pub generated_unix_ms: u64,
    pub target: String,
    pub results: Vec<WorkloadResult>,
}

/// Current baseline schema version.
pub const SCHEMA: u32 = 2;

impl BenchReport {
    #[must_use]
    pub fn new(target: impl Into<String>) -> Self {
        Self {
            schema: SCHEMA,
            tool: format!("vut-bench {}", env!("CARGO_PKG_VERSION")),
            generated_unix_ms: now_unix_ms(),
            target: target.into(),
            results: Vec::new(),
        }
    }

    #[must_use]
    pub fn find(&self, name: &str, level: Level) -> Option<&WorkloadResult> {
        self.results
            .iter()
            .find(|result| result.name == name && result.level == level)
    }
}

#[must_use]
pub fn now_unix_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |duration| {
            u64::try_from(duration.as_millis()).unwrap_or(u64::MAX)
        })
}
