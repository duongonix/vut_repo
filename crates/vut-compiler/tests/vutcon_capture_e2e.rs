//! `vut(...)` closure capture: a spawned task must receive its environment and
//! own it until the task completes or is dropped.
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    sync::atomic::{AtomicU64, Ordering},
    time::SystemTime,
};

use vut_compiler::{CompilerConfig, CompilerSession};

static SEQUENCE: AtomicU64 = AtomicU64::new(0);

fn runtime_library() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/debug/libvut_runtime.rlib")
}

fn run(source: &str) -> (Option<i32>, String) {
    let nonce = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .expect("valid clock")
        .as_nanos();
    let sequence = SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let root = std::env::temp_dir().join(format!("vut-capture-e2e-{nonce}-{sequence}"));
    fs::create_dir(&root).expect("create root");
    fs::write(root.join("main.vut"), source).expect("write source");
    let executable = root.join(if cfg!(windows) {
        "program.exe"
    } else {
        "program"
    });
    let config = CompilerConfig {
        runtime_library: Some(runtime_library()),
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
fn sync_vut_callable_captures_scalar() {
    let source =
        "async fn main():\n  value = 42\n  job = vut(fn():\n    out(value)\n  )\n  await job\n";
    let (code, stdout) = run(source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "42\n");
}

#[test]
fn sync_vut_callable_captures_str() {
    let source = "async fn main():\n  name = \"hello\"\n  job = vut(fn():\n    out(\"name=$name\")\n  )\n  await job\n";
    let (code, stdout) = run(source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "name=hello\n");
}

#[test]
fn sync_vut_callable_captures_list() {
    let source = "async fn main():\n  xs = @[1, 2, 3]\n  job = vut(fn():\n    out(\"len=$(xs.len())\")\n  )\n  await job\n";
    let (code, stdout) = run(source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "len=3\n");
}

#[test]
fn async_vut_callable_captures_and_awaits() {
    let source = "async fn load() -> int:\n  7\n\nasync fn main():\n  value = 40\n  job = vut(async fn():\n    extra = await load()\n    out(\"sum=$(value + extra)\")\n  )\n  await job\n";
    let (code, stdout) = run(source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "sum=47\n");
}

#[test]
fn capture_outlives_parent_scope() {
    // The environment must stay alive after `main` stops using the local; the
    // spawned task owns the captured value.
    let source = "async fn main():\n  job = vut(fn():\n    text = \"kept\"\n    out(text)\n  )\n  await job\n";
    let (code, stdout) = run(source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "kept\n");
}

#[test]
fn captured_environment_is_released_after_completion() {
    let source = "extern \"C\" fn vut_rt_closure_live_count_v1() -> usize\n\nasync fn main():\n  job = vut(fn():\n    text = \"captured\"\n    out(text)\n  )\n  await job\n  unsafe:\n    out(\"live=$(vut_rt_closure_live_count_v1())\")\n";
    let (code, stdout) = run(source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "captured\nlive=0\n");
}

#[test]
fn captured_managed_value_is_released_after_completion() {
    let source = "extern \"C\" fn vut_rt_closure_live_count_v1() -> usize\n\nasync fn main():\n  xs = @[1, 2, 3]\n  job = vut(fn():\n    out(\"len=$(xs.len())\")\n  )\n  await job\n  unsafe:\n    out(\"live=$(vut_rt_closure_live_count_v1())\")\n";
    let (code, stdout) = run(source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "len=3\nlive=0\n");
}

#[test]
fn async_capture_is_released_after_completion() {
    let source = "extern \"C\" fn vut_rt_closure_live_count_v1() -> usize\n\nasync fn load() -> int:\n  1\n\nasync fn main():\n  base = 10\n  job = vut(async fn():\n    extra = await load()\n    out(\"sum=$(base + extra)\")\n  )\n  await job\n  unsafe:\n    out(\"live=$(vut_rt_closure_live_count_v1())\")\n";
    let (code, stdout) = run(source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "sum=11\nlive=0\n");
}

#[test]
fn dropping_a_pending_capturing_task_is_safe() {
    // `job` is never awaited: the executor drops the suspended task, running the
    // frame drop thunk (which releases the captured channel and the frame).
    let source = "async fn main():\n  ch = channel[int]()\n  job = vut(fn():\n    value = ch.recv()\n    if value != null:\n      out(\"unreachable\")\n  )\n  out(\"spawned\")\n";
    let (code, stdout) = run(source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "spawned\n");
}
