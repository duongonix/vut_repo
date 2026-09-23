//! Single-process execution and sampling.
//!
//! Wall time is measured only around the child's spawn/wait so that sampling
//! overhead can never inflate it. RSS and CPU are sampled by a background
//! thread that stops when the child exits.

use std::{
    io::{Read as _, Write as _},
    path::Path,
    process::{Command, Stdio},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread,
    time::{Duration, Instant},
};

pub struct RawRun {
    pub wall: Duration,
    pub exit_code: i32,
    pub stdout: String,
    pub cpu_ms: f64,
    pub peak_rss_bytes: u64,
}

/// Runs `executable` once. `stdin` is written and closed when provided.
pub fn run_once(executable: &Path, stdin: Option<&str>) -> Result<RawRun, String> {
    let mut command = Command::new(executable);
    command.stdout(Stdio::piped()).stderr(Stdio::piped());
    if stdin.is_some() {
        command.stdin(Stdio::piped());
    }
    let mut child = command.spawn().map_err(|error| error.to_string())?;
    if let Some(input) = stdin
        && let Some(mut handle) = child.stdin.take()
    {
        let _ = handle.write_all(input.as_bytes());
    }

    let pid = sysinfo::Pid::from_u32(child.id());
    let stop = Arc::new(AtomicBool::new(false));
    // Process sampling perturbs short-running children on some platforms (the
    // sysinfo refresh can add a fixed cost per run), so it is opt-in. Wall time
    // is always sampled cleanly. Set `VUT_BENCH_SAMPLE=1` to collect peak RSS
    // and an approximate CPU time.
    let sampling = std::env::var_os("VUT_BENCH_SAMPLE").is_some()
        && std::env::var_os("VUT_BENCH_NO_SAMPLE").is_none();
    let sampler = if sampling {
        let stop = Arc::clone(&stop);
        Some(thread::spawn(move || sample(pid, &stop)))
    } else {
        None
    };

    let start = Instant::now();
    let status = child.wait().map_err(|error| error.to_string())?;
    let wall = start.elapsed();
    stop.store(true, Ordering::Relaxed);
    let (peak_rss_bytes, cpu_ms) = sampler
        .and_then(|handle| handle.join().ok())
        .unwrap_or((0, 0.0));

    let mut stdout = String::new();
    if let Some(mut handle) = child.stdout.take() {
        let _ = handle.read_to_string(&mut stdout);
    }
    let mut stderr = String::new();
    if let Some(mut handle) = child.stderr.take() {
        let _ = handle.read_to_string(&mut stderr);
    }
    if !stderr.is_empty() {
        return Err(format!(
            "{} wrote to stderr: {}",
            executable.display(),
            stderr.trim()
        ));
    }
    Ok(RawRun {
        wall,
        exit_code: status.code().unwrap_or(-1),
        stdout,
        cpu_ms,
        peak_rss_bytes,
    })
}

fn sample(pid: sysinfo::Pid, stop: &AtomicBool) -> (u64, f64) {
    let mut system = sysinfo::System::new();
    let mut peak_rss_bytes = 0_u64;
    let mut cpu_ms = 0.0_f64;
    let mut sampled_at = Instant::now();
    while !stop.load(Ordering::Relaxed) {
        system.refresh_processes(sysinfo::ProcessesToUpdate::Some(&[pid]));
        let Some(process) = system.process(pid) else {
            break;
        };
        peak_rss_bytes = peak_rss_bytes.max(process.memory());
        let now = Instant::now();
        let elapsed_ms = now.duration_since(sampled_at).as_secs_f64() * 1000.0;
        cpu_ms += f64::from(process.cpu_usage()) / 100.0 * elapsed_ms;
        sampled_at = now;
        thread::sleep(Duration::from_millis(25));
    }
    (peak_rss_bytes, cpu_ms)
}
