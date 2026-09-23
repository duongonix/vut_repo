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
    let root = std::env::temp_dir().join(format!("vut-generic-call-e2e-{nonce}-{sequence}"));
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
    let root = std::env::temp_dir().join(format!("vut-generic-call-diag-{nonce}"));
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
fn explicit_generic_call_specializes_a_function() {
    let source = "fn identity[T](value: T) -> T:\n  value\n\nfn main():\n  out(identity[int](5))\n  out(identity[str](\"hi\"))\n";
    let (code, stdout) = run(source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "5\nhi\n");
}

#[test]
fn explicit_generic_call_accepts_multiple_type_arguments() {
    let source = "fn describe[A, B](a: A, b: B) -> str:\n  \"two\"\n\nfn main():\n  out(describe[int, str](1, \"x\"))\n";
    let (code, stdout) = run(source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "two\n");
}

#[test]
fn explicit_generic_call_may_use_a_type_parameter() {
    let source = "fn identity[T](value: T) -> T:\n  value\n\nfn wrap[T](value: T) -> T:\n  identity[T](value)\n\nfn main():\n  out(wrap[int](9))\n";
    let (code, stdout) = run(source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "9\n");
}

#[test]
fn explicit_and_inferred_calls_agree() {
    let source = "fn identity[T](value: T) -> T:\n  value\n\nfn main():\n  out(identity[int](11))\n  out(identity(11))\n";
    let (code, stdout) = run(source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "11\n11\n");
}

#[test]
fn indexing_then_calling_a_function_value_works() {
    let source = "fn inc(x: int) -> int:\n  x + 1\n\nfn main():\n  parsers: list[fn(int) -> int] = @[inc]\n  out(parsers[0](4))\n  i = 0\n  out(parsers[i](4))\n";
    let (code, stdout) = run(source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "5\n5\n");
}

#[test]
fn collection_indexing_and_chained_indexing_still_work() {
    let source = "fn main():\n  values: array[int, 3] = [10, 20, 30]\n  out(values[0])\n  matrix: list[list[int]] = @[@[1, 2], @[3, 4]]\n  out(matrix[0][1])\n";
    let (code, stdout) = run(source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "10\n2\n");
}

#[test]
fn indexed_assignment_accepts_a_variable_index() {
    let source = "fn main():\n  numbers: array[int, 3] = [10, 20, 30]\n  i = 1\n  numbers[i] = 99\n  out(numbers[1])\n  matrix: list[list[int]] = @[@[1, 2], @[3, 4]]\n  matrix[0][1] = 42\n  out(matrix[0][1])\n";
    let (code, stdout) = run(source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "99\n42\n");
}

#[test]
fn generic_data_application_still_works() {
    let source =
        "data Box[T]:\n  value: T\n\nfn main():\n  box = Box[int](value: 100)\n  out(box.value)\n";
    let (code, stdout) = run(source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "100\n");
}

#[test]
fn a_non_generic_function_subscript_is_indexing() {
    // `plain` is not generic, so `[0]` is a collection access on a function,
    // which is not indexable.
    let codes =
        diagnostic_codes("fn plain(value: int) -> int:\n  value\nfn main():\n  out(plain[0])\n");
    assert!(codes.iter().any(|code| code == "E2004"), "{codes:?}");
}

#[test]
fn explicit_generic_call_checks_constraints() {
    let source = "interface Encodable:\n  to_json() -> int\ndata User:\n  age: int\nfn User.to_json() -> int:\n  self.age\nfn encode[T: Encodable](value: T) -> int:\n  value.to_json()\nfn main():\n  out(\"$(encode[User](User(age: 7)))\")\n";
    let (code, stdout) = run(source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "7\n");
}

#[test]
fn explicit_generic_call_combines_with_named_arguments_and_defaults() {
    let source = "fn label[T](value: T, prefix: str = \"v\") -> str:\n  \"$(prefix):$(value)\"\nfn main():\n  out(label[int](value = 5))\n  out(label[str](prefix = \"p\", value = \"x\"))\n";
    let (code, stdout) = run(source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "v:5\np:x\n");
}

#[test]
fn wrong_type_argument_count_is_rejected() {
    let codes = diagnostic_codes(
        "fn identity[T](value: T) -> T:\n  value\nfn main():\n  out(identity[int, str](5))\n",
    );
    assert!(codes.iter().any(|code| code == "E1003"), "{codes:?}");
}

#[test]
fn unknown_type_argument_is_rejected() {
    let codes = diagnostic_codes(
        "fn identity[T](value: T) -> T:\n  value\nfn main():\n  out(identity[Missing](5))\n",
    );
    assert!(codes.iter().any(|code| code == "E2001"), "{codes:?}");
}

#[test]
fn indexing_a_non_collection_is_rejected() {
    let codes = diagnostic_codes("fn main():\n  value = 5\n  out(value[0])\n");
    assert!(codes.iter().any(|code| code == "E2004"), "{codes:?}");
}

#[test]
fn a_value_target_with_a_type_like_identifier_is_indexed() {
    // `i` is a value, so `i[int]` is an index attempt on a non-collection, not
    // a generic application.
    let codes = diagnostic_codes("fn main():\n  i = 0\n  out(i[int])\n");
    assert!(codes.iter().any(|code| code == "E2004"), "{codes:?}");
}

#[test]
fn a_generic_target_with_a_value_identifier_is_rejected() {
    // `identity` is generic, so `[value]` is read as a type argument; the value
    // is not a valid type, so the call is rejected.
    let codes = diagnostic_codes(
        "fn identity[T](value: T) -> T:\n  value\nfn main():\n  value = 1\n  out(identity[value](2))\n",
    );
    assert!(codes.iter().any(|code| code == "E1003"), "{codes:?}");
}
