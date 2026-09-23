//! M:N parallelism tests for `vut(...)`.
//!
//! Parallelism is proven with runtime instrumentation and a synchronization
//! barrier, never with timing: each callback blocks until the other has started,
//! and the runtime records how many polls ever overlapped. A single-thread
//! scheduler can never satisfy the barrier (it times out) and records
//! `max_running == 1`.
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    sync::{
        OnceLock,
        atomic::{AtomicU64, Ordering},
    },
    time::SystemTime,
};

use vut_compiler::{CompilerConfig, CompilerSession};

static SEQUENCE: AtomicU64 = AtomicU64::new(0);

fn runtime_library() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/debug/libvut_runtime.rlib")
}

fn scratch(prefix: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .expect("valid clock")
        .as_nanos();
    let sequence = SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let root = std::env::temp_dir().join(format!("{prefix}-{nonce}-{sequence}"));
    fs::create_dir_all(&root).expect("create scratch");
    root
}

/// Native fixture: a CPU-bound barrier plus counters.
///
/// `native_barrier(expected, timeout_ms)` increments the number of *active*
/// callbacks and spins until `expected` are active at the same time (or the
/// timeout elapses). Two callbacks only both proceed if they overlap, so the
/// recorded peak is 1 on a single worker and 2 when two workers run in parallel.
const RUST_FIXTURE: &str = r#"
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

static ACTIVE: AtomicUsize = AtomicUsize::new(0);
static MAX_ACTIVE: AtomicUsize = AtomicUsize::new(0);

#[no_mangle]
pub extern "C" fn native_barrier(expected: usize, timeout_ms: u64) {
    let now = ACTIVE.fetch_add(1, Ordering::SeqCst) + 1;
    MAX_ACTIVE.fetch_max(now, Ordering::SeqCst);
    let deadline = Instant::now() + Duration::from_millis(timeout_ms);
    while ACTIVE.load(Ordering::SeqCst) < expected && Instant::now() < deadline {
        std::hint::spin_loop();
    }
    ACTIVE.fetch_sub(1, Ordering::SeqCst);
}

#[no_mangle]
pub extern "C" fn native_reset() {
    ACTIVE.store(0, Ordering::SeqCst);
    MAX_ACTIVE.store(0, Ordering::SeqCst);
}

#[no_mangle]
pub extern "C" fn native_max_in_flight() -> usize {
    MAX_ACTIVE.load(Ordering::SeqCst)
}
"#;

fn fixture() -> &'static Path {
    static LIB: OnceLock<PathBuf> = OnceLock::new();
    LIB.get_or_init(|| {
        let root = scratch("vut-parallel-native");
        let source = root.join("native.rs");
        fs::write(&source, RUST_FIXTURE).expect("write fixture");
        let library = root.join(if cfg!(windows) {
            "vut_parallel_native.lib"
        } else {
            "libvut_parallel_native.a"
        });
        let status = Command::new("rustc")
            .args(["--crate-type=staticlib", "--edition", "2021", "-O"])
            .arg("-o")
            .arg(&library)
            .arg(&source)
            .status()
            .expect("run rustc");
        assert!(status.success(), "parallel fixture failed to compile");
        library
    })
    .as_path()
}

fn run_with_env(source: &str, maxprocs: Option<&str>) -> (Option<i32>, String) {
    let root = scratch("vut-parallel-e2e");
    fs::write(root.join("main.vut"), source).expect("write source");
    let executable = root.join(if cfg!(windows) {
        "program.exe"
    } else {
        "program"
    });
    let config = CompilerConfig {
        runtime_library: Some(runtime_library()),
        native_libraries: vec![fixture().to_owned()],
        ..CompilerConfig::default()
    };
    CompilerSession::new(config)
        .emit_executable(&root, &[], &executable)
        .expect("emit executable");
    let mut command = Command::new(&executable);
    if let Some(value) = maxprocs {
        command.env("VUT_MAXPROCS", value);
    }
    let output = command.output().expect("run executable");
    let stdout = String::from_utf8(output.stdout).expect("utf-8 stdout");
    fs::remove_dir_all(&root).ok();
    (output.status.code(), stdout)
}

const BARRIER_PROGRAM: &str = "extern \"C\" fn native_reset()\nextern \"C\" fn native_barrier(expected: usize, timeout_ms: usize)\nextern \"C\" fn native_max_in_flight() -> usize\nextern \"C\" fn vut_rt_scheduler_max_running_v1() -> usize\n\nasync fn main():\n  unsafe:\n    native_reset()\n    a = vut(fn():\n      unsafe:\n        native_barrier(2, 300)\n    )\n    b = vut(fn():\n      unsafe:\n        native_barrier(2, 300)\n    )\n    await a\n    await b\n    out(\"in_flight=$(native_max_in_flight()) max_running=$(vut_rt_scheduler_max_running_v1())\")\n";

fn parse_value(stdout: &str, key: &str) -> usize {
    stdout
        .split_whitespace()
        .find_map(|token| token.strip_prefix(&format!("{key}=")))
        .and_then(|value| value.trim().parse().ok())
        .unwrap_or_else(|| panic!("missing `{key}` in {stdout:?}"))
}

#[test]
fn single_worker_runs_tasks_serially() {
    let (code, stdout) = run_with_env(BARRIER_PROGRAM, Some("1"));
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(
        parse_value(&stdout, "in_flight"),
        1,
        "one worker must never run two tasks at once: {stdout:?}"
    );
    assert_eq!(
        parse_value(&stdout, "max_running"),
        1,
        "one worker must record a single concurrent poll: {stdout:?}"
    );
}

#[test]
fn multiple_workers_run_tasks_in_parallel() {
    let parallelism = std::thread::available_parallelism().map_or(1, std::num::NonZeroUsize::get);
    if parallelism < 2 {
        // A single logical CPU cannot demonstrate true parallel progress; the
        // single-worker test above still covers the scheduler invariant.
        println!("skipping parallel assertion: only {parallelism} logical CPU(s) available");
        return;
    }
    let (code, stdout) = run_with_env(BARRIER_PROGRAM, Some("2"));
    assert_eq!(code, Some(0), "{stdout}");
    assert!(
        parse_value(&stdout, "in_flight") >= 2,
        "two workers must overlap the barrier: {stdout:?}"
    );
    assert!(
        parse_value(&stdout, "max_running") >= 2,
        "runtime must record two concurrent polls: {stdout:?}"
    );
}

#[test]
fn many_parallel_tasks_complete_and_release() {
    let source = "fn work(value: int) -> int:\n  value * 2\n\nasync fn main():\n  a = vut(() => work(1))\n  b = vut(() => work(2))\n  c = vut(() => work(3))\n  d = vut(() => work(4))\n  sum = await a + await b + await c + await d\n  out(\"sum=$sum\")\n";
    let (code, stdout) = run_with_env(source, None);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "sum=20\n");
}

#[test]
fn stress_spawn_and_drop_many_tasks() {
    // 16 tasks are spawned but never awaited; `job` is awaited. The executor
    // must stop after `job` completes and drain the rest without leaking,
    // double-freeing, or racing a drop against an in-flight poll.
    let source = "fn work(value: int) -> int:\n  value + 1\n\nasync fn main():\n  items = @[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15]\n  for item in items:\n    vut(() => work(1))\n  job = vut(() => work(41))\n  out(\"v=$(await job)\")\n";
    let (code, stdout) = run_with_env(source, None);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "v=42\n");
}

const BENCH_PROGRAM: &str = "extern \"C\" fn native_reset()\nextern \"C\" fn native_barrier(expected: usize, timeout_ms: usize)\nextern \"C\" fn native_max_in_flight() -> usize\nextern \"C\" fn vut_rt_scheduler_max_running_v1() -> usize\n\nasync fn main():\n  unsafe:\n    native_reset()\n    a = vut(fn():\n      unsafe:\n        native_barrier(4, 500)\n    )\n    b = vut(fn():\n      unsafe:\n        native_barrier(4, 500)\n    )\n    c = vut(fn():\n      unsafe:\n        native_barrier(4, 500)\n    )\n    d = vut(fn():\n      unsafe:\n        native_barrier(4, 500)\n    )\n    await a\n    await b\n    await c\n    await d\n    out(\"in_flight=$(native_max_in_flight()) max_running=$(vut_rt_scheduler_max_running_v1())\")\n";

#[test]
#[ignore = "benchmark; run with --ignored --nocapture"]
fn benchmark_worker_scaling() {
    for procs in ["1", "2", "4", "8"] {
        let start = std::time::Instant::now();
        let (code, stdout) = run_with_env(BENCH_PROGRAM, Some(procs));
        let elapsed = start.elapsed();
        println!(
            "VUT_MAXPROCS={procs} code={code:?} elapsed={elapsed:?} {}",
            stdout.trim()
        );
        assert_eq!(code, Some(0), "{stdout}");
    }
}
