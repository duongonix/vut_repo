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
    let root = std::env::temp_dir().join(format!("vut-print-e2e-{nonce}-{sequence}"));
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
fn prints_scalars_and_strings_directly() {
    let source = "fn main():\n  out(1)\n  out(true)\n  out(false)\n  out(\"hi\")\n  print(42)\n";
    let (code, stdout) = run(source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "1\ntrue\nfalse\nhi\n42");
}

#[test]
fn prints_data_with_named_fields() {
    let source =
        "data User:\n  a: int\n  name: str\n\nfn main():\n  out(User(a: 1, name: \"bob\"))\n";
    let (code, stdout) = run(source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "User(a: 1, name: \"bob\")\n");
}

#[test]
fn prints_lists_arrays_and_nested_collections() {
    let source = "fn main():\n  out(@[1, 2, 3])\n  out(@[@[1, 2], @[3, 4]])\n";
    let (code, stdout) = run(source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "[1, 2, 3]\n[[1, 2], [3, 4]]\n");
}

#[test]
fn prints_enums_and_results() {
    let source = "enum Color:\n  red\n  green\n\nenum Status:\n  ready\n  failed(reason: str)\n\nfn main():\n  out(Color.red)\n  out(Status.failed(reason: \"boom\"))\n  good: result[int, str] = ok(7)\n  out(good)\n  bad: result[int, str] = err(\"nope\")\n  out(bad)\n";
    let (code, stdout) = run(source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "red\nfailed(\"boom\")\nok(7)\nerr(\"nope\")\n");
}

#[test]
fn prints_bytes() {
    let source = "fn main():\n  raw: list[u8] = @[104, 105]\n  out(bytes.from_list(raw))\n";
    let (code, stdout) = run(source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "[104, 105]\n");
}

#[test]
fn prints_multiple_and_zero_arguments_joined_by_spaces() {
    let source = "fn main():\n  out(1, true, \"x\")\n  out()\n  out(\"a\", \"b\")\n";
    let (code, stdout) = run(source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "1 true x\n\na b\n");
}

#[test]
fn interpolates_aggregates_and_expressions_in_templates() {
    let source = "data User:\n  name: str\n\nfn main():\n  user = User(name: \"ann\")\n  out(\"user=$user\")\n  out(\"sum=$(1 + 2)\")\n  out(\"list=$(@[1, 2])\")\n";
    let (code, stdout) = run(source);
    assert_eq!(code, Some(0), "{stdout}");
    assert_eq!(stdout, "user=User(name: \"ann\")\nsum=3\nlist=[1, 2]\n");
}

#[test]
fn printing_nested_managed_values_does_not_leak() {
    let source = "data User:\n  name: str\n  age: int\n\nfn main():\n  users = @[User(name: \"ann\", age: 1), User(name: \"bob\", age: 2)]\n  out(users)\n  for user in users:\n    out(\"user:\", user)\n";
    let (code, stdout) = run(source);
    assert_eq!(
        code,
        Some(0),
        "leak detector must report a clean exit: {stdout}"
    );
    assert_eq!(
        stdout,
        "[User(name: \"ann\", age: 1), User(name: \"bob\", age: 2)]\nuser: User(name: \"ann\", age: 1)\nuser: User(name: \"bob\", age: 2)\n"
    );
}
