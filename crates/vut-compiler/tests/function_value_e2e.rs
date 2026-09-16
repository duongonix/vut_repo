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
    let root = std::env::temp_dir().join(format!("vut-function-value-e2e-{nonce}-{sequence}"));
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

fn diagnostic_codes(source: &str) -> Vec<String> {
    let nonce = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .expect("valid clock")
        .as_nanos();
    let root = std::env::temp_dir().join(format!("vut-function-value-diag-{nonce}"));
    fs::create_dir(&root).expect("create root");
    let path = root.join("main.vut");
    fs::write(&path, source).expect("write source");
    let mut session = CompilerSession::new(CompilerConfig::default());
    let checked = session.check_source_file(Path::new(&path)).expect("check");
    let codes = checked
        .semantics
        .diagnostics
        .as_slice()
        .iter()
        .filter_map(|diagnostic| diagnostic.code.as_deref().map(str::to_owned))
        .collect();
    fs::remove_dir_all(&root).ok();
    codes
}

#[test]
fn named_function_used_as_a_value() {
    let source = "fn double(x: int) -> int:\n  x * 2\nfn main():\n  f = double\n  result = f(3)\n  out(\"result=$result\")\n";
    let (code, stdout) = run(source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "result=6\n");
}

#[test]
fn calling_the_result_of_a_call_directly() {
    let source = "fn make() -> fn(int) -> int:\n  fn(x):\n    x + 1\n\nfn main():\n  result = make()(5)\n  out(\"result=$result\")\n";
    let (code, stdout) = run(source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "result=6\n");
}

#[test]
fn chained_calls_through_a_local_binding() {
    let source = "fn make() -> fn(int) -> int:\n  fn(x):\n    x + 1\n\nfn main():\n  temp = make()\n  result = temp(5)\n  out(\"result=$result\")\n";
    let (code, stdout) = run(source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "result=6\n");
}

#[test]
fn multi_level_chained_calls() {
    let source = "fn outer() -> fn(int) -> fn(str) -> str:\n  fn(a):\n    fn(s):\n      s\n\nfn main():\n  result = outer()(1)(\"x\")\n  out(\"result=$result\")\n";
    let (code, stdout) = run(source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "result=x\n");
}

#[test]
fn calling_a_non_callable_result_is_an_error() {
    let codes = diagnostic_codes(
        "fn five() -> int:\n  5\nfn main():\n  result = five()(1)\n  out(\"$result\")\n",
    );
    assert!(codes.iter().any(|code| code == "E1014"), "{codes:?}");
}

#[test]
fn non_callable_result_at_a_deeper_level_is_an_error() {
    let codes = diagnostic_codes(
        "fn make() -> fn(int) -> int:\n  fn(x):\n    x + 1\nfn main():\n  result = make()(5)(3)\n  out(\"$result\")\n",
    );
    assert!(codes.iter().any(|code| code == "E1014"), "{codes:?}");
}

#[test]
fn method_returning_a_function_is_chained() {
    let source = "data Factory:\n  unused: int\n\nfn Factory.build() -> fn(int) -> int:\n  fn(x):\n    x + 1\n\nfn main():\n  factory = Factory(unused = 0)\n  result = factory.build()(5)\n  out(\"result=$result\")\n";
    let (code, stdout) = run(source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "result=6\n");
}

#[test]
fn generic_function_returns_a_callable_value() {
    let source = "fn double(x: int) -> int:\n  x * 2\n\nfn identity(T)(value: T) -> T:\n  value\n\nfn select() -> fn(int) -> int:\n  identity(double)\n\nfn main():\n  result = select()(6)\n  out(\"result=$result\")\n";
    let (code, stdout) = run(source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "result=12\n");
}

#[test]
fn function_stored_in_a_data_field_is_callable() {
    let source = "fn double(x: int) -> int:\n  x * 2\n\ndata Holder:\n  op: fn(int) -> int\n\nfn main():\n  holder = Holder(op = double)\n  result = holder.op(5)\n  out(\"result=$result\")\n";
    let (code, stdout) = run(source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "result=10\n");
}

#[test]
fn chained_argument_and_deep_chain() {
    let source = "fn double(x: int) -> int:\n  x * 2\n\nfn build() -> fn(int) -> fn(int) -> int:\n  fn(a):\n    fn(b):\n      b + 1\n\nfn apply(value: int, f: fn(int) -> int) -> int:\n  f(value)\n\nfn main():\n  deep = build()(9)(4)\n  out(\"deep=$deep\")\n  arg = apply(10, build()(0))\n  out(\"arg=$arg\")\n  value = apply(3, double)\n  out(\"value=$value\")\n";
    let (code, stdout) = run(source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "deep=5\narg=11\nvalue=6\n");
}
