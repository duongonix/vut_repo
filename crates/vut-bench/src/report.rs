//! Report persistence and baseline comparison.

use crate::metrics::percent_delta;
use crate::model::{BenchReport, Level, StructureMetrics};
use std::{
    fmt,
    path::{Path, PathBuf},
};

pub fn save(report: &BenchReport, path: &Path) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    let text = serde_json::to_string_pretty(report).map_err(|error| error.to_string())?;
    let temp = path.with_extension("json.tmp");
    std::fs::write(&temp, format!("{text}\n")).map_err(|error| error.to_string())?;
    std::fs::rename(&temp, path).map_err(|error| error.to_string())
}

pub fn load(path: &Path) -> Result<BenchReport, String> {
    let text = std::fs::read_to_string(path).map_err(|error| error.to_string())?;
    serde_json::from_str(&text)
        .map_err(|error| format!("invalid report `{}`: {error}", path.display()))
}

/// Default committed baseline path.
#[must_use]
pub fn default_baseline() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../benchmarks/baseline.json")
}

/// Deterministic structural differences between two runs (current - baseline).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct StructureDelta {
    pub instructions: i64,
    pub allocations: i64,
    pub retains: i64,
    pub releases: i64,
    pub make_unique: i64,
    pub bounds_checked: i64,
    pub frame_bytes: i64,
}

impl StructureDelta {
    fn between(baseline: &StructureMetrics, current: &StructureMetrics) -> Self {
        Self {
            instructions: diff(baseline.instructions, current.instructions),
            allocations: diff(baseline.allocations, current.allocations),
            retains: diff(baseline.retains, current.retains),
            releases: diff(baseline.releases, current.releases),
            make_unique: diff(baseline.make_unique, current.make_unique),
            bounds_checked: diff(checked_bounds(baseline), checked_bounds(current)),
            frame_bytes: diff(baseline.frame_bytes, current.frame_bytes),
        }
    }

    #[must_use]
    pub fn is_clean(&self) -> bool {
        *self == Self::default()
    }
}

fn diff(baseline: usize, current: usize) -> i64 {
    i64::try_from(current).unwrap_or(i64::MAX) - i64::try_from(baseline).unwrap_or(i64::MAX)
}

/// Total bounds-checked accesses: checked list/array/variadic reads.
#[must_use]
pub fn checked_bounds(metrics: &StructureMetrics) -> usize {
    metrics.list_at + metrics.array_at + (metrics.variadic_at - metrics.variadic_at_in_bounds)
}

#[derive(Clone, Debug)]
pub struct ComparisonRow {
    pub name: String,
    pub level: Level,
    pub baseline_ms: f64,
    pub current_ms: f64,
    pub structure: StructureDelta,
}

impl ComparisonRow {
    #[must_use]
    pub fn speed_delta_pct(&self) -> f64 {
        percent_delta(self.baseline_ms, self.current_ms)
    }
}

#[derive(Clone, Debug, Default)]
pub struct Comparison {
    pub rows: Vec<ComparisonRow>,
}

impl Comparison {
    #[must_use]
    pub fn between(baseline: &BenchReport, current: &BenchReport) -> Self {
        let mut rows = Vec::new();
        for result in &current.results {
            let Some(base) = baseline.find(&result.name, result.level) else {
                continue;
            };
            rows.push(ComparisonRow {
                name: result.name.clone(),
                level: result.level,
                baseline_ms: base.run.trimmed_median_ms,
                current_ms: result.run.trimmed_median_ms,
                structure: StructureDelta::between(&base.structure, &result.structure),
            });
        }
        rows.sort_by(|left, right| {
            left.name
                .cmp(&right.name)
                .then_with(|| left.level.cmp(&right.level))
        });
        Self { rows }
    }

    #[must_use]
    pub fn regressions(&self, threshold_pct: f64) -> Vec<&ComparisonRow> {
        self.rows
            .iter()
            .filter(|row| row.speed_delta_pct() > threshold_pct)
            .collect()
    }

    #[must_use]
    pub fn structural_changes(&self) -> Vec<&ComparisonRow> {
        self.rows
            .iter()
            .filter(|row| !row.structure.is_clean())
            .collect()
    }
}

impl fmt::Display for Comparison {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(
            formatter,
            "{:<24} {:<3} {:>10} {:>10} {:>9}  structural",
            "workload", "lvl", "baseline", "current", "delta%"
        )?;
        for row in &self.rows {
            let delta = row.speed_delta_pct();
            let verdict = if delta > 5.0 {
                "slower"
            } else if delta < -5.0 {
                "faster"
            } else {
                "tie"
            };
            let structure = if row.structure.is_clean() {
                "same".to_owned()
            } else {
                format!(
                    "alloc{:+} rc{:+}/{:+} mu{:+} bounds{:+} frame{:+}B inst{:+}",
                    row.structure.allocations,
                    row.structure.retains,
                    row.structure.releases,
                    row.structure.make_unique,
                    row.structure.bounds_checked,
                    row.structure.frame_bytes,
                    row.structure.instructions
                )
            };
            writeln!(
                formatter,
                "{:<24} {:<3} {:>10.3} {:>10.3} {delta:>+8.1}%  {verdict}  {structure}",
                row.name,
                row.level.label(),
                row.baseline_ms,
                row.current_ms,
            )?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{RunStats, WorkloadResult};

    fn report(median_ms: f64, allocations: usize) -> BenchReport {
        let mut report = BenchReport::new("test");
        report.results.push(WorkloadResult {
            name: "x".into(),
            level: Level::O2,
            structure: StructureMetrics {
                allocations,
                ..StructureMetrics::default()
            },
            compile_ms: 1.0,
            binary_bytes: 1,
            exit_code: 0,
            verified: true,
            run: RunStats {
                iterations: 1,
                median_ms,
                p95_ms: median_ms,
                min_ms: median_ms,
                max_ms: median_ms,
                trimmed_median_ms: median_ms,
                cpu_ms_estimate: 0.0,
                peak_rss_bytes: 0,
            },
        });
        report
    }

    #[test]
    fn compare_detects_speed_and_structure_changes() {
        let baseline = report(100.0, 10);
        let current = report(110.0, 8);
        let comparison = Comparison::between(&baseline, &current);
        assert_eq!(comparison.rows.len(), 1);
        let row = &comparison.rows[0];
        assert!((row.speed_delta_pct() - 10.0).abs() < 1e-9);
        assert_eq!(row.structure.allocations, -2);
        assert_eq!(comparison.regressions(5.0).len(), 1);
        assert_eq!(comparison.structural_changes().len(), 1);
    }

    #[test]
    fn report_round_trips_through_disk() {
        let report = report(42.0, 3);
        let path =
            std::env::temp_dir().join(format!("vut-bench-report-{}.json", std::process::id()));
        save(&report, &path).unwrap();
        let loaded = load(&path).unwrap();
        assert!((loaded.results[0].run.trimmed_median_ms - 42.0).abs() < f64::EPSILON);
        std::fs::remove_file(path).unwrap();
    }
}
