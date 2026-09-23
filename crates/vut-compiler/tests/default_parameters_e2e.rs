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
    let root = std::env::temp_dir().join(format!("vut-default-param-e2e-{nonce}-{sequence}"));
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
    let root = std::env::temp_dir().join(format!("vut-default-param-diag-{nonce}"));
    fs::create_dir(&root).expect("create root");
    let path = root.join("main.vut");
    fs::write(&path, source).expect("write source");
    let mut session = CompilerSession::new(CompilerConfig::default());
    let checked = session.check_source_file(Path::new(&path)).expect("check");
    let codes = checked
        .resolution
        .diagnostics
        .as_slice()
        .iter()
        .chain(checked.semantics.diagnostics.as_slice().iter())
        .filter_map(|diagnostic| diagnostic.code.as_deref().map(str::to_owned))
        .collect();
    fs::remove_dir_all(&root).ok();
    codes
}

#[test]
fn default_parameter_is_used_when_omitted() {
    let source = "fn add(a: int, b: int = 10) -> int:\n  a + b\n\nfn main():\n  out(add(5))\n  out(add(5, 7))\n";
    let (code, stdout) = run(source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "15\n12\n");
}

#[test]
fn named_arguments_are_order_independent() {
    let source =
        "fn sub(a: int, b: int) -> int:\n  a - b\n\nfn main():\n  out(sub(b = 2, a = 5))\n";
    let (code, stdout) = run(source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "3\n");
}

#[test]
fn defaults_combine_with_named_arguments() {
    let source = "fn total(a: int, b: int = 2, c: int = 3) -> int:\n  a + b + c\n\nfn main():\n  out(total(a = 1))\n  out(total(1, c = 5))\n  out(total(c = 9, a = 1))\n  out(total(a = 1, b = 1, c = 1))\n";
    let (code, stdout) = run(source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "6\n8\n12\n3\n");
}

#[test]
fn method_parameters_can_have_defaults() {
    let source = "data Counter:\n  value: int\n\nfn Counter.bump(amount: int = 1):\n  self.value = self.value + amount\n\nfn main():\n  counter = Counter(value: 0)\n  counter.bump()\n  counter.bump(amount = 4)\n  out(counter.value)\n";
    let (code, stdout) = run(source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "5\n");
}

#[test]
fn generic_function_defaults_and_named_arguments() {
    let source = "fn tag[T](value: T, label: str = \"x\") -> str:\n  \"$(label):$(value)\"\n\nfn pair[T](a: T, b: T) -> str:\n  \"$(a)-$(b)\"\n\nfn main():\n  out(tag(value = 9))\n  out(tag(label = \"L\", value = 2))\n  out(pair(b = 2, a = 1))\n";
    let (code, stdout) = run(source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "x:9\nL:2\n1-2\n");
}

#[test]
fn default_may_call_a_global_function() {
    let source = "fn fallback() -> int:\n  4\n\nfn clamp(value: int, limit: int = fallback()) -> int:\n  if value > limit:\n    return limit\n  value\n\nfn main():\n  out(clamp(9))\n  out(clamp(9, 2))\n";
    let (code, stdout) = run(source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "4\n2\n");
}

#[test]
fn default_referencing_a_parameter_is_rejected() {
    let codes =
        diagnostic_codes("fn f(a: int, b: int = a) -> int:\n  a + b\nfn main():\n  out(f(1))\n");
    assert!(codes.iter().any(|code| code == "E2001"), "{codes:?}");
}

#[test]
fn required_parameter_after_default_is_rejected() {
    let codes =
        diagnostic_codes("fn f(a: int = 1, b: int) -> int:\n  a + b\nfn main():\n  out(f(1, 2))\n");
    assert!(codes.iter().any(|code| code == "E6015"), "{codes:?}");
}

#[test]
fn default_on_variadic_parameter_is_rejected() {
    let codes = diagnostic_codes("fn f(a: int, ...rest: int = 1):\n  out(a)\nfn main():\n  f(1)\n");
    assert!(codes.iter().any(|code| code == "E6015"), "{codes:?}");
}

#[test]
fn default_on_extern_parameter_is_rejected() {
    let codes =
        diagnostic_codes("extern \"C\" fn native(x: i64 = 1) -> i64\n\nfn main():\n  out(0)\n");
    assert!(codes.iter().any(|code| code == "E6015"), "{codes:?}");
}

#[test]
fn missing_required_argument_is_rejected() {
    let codes =
        diagnostic_codes("fn f(a: int, b: int = 1) -> int:\n  a + b\nfn main():\n  out(f())\n");
    assert!(codes.iter().any(|code| code == "E6002"), "{codes:?}");
}

#[test]
fn too_many_arguments_is_rejected() {
    let codes = diagnostic_codes(
        "fn f(a: int, b: int = 1) -> int:\n  a + b\nfn main():\n  out(f(1, 2, 3))\n",
    );
    assert!(codes.iter().any(|code| code == "E6003"), "{codes:?}");
}

#[test]
fn default_type_mismatch_is_rejected() {
    let codes = diagnostic_codes("fn f(a: int = \"x\") -> int:\n  a\nfn main():\n  out(f())\n");
    assert!(codes.iter().any(|code| code == "E1003"), "{codes:?}");
}
