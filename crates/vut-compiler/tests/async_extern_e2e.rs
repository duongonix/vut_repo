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

const RUST_FIXTURE: &str = r#"
use std::ffi::c_void;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

type PollFn = unsafe extern "C" fn(*mut c_void, *mut u8) -> i32;
type DropFn = unsafe extern "C" fn(*mut c_void);

extern "C" {
    fn vut_rt_async_new_v1(op: *mut c_void, poll_fn: PollFn, drop_fn: DropFn) -> *mut c_void;
    fn vut_rt_async_signal_v1(handle: *mut c_void);
}

static READY: AtomicBool = AtomicBool::new(false);
static DROPPED: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

unsafe extern "C" fn poll_value(_op: *mut c_void, out: *mut u8) -> i32 {
    unsafe { *(out as *mut i32) = 42; }
    1
}

unsafe extern "C" fn drop_value(_op: *mut c_void) {
    DROPPED.fetch_add(1, Ordering::SeqCst);
}

#[no_mangle]
pub extern "C" fn native_async_value() -> *mut c_void {
    unsafe { vut_rt_async_new_v1(std::ptr::null_mut(), poll_value, drop_value) }
}

unsafe extern "C" fn poll_delayed(_op: *mut c_void, out: *mut u8) -> i32 {
    if READY.load(Ordering::SeqCst) {
        unsafe { *(out as *mut i32) = 7; }
        1
    } else {
        0
    }
}

#[no_mangle]
pub extern "C" fn native_async_delayed() -> *mut c_void {
    READY.store(false, Ordering::SeqCst);
    let handle = unsafe { vut_rt_async_new_v1(std::ptr::null_mut(), poll_delayed, drop_value) };
    let raw = handle as usize;
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(10));
        READY.store(true, Ordering::SeqCst);
        unsafe { vut_rt_async_signal_v1(raw as *mut c_void) };
    });
    handle
}

#[no_mangle]
pub extern "C" fn native_async_dropped() -> usize {
    DROPPED.load(Ordering::SeqCst)
}

#[no_mangle]
pub extern "C" fn native_one() -> i32 {
    1
}
"#;

fn fixture() -> &'static Path {
    static LIB: OnceLock<PathBuf> = OnceLock::new();
    LIB.get_or_init(|| {
        let root = scratch("vut-async-native");
        let source = root.join("native.rs");
        fs::write(&source, RUST_FIXTURE).expect("write fixture");
        let library = root.join(if cfg!(windows) {
            "async_native.lib"
        } else {
            "libasync_native.a"
        });
        let status = Command::new("rustc")
            .args(["--crate-type=staticlib", "--edition", "2021", "-O"])
            .arg("-o")
            .arg(&library)
            .arg(&source)
            .status()
            .expect("run rustc");
        assert!(status.success(), "async fixture failed to compile");
        library
    })
    .as_path()
}

fn run(source: &str) -> (Option<i32>, String) {
    let root = scratch("vut-async-e2e");
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
fn awaits_an_immediately_ready_native_future() {
    let source = "extern \"C\" async fn native_async_value() -> i32\n\nasync fn main():\n  unsafe:\n    value = await native_async_value()\n    out(\"value=$value\")\n";
    let (code, stdout) = run(source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "value=42\n");
}

#[test]
fn awaits_a_native_future_woken_from_another_thread() {
    let source = "extern \"C\" async fn native_async_delayed() -> i32\n\nasync fn main():\n  unsafe:\n    value = await native_async_delayed()\n    out(\"value=$value\")\n";
    let (code, stdout) = run(source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "value=7\n");
}

#[test]
fn dropping_an_unawaited_future_cancels_it() {
    let source = "extern \"C\" async fn native_async_value() -> i32\nextern \"C\" fn native_async_dropped() -> usize\n\nfn make_and_drop():\n  unsafe:\n    pending = native_async_value()\n    out(\"made\")\n\nasync fn main():\n  make_and_drop()\n  unsafe:\n    out(\"dropped=$(native_async_dropped())\")\n";
    let (code, stdout) = run(source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "made\ndropped=1\n");
}

#[test]
fn async_body_suspends_twice_and_resumes_with_live_locals() {
    let source = "extern \"C\" async fn native_async_delayed() -> i32\n\nasync fn combine(base: i32) -> i32:\n  total: i32 = 0\n  unsafe:\n    first = await native_async_delayed()\n    second = await native_async_delayed()\n    total = base + first + second\n  total\n\nasync fn main():\n  unsafe:\n    out(\"sum=$(await combine(2))\")\n";
    let (code, stdout) = run(source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "sum=16\n");
}

#[test]
fn await_result_before_a_native_suspension_survives_resume() {
    let source = "async fn simple() -> i32:\n  3\nextern \"C\" async fn native_async_delayed() -> i32\n\nasync fn chain() -> i32:\n  a = await simple()\n  b: i32 = 0\n  unsafe:\n    b = await native_async_delayed()\n  c = await simple()\n  a + b + c\n\nasync fn main():\n  unsafe:\n    out(\"chain=$(await chain())\")\n";
    let (code, stdout) = run(source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "chain=13\n");
}

#[test]
fn suspends_inside_a_conditional_loop_and_resumes() {
    let source = "extern \"C\" async fn native_async_delayed() -> i32\n\nasync fn countdown() -> i32:\n  total: i32 = 0\n  for total < 3:\n    unsafe:\n      await native_async_delayed()\n    total = total + 1\n  total\n\nasync fn main():\n  unsafe:\n    out(\"count=$(await countdown())\")\n";
    let (code, stdout) = run(source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "count=3\n");
}

#[test]
fn suspends_inside_an_if_branch_and_resumes() {
    let source = "extern \"C\" async fn native_async_delayed() -> i32\nextern \"C\" fn native_one() -> i32\n\nasync fn branch(flag: bool) -> i32:\n  value: i32 = 0\n  if flag:\n    unsafe:\n      value = await native_async_delayed()\n  else:\n    unsafe:\n      value = native_one()\n  value\n\nasync fn main():\n  unsafe:\n    out(\"a=$(await branch(true)) b=$(await branch(false))\")\n";
    let (code, stdout) = run(source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "a=7 b=1\n");
}

#[test]
fn spills_a_value_crossing_a_native_suspension() {
    let source = "extern \"C\" async fn native_async_delayed() -> i32\n\nfn add(a: i32, b: i32) -> i32:\n  a + b\n\nasync fn work() -> i32:\n  unsafe:\n    add(5, await native_async_delayed())\n\nasync fn main():\n  unsafe:\n    out(\"work=$(await work())\")\n";
    let (code, stdout) = run(source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "work=12\n");
}

#[test]
fn suspends_inside_an_iterable_literal_loop_and_resumes() {
    let source = "extern \"C\" async fn native_async_delayed() -> i32\n\nasync fn total_of() -> i32:\n  total: i32 = 0\n  for value in @[1, 2, 3]:\n    delta: i32 = 0\n    unsafe:\n      delta = await native_async_delayed()\n    total = total + delta\n  total\n\nasync fn main():\n  unsafe:\n    out(\"loop=$(await total_of())\")\n";
    let (code, stdout) = run(source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "loop=21\n");
}

#[test]
fn suspends_inside_a_match_arm_and_resumes() {
    let source = "enum Kind:\n  yes\n  no\n\nextern \"C\" async fn native_async_delayed() -> i32\n\nasync fn delayed() -> i32:\n  value: i32 = 0\n  unsafe:\n    value = await native_async_delayed()\n  value\n\nasync fn zero() -> i32:\n  0\n\nasync fn pick(kind: Kind) -> i32:\n  match kind:\n    yes: await delayed()\n    no: await zero()\n\nasync fn main():\n  unsafe:\n    out(\"yes=$(await pick(Kind.yes)) no=$(await pick(Kind.no))\")\n";
    let (code, stdout) = run(source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "yes=7 no=0\n");
}

#[test]
fn early_return_after_a_native_suspension() {
    let source = "extern \"C\" async fn native_async_delayed() -> i32\n\nasync fn value_or(flag: bool) -> i32:\n  if flag:\n    value: i32 = 0\n    unsafe:\n      value = await native_async_delayed()\n    return value\n  99\n\nasync fn main():\n  unsafe:\n    out(\"a=$(await value_or(true)) b=$(await value_or(false))\")\n";
    let (code, stdout) = run(source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "a=7 b=99\n");
}
