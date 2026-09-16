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
    let root = std::env::temp_dir().join(format!("vut-async-state-{nonce}-{sequence}"));
    fs::create_dir_all(&root).expect("create root");
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
fn awaits_a_vut_async_function() {
    let source = "async fn value() -> int:\n  7\n\nasync fn main():\n  out(\"$(await value())\")\n";
    let (code, stdout) = run(source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "7\n");
}

#[test]
fn awaits_multiple_sequential_async_functions() {
    let source = "async fn one() -> int:\n  1\nasync fn two() -> int:\n  2\n\nasync fn main():\n  total = (await one()) + (await two())\n  out(\"$(total)\")\n";
    let (code, stdout) = run(source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "3\n");
}

#[test]
fn keeps_a_managed_local_across_an_await() {
    let source = "async fn inner() -> int:\n  5\nasync fn work() -> str:\n  text = \"v=\"\n  n = await inner()\n  text + \"$n\"\n\nasync fn main():\n  out(\"$(await work())\")\n";
    let (code, stdout) = run(source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "v=5\n");
}

#[test]
fn awaits_inside_conditionals_and_loops() {
    let source = "async fn one() -> int:\n  1\nasync fn choose(flag: bool) -> int:\n  if flag:\n    return await one()\n  0\nasync fn total() -> int:\n  sum = 0\n  for index in @(0, 1, 2):\n    sum = sum + await one()\n  sum\n\nasync fn main():\n  out(\"$(await choose(true))\")\n  out(\"$(await total())\")\n";
    let (code, stdout) = run(source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "1\n3\n");
}

#[test]
fn propagates_async_result_with_question_operator() {
    let source = "async fn maybe(flag: bool) -> result(int, int):\n  if flag:\n    return ok(5)\n  err(9)\nasync fn work(flag: bool) -> result(int, int):\n  v = await maybe(flag)?\n  ok(v + 1)\n\nasync fn main():\n  match await work(true):\n    ok(v): out(\"ok=$v\")\n    err(e): out(\"err=$e\")\n  match await work(false):\n    ok(v): out(\"ok=$v\")\n    err(e): out(\"err=$e\")\n";
    let (code, stdout) = run(source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "ok=6\nerr=9\n");
}

#[test]
fn awaits_nested_async_functions() {
    let source = "async fn inner() -> int:\n  4\nasync fn outer() -> int:\n  (await inner()) * 2\n\nasync fn main():\n  out(\"$(await outer())\")\n";
    let (code, stdout) = run(source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "8\n");
}

#[test]
fn passes_parameters_through_the_future_frame() {
    let source = "async fn add(a: int, b: int) -> int:\n  a + b\nasync fn greet(name: str) -> str:\n  \"hi \" + name\n\nasync fn main():\n  total = await add(2, 3)\n  out(\"total=$total\")\n  name = \"vut\"\n  greeting = await greet(name)\n  out(\"greeting=$greeting\")\n";
    let (code, stdout) = run(source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "total=5\ngreeting=hi vut\n");
}

#[test]
fn passes_an_aggregate_receiver_through_the_frame() {
    let source = "data Counter:\n  value: int\nasync fn Counter.doubled() -> int:\n  self.value * 2\n\nasync fn main():\n  counter = Counter(value = 21)\n  out(\"$(await counter.doubled())\")\n";
    let (code, stdout) = run(source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "42\n");
}

#[test]
fn releases_future_frames_after_await() {
    // After the awaited future completes its frame is released; the only frame
    // still live is `async main`'s own root frame.
    let source = "extern \"C\" fn vut_rt_frame_live_count_v1() -> usize\nasync fn value() -> int:\n  7\n\nasync fn main():\n  unsafe:\n    result = await value()\n    out(\"value=$result\")\n    out(\"frames=$(vut_rt_frame_live_count_v1())\")\n";
    let (code, stdout) = run(source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "value=7\nframes=1\n");
}
