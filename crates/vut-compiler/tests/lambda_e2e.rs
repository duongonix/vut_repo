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
    let root = std::env::temp_dir().join(format!("vut-lambda-e2e-{nonce}-{sequence}"));
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

fn check_errors(source: &str) -> Vec<String> {
    let nonce = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .expect("valid clock")
        .as_nanos();
    let sequence = SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let root = std::env::temp_dir().join(format!("vut-lambda-check-{nonce}-{sequence}"));
    fs::create_dir(&root).expect("create root");
    fs::write(root.join("main.vut"), source).expect("write source");
    let checked = CompilerSession::new(CompilerConfig::default())
        .check_source_root(&root, &[])
        .expect("check source");
    fs::remove_dir_all(&root).ok();
    let mut codes: Vec<String> = checked
        .resolution
        .diagnostics
        .as_slice()
        .iter()
        .filter_map(|diagnostic| diagnostic.code.as_deref().map(str::to_owned))
        .collect();
    codes.extend(
        checked
            .semantics
            .diagnostics
            .as_slice()
            .iter()
            .filter_map(|diagnostic| diagnostic.code.as_deref().map(str::to_owned)),
    );
    codes
}

#[test]
fn arrow_lambda_single_parameter() {
    let source = "fn apply(value: int, callback: fn(int) -> int) -> int:\n  callback(value)\nfn main():\n  out(\"$(apply(10, x => x * 2))\")\n";
    let (code, stdout) = run(source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "20\n");
}

#[test]
fn arrow_lambda_multiple_parameters() {
    let source = "fn combine(a: int, b: int, callback: fn(int, int) -> int) -> int:\n  callback(a, b)\nfn main():\n  out(\"$(combine(3, 4, (a, b) => a + b))\")\n";
    let (code, stdout) = run(source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "7\n");
}

#[test]
fn arrow_lambda_without_parameters() {
    let source = "fn hello(callback: fn()) -> void:\n  callback()\nfn main():\n  hello(() => out(\"Hello\"))\n";
    let (code, stdout) = run(source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "Hello\n");
}

#[test]
fn multiline_fn_lambda_returns_final_expression() {
    let source = "fn apply(value: int, callback: fn(int) -> int) -> int:\n  callback(value)\nfn main():\n  result = apply(10, fn(x):\n    value = x * 2\n    value + 10\n  )\n  out(\"$result\")\n";
    let (code, stdout) = run(source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "30\n");
}

#[test]
fn named_function_is_a_valid_callable_value() {
    let source = "fn double(value: int) -> int:\n  value * 2\nfn apply(value: int, callback: fn(int) -> int) -> int:\n  callback(value)\nfn main():\n  out(\"$(apply(21, double))\")\n";
    let (code, stdout) = run(source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "42\n");
}

#[test]
fn lambda_stored_in_annotated_binding_and_invoked() {
    let source = "fn main():\n  make: fn(int) -> int = x => x + 1\n  out(\"$(make(41))\")\n";
    let (code, stdout) = run(source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "42\n");
}

#[test]
fn multiline_lambda_with_annotated_parameter_is_inferred() {
    let source = "fn main():\n  triple = fn(x: int):\n    x * 3\n  out(\"$(triple(4))\")\n";
    let (code, stdout) = run(source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "12\n");
}

#[test]
fn nested_composition_of_non_capturing_lambdas() {
    let source = "fn apply(value: int, callback: fn(int) -> int) -> int:\n  callback(value)\nfn main():\n  add_one: fn(int) -> int = x => x + 1\n  twice: fn(int) -> int = x => x * 2\n  first = apply(5, add_one)\n  out(\"$(apply(first, twice))\")\n";
    let (code, stdout) = run(source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "12\n");
}

#[test]
fn zero_parameter_block_lambda_with_void_result() {
    let source = "fn hello(callback: fn()) -> void:\n  callback()\nfn main():\n  hello(fn():\n    out(\"A\")\n    out(\"B\")\n  )\n";
    let (code, stdout) = run(source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "A\nB\n");
}

#[test]
fn captures_a_managed_string() {
    let source = "fn main():\n  name = \"hi\"\n  greet = fn():\n    out(name)\n  greet()\n";
    let (code, stdout) = run(source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "hi\n");
}

#[test]
fn returned_closure_captures_a_managed_string() {
    let source = "fn make() -> fn() -> void:\n  msg = \"hi\"\n  fn():\n    out(msg)\nfn main():\n  f = make()\n  f()\n";
    let (code, stdout) = run(source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "hi\n");
}

#[test]
fn captured_managed_value_survives_later_outer_use() {
    let source = "fn main():\n  name = \"hi\"\n  greet = fn():\n    out(\"in $name\")\n  greet()\n  out(\"out $name\")\n";
    let (code, stdout) = run(source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "in hi\nout hi\n");
}

#[test]
fn shared_managed_capture_by_two_closures() {
    let source = "fn main():\n  name = \"hi\"\n  f = fn():\n    out(name)\n  g = fn():\n    out(\"g $name\")\n  f()\n  g()\n";
    let (code, stdout) = run(source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "hi\ng hi\n");
}

#[test]
fn returned_capturing_closure_is_callable() {
    let source = "fn make_adder(a: int) -> fn(int) -> int:\n  fn(b):\n    a + b\nfn main():\n  add = make_adder(10)\n  out(\"$(add(5))\")\n";
    let (code, stdout) = run(source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "15\n");
}

#[test]
fn capturing_closure_environment_is_released() {
    let source = "extern \"C\" fn vut_rt_closure_live_count_v1() -> usize\nfn apply(value: int, callback: fn(int) -> int) -> int:\n  callback(value)\nfn main():\n  base = 10\n  out(\"$(apply(3, x => x + base))\")\n  unsafe:\n    out(\"live=$(vut_rt_closure_live_count_v1())\")\n";
    let (code, stdout) = run(source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "13\nlive=0\n");
}

#[test]
fn lambda_without_context_needs_annotation() {
    let errors = check_errors("fn main():\n  unknown = x => x + 1\n  out(\"$unknown\")\n");
    assert!(errors.contains(&"E1012".to_owned()), "{errors:?}");
}

#[test]
fn non_callable_value_cannot_be_called() {
    let errors = check_errors("fn main():\n  value = 1\n  value(2)\n");
    assert!(errors.contains(&"E1014".to_owned()), "{errors:?}");
}

#[test]
fn callable_argument_type_mismatch_is_rejected() {
    let errors = check_errors(
        "fn apply(value: int, callback: fn(str) -> int) -> int:\n  callback(value)\nfn main():\n  out(\"$(apply(3, x => x * 2))\")\n",
    );
    assert!(errors.contains(&"E1009".to_owned()), "{errors:?}");
}
