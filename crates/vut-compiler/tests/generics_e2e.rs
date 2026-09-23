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
    let root = std::env::temp_dir().join(format!("vut-generics-e2e-{nonce}-{sequence}"));
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
fn generic_function_inference_specializes_per_argument_type() {
    let source = "fn identity[T](value: T) -> T:\n  value\n\nfn first[T](a: T, b: T) -> T:\n  a\n\nfn main():\n  a = identity(123)\n  b = identity(\"hello\")\n  c = first(1, 2)\n  out(\"$a $b $c\")\n";
    let (code, stdout) = run(source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "123 hello 1\n");
}

#[test]
fn generic_data_construction_infers_and_specializes_layout() {
    let source = "data Box[T]:\n  value: T\n\ndata Pair[A, B]:\n  first: A\n  second: B\n\nfn main():\n  b = Box(value: 42)\n  out(\"v=$(b.value)\")\n  p = Pair(first: \"x\", second: Box(value: 7))\n  out(\"p=$(p.first) $(p.second.value)\")\n";
    let (code, stdout) = run(source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "v=42\np=x 7\n");
}

#[test]
fn generic_enum_construction_and_match() {
    let source = "enum Maybe[T]:\n  none\n  some(value: T)\n\nfn main():\n  m: Maybe[int] = Maybe.none\n  match m:\n    none: out(\"none\")\n    some(value): out(\"some=$value\")\n  s: Maybe[str] = Maybe.some(value: \"hi\")\n  match s:\n    none: out(\"none2\")\n    some(value): out(\"some2=$value\")\n";
    let (code, stdout) = run(source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "none\nsome2=hi\n");
}

#[test]
fn constrained_generic_function_accepts_satisfying_type() {
    let source = "interface Comparable:\n  compare(other: int) -> int\n\ndata Num:\n  n: int\n\nfn Num.compare(other: int) -> int:\n  self.n - other\n\nfn max[T: Comparable](a: T, b: T) -> T:\n  a\n\nfn main():\n  x = max(Num(n: 1), Num(n: 2))\n  out(\"n=$(x.n)\")\n";
    let (code, stdout) = run(source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "n=1\n");
}

#[test]
fn generic_function_returns_generic_data_parameterized_by_param() {
    let source = "data Box[T]:\n  value: T\n\nfn wrap[T](v: T) -> Box[T]:\n  Box(value: v)\n\nfn main():\n  b = wrap(5)\n  out(\"v=$(b.value)\")\n";
    let (code, stdout) = run(source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "v=5\n");
}

#[test]
fn generic_function_calling_generic_function_is_monomorphized() {
    // Regression: a generic callee whose type argument is the caller's type
    // parameter must be enqueued and instantiated for the concrete caller
    // specialization. It previously produced
    // `invalid typed MIR: call target N has no declaration`.
    let source = "fn inner[T](value: T) -> T:\n  value\n\nfn outer[U](value: U) -> U:\n  inner(value)\n\nfn main():\n  out(\"$(outer(42))\")\n";
    let (code, stdout) = run(source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "42\n");
}

#[test]
fn nested_generic_chain_of_three_levels_is_monomorphized() {
    let source = "fn deep[T](value: T) -> T:\n  value\n\nfn middle[U](value: U) -> U:\n  deep(value)\n\nfn outer[V](value: V) -> V:\n  middle(value)\n\nfn main():\n  out(\"$(outer(7))\")\n";
    let (code, stdout) = run(source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "7\n");
}

#[test]
fn nested_generic_with_managed_type_argument_is_monomorphized() {
    let source = "fn deep[T](value: T) -> T:\n  value\n\nfn middle[U](value: U) -> U:\n  deep(value)\n\nfn outer[V](value: V) -> V:\n  middle(value)\n\nfn main():\n  text = \"managed\"\n  out(\"$(outer(text))\")\n";
    let (code, stdout) = run(source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "managed\n");
}

#[test]
fn nested_generic_collection_substitution_is_monomorphized() {
    let source = "fn inner[T](values: list[T]) -> int:\n  values.len()\n\nfn outer[U](values: list[U]) -> int:\n  inner(values)\n\nfn main():\n  out(\"$(outer(@[1, 2, 3]))\")\n";
    let (code, stdout) = run(source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "3\n");
}

#[test]
fn nested_generic_with_multiple_parameters_is_monomorphized() {
    let source = "fn inner[A, B](a: A, b: B) -> A:\n  a\n\nfn outer[X, Y](x: X, y: Y) -> X:\n  inner(x, y)\n\nfn main():\n  x = 1\n  y = \"two\"\n  out(\"$(outer(x, y))\")\n";
    let (code, stdout) = run(source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "1\n");
}
