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
    let root = std::env::temp_dir().join(format!("vut-vutcon-e2e-{nonce}-{sequence}"));
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
fn spawn_and_await_scalar_results() {
    let source = "fn calc() -> int:\n  20\n\nasync fn main():\n  a = vut(() => calc())\n  b = vut(fn():\n    value = calc()\n    value * 2\n  )\n  x = await a\n  y = await b\n  out(\"$(x + y)\")\n";
    let (code, stdout) = run(source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "60\n");
}

#[test]
fn spawn_and_await_str_handle() {
    let source = "fn text() -> str:\n  \"hi\"\n\nasync fn main():\n  t: vutcon[str] = vut(() => text())\n  out(\"s=$(await t)\")\n";
    let (code, stdout) = run(source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "s=hi\n");
}

#[test]
fn spawn_and_await_result_handle_with_propagation() {
    let source = "fn load() -> result[int, int]:\n  ok(7)\n\nasync fn get() -> result[int, int]:\n  job: vutcon[result[int, int]] = vut(() => load())\n  inner = await job?\n  ok(inner)\n\nasync fn main():\n  match await get():\n    ok(v): out(\"ok=$v\")\n    err(e): out(\"err=$e\")\n";
    let (code, stdout) = run(source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "ok=7\n");
}

#[test]
fn discarded_handle_is_cleaned_up() {
    // Eager scheduling means a discarded handle may run once the root yields,
    // so this only asserts the handle is released and no leak is reported.
    let source =
        "fn side() -> int:\n  1\n\nasync fn main():\n  vut(() => side())\n  out(\"done\")\n";
    let (code, stdout) = run(source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "done\n");
}

#[test]
fn awaiting_a_handle_twice_is_rejected() {
    let nonce = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .expect("valid clock")
        .as_nanos();
    let root = std::env::temp_dir().join(format!("vut-vutcon-twice-{nonce}"));
    fs::create_dir(&root).expect("create root");
    fs::write(
        root.join("main.vut"),
        "fn calc() -> int:\n  1\n\nasync fn main():\n  a = vut(() => calc())\n  x = await a\n  y = await a\n  out(\"$(x + y)\")\n",
    )
    .expect("write source");
    let checked = CompilerSession::new(CompilerConfig::default())
        .check_source_root(&root, &[])
        .expect("check source");
    fs::remove_dir_all(&root).ok();
    let codes: Vec<String> = checked
        .semantics
        .diagnostics
        .as_slice()
        .iter()
        .filter_map(|diagnostic| diagnostic.code.as_deref().map(str::to_owned))
        .collect();
    assert!(
        codes.iter().any(|code| code == "E8009" || code == "E8010"),
        "{codes:?}"
    );
}
