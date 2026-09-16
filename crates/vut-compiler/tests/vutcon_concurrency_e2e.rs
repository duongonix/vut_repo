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

/// Native fixture: a rendezvous async operation plus counters, so Vut-side
/// concurrency and cleanup can be observed without depending on std/http.
const RUST_FIXTURE: &str = r#"
use std::ffi::c_void;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::time::Duration;

type PollFn = unsafe extern "C" fn(*mut c_void, *mut u8) -> i32;
type DropFn = unsafe extern "C" fn(*mut c_void);

extern "C" {
    fn vut_rt_async_new_v1(op: *mut c_void, poll_fn: PollFn, drop_fn: DropFn) -> *mut c_void;
    fn vut_rt_async_signal_v1(handle: *mut c_void);
}

struct Gate {
    ready: AtomicBool,
}

static STARTED: AtomicUsize = AtomicUsize::new(0);
static MAX_IN_FLIGHT: AtomicUsize = AtomicUsize::new(0);
static COMPLETED: AtomicUsize = AtomicUsize::new(0);
static OP_DROPPED: AtomicUsize = AtomicUsize::new(0);

unsafe extern "C" fn poll_gate(op: *mut c_void, out: *mut u8) -> i32 {
    let gate = unsafe { &*(op as *const Gate) };
    if gate.ready.load(Ordering::SeqCst) {
        unsafe { *(out as *mut i32) = 99; }
        COMPLETED.fetch_add(1, Ordering::SeqCst);
        1
    } else {
        0
    }
}

unsafe extern "C" fn drop_gate(op: *mut c_void) {
    OP_DROPPED.fetch_add(1, Ordering::SeqCst);
    drop(unsafe { Box::from_raw(op as *mut Gate) });
}

/// An async operation that only completes once two of them have started, so a
/// serialized execution would time out instead of overlapping.
#[no_mangle]
pub extern "C" fn native_gate() -> *mut c_void {
    let gate = Box::into_raw(Box::new(Gate { ready: AtomicBool::new(false) }));
    let handle = unsafe { vut_rt_async_new_v1(gate.cast(), poll_gate, drop_gate) };
    let raw = handle as usize;
    let gate_ptr = gate as usize;
    std::thread::spawn(move || {
        let now = STARTED.fetch_add(1, Ordering::SeqCst) + 1;
        MAX_IN_FLIGHT.fetch_max(now, Ordering::SeqCst);
        let deadline = std::time::Instant::now() + Duration::from_millis(1000);
        while STARTED.load(Ordering::SeqCst) < 2 && std::time::Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(1));
        }
        unsafe { (*(gate_ptr as *mut Gate)).ready.store(true, Ordering::SeqCst); }
        unsafe { vut_rt_async_signal_v1(raw as *mut c_void) };
        STARTED.fetch_sub(1, Ordering::SeqCst);
    });
    handle
}

#[no_mangle]
pub extern "C" fn native_reset() {
    STARTED.store(0, Ordering::SeqCst);
    MAX_IN_FLIGHT.store(0, Ordering::SeqCst);
    COMPLETED.store(0, Ordering::SeqCst);
    OP_DROPPED.store(0, Ordering::SeqCst);
}

#[no_mangle]
pub extern "C" fn native_max_in_flight() -> usize {
    MAX_IN_FLIGHT.load(Ordering::SeqCst)
}

#[no_mangle]
pub extern "C" fn native_completed() -> usize {
    COMPLETED.load(Ordering::SeqCst)
}

#[no_mangle]
pub extern "C" fn native_op_dropped() -> usize {
    OP_DROPPED.load(Ordering::SeqCst)
}
"#;

fn fixture() -> &'static Path {
    static LIB: OnceLock<PathBuf> = OnceLock::new();
    LIB.get_or_init(|| {
        let root = scratch("vut-vutcon-native");
        let source = root.join("native.rs");
        fs::write(&source, RUST_FIXTURE).expect("write fixture");
        let library = root.join(if cfg!(windows) {
            "vutcon_native.lib"
        } else {
            "libvutcon_native.a"
        });
        let status = Command::new("rustc")
            .args(["--crate-type=staticlib", "--edition", "2021", "-O"])
            .arg("-o")
            .arg(&library)
            .arg(&source)
            .status()
            .expect("run rustc");
        assert!(status.success(), "vutcon fixture failed to compile");
        library
    })
    .as_path()
}

fn run(source: &str) -> (Option<i32>, String) {
    let root = scratch("vut-vutcon-e2e");
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
    let output = Command::new(&executable).output().expect("run executable");
    let stdout = String::from_utf8(output.stdout).expect("utf-8 stdout");
    fs::remove_dir_all(&root).ok();
    (output.status.code(), stdout)
}

#[test]
fn sync_vutcon_runs_on_first_poll() {
    let source = "fn calc() -> int:\n  41\n\nasync fn main():\n  job = vut(fn():\n    calc() + 1\n  )\n  out(\"v=$(await job)\")\n";
    let (code, stdout) = run(source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "v=42\n");
}

#[test]
fn async_vutcon_suspends_and_resumes() {
    let source = "extern \"C\" async fn native_gate() -> i32\n\nasync fn main():\n  unsafe:\n    job = vut(async fn():\n      unsafe:\n        return await native_gate()\n    )\n    out(\"v=$(await job)\")\n";
    let (code, stdout) = run(source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "v=99\n");
}

#[test]
fn async_vutcon_awaits_another_vutcon() {
    let source = "extern \"C\" async fn native_gate() -> i32\n\nfn bump(v: i32) -> i32:\n  v + 1\n\nasync fn nested() -> i32:\n  inner = vut(async fn():\n    unsafe:\n      return await native_gate()\n  )\n  value = await inner\n  bump(value)\n\nasync fn main():\n  outer = vut(async fn():\n    return await nested()\n  )\n  out(\"v=$(await outer)\")\n";
    let (code, stdout) = run(source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "v=100\n");
}

#[test]
fn multiple_vutcons_run_and_await_independently() {
    let source = "fn one() -> int:\n  1\nfn two() -> int:\n  2\nfn three() -> int:\n  3\n\nasync fn main():\n  a = vut(() => one())\n  b = vut(() => two())\n  c = vut(() => three())\n  sum = await a + await b + await c\n  out(\"sum=$sum\")\n";
    let (code, stdout) = run(source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "sum=6\n");
}

#[test]
fn vutcons_overlap_while_suspended() {
    let source = "extern \"C\" async fn native_gate() -> i32\nextern \"C\" fn native_reset()\nextern \"C\" fn native_max_in_flight() -> usize\n\nasync fn main():\n  unsafe:\n    native_reset()\n    a = vut(async fn():\n      unsafe:\n        return await native_gate()\n    )\n    b = vut(async fn():\n      unsafe:\n        return await native_gate()\n    )\n    x = await a\n    y = await b\n    out(\"x=$x y=$y max=$(native_max_in_flight())\")\n";
    let (code, stdout) = run(source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "x=99 y=99 max=2\n");
}

#[test]
fn vutcon_result_and_drop_are_exactly_once() {
    let source = "extern \"C\" async fn native_gate() -> i32\nextern \"C\" fn native_completed() -> usize\nextern \"C\" fn native_op_dropped() -> usize\n\nasync fn main():\n  unsafe:\n    job = vut(async fn():\n      unsafe:\n        return await native_gate()\n    )\n    value = await job\n    out(\"v=$value completed=$(native_completed()) dropped=$(native_op_dropped())\")\n";
    let (code, stdout) = run(source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "v=99 completed=1 dropped=1\n");
}

#[test]
fn async_vutcon_error_path_propagates_result() {
    let source = "async fn fail() -> result(int, int):\n  err(5)\n\nasync fn main():\n  job = vut(async fn():\n    return await fail()\n  )\n  match await job:\n    ok(v): out(\"ok=$v\")\n    err(e): out(\"err=$e\")\n";
    let (code, stdout) = run(source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "err=5\n");
}

#[test]
fn dropped_scheduled_handle_is_cleaned_up() {
    let source = "extern \"C\" async fn native_gate() -> i32\nextern \"C\" fn native_op_dropped() -> usize\n\nasync fn main():\n  unsafe:\n    vut(async fn():\n      unsafe:\n        return await native_gate()\n    )\n    out(\"dropped=$(native_op_dropped())\")\n";
    let (code, stdout) = run(source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "dropped=0\n");
}
