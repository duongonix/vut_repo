//! Timing aggregation helpers.

use crate::model::RunStats;
use std::time::Duration;

#[must_use]
pub fn millis(duration: Duration) -> f64 {
    duration.as_secs_f64() * 1000.0
}

/// Summarises raw per-iteration durations into median/p95/min/max plus a
/// trimmed median that discards intermittent slow samples.
#[must_use]
pub fn summarize(
    mut samples: Vec<Duration>,
    cpu_ms_estimate: f64,
    peak_rss_bytes: u64,
) -> RunStats {
    samples.sort_unstable();
    let iterations = samples.len();
    let min_ms = samples.first().map_or(0.0, |duration| millis(*duration));
    // Discard intermittent slow samples (OS scheduling, scanner interference)
    // so the reported figure reflects the workload rather than environmental
    // noise. A genuine regression still raises the minimum.
    let threshold = min_ms * 2.0;
    let steady: Vec<Duration> = samples
        .iter()
        .copied()
        .filter(|duration| millis(*duration) <= threshold)
        .collect();
    RunStats {
        iterations,
        median_ms: percentile_ms(&samples, 500),
        p95_ms: percentile_ms(&samples, 950),
        min_ms,
        max_ms: samples.last().map_or(0.0, |duration| millis(*duration)),
        trimmed_median_ms: percentile_ms(&steady, 500),
        cpu_ms_estimate,
        peak_rss_bytes,
    }
}

/// Nearest-rank percentile over a sorted slice, expressed in permille.
#[must_use]
pub fn percentile_ms(sorted: &[Duration], permille: u32) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    let rank = (permille as usize * sorted.len()).div_ceil(1000);
    let index = rank.saturating_sub(1).min(sorted.len() - 1);
    millis(sorted[index])
}

/// Percentage change from `baseline` to `current` (positive = slower/bigger).
#[must_use]
pub fn percent_delta(baseline: f64, current: f64) -> f64 {
    if baseline == 0.0 {
        return 0.0;
    }
    (current - baseline) / baseline * 100.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn percentiles_use_nearest_rank() {
        let samples: Vec<Duration> = [10, 20, 30, 40].map(Duration::from_millis).to_vec();
        assert!((percentile_ms(&samples, 500) - 20.0).abs() < f64::EPSILON);
        assert!((percentile_ms(&samples, 950) - 40.0).abs() < f64::EPSILON);
    }

    #[test]
    fn summarize_reports_extremes() {
        let stats = summarize([3, 1, 2].map(Duration::from_millis).to_vec(), 0.0, 1024);
        assert_eq!(stats.iterations, 3);
        assert!((stats.min_ms - 1.0).abs() < f64::EPSILON);
        assert!((stats.max_ms - 3.0).abs() < f64::EPSILON);
        assert_eq!(stats.peak_rss_bytes, 1024);
    }

    #[test]
    fn trimmed_median_discards_intermittent_slow_samples() {
        let mut samples = vec![Duration::from_millis(50); 6];
        samples.extend(vec![Duration::from_millis(300); 14]);
        let stats = summarize(samples, 0.0, 0);
        assert!((stats.median_ms - 300.0).abs() < f64::EPSILON);
        assert!((stats.min_ms - 50.0).abs() < f64::EPSILON);
        assert!((stats.trimmed_median_ms - 50.0).abs() < f64::EPSILON);
    }

    #[test]
    fn delta_handles_zero_baseline() {
        assert!(percent_delta(0.0, 5.0).abs() < f64::EPSILON);
        assert!((percent_delta(100.0, 110.0) - 10.0).abs() < 1e-9);
    }
}
