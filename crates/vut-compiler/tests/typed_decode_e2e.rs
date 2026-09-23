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

fn write_source(source: &str, prefix: &str) -> (PathBuf, PathBuf) {
    let nonce = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .expect("valid clock")
        .as_nanos();
    let sequence = SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let root = std::env::temp_dir().join(format!("{prefix}-{nonce}-{sequence}"));
    fs::create_dir(&root).expect("create root");
    let path = root.join("main.vut");
    fs::write(&path, source).expect("write source");
    (root, path)
}

fn run(source: &str) -> (Option<i32>, String) {
    let (root, _path) = write_source(source, "vut-typed-decode-e2e");
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
    let (root, path) = write_source(source, "vut-typed-decode-diag");
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

const DECODER_PRELUDE: &str = "import json\nimport json at Value, Error, Decodable\n\ndata User:\n  name: str\n  age: int\n\nfn read_str(value: Value, key: str) -> result[str, Error]:\n  match value.get(key):\n    ok(field): field.as_str()\n    err(error): err(error)\n\nfn read_int(value: Value, key: str) -> result[int, Error]:\n  match value.get(key):\n    ok(field): field.as_int()\n    err(error): err(error)\n\nfn build_user(name: str, value: Value) -> result[User, Error]:\n  age_result = read_int(value, \"age\")\n  match age_result:\n    ok(age): ok(User(name: name, age: age))\n    err(error): err(error)\n\nstatic fn User.from_json(value: Value) -> result[User, Error]:\n  name_result = read_str(value, \"name\")\n  match name_result:\n    ok(name): build_user(name, value)\n    err(error): err(error)\n\n";

#[test]
fn typed_decode_uses_static_bound_call() {
    let source = format!(
        "{DECODER_PRELUDE}fn main():\n  parsed: result[User, Error] = json.decode(\"{{\\\"name\\\":\\\"Nam\\\",\\\"age\\\":20}}\")\n  match parsed:\n    ok(user): out(\"$(user.name) $(user.age)\")\n    err(error): out(\"bad\")\n"
    );
    let (code, stdout) = run(&source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "Nam 20\n");
}

#[test]
fn typed_decode_reports_missing_field() {
    let source = format!(
        "{DECODER_PRELUDE}fn main():\n  parsed: result[User, Error] = json.decode(\"{{\\\"name\\\":\\\"Nam\\\"}}\")\n  match parsed:\n    ok(user): out(\"unexpected\")\n    err(error): out(\"missing age\")\n"
    );
    let (code, stdout) = run(&source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "missing age\n");
}

#[test]
fn concrete_static_call_constructs_a_value() {
    let source = "data User:\n  name: str\n\nstatic fn User.make(text: str) -> User:\n  User(name: text)\n\nfn main():\n  user = User.make(\"x\")\n  out(\"name=$(user.name)\")\n";
    let (code, stdout) = run(source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "name=x\n");
}

#[test]
fn static_method_cannot_be_called_as_instance_method() {
    let codes = diagnostic_codes(
        "data User:\n  name: str\nstatic fn User.make(text: str) -> User:\n  User(name: text)\nfn bad(value: User) -> User:\n  value.make(\"x\")\nfn main():\n  out(\"x\")\n",
    );
    assert!(codes.iter().any(|code| code == "E2005"), "{codes:?}");
}

#[test]
fn unsatisfied_static_bound_reports_e1017() {
    let codes = diagnostic_codes(
        "interface Decodable:\n  static from_text(text: str) -> result[Self, int]\ndata Plain:\n  n: int\nfn decode[T: Decodable](text: str) -> result[T, int]:\n  T.from_text(text)\nfn main():\n  value: result[Plain, int] = decode(\"hi\")\n  match value:\n    ok(item): out(\"ok\")\n    err(code): out(\"err\")\n",
    );
    assert!(codes.iter().any(|code| code == "E1017"), "{codes:?}");
}
